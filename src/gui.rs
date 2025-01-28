use eframe::egui;
use rfd::FileDialog;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::email::{self, EmailTemplate};
use crate::oauth::OAuthClient;
use crate::{send_email, Args};

// Trait for email operations to allow mocking in tests
pub trait EmailOperations: Send + Sync {
    fn send_email(
        &self,
        args: &Args,
        token: String,
        path: &PathBuf,
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
        path: &PathBuf,
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
#[derive(Default)]
pub struct AppState {
    pub args: Args,
    pub image_paths: Vec<PathBuf>,
    pub status: Arc<Mutex<String>>,
    pub email_preview: String,
    pub is_sending: Arc<Mutex<bool>>,
}

pub struct EsimMailerApp {
    state: AppState,
    email_ops: Arc<dyn EmailOperations>,
}

impl Default for EsimMailerApp {
    fn default() -> Self {
        let oauth_client = Arc::new(Mutex::new(OAuthClient::default()));
        Self {
            state: AppState::default(),
            email_ops: Arc::new(DefaultEmailOperations::new(oauth_client)),
        }
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
                        *is_sending.lock().unwrap() = false;
                    }
                }
                Err(e) => {
                    let mut status_lock = status.lock().unwrap();
                    *status_lock = format!("Error getting OAuth token: {}", e);
                    *is_sending.lock().unwrap() = false;
                }
            }
        });
    }
}

impl eframe::App for EsimMailerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("eSIM Mailer");
                ui.add_space(10.0);

                let mut preview_changed = false;

                egui::Grid::new("email_form")
                    .num_columns(1)
                    .spacing([0.0, 10.0])
                    .show(ui, |ui| {
                        let Args {
                            email_from,
                            email_to,
                            bcc,
                            provider,
                            name,
                            data_amount,
                            time_period,
                            location,
                        } = &mut self.state.args;

                        preview_changed |= add_form_field(ui, "From:", email_from);
                        preview_changed |= add_form_field(ui, "To:", email_to);
                        preview_changed |=
                            add_form_field(ui, "BCC:", bcc.get_or_insert_with(String::new));
                        preview_changed |= add_form_field(ui, "Provider:", provider);
                        preview_changed |= add_form_field(ui, "Name:", name);
                        preview_changed |= add_form_field(ui, "Data Amount:", data_amount);
                        preview_changed |= add_form_field(ui, "Time Period:", time_period);
                        preview_changed |= add_form_field(ui, "Location:", location);
                    });

                ui.add_space(10.0);

                if ui.button("Select QR codes").clicked() {
                    if let Some(paths) = FileDialog::new()
                        .add_filter("Image Files", &["png", "jpg", "jpeg", "gif"])
                        .pick_files()
                    {
                        self.state.image_paths = paths;
                        preview_changed = true;
                    }
                }

                ui.label(format!(
                    "Selected QR codes: {}",
                    self.state.image_paths.len()
                ));

                ui.add_space(10.0);

                if preview_changed {
                    self.generate_preview();
                    if let Ok(mut status_lock) = self.state.status.lock() {
                        status_lock.clear();
                    }
                }

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
}

fn add_form_field(ui: &mut egui::Ui, label: &str, value: &mut String) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        changed = ui
            .add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY))
            .changed();
    });
    ui.end_row();
    changed
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
            _path: &PathBuf,
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
        assert_eq!(app.state.email_preview, "");
        assert_eq!(*app.state.is_sending.lock().unwrap(), false);
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

        assert!(app
            .state
            .email_preview
            .contains("Subject: [TestProvider] Egypt eSIM"));
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
        assert!(app
            .state
            .status
            .lock()
            .unwrap()
            .contains("sent successfully"));
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
}
