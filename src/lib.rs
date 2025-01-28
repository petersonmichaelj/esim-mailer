pub mod args;
pub mod email;
mod embedded;
pub mod gui;
pub mod oauth;

// Re-export commonly used items
pub use args::Args;
pub use email::send_email;
pub use oauth::OAuthClient;
