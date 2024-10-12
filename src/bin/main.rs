use clap::Parser;
use esim_mailer::{get_or_refresh_token, send_email, Args};
use rfd::FileDialog;
use std::io;

fn main() -> io::Result<()> {
    let args = Args::parse();

    // Open file dialog to select images
    let image_paths = FileDialog::new()
        .add_filter("Image Files", &["png", "jpg", "jpeg", "gif"])
        .pick_files();

    if let Some(paths) = image_paths {
        // Determine the provider from the email address
        let provider = esim_mailer::oauth::determine_provider(&args.email_from);

        // Perform OAuth authentication or use cached token
        let token = get_or_refresh_token(provider, &args.email_from)?;

        // Send an email for each selected image
        for (index, path) in paths.iter().enumerate() {
            send_email(&args, token.clone(), path, index + 1)?;
        }
    }

    Ok(())
}
