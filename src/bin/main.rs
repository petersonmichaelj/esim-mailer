use eframe::egui;
use esim_mailer::gui::EsimMailerApp;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "eSIM Mailer",
        options,
        Box::new(|_cc| Ok(Box::new(EsimMailerApp::default()))),
    )
}
