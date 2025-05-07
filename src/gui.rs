use eframe::egui;
use rfd::FileDialog;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::email::{self, EmailTemplate};
use crate::oauth::OAuthClient;
use crate::{Args, send_email};

// Trait for email operations to allow mocking in tests
pub trait EmailOperations: Send + Sync {
    fn send_email(
        &self,
        args: &Args,
        token: String,
        path: &Path,
        count: usize,
    ) -> std::io::Result<()>;
    fn get_token(
        &self,
        provider: &email::Provider,
        email: &str,
    ) -> Result<String, Box<dyn std::error::Error>>;
}

// Default implementation that uses real email functionality
pub struct DefaultEmailOperations {
    oauth_client: Arc<Mutex<OAuthClient>>,
}

impl DefaultEmailOperations {
    pub fn new(oauth_client: Arc<Mutex<OAuthClient>>) -> Self {
        Self { oauth_client }
    }
}

impl EmailOperations for DefaultEmailOperations {
    fn send_email(
        &self,
        args: &Args,
        token: String,
        path: &Path,
        count: usize,
    ) -> std::io::Result<()> {
        send_email(args, token, path, count)
    }

    fn get_token(
        &self,
        provider: &email::Provider,
        email: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut client = self.oauth_client.lock().unwrap();
        Ok(client.get_or_refresh_token(provider, email)?)
    }
}

// Separate state management
#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct AppState {
    pub args: Args,

    #[serde(skip)]
    pub image_paths: Vec<PathBuf>,

    #[serde(skip)]
    pub status: Arc<Mutex<String>>,

    #[serde(skip)]
    pub email_preview: String,

    #[serde(skip)]
    pub is_sending: Arc<Mutex<bool>>,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct EsimMailerApp {
    state: AppState,

    #[serde(skip)]
    email_ops: Arc<dyn EmailOperations>,
}

impl Default for EsimMailerApp {
    fn default() -> Self {
        let oauth_client = Arc::new(Mutex::new(OAuthClient::default()));

        let mut app = Self {
            state: AppState::default(),
            email_ops: Arc::new(DefaultEmailOperations::new(oauth_client)),
        };
        app.generate_preview(); // Generate preview with loaded args
        app
    }
}

// Constructor for testing
impl EsimMailerApp {
    #[cfg(test)]
    pub fn new_with_email_ops(email_ops: Arc<dyn EmailOperations>) -> Self {
        Self {
            state: AppState::default(),
            email_ops,
        }
    }

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            let mut app: EsimMailerApp =
                eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
            app.email_ops = Arc::new(DefaultEmailOperations::new(Arc::new(Mutex::new(
                OAuthClient::default(),
            ))));
            app.generate_preview();
            return app;
        }

        Default::default()
    }

    fn generate_preview(&mut self) {
        let template = EmailTemplate::new();
        let subject = template.subject(&self.state.args, 1);
        let body = template.body(&self.state.args);
        self.state.email_preview = format!("Subject: {}\n\nBody:\n{}", subject, body);
    }

    fn send_email_async(&self) {
        let status = Arc::clone(&self.state.status);
        let is_sending = Arc::clone(&self.state.is_sending);
        let email_ops = Arc::clone(&self.email_ops);
        *is_sending.lock().unwrap() = true;

        let args = self.state.args.clone();
        let image_paths = self.state.image_paths.clone();

        let email_provider: email::Provider =
            args.email_from.parse().expect("Invalid email provider");

        thread::spawn(move || {
            let token = email_ops.get_token(&email_provider, &args.email_from);

            match token {
                Ok(token) => {
                    for (index, path) in image_paths.iter().enumerate() {
                        match email_ops.send_email(&args, token.clone(), path, index + 1) {
                            Ok(_) => {
                                let mut status_lock = status.lock().unwrap();
                                *status_lock =
                                    format!("{} eSIM emails sent successfully.", index + 1);
                            }
                            Err(e) => {
                                let mut status_lock = status.lock().unwrap();
                                *status_lock = format!("Error sending email: {}", e);
                                break;
                            }
                        }
                    }
                    *is_sending.lock().unwrap() = false;
                }
                Err(e) => {
                    let mut status_lock = status.lock().unwrap();
                    *status_lock = format!("Error getting OAuth token: {}", e);
                    *is_sending.lock().unwrap() = false;
                }
            }
        });
    }

    pub fn update_form_field(&mut self, field: &str, value: String) -> bool {
        let mut changed = false;
        match field {
            "From" => {
                if self.state.args.email_from != value {
                    self.state.args.email_from = value;
                    changed = true;
                }
            }
            "To" => {
                if self.state.args.email_to != value {
                    self.state.args.email_to = value;
                    changed = true;
                }
            }
            "BCC" => {
                if self.state.args.bcc.as_deref().unwrap_or("") != value {
                    self.state.args.bcc = Some(value);
                    changed = true;
                }
            }
            "Provider" => {
                if self.state.args.provider != value {
                    self.state.args.provider = value;
                    changed = true;
                }
            }
            "Name" => {
                if self.state.args.name != value {
                    self.state.args.name = value;
                    changed = true;
                }
            }
            "Data Amount" => {
                if self.state.args.data_amount != value {
                    self.state.args.data_amount = value;
                    changed = true;
                }
            }
            "Time Period" => {
                if self.state.args.time_period != value {
                    self.state.args.time_period = value;
                    changed = true;
                }
            }
            "Location" => {
                if self.state.args.location != value {
                    self.state.args.location = value;
                    changed = true;
                }
            }
            _ => {}
        }
        if changed {
            self.generate_preview();
        }
        changed
    }

    #[cfg(test)]
    pub fn get_form_state(&self) -> Args {
        self.state.args.clone()
    }

    #[cfg(test)]
    pub fn get_preview(&self) -> String {
        self.state.email_preview.clone()
    }
}

