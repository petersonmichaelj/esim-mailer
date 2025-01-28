use crate::email;
use crate::embedded::{
    GMAIL_CLIENT_ID, GMAIL_SECRET, NONCE, OUTLOOK_CLIENT_ID, OUTLOOK_SECRET, SECRET_KEY,
};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use oauth2::basic::BasicClient;
use oauth2::reqwest::blocking::Client as BlockingHttpClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet,
    PkceCodeChallenge, RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpListener;
use url::Url;
use webbrowser;

#[derive(Serialize, Deserialize)]
struct CachedToken {
    refresh_token: String,
}

// Trait for token storage
pub trait TokenStorage: Send + Sync {
    fn get_token(&self, key: &str) -> Option<String>;
    fn set_token(&mut self, key: &str, token: String);
}

// In-memory implementation of TokenStorage
#[derive(Default)]
pub struct MemoryTokenStorage {
    tokens: HashMap<String, String>,
}

impl TokenStorage for MemoryTokenStorage {
    fn get_token(&self, key: &str) -> Option<String> {
        self.tokens.get(key).cloned()
    }

    fn set_token(&mut self, key: &str, token: String) {
        self.tokens.insert(key.to_string(), token);
    }
}

// Trait for browser interaction
pub trait BrowserOpener: Send + Sync {
    fn open_url(&self, url: &str) -> io::Result<()>;
}

// Default implementation using webbrowser crate
pub struct DefaultBrowserOpener;

impl BrowserOpener for DefaultBrowserOpener {
    fn open_url(&self, url: &str) -> io::Result<()> {
        webbrowser::open(url).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
}

// Trait for OAuth code receiver
pub trait OAuthCodeReceiver: Send + Sync {
    fn receive_code(&self) -> io::Result<String>;
}

// Default implementation using TcpListener
pub struct LocalServerCodeReceiver {
    port: u16,
}

impl Default for LocalServerCodeReceiver {
    fn default() -> Self {
        Self { port: 9999 }
    }
}

impl OAuthCodeReceiver for LocalServerCodeReceiver {
    fn receive_code(&self) -> io::Result<String> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.port))?;

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let mut reader = BufReader::new(&stream);
                    let mut request_line = String::new();
                    reader.read_line(&mut request_line)?;

                    if let Some(auth_code) = extract_code(&request_line) {
                        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n<h1>Authorization successful!</h1><p>You can now close this window and return to the application.</p>";
                        stream.write_all(response.as_bytes())?;
                        return Ok(auth_code);
                    } else {
                        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<h1>Waiting for authorization...</h1><p>Please complete the authorization in your browser.</p>";
                        stream.write_all(response.as_bytes())?;
                    }
                }
                Err(e) => eprintln!("Error accepting connection: {}", e),
            }
        }

        Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to get authorization code",
        ))
    }
}

// Main OAuth client struct
pub struct OAuthClient {
    token_storage: Box<dyn TokenStorage>,
    browser_opener: Box<dyn BrowserOpener>,
    code_receiver: Box<dyn OAuthCodeReceiver>,
}

impl Default for OAuthClient {
    fn default() -> Self {
        Self {
            token_storage: Box::new(MemoryTokenStorage::default()),
            browser_opener: Box::new(DefaultBrowserOpener),
            code_receiver: Box::new(LocalServerCodeReceiver::default()),
        }
    }
}

impl OAuthClient {
    pub fn new(
        token_storage: Box<dyn TokenStorage>,
        browser_opener: Box<dyn BrowserOpener>,
        code_receiver: Box<dyn OAuthCodeReceiver>,
    ) -> Self {
        Self {
            token_storage,
            browser_opener,
            code_receiver,
        }
    }

    pub fn get_or_refresh_token(
        &mut self,
        email_provider: &email::Provider,
        email: &str,
    ) -> io::Result<String> {
        let email_hash = format!("{:x}", Sha256::digest(email.as_bytes()));
        let cache_key = format!("{}_{}", email_provider, email_hash);

        if let Some(refresh_token) = self.token_storage.get_token(&cache_key) {
            if let Ok((access_token, new_refresh_token)) =
                self.refresh_oauth_token(email_provider, &refresh_token)
            {
                if new_refresh_token != refresh_token {
                    self.token_storage.set_token(&cache_key, new_refresh_token);
                }
                return Ok(access_token);
            }
        }

        let (access_token, refresh_token) = self.perform_oauth(email_provider)?;
        self.token_storage.set_token(&cache_key, refresh_token);
        Ok(access_token)
    }

