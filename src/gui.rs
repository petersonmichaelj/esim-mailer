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
                        name,
                        data_amount,
                        time_period,
                        ..
                    } = &mut self.args;
                    let mut preview_changed = false;

                    add_form_field(ui, "From:", email_from, &mut preview_changed);
                    add_form_field(ui, "To:", email_to, &mut preview_changed);
                    add_form_field(
                        ui,
                        "BCC:",
                        bcc.get_or_insert_with(String::new),
                        &mut preview_changed,
                    );
                    add_form_field(ui, "Name:", name, &mut preview_changed);
                    add_form_field(ui, "Data Amount:", data_amount, &mut preview_changed);
                    add_form_field(ui, "Time Period:", time_period, &mut preview_changed);
                });

            ui.add_space(10.0);

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

fn add_form_field(ui: &mut egui::Ui, label: &str, value: &mut String, preview_changed: &mut bool) {
    ui.horizontal(|ui| {
        ui.label(label);
        if ui
            .add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY))
            .changed()
        {
            *preview_changed = true;
        }
    });
    ui.end_row();
}
