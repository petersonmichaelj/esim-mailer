use crate::embedded::{
    GMAIL_CLIENT_ID, GMAIL_SECRET, NONCE, OUTLOOK_CLIENT_ID, OUTLOOK_SECRET, SECRET_KEY,
};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use oauth2::basic::BasicClient;
use oauth2::reqwest::http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
    RefreshToken, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpListener;
use url::Url;
use webbrowser;

#[derive(Serialize, Deserialize)]
struct CachedToken {
    refresh_token: String,
}

pub fn determine_provider(email: &str) -> &'static str {
    if email.ends_with("@gmail.com") {
        "gmail"
    } else if email.ends_with("@outlook.com") || email.ends_with("@hotmail.com") {
        "outlook"
    } else {
        panic!("Unsupported email provider")
    }
}

pub fn get_or_refresh_token(provider: &str, email: &str) -> io::Result<String> {
    let email_hash = format!("{:x}", Sha256::digest(email.as_bytes()));
    let cache_file = format!("{}_{}_token_cache.json", provider, email_hash);

    if let Ok(cached_data) = fs::read_to_string(&cache_file) {
        if let Ok(cached_token) = serde_json::from_str::<CachedToken>(&cached_data) {
            // Always try to refresh the token
            if let Ok((access_token, new_refresh_token)) =
                refresh_oauth_token(provider, &cached_token.refresh_token)
            {
                // Update the cached refresh token if it has changed
                if new_refresh_token != cached_token.refresh_token {
                    let new_cached_token = CachedToken {
                        refresh_token: new_refresh_token,
                    };
                    let new_cached_data = serde_json::to_string(&new_cached_token)?;
                    fs::write(&cache_file, new_cached_data)?;
                }
                return Ok(access_token);
            }
        }
    }

    // If we couldn't refresh, perform a new OAuth flow
    let (access_token, refresh_token) = perform_oauth(provider)?;

    // Cache the new refresh token
    let cached_token = CachedToken { refresh_token };
    let cached_data = serde_json::to_string(&cached_token)?;
    fs::write(&cache_file, cached_data)?;

    Ok(access_token)
}

pub fn perform_oauth(provider: &str) -> io::Result<(String, String)> {
    let config = get_provider_config(provider);
    let client = create_oauth_client(provider);

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(config.scope.to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    // Start a local server to listen for the callback
    let listener = TcpListener::bind("127.0.0.1:9999").unwrap();

    if webbrowser::open(&auth_url.to_string()).is_err() {
        println!(
            "Failed to open the browser. Please open this URL manually: {}",
            auth_url
        );
    }

    let mut code = String::new();
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut reader = BufReader::new(&stream);
                let mut request_line = String::new();
                reader.read_line(&mut request_line)?;

                if let Some(auth_code) = extract_code(&request_line) {
                    code = auth_code;
                    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n<h1>Authorization successful!</h1><p>You can now close this window and return to the application.</p>";
                    stream.write_all(response.as_bytes())?;
                    break;
                } else {
                    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<h1>Waiting for authorization...</h1><p>Please complete the authorization in your browser.</p>";
                    stream.write_all(response.as_bytes())?;
                }
            }
            Err(e) => eprintln!("Error accepting connection: {}", e),
        }
    }

    if code.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to get authorization code",
        ));
    }

    let token = client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_verifier)
        .request(http_client)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let access_token = token.access_token().secret().clone();
    let refresh_token = token
        .refresh_token()
        .map(|rt| rt.secret().clone())
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "No refresh token provided"))?;

    Ok((access_token, refresh_token))
}

pub fn extract_code(request: &str) -> Option<String> {
    request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|path| Url::parse(&format!("http://localhost{}", path)).ok())
        .and_then(|url| {
            url.query_pairs()
                .find(|(key, _)| key == "code")
                .map(|(_, value)| value.to_string())
        })
}

fn refresh_oauth_token(provider: &str, refresh_token: &str) -> io::Result<(String, String)> {
    let client = create_oauth_client(provider);

    let token_result = client
        .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
        .request(http_client)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let access_token = token_result.access_token().secret().clone();
    let refresh_token = token_result
        .refresh_token()
        .map(|rt| rt.secret().clone())
        .unwrap_or_else(|| refresh_token.to_string());

    Ok((access_token, refresh_token))
}

struct ProviderConfig {
    client_id: &'static str,
    encrypted_client_secret: Option<&'static [u8]>,
    auth_url: &'static str,
    token_url: &'static str,
    redirect_uri: &'static str,
    scope: &'static str,
}

fn get_provider_config(provider: &str) -> ProviderConfig {
    match provider {
        "gmail" => ProviderConfig {
            client_id: GMAIL_CLIENT_ID,
            encrypted_client_secret: if GMAIL_SECRET.is_empty() {
                None
            } else {
                Some(GMAIL_SECRET)
            },
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth",
            token_url: "https://oauth2.googleapis.com/token",
            redirect_uri: "http://localhost:9999",
            scope: "https://mail.google.com/",
        },
        "outlook" => ProviderConfig {
            client_id: OUTLOOK_CLIENT_ID,
            encrypted_client_secret: if OUTLOOK_SECRET.is_empty() {
                None
            } else {
                Some(OUTLOOK_SECRET)
            },
            auth_url: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize",
            token_url: "https://login.microsoftonline.com/common/oauth2/v2.0/token",
            redirect_uri: "http://localhost:9999",
            scope: "https://outlook.office.com/SMTP.Send offline_access",
        },
        _ => panic!("Unsupported email provider"),
    }
}

fn create_oauth_client(provider: &str) -> BasicClient {
    let config = get_provider_config(provider);
    let client_secret = config
        .encrypted_client_secret
        .map(|secret| decrypt_client_secret(secret));

    let mut client = BasicClient::new(
        ClientId::new(config.client_id.to_string()),
        client_secret.map(ClientSecret::new),
        AuthUrl::new(config.auth_url.to_string()).unwrap(),
        Some(TokenUrl::new(config.token_url.to_string()).unwrap()),
    );

    client = client.set_redirect_uri(RedirectUrl::new(config.redirect_uri.to_string()).unwrap());

    client
}

fn decrypt_client_secret(encrypted_secret: &[u8]) -> String {
    let key = Key::<Aes256Gcm>::from_slice(SECRET_KEY);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(NONCE);

    let plaintext = cipher
        .decrypt(nonce, encrypted_secret.as_ref())
        .expect("decryption failure!");

    String::from_utf8(plaintext).expect("invalid utf8")
}
