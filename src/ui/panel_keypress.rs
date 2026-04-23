use egui::Ui;

use crate::engine::{KeyMods, TriggerMode};
use crate::ui::{nav, widgets};

pub struct KeyPressConfig {
    /// VK code of the key to auto-press. Captured via the Press chip.
    pub selected_vk: u16,
    pub mods: KeyMods,
    pub mode: TriggerMode,
    /// True while the Press chip is in capture mode (waiting for a
    /// keypress to adopt as the target key). The LL hook reads this via
    /// REBIND_ACTIVE so bound hotkeys don't fire during the capture.
    pub capturing_key: bool,
}

impl Default for KeyPressConfig {
    fn default() -> Self {
        Self {
            selected_vk: 0x41, // 'A'
            mods: KeyMods::default(),
            mode: TriggerMode::Auto,
            capturing_key: false,
        }
    }
}

pub fn show(ui: &mut Ui, cfg: &mut KeyPressConfig) {
    widgets::surface_card(ui, |ui| {
        widgets::card_header(ui, "KEY");
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            widgets::row_label(ui, "Mode");
            ui.add_space(4.0);
            nav::segmented(
                ui,
                &mut cfg.mode,
                &[
                    (TriggerMode::Auto, "Autopress"),
                    (TriggerMode::Hold, "Hold"),
                ],
            );
        });

        ui.add_space(6.0);

        ui.horizontal(|ui| {
            widgets::row_label(ui, "Press");
            widgets::rebindable_key_chip(ui, &mut cfg.selected_vk, &mut cfg.capturing_key);
        });

        ui.add_space(6.0);

        ui.horizontal(|ui| {
            widgets::row_label(ui, "Mods");
            ui.add_space(4.0);
            widgets::checkbox(ui, &mut cfg.mods.ctrl, "Ctrl");
            widgets::checkbox(ui, &mut cfg.mods.shift, "Shift");
            widgets::checkbox(ui, &mut cfg.mods.alt, "Alt");
            widgets::checkbox(ui, &mut cfg.mods.win, "Win");
        });
    });
}
