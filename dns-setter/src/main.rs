//! DNS Setter - Windows GUI for managing DNS settings

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod domain;
mod system;

use app::MyApp;
use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_inner_size([250.0, 520.0])
            .with_min_inner_size([250.0, 520.0])
            .with_transparent(true),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "DNS SETTER",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(MyApp::new()))
        }),
    )
}
