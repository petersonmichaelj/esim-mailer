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
    let template_name = match args.template {
        crate::args::Template::Nomad => "nomad",
        crate::args::Template::Test => "test",
    };
    let name = &args.name;
    let data_amount = &args.data_amount;
    let time_period = &args.time_period;

    // Load templates
    let templates = load_templates();

    // Get template content
    let template = match templates.get(template_name) {
        Some(content) => content,
        None => {
            eprintln!("Template '{}' not found", template_name);
            eprintln!("Available templates: {:?}", templates.keys());
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Template not found",
            ));
        }
    };

    // Read image file
    let image_data = fs::read(image_path)?;

    // Replace placeholders in the template subject and body
    let subject = format!(
        "{} - {}",
        replace_placeholders(template.subject, name, data_amount, time_period),
        count
    );
    let body = format!(
        "{}<br><img src='data:image/png;base64,{}'/>",
        replace_placeholders(template.body, name, data_amount, time_period),
        base64::engine::general_purpose::STANDARD.encode(&image_data)
    );

    // Create email
    let mut email_builder = Message::builder()
        .from(email_from.parse().unwrap())
        .to(email_to.parse().unwrap())
        .subject(subject)
        .header(header::ContentType::TEXT_HTML);

    // Add BCC if provided
    if let Some(bcc) = &args.bcc {
        email_builder = email_builder.bcc(bcc.parse().unwrap());
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

fn replace_placeholders(content: &str, name: &str, data_amount: &str, time_period: &str) -> String {
    content
        .replace("{{name}}", name)
        .replace("{{data_amount}}", data_amount)
        .replace("{{time_period}}", time_period)
        .replace("\n", "<br>") // Add this line to replace newlines with HTML line breaks
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
