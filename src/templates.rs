use std::collections::HashMap;

pub struct EmailTemplate {
    pub subject: &'static str,
    pub body: &'static str,
}

pub fn load_templates() -> HashMap<String, EmailTemplate> {
    let mut templates = HashMap::new();
    templates.insert(
        "shared".to_string(),
        EmailTemplate {
            subject: "[{{provider}}] {{location}} eSIM",
            body: include_str!("../templates/shared.html"),
        },
    );
    // Remove other templates
    templates
}
