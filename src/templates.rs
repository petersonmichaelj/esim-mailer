use std::collections::HashMap;

pub struct EmailTemplate {
    pub subject: &'static str,
    pub body: &'static str,
}

pub fn load_templates() -> HashMap<String, EmailTemplate> {
    let mut templates = HashMap::new();
    templates.insert(
        "nomad".to_string(),
        EmailTemplate {
            subject: "[Nomad] eSIM",
            body: include_str!("../templates/nomad.html"),
        },
    );
    templates.insert(
        "test".to_string(),
        EmailTemplate {
            subject: "Test Email",
            body: include_str!("../templates/test.html"),
        },
    );
    // Add more templates as needed
    templates
}
