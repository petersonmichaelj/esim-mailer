use eframe::egui;
use rfd::FileDialog;
use std::path::PathBuf;

use crate::{args::Template, get_or_refresh_token, send_email, Args};

pub struct EsimMailerApp {
    args: Args,
    image_paths: Vec<PathBuf>,
    status: String,
    email_preview: String,
}

impl Default for EsimMailerApp {
    fn default() -> Self {
        let mut app = Self {
            args: Args {
                email_from: String::new(),
                email_to: String::new(),
                bcc: None,
                template: Template::Nomad,
                name: String::new(),
                data_amount: String::new(),
                time_period: String::new(),
            },
            image_paths: Vec::new(),
            status: String::new(),
            email_preview: String::new(),
        };
        app.generate_preview(); // Generate preview on initial load
        app
    }
}

impl eframe::App for EsimMailerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("eSIM Mailer");

            let mut preview_changed = false;

            ui.horizontal(|ui| {
                ui.label("From:");
                if ui.text_edit_singleline(&mut self.args.email_from).changed() {
                    preview_changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("To:");
                if ui.text_edit_singleline(&mut self.args.email_to).changed() {
                    preview_changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("BCC:");
                if let Some(bcc) = &mut self.args.bcc {
                    if ui.text_edit_singleline(bcc).changed() {
                        preview_changed = true;
                    }
                } else {
                    let mut new_bcc = String::new();
                    if ui.text_edit_singleline(&mut new_bcc).changed() {
                        if !new_bcc.is_empty() {
                            self.args.bcc = Some(new_bcc);
                            preview_changed = true;
                        }
                    }
                }
            });

            ui.horizontal(|ui| {
                ui.label("Template:");
                if ui
                    .radio_value(&mut self.args.template, Template::Nomad, "Nomad")
                    .changed()
                    || ui
                        .radio_value(&mut self.args.template, Template::Test, "Test")
                        .changed()
                {
                    preview_changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Name:");
                if ui.text_edit_singleline(&mut self.args.name).changed() {
                    preview_changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Data Amount:");
                if ui
                    .text_edit_singleline(&mut self.args.data_amount)
                    .changed()
                {
                    preview_changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Time Period:");
                if ui
                    .text_edit_singleline(&mut self.args.time_period)
                    .changed()
                {
                    preview_changed = true;
                }
            });

            if ui.button("Select Images").clicked() {
                if let Some(paths) = FileDialog::new()
                    .add_filter("Image Files", &["png", "jpg", "jpeg", "gif"])
                    .pick_files()
                {
                    self.image_paths = paths;
                    preview_changed = true;
                }
            }

            ui.label(format!("Selected images: {}", self.image_paths.len()));

            if preview_changed {
                self.generate_preview();
            }

            ui.label("Email Preview:");
            ui.add(
                egui::TextEdit::multiline(&mut self.email_preview)
                    .desired_width(f32::INFINITY)
                    .interactive(false),
            ); // Make the field read-only

            if ui.button("Send Email").clicked() {
                self.send_email();
            }

            ui.label(&self.status);
        });
    }
}

impl EsimMailerApp {
    fn send_email(&mut self) {
        self.generate_preview(); // Regenerate preview before sending

        if self.image_paths.is_empty() {
            self.status = "Please select at least one image.".to_string();
            return;
        }

        let provider = crate::oauth::determine_provider(&self.args.email_from);

        match get_or_refresh_token(provider, &self.args.email_from) {
            Ok(token) => {
                for (index, path) in self.image_paths.iter().enumerate() {
                    match send_email(&self.args, token.clone(), path, index + 1) {
                        Ok(_) => {
                            self.status =
                                format!("Email sent successfully for {} images.", index + 1);
                        }
                        Err(e) => {
                            self.status = format!("Error sending email: {}", e);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                self.status = format!("Error getting OAuth token: {}", e);
            }
        }
    }

    fn generate_preview(&mut self) {
        let template_name = match self.args.template {
            Template::Nomad => "nomad",
            Template::Test => "test",
        };

        let templates = crate::templates::load_templates();

        if let Some(template) = templates.get(template_name) {
            let subject = crate::email::replace_placeholders(
                template.subject,
                &self.args.name,
                &self.args.data_amount,
                &self.args.time_period,
            );

            let body = crate::email::replace_placeholders(
                template.body,
                &self.args.name,
                &self.args.data_amount,
                &self.args.time_period,
            );

            self.email_preview = format!("Subject: {}\n\nBody:\n{}", subject, body);
        } else {
            self.email_preview = "Error: Template not found".to_string();
        }
    }
}
