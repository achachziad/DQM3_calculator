//! DQM3 synthesis calculator: desktop GUI over `dqm3-core`.
//! Display only: every game rule lives in the core.

// No console window behind the app on Windows (release builds only, so
// that `cargo run` in debug still shows panics and logs).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod assets;
mod theme;
mod widgets;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("DQM3 — Calculateur de synthèse")
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([820.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native(
        "DQM3 Synthèse",
        options,
        // `?` turns a `CoreError` into eframe's boxed error type.
        Box::new(|cc| Ok(Box::new(app::App::new(cc)?))),
    )
}
