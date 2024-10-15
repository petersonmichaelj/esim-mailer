use crate::args::Args;
use crate::oauth::determine_provider;
use crate::templates::load_templates;
use base64::{self, Engine};
use lettre::message::header;
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::{Message, SmtpTransport, Transport};
use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;

pub fn send_email(args: &Args, token: String, image_path: &Path, count: usize) -> io::Result<()> {
    let email_from = &args.email_from;
    let email_to = &args.email_to;
    let name = &args.name;
    let data_amount = &args.data_amount;
    let time_period = &args.time_period;

    // Load templates
    let templates = load_templates();

    // Get template content
    let template = match templates.get("shared") {
        Some(content) => content,
        None => {
            eprintln!("Shared template not found");
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Shared template not found",
            ));
        }
    };

    // Read image file
    let image_data = fs::read(image_path)?;

    // Replace placeholders in the template subject and body
    let subject = format!(
        "{} - {} - {}",
        replace_placeholders(
            template.subject,
            &args.provider,
            name,
            data_amount,
            time_period,
            &args.location
        ),
        args.location,
        count
    );
    let body_content = replace_placeholders(
        template.body,
        &args.provider,
        name,
        data_amount,
        time_period,
        &args.location,
    );
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

pub fn replace_placeholders(
    content: &str,
    provider: &str,
    name: &str,
    data_amount: &str,
    time_period: &str,
    location: &str,
) -> String {
    content
        .replace("{{provider}}", provider)
        .replace("{{name}}", name)
        .replace("{{data_amount}}", data_amount)
        .replace("{{time_period}}", time_period)
        .replace("{{location}}", location)
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