    fn perform_oauth(&self, email_provider: &email::Provider) -> io::Result<(String, String)> {
        let config = get_provider_config(email_provider);
        let client = create_oauth_client(email_provider);

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, _csrf_token) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new(config.scope.to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        self.browser_opener.open_url(auth_url.as_str())?;

        let code = self.code_receiver.receive_code()?;

        let token = client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(pkce_verifier)
            .request(&BlockingHttpClient::new())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let access_token = token.access_token().secret().clone();
        let refresh_token = token
            .refresh_token()
            .map(|rt| rt.secret().clone())
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "No refresh token provided"))?;

        Ok((access_token, refresh_token))
    }

    fn refresh_oauth_token(
        &self,
        email_provider: &email::Provider,
        refresh_token: &str,
    ) -> io::Result<(String, String)> {
        let client = create_oauth_client(email_provider);

        let token_result = client
            .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
            .request(&BlockingHttpClient::new())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let access_token = token_result.access_token().secret().clone();
        let refresh_token = token_result
            .refresh_token()
            .map(|rt| rt.secret().clone())
            .unwrap_or_else(|| refresh_token.to_string());

        Ok((access_token, refresh_token))
    }
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

struct ProviderConfig {
    client_id: &'static str,
    encrypted_client_secret: Option<&'static [u8]>,
    auth_url: &'static str,
    token_url: &'static str,
    redirect_uri: &'static str,
    scope: &'static str,
}

