#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod autostart;
mod config;
mod engine;
mod hooks;
mod icon;
mod platform;
mod tray;
mod ui;
mod util;

use app::App;
use engine::EngineHandle;
use hooks::HookHandle;

fn main() -> eframe::Result<()> {
    let _ = config::ensure_dirs();
    let settings = config::settings::Settings::load();

    let cli_minimized = std::env::args().any(|a| a == "--minimized");
    let start_hidden = settings.start_minimized || cli_minimized;

    let engine = EngineHandle::spawn();
    let hooks = HookHandle::spawn(engine.cmd_tx.clone());

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([669.0, 547.0])
        .with_resizable(settings.resizable_window)
        .with_title("MKAC")
        .with_icon(egui::IconData {
            rgba: icon::RGBA.to_vec(),
            width: icon::SIZE,
            height: icon::SIZE,
        })
        .with_visible(!start_hidden);
    if settings.resizable_window {
        viewport = viewport.with_min_inner_size([520.0, 460.0]);
    }

    // Don't restore persisted window size — otherwise an earlier resize from
    // "resizable (experimental)" mode can stick the compact default in a
    // too-narrow size that collapses the 2-column layout.
    let options = eframe::NativeOptions {
        viewport,
        persist_window: false,
        ..Default::default()
    };

    eframe::run_native(
        "MKAC",
        options,
        Box::new(move |cc| {
            // Tray must be spawned here so it can hook egui's repaint scheduler
            // — otherwise tray events never reach `ui()` while the window is
            // hidden.
            let tray = match tray::spawn(cc.egui_ctx.clone()) {
                Ok(t) => Some(t),
                Err(e) => {
                    eprintln!("tray icon init failed: {e}");
                    None
                }
            };
            Ok(Box::new(App::new(cc, engine, hooks, tray, settings)))
        }),
    )
}
