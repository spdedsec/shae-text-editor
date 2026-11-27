//! Shae Text Editor — entry point.
//!
//! A fast, GPU-accelerated native text editor built with egui.

#![allow(dead_code)]

mod app;
mod buffer;
mod config;
mod editor;
mod find;
mod git;
mod highlight;
mod palette;
mod theme;

use app::ShaeApp;

fn main() -> eframe::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([640.0, 400.0])
            .with_title("Shae Text Editor")
            .with_app_id("shae-text-editor"),
        renderer: eframe::Renderer::Glow,
        vsync: true,
        ..Default::default()
    };

    eframe::run_native(
        "Shae Text Editor",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(ShaeApp::new(cc)))
        }),
    )
}

fn setup_fonts(ctx: &egui::Context) {
    use egui::FontData;
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "jetbrains".to_owned(),
        FontData::from_static(include_bytes!("../assets/JetBrainsMono-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "jetbrains_bold".to_owned(),
        FontData::from_static(include_bytes!("../assets/JetBrainsMono-Bold.ttf")).into(),
    );
    // Make JetBrains Mono the monospace family; keep the default proportional font.
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "jetbrains".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Name("jetbrains_bold".into()))
        .or_default()
        .insert(0, "jetbrains_bold".to_owned());
    ctx.set_fonts(fonts);
}