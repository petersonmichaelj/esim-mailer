use eframe::egui;
use rfd::FileDialog;
use std::path::PathBuf;

use crate::{args::Args, get_or_refresh_token, send_email};

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
                provider: String::new(),
                name: String::new(),
                data_amount: String::new(),
                time_period: String::new(),
                location: String::new(),
            },
            image_paths: Vec::new(),
            status: String::new(),
            email_preview: String::new(),
        };
        app.generate_preview();
        app
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

                ui.add_space(10.0);

                if preview_changed {
                    self.generate_preview();
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
                    if ui.button("Send Email").clicked() {
                        self.send_email();
                    }
                    ui.label(&self.status);
                });
            });
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
