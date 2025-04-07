pub const GMAIL_SECRET: &[u8] = include_bytes!("embedded/encrypted_gmail_secret.bin");
pub const SECRET_KEY: &[u8] = include_bytes!("embedded/secret.key");
pub const NONCE: &[u8] = include_bytes!("embedded/nonce.bin");
pub const GMAIL_CLIENT_ID: &str = include_str!("embedded/gmail_client_id.txt");
pub const OUTLOOK_CLIENT_ID: &str = include_str!("embedded/outlook_client_id.txt");
