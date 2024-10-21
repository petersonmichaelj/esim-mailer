use eframe::egui;
use rfd::FileDialog;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::email::EmailTemplate;
use crate::{get_or_refresh_token, send_email, Args};

pub struct EsimMailerApp {
    args: Args,
    image_paths: Vec<PathBuf>,
    status: Arc<Mutex<String>>,
    email_preview: String,
    is_sending: Arc<Mutex<bool>>,
}

impl Default for EsimMailerApp {
    fn default() -> Self {
        Self {
            args: Args::default(),
            image_paths: Vec::new(),
            status: Arc::new(Mutex::new(String::new())),
            email_preview: String::new(),
            is_sending: Arc::new(Mutex::new(false)),
        }
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
                        } = &mut self.args;

                        // If any form field changes, set preview_changed to true
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
                        self.image_paths = paths;
                        preview_changed = true;
                    }
                }

                ui.label(format!("Selected QR codes: {}", self.image_paths.len()));

                ui.add_space(10.0);

                if preview_changed {
                    self.generate_preview();
                    // Clear the status message when form fields are updated
                    if let Ok(mut status_lock) = self.status.lock() {
                        status_lock.clear();
                    }
                }

                ui.group(|ui| {
                    ui.label("Email Preview:");
                    ui.add(
                        egui::TextEdit::multiline(&mut self.email_preview)
                            .desired_width(f32::INFINITY)
                            .desired_rows(10)
                            .interactive(false),
                    );
                });

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if !*self.is_sending.lock().unwrap() {
                        if ui.button("Send Email").clicked() {
                            self.send_email_async();
                        }
                    } else {
                        ui.add(egui::Spinner::new());
                        ui.label("Sending email...");
                    }
                });

                ui.add_space(10.0);

                // Display status messages when not sending
                if !*self.is_sending.lock().unwrap() {
                    let status = self.status.lock().unwrap().clone();
                    if !status.is_empty() {
                        ui.label(status);
                    }
                }
            });
        });
    }
}

impl EsimMailerApp {
    fn generate_preview(&mut self) {
        let template = EmailTemplate::new();

        let subject = template.subject(&self.args, 1); // Use 1 as a placeholder count
        let body = template.body(&self.args);

        self.email_preview = format!("Subject: {}\n\nBody:\n{}", subject, body);
    }

    fn send_email_async(&self) {
        let status = Arc::clone(&self.status);
        let is_sending = Arc::clone(&self.is_sending);
        *is_sending.lock().unwrap() = true;

        let args = self.args.clone();
        let image_paths = self.image_paths.clone();

        thread::spawn(move || {
            let provider = crate::oauth::determine_provider(&args.email_from);

            match get_or_refresh_token(provider, &args.email_from) {
                Ok(token) => {
                    for (index, path) in image_paths.iter().enumerate() {
                        match send_email(&args, token.clone(), path, index + 1) {
                            Ok(_) => {
                                let mut status_lock = status.lock().unwrap();
                                *status_lock =
                                    format!("{} eSIM emails sent successfully.", index + 1);
                                drop(status_lock);
                                let mut sending_lock = is_sending.lock().unwrap();
                                *sending_lock = false;
                            }
                            Err(e) => {
                                let mut status_lock = status.lock().unwrap();
                                *status_lock = format!("Error sending email: {}", e);
                                drop(status_lock);
                                let mut sending_lock = is_sending.lock().unwrap();
                                *sending_lock = false;
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    let mut status_lock = status.lock().unwrap();
                    *status_lock = format!("Error getting OAuth token: {}", e);
                    drop(status_lock);
                    let mut sending_lock = is_sending.lock().unwrap();
                    *sending_lock = false;
                }
            }
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

    #[test]
    fn test_esim_mailer_app_default() {
        let app = EsimMailerApp::default();
        assert_eq!(app.args, Args::default());
        assert!(app.image_paths.is_empty());
        assert_eq!(app.status.lock().unwrap().as_str(), "");
        assert_eq!(app.email_preview, "");
        assert_eq!(*app.is_sending.lock().unwrap(), false);
    }

    #[test]
    fn test_generate_preview() {
        let mut app = EsimMailerApp::default();
        app.args = Args {
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
            .email_preview
            .contains("Subject: [TestProvider] Egypt eSIM"));
        assert!(app.email_preview.contains("John"));
        assert!(app.email_preview.contains("5GB"));
        assert!(app.email_preview.contains("30 days"));
        assert!(app.email_preview.contains("Egypt"));
    }
}
