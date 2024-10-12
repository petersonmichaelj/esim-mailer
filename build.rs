use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
use dotenv::dotenv;
use rand::Rng;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    dotenv().ok(); // Load .env file if it exists

    let gmail_secret = env::var("GMAIL_CLIENT_SECRET").ok();
    let outlook_secret = env::var("OUTLOOK_CLIENT_SECRET").ok();

    let mut key = [0u8; 32];
    rand::thread_rng().fill(&mut key);
    let key = Key::<Aes256Gcm>::from_slice(&key);
    let cipher = Aes256Gcm::new(&key);

    let mut nonce = [0u8; 12];
    rand::thread_rng().fill(&mut nonce);
    let nonce = Nonce::from_slice(&nonce);

    let out_dir = Path::new("src").join("embedded");
    std::fs::create_dir_all(&out_dir).unwrap();

    if let Some(gmail_secret) = gmail_secret {
        let encrypted_gmail = cipher.encrypt(nonce, gmail_secret.as_bytes()).unwrap();
        let mut file = File::create(out_dir.join("encrypted_gmail_secret.bin")).unwrap();
        file.write_all(&encrypted_gmail).unwrap();
    }

    if let Some(outlook_secret) = outlook_secret {
        let encrypted_outlook = cipher.encrypt(nonce, outlook_secret.as_bytes()).unwrap();
        let mut file = File::create(out_dir.join("encrypted_outlook_secret.bin")).unwrap();
        file.write_all(&encrypted_outlook).unwrap();
    }

    let mut file = File::create(out_dir.join("secret.key")).unwrap();
    file.write_all(key.as_slice()).unwrap();

    let mut file = File::create(out_dir.join("nonce.bin")).unwrap();
    file.write_all(nonce.as_slice()).unwrap();

    println!("cargo:rerun-if-env-changed=GMAIL_CLIENT_SECRET");
    println!("cargo:rerun-if-env-changed=OUTLOOK_CLIENT_SECRET");
    println!("cargo:rerun-if-changed=.env");
}
