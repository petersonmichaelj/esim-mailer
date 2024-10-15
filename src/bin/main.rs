#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use esim_mailer::gui::EsimMailerApp;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_min_inner_size([320.0, 480.0])
            .with_resizable(true),
        ..Default::default()
    };
    eframe::run_native(
        "eSIM Mailer",
        options,
        Box::new(|_cc| Ok(Box::new(EsimMailerApp::default()))),
    )
}
