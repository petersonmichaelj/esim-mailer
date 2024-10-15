use clap::Parser;

#[derive(Parser, Debug, Default, Clone)]
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

    /// Provider name
    #[arg(long, help = "The name of the eSIM provider")]
    pub provider: String,

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

    /// Location
    #[arg(
        short,
        long,
        help = "The location for the eSIM (e.g., 'Egypt', 'Middle East')"
    )]
    pub location: String,
}