impl eframe::App for EsimMailerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("eSIM Mailer");
                ui.add_space(10.0);

                egui::Grid::new("email_form")
                    .num_columns(1)
                    .spacing([0.0, 10.0])
                    .show(ui, |ui| {
                        let fields = [
                            ("From", self.state.args.email_from.clone()),
                            ("To", self.state.args.email_to.clone()),
                            ("BCC", self.state.args.bcc.clone().unwrap_or_default()),
                            ("Provider", self.state.args.provider.clone()),
                            ("Name", self.state.args.name.clone()),
                            ("Data Amount", self.state.args.data_amount.clone()),
                            ("Time Period", self.state.args.time_period.clone()),
                            ("Location", self.state.args.location.clone()),
                        ];

                        for (label, value) in fields.iter() {
                            let mut current_value = value.clone();
                            ui.horizontal(|ui| {
                                ui.label(format!("{}:", label));
                                if ui
                                    .add(
                                        egui::TextEdit::singleline(&mut current_value)
                                            .desired_width(f32::INFINITY),
                                    )
                                    .changed()
                                {
                                    self.update_form_field(label, current_value);
                                }
                            });
                            ui.end_row();
                        }
                    });

                ui.add_space(10.0);

                if ui.button("Select QR codes").clicked() {
                    if let Some(paths) = FileDialog::new()
                        .add_filter("Image Files", &["png", "jpg", "jpeg", "gif"])
                        .pick_files()
                    {
                        self.state.image_paths = paths;
                    }
                }

                ui.label(format!(
                    "Selected QR codes: {}",
                    self.state.image_paths.len()
                ));

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.label("Email Preview:");
                    ui.add(
                        egui::TextEdit::multiline(&mut self.state.email_preview)
                            .desired_width(f32::INFINITY)
                            .desired_rows(10)
                            .interactive(false),
                    );
                });

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if !*self.state.is_sending.lock().unwrap() {
                        if ui.button("Send Email").clicked() {
                            self.send_email_async();
                        }
                    } else {
                        ui.add(egui::Spinner::new());
                        ui.label("Sending email...");
                    }
                });

                ui.add_space(10.0);

                if !*self.state.is_sending.lock().unwrap() {
                    let status = self.state.status.lock().unwrap().clone();
                    if !status.is_empty() {
                        ui.label(status);
                    }
                }
            });
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mock email operations for testing
    struct MockEmailOperations {
        send_count: Arc<Mutex<usize>>,
        should_fail: bool,
    }

    impl MockEmailOperations {
        fn new(should_fail: bool) -> Self {
            Self {
                send_count: Arc::new(Mutex::new(0)),
                should_fail,
            }
        }
    }

    impl EmailOperations for MockEmailOperations {
        fn send_email(
            &self,
            _args: &Args,
            _token: String,
            _path: &Path,
            _count: usize,
        ) -> std::io::Result<()> {
            if self.should_fail {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "Mock error"));
            }
            let mut count = self.send_count.lock().unwrap();
            *count += 1;
            Ok(())
        }

        fn get_token(
            &self,
            _provider: &email::Provider,
            _email: &str,
        ) -> Result<String, Box<dyn std::error::Error>> {
            if self.should_fail {
                return Err("Mock token error".into());
            }
            Ok("mock_token".to_string())
        }
    }

    #[test]
    fn test_esim_mailer_app_default() {
        let app = EsimMailerApp::default();
        assert_eq!(app.state.args, Args::default());
        assert!(app.state.image_paths.is_empty());
        assert_eq!(app.state.status.lock().unwrap().as_str(), "");
        assert_ne!(app.state.email_preview, "");
        assert!(!(*app.state.is_sending.lock().unwrap()));
    }

    #[test]
    fn test_generate_preview() {
        let mut app = EsimMailerApp::default();
        app.state.args = Args {
            email_from: "from@example.com".to_string(),
            email_to: "to@example.com".to_string(),
            bcc: Some("bcc@example.com".to_string()),
            provider: "TestProvider".to_string(),
            name: "John".to_string(),
            data_amount: "5GB".to_string(),
            time_period: "30 days".to_string(),
            location: "Egypt".to_string(),
        };

        app.generate_preview();

        assert!(
            app.state
                .email_preview
                .contains("Subject: [TestProvider] Egypt eSIM")
        );
        assert!(app.state.email_preview.contains("John"));
        assert!(app.state.email_preview.contains("5GB"));
        assert!(app.state.email_preview.contains("30 days"));
        assert!(app.state.email_preview.contains("Egypt"));
    }

    #[test]
    fn test_send_email_success() {
        let mock_ops = Arc::new(MockEmailOperations::new(false));
        let mut app = EsimMailerApp::new_with_email_ops(mock_ops.clone());

        // Setup test data
        app.state.args.email_from = "test@gmail.com".to_string();
        app.state.image_paths = vec![PathBuf::from("test.png")];

        app.send_email_async();

        // Give the async operation time to complete
        std::thread::sleep(std::time::Duration::from_millis(100));

        assert_eq!(*mock_ops.send_count.lock().unwrap(), 1);
        assert!(
            app.state
                .status
                .lock()
                .unwrap()
                .contains("sent successfully")
        );
    }

    #[test]
    fn test_send_email_failure() {
        let mock_ops = Arc::new(MockEmailOperations::new(true));
        let mut app = EsimMailerApp::new_with_email_ops(mock_ops);

        // Setup test data
        app.state.args.email_from = "test@gmail.com".to_string();
        app.state.image_paths = vec![PathBuf::from("test.png")];

        app.send_email_async();

        // Give the async operation time to complete
        std::thread::sleep(std::time::Duration::from_millis(100));

        assert!(app.state.status.lock().unwrap().contains("Error"));
    }

    #[test]
    fn test_form_field_updates() {
        let email_ops = Arc::new(MockEmailOperations::new(false));
        let mut app = EsimMailerApp::new_with_email_ops(email_ops);

        // Test email from update
        assert!(app.update_form_field("From", "test@example.com".to_string()));
        assert_eq!(app.get_form_state().email_from, "test@example.com");

        // Test no change when same value
        assert!(!app.update_form_field("From", "test@example.com".to_string()));

        // Test multiple field updates
        assert!(app.update_form_field("To", "recipient@example.com".to_string()));
        assert!(app.update_form_field("Name", "John Doe".to_string()));
        assert!(app.update_form_field("Provider", "TestProvider".to_string()));
        assert!(app.update_form_field("Data Amount", "10GB".to_string()));
        assert!(app.update_form_field("Time Period", "60 days".to_string()));
        assert!(app.update_form_field("Location", "Japan".to_string()));
        assert!(app.update_form_field("BCC", "bcc@example.com".to_string()));

        let state = app.get_form_state();
        assert_eq!(state.email_to, "recipient@example.com");
        assert_eq!(state.name, "John Doe");
        assert_eq!(state.provider, "TestProvider");
        assert_eq!(state.data_amount, "10GB");
        assert_eq!(state.time_period, "60 days");
        assert_eq!(state.location, "Japan");
        assert_eq!(state.bcc, Some("bcc@example.com".to_string()));

        // Test no change when setting same values again
        assert!(!app.update_form_field("Data Amount", "10GB".to_string()));
        assert!(!app.update_form_field("Time Period", "60 days".to_string()));
        assert!(!app.update_form_field("Location", "Japan".to_string()));
        assert!(!app.update_form_field("BCC", "bcc@example.com".to_string()));

        // Test empty BCC
        assert!(app.update_form_field("BCC", "".to_string()));
        assert_eq!(app.get_form_state().bcc, Some("".to_string()));

        // Verify preview is updated with all fields
        let preview = app.get_preview();
        assert!(preview.contains("John Doe"));
        assert!(preview.contains("TestProvider"));
        assert!(preview.contains("10GB"));
        assert!(preview.contains("60 days"));
        assert!(preview.contains("Japan"));
    }
}
