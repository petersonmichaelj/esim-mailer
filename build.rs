use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
use dotenv::dotenv;
use rand::Rng;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

// Import the winres crate
#[cfg(target_os = "windows")]
use winres::WindowsResource;

#[cfg(target_os = "windows")]
fn embed_icon() {
    let mut res = WindowsResource::new();
    res.set_icon("appIcon.ico");
    res.compile().unwrap();
}

fn main() {
    dotenv().ok(); // Load .env file if it exists

    // Embed the icon on Windows
    #[cfg(target_os = "windows")]
    embed_icon();

    let gmail_secret = env::var("GMAIL_CLIENT_SECRET").ok();
    let outlook_secret = env::var("OUTLOOK_CLIENT_SECRET").ok();

    // Add these lines to get the client IDs
    let gmail_client_id = env::var("GMAIL_CLIENT_ID").expect("GMAIL_CLIENT_ID must be set");
    let outlook_client_id = env::var("OUTLOOK_CLIENT_ID").expect("OUTLOOK_CLIENT_ID must be set");

    let mut key = [0u8; 32];
    rand::thread_rng().fill(&mut key);
    let key = Key::<Aes256Gcm>::from_slice(&key);
    let cipher = Aes256Gcm::new(&key);

    let mut nonce = [0u8; 12];
    rand::thread_rng().fill(&mut nonce);
    let nonce = Nonce::from_slice(&nonce);

    let out_dir = Path::new("src").join("embedded");
    std::fs::create_dir_all(&out_dir).unwrap();

    let mut file = File::create(out_dir.join("encrypted_gmail_secret.bin")).unwrap();
    if let Some(gmail_secret) = gmail_secret {
        let encrypted_gmail = cipher.encrypt(nonce, gmail_secret.as_bytes()).unwrap();
        file.write_all(&encrypted_gmail).unwrap();
    }

    let mut file = File::create(out_dir.join("encrypted_outlook_secret.bin")).unwrap();
    if let Some(outlook_secret) = outlook_secret {
        let encrypted_outlook = cipher.encrypt(nonce, outlook_secret.as_bytes()).unwrap();
        file.write_all(&encrypted_outlook).unwrap();
    }

    let mut file = File::create(out_dir.join("secret.key")).unwrap();
    file.write_all(key.as_slice()).unwrap();

    let mut file = File::create(out_dir.join("nonce.bin")).unwrap();
    file.write_all(nonce.as_slice()).unwrap();

    // Write client IDs to files
    let mut file = File::create(out_dir.join("gmail_client_id.txt")).unwrap();
    file.write_all(gmail_client_id.as_bytes()).unwrap();

    let mut file = File::create(out_dir.join("outlook_client_id.txt")).unwrap();
    file.write_all(outlook_client_id.as_bytes()).unwrap();

    println!("cargo:rerun-if-env-changed=GMAIL_CLIENT_SECRET");
    println!("cargo:rerun-if-env-changed=OUTLOOK_CLIENT_SECRET");
    println!("cargo:rerun-if-env-changed=GMAIL_CLIENT_ID");
    println!("cargo:rerun-if-env-changed=OUTLOOK_CLIENT_ID");
    println!("cargo:rerun-if-changed=.env");
}
