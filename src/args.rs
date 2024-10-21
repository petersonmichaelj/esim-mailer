#[derive(Debug, Default, Clone, PartialEq)]
pub struct Args {
    /// Email address of the sender
    pub email_from: String,

    /// Email address of the recipient
    pub email_to: String,

    /// BCC email address (optional)
    pub bcc: Option<String>,

    /// Provider name
    pub provider: String,

    /// Customer name
    pub name: String,

    /// Data amount
    pub data_amount: String,

    /// Time period
    pub time_period: String,

    /// Location
    pub location: String,
}