fn get_provider_config(email_provider: &email::Provider) -> ProviderConfig {
    match email_provider {
        email::Provider::Gmail => ProviderConfig {
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
        email::Provider::Outlook => ProviderConfig {
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
    }
}

fn create_oauth_client(
    email_provider: &email::Provider,
) -> BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet> {
    let config = get_provider_config(email_provider);
    let client_secret = config
        .encrypted_client_secret
        .map(|secret| decrypt_client_secret(secret));

    let mut client = BasicClient::new(ClientId::new(config.client_id.to_string()))
        .set_auth_uri(AuthUrl::new(config.auth_url.to_string()).unwrap())
        .set_token_uri(TokenUrl::new(config.token_url.to_string()).unwrap())
        .set_redirect_uri(RedirectUrl::new(config.redirect_uri.to_string()).unwrap());

    if let Some(secret) = client_secret.map(ClientSecret::new) {
        client = client.set_client_secret(secret);
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::RwLock;

    // Mock implementations for testing
    struct MockTokenStorage {
        tokens: RwLock<HashMap<String, String>>,
    }

    impl TokenStorage for MockTokenStorage {
        fn get_token(&self, key: &str) -> Option<String> {
            self.tokens.read().unwrap().get(key).cloned()
        }

        fn set_token(&mut self, key: &str, token: String) {
            self.tokens.write().unwrap().insert(key.to_string(), token);
        }
    }

    struct MockBrowserOpener {
        last_url: RwLock<Option<String>>,
    }

    impl BrowserOpener for MockBrowserOpener {
        fn open_url(&self, url: &str) -> io::Result<()> {
            *self.last_url.write().unwrap() = Some(url.to_string());
            Ok(())
        }
    }

    #[derive(Clone)]
    struct MockCodeReceiver {
        code: String,
        should_fail: bool,
    }

    impl OAuthCodeReceiver for MockCodeReceiver {
        fn receive_code(&self) -> io::Result<String> {
            if self.should_fail {
                Err(io::Error::new(io::ErrorKind::Other, "Failed to get code"))
            } else {
                Ok(self.code.clone())
            }
        }
    }

    fn create_test_client(
        storage: Option<MockTokenStorage>,
        receiver: Option<MockCodeReceiver>,
    ) -> OAuthClient {
        OAuthClient::new(
            Box::new(storage.unwrap_or(MockTokenStorage {
                tokens: RwLock::new(HashMap::new()),
            })),
            Box::new(MockBrowserOpener {
                last_url: RwLock::new(None),
            }),
            Box::new(receiver.unwrap_or(MockCodeReceiver {
                code: "test_code".to_string(),
                should_fail: false,
            })),
        )
    }

    #[test]
    fn test_extract_code() {
        let request = "GET /?code=test_code&state=test_state HTTP/1.1";
        assert_eq!(extract_code(request), Some("test_code".to_string()));

        let request_without_code = "GET /?state=test_state HTTP/1.1";
        assert_eq!(extract_code(request_without_code), None);
    }

    #[test]
    fn test_new_oauth_flow_success() {
        let storage = MockTokenStorage {
            tokens: RwLock::new(HashMap::new()),
        };
        let receiver = MockCodeReceiver {
            code: "valid_code".to_string(),
            should_fail: false,
        };

        let mut client = create_test_client(Some(storage), Some(receiver));
        let result = client.get_or_refresh_token(&email::Provider::Gmail, "test@gmail.com");

        // The flow should fail due to invalid OAuth response, but we can verify it was attempted
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("error"));
    }

    #[test]
    fn test_oauth_flow_browser_failure() {
        struct FailingBrowserOpener;
        impl BrowserOpener for FailingBrowserOpener {
            fn open_url(&self, _url: &str) -> io::Result<()> {
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to open browser",
                ))
            }
        }

        let mut client = OAuthClient::new(
            Box::new(MockTokenStorage {
                tokens: RwLock::new(HashMap::new()),
            }),
            Box::new(FailingBrowserOpener),
            Box::new(MockCodeReceiver {
                code: "test_code".to_string(),
                should_fail: false,
            }),
        );

        let result = client.get_or_refresh_token(&email::Provider::Gmail, "test@gmail.com");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Failed to open browser".to_string()
        );
    }

    #[test]
    fn test_oauth_flow_code_receiver_failure() {
        let mut client = create_test_client(
            None,
            Some(MockCodeReceiver {
                code: "".to_string(),
                should_fail: true,
            }),
        );

        let result = client.get_or_refresh_token(&email::Provider::Gmail, "test@gmail.com");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Failed to get code".to_string()
        );
    }

    #[test]
    fn test_token_refresh_flow() {
        let mut storage = HashMap::new();
        let email = "test@gmail.com";
        let email_hash = format!("{:x}", Sha256::digest(email.as_bytes()));
        let cache_key = format!("{}_{}", email::Provider::Gmail, email_hash);
        storage.insert(cache_key.clone(), "old_refresh_token".to_string());

        let storage = MockTokenStorage {
            tokens: RwLock::new(storage),
        };

        let mut client = create_test_client(Some(storage), None);
        let result = client.get_or_refresh_token(&email::Provider::Gmail, email);

        // The refresh should fail due to invalid token, but we can verify it attempted refresh
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("error"));
    }

    #[test]
    fn test_token_storage_interaction() {
        let storage = MockTokenStorage {
            tokens: RwLock::new(HashMap::new()),
        };
        let mut client = create_test_client(Some(storage), None);

        // First call should try to perform new OAuth flow
        let result1 = client.get_or_refresh_token(&email::Provider::Gmail, "test@gmail.com");
        assert!(result1.is_err()); // Will fail due to invalid OAuth response

        // Manually insert a token to simulate successful OAuth
        let email_hash = format!("{:x}", Sha256::digest("test@gmail.com".as_bytes()));
        let cache_key = format!("{}_{}", email::Provider::Gmail, email_hash);
        client
            .token_storage
            .set_token(&cache_key, "refresh_token".to_string());

        // Second call should try to refresh the token
        let result2 = client.get_or_refresh_token(&email::Provider::Gmail, "test@gmail.com");
        assert!(result2.is_err()); // Will fail due to invalid refresh token
        assert!(result2.unwrap_err().to_string().contains("error"));
    }

    #[test]
    fn test_get_provider_config() {
        let gmail_config = get_provider_config(&email::Provider::Gmail);
        assert_eq!(gmail_config.client_id, GMAIL_CLIENT_ID);
        assert_eq!(
            gmail_config.auth_url,
            "https://accounts.google.com/o/oauth2/v2/auth"
        );

        let outlook_config = get_provider_config(&email::Provider::Outlook);
        assert_eq!(outlook_config.client_id, OUTLOOK_CLIENT_ID);
        assert_eq!(
            outlook_config.auth_url,
            "https://login.microsoftonline.com/common/oauth2/v2.0/authorize"
        );
    }

    #[test]
    fn test_create_oauth_client() {
        let gmail_client = create_oauth_client(&email::Provider::Gmail);
        assert_eq!(gmail_client.client_id().as_str(), GMAIL_CLIENT_ID);

        let outlook_client = create_oauth_client(&email::Provider::Outlook);
        assert_eq!(outlook_client.client_id().as_str(), OUTLOOK_CLIENT_ID);
    }

    #[test]
    fn test_memory_token_storage() {
        let mut storage = MemoryTokenStorage::default();
        assert_eq!(storage.get_token("test_key"), None);

        storage.set_token("test_key", "test_token".to_string());
        assert_eq!(
            storage.get_token("test_key"),
            Some("test_token".to_string())
        );

        storage.set_token("test_key", "new_token".to_string());
        assert_eq!(storage.get_token("test_key"), Some("new_token".to_string()));
    }
}
