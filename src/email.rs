use crate::oauth::determine_provider;
use crate::Args;
use base64::{self, Engine};
use lettre::message::header;
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::{Message, SmtpTransport, Transport};
use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;

pub struct EmailTemplate {
    subject_template: &'static str,
    body_template: &'static str,
}

impl EmailTemplate {
    pub fn new() -> Self {
        Self {
            subject_template: "[{{provider}}] {{location}} eSIM",
            body_template: include_str!("../templates/email_template.html"),
        }
    }

    pub fn subject(&self, args: &Args, count: usize) -> String {
        let subject = self
            .subject_template
            .replace("{{provider}}", &args.provider)
            .replace("{{location}}", &args.location);
        format!("{} - {}", subject, count)
    }

    pub fn body(&self, args: &Args) -> String {
        self.body_template
            .replace("{{provider}}", &args.provider)
            .replace("{{name}}", &args.name)
            .replace("{{data_amount}}", &args.data_amount)
            .replace("{{time_period}}", &args.time_period)
            .replace("{{location}}", &args.location)
    }
}

pub fn send_email(args: &Args, token: String, image_path: &Path, count: usize) -> io::Result<()> {
    let email_from = &args.email_from;
    let email_to = &args.email_to;

    // Get template content
    let template = EmailTemplate::new();

    // Read image file
    let image_data = fs::read(image_path)?;

    // Get subject and body content
    let subject = template.subject(args, count);
    let body_content = template.body(args);
    let body = format!(
        "<html><body>{}<br><img src='data:image/png;base64,{}'/></body></html>",
        body_content.replace("\n", "<br>"), // Replace newlines with <br> tags here
        base64::engine::general_purpose::STANDARD.encode(&image_data)
    );

    // Create email
    let mut email_builder = Message::builder()
        .from(
            email_from
                .parse()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?,
        )
        .to(email_to
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?)
        .subject(subject)
        .header(header::ContentType::TEXT_HTML);

    // Add BCC if provided and not empty
    if let Some(bcc) = &args.bcc {
        if !bcc.is_empty() {
            email_builder = email_builder.bcc(
                bcc.parse()
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?,
            );
        }
    }

    let email = email_builder.body(body).unwrap();

    // Configure SMTP client with TLS
    let provider = determine_provider(email_from);
    let mailer = configure_mailer(provider, email_from, token)?;

    // Send the email
    match mailer.send(&email) {
        Ok(_) => println!("Email sent successfully!"),
        Err(e) => {
            eprintln!("Could not send email: {:?}", e);
            if let Some(source) = e.source() {
                eprintln!("Error source: {:?}", source);
            }
        }
    }

    Ok(())
}

fn configure_mailer(
    provider: &str,
    email_address: &str,
    token: String,
) -> io::Result<SmtpTransport> {
    match provider {
        "gmail" => Ok(SmtpTransport::relay("smtp.gmail.com")
            .unwrap()
            .credentials(Credentials::new(email_address.to_string(), token))
            .authentication(vec![Mechanism::Xoauth2])
            .port(587)
            .tls(lettre::transport::smtp::client::Tls::Required(
                lettre::transport::smtp::client::TlsParameters::new("smtp.gmail.com".to_string())
                    .unwrap(),
            ))
            .build()),
        "outlook" => Ok(SmtpTransport::relay("smtp-mail.outlook.com")
            .unwrap()
            .credentials(Credentials::new(email_address.to_string(), token))
            .authentication(vec![Mechanism::Xoauth2])
            .port(587)
            .tls(lettre::transport::smtp::client::Tls::Required(
                lettre::transport::smtp::client::TlsParameters::new(
                    "smtp-mail.outlook.com".to_string(),
                )
                .unwrap(),
            ))
            .build()),
        _ => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Unsupported email provider",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_template_subject() {
        let template = EmailTemplate::new();
        let args = Args {
            email_from: "sender@example.com".to_string(),
            email_to: "recipient@example.com".to_string(),
            bcc: None,
            provider: "TestProvider".to_string(),
            name: "John".to_string(),
            data_amount: "5GB".to_string(),
            time_period: "30 days".to_string(),
            location: "Egypt".to_string(),
        };
        let result = template.subject(&args, 1);
        assert_eq!(result, "[TestProvider] Egypt eSIM - 1");
    }

    #[test]
    fn test_email_template_body() {
        let template = EmailTemplate::new();
        let args = Args {
            email_from: "sender@example.com".to_string(),
            email_to: "recipient@example.com".to_string(),
            bcc: None,
            provider: "TestProvider".to_string(),
            name: "John".to_string(),
            data_amount: "5GB".to_string(),
            time_period: "30 days".to_string(),
            location: "Egypt".to_string(),
        };
        let result = template.body(&args);
        assert!(result.contains("John"));
        assert!(result.contains("TestProvider"));
        assert!(result.contains("5GB"));
        assert!(result.contains("30 days"));
        assert!(result.contains("Egypt"));
    }

    #[test]
    fn test_configure_mailer_gmail() {
        let result = configure_mailer("gmail", "test@gmail.com", "token".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_configure_mailer_outlook() {
        let result = configure_mailer("outlook", "test@outlook.com", "token".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_configure_mailer_unsupported() {
        let result = configure_mailer("unsupported", "test@unsupported.com", "token".to_string());
        assert!(result.is_err());
    }
}
