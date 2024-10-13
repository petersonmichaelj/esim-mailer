use eframe::egui;
use rfd::FileDialog;
use std::path::PathBuf;

use crate::{args::Template, get_or_refresh_token, send_email, Args};

pub struct EsimMailerApp {
    args: Args,
    image_paths: Vec<PathBuf>,
    status: String,
}

impl Default for EsimMailerApp {
    fn default() -> Self {
        Self {
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
        }
    }
}

impl eframe::App for EsimMailerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("eSIM Mailer");

            ui.horizontal(|ui| {
                ui.label("From:");
                ui.text_edit_singleline(&mut self.args.email_from);
            });

            ui.horizontal(|ui| {
                ui.label("To:");
                ui.text_edit_singleline(&mut self.args.email_to);
            });

            ui.horizontal(|ui| {
                ui.label("BCC:");
                if let Some(bcc) = &mut self.args.bcc {
                    ui.text_edit_singleline(bcc);
                } else {
                    let mut new_bcc = String::new();
                    ui.text_edit_singleline(&mut new_bcc);
                    if !new_bcc.is_empty() {
                        self.args.bcc = Some(new_bcc);
                    }
                }
            });

            ui.horizontal(|ui| {
                ui.label("Template:");
                ui.radio_value(&mut self.args.template, Template::Nomad, "Nomad");
                ui.radio_value(&mut self.args.template, Template::Test, "Test");
            });

            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.args.name);
            });

            ui.horizontal(|ui| {
                ui.label("Data Amount:");
                ui.text_edit_singleline(&mut self.args.data_amount);
            });

            ui.horizontal(|ui| {
                ui.label("Time Period:");
                ui.text_edit_singleline(&mut self.args.time_period);
            });

            if ui.button("Select Images").clicked() {
                if let Some(paths) = FileDialog::new()
                    .add_filter("Image Files", &["png", "jpg", "jpeg", "gif"])
                    .pick_files()
                {
                    self.image_paths = paths;
                }
            }

            ui.label(format!("Selected images: {}", self.image_paths.len()));

            if ui.button("Send Email").clicked() {
                self.send_email();
            }

            ui.label(&self.status);
        });
    }
}

impl EsimMailerApp {
    fn send_email(&mut self) {
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
}
