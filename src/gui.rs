use eframe::egui;
use rfd::FileDialog;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::{args::Args, get_or_refresh_token, send_email};

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
        let templates = crate::templates::load_templates();

        if let Some(template) = templates.get("shared") {
            let subject = crate::email::replace_placeholders(
                template.subject,
                &self.args.provider,
                &self.args.name,
                &self.args.data_amount,
                &self.args.time_period,
                &self.args.location,
            );

            let body = crate::email::replace_placeholders(
                template.body,
                &self.args.provider,
                &self.args.name,
                &self.args.data_amount,
                &self.args.time_period,
                &self.args.location,
            );

            self.email_preview = format!("Subject: {}\n\nBody:\n{}", subject, body);
        } else {
            self.email_preview = "Error: Shared template not found".to_string();
        }
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
