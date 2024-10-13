use clap::{Parser, ValueEnum};

#[derive(Parser, Debug, Default)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Email address of the sender
    #[arg(
        short = 'f',
        long,
        help = "The email address to send the eSIM activation details from"
    )]
    pub email_from: String,

    /// Email address of the recipient
    #[arg(
        short = 't',
        long,
        help = "The email address to send the eSIM activation details to"
    )]
    pub email_to: String,

    /// BCC email address (optional)
    #[arg(long, help = "The email address to BCC (optional)")]
    pub bcc: Option<String>,

    /// Template to use for the email
    #[arg(
        long,
        value_enum,
        help = "The template to use for the email content. Valid options: nomad, test"
    )]
    pub template: Template,

    /// Customer name
    #[arg(short, long, help = "The name used to sign the email")]
    pub name: String,

    /// Data amount
    #[arg(
        short,
        long,
        help = "The amount of data included in the eSIM plan (e.g., '5GB')"
    )]
    pub data_amount: String,

    /// Time period
    #[arg(
        short = 'p',
        long,
        help = "The validity period of the eSIM plan (e.g., '30 days')"
    )]
    pub time_period: String,
}

#[derive(Debug, Clone, ValueEnum, Default, PartialEq)]
pub enum Template {
    #[default]
    Nomad,
    Test,
}
