use egui::{RichText, Ui};

use crate::config::settings::Settings;
use crate::ui::{theme, widgets};

pub struct SettingsState {
    /// Shared with the footer rebind chip. Kept here so the settings panel
    /// and footer widget use the same capture state.
    pub capturing_toggle: bool,
    pub feedback: Option<(String, bool)>,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            capturing_toggle: false,
            feedback: None,
        }
    }
}

#[derive(Debug)]
pub enum SettingsAction {
    SetAutostart(bool),
    SetCloseToTray(bool),
    SetStartMinimized(bool),
    SetResizableWindow(bool),
}

pub fn show_options(
    ui: &mut Ui,
    settings: &mut Settings,
    actions: &mut Vec<SettingsAction>,
) {
    widgets::surface_card(ui, |ui| {
        widgets::card_header(ui, "OPTIONS");
        ui.add_space(6.0);

        let mut autostart = settings.autostart;
        if widgets::checkbox(ui, &mut autostart, "Launch MKAC when Windows starts")
            .clicked()
        {
            actions.push(SettingsAction::SetAutostart(autostart));
        }

        let mut close_to_tray = settings.close_to_tray;
        if widgets::checkbox(
            ui,
            &mut close_to_tray,
            "Minimize to tray when window closes",
        )
        .clicked()
        {
            actions.push(SettingsAction::SetCloseToTray(close_to_tray));
        }

        let mut start_min = settings.start_minimized;
        if widgets::checkbox(ui, &mut start_min, "Start minimized").clicked() {
            actions.push(SettingsAction::SetStartMinimized(start_min));
        }

        let mut resizable = settings.resizable_window;
        if widgets::checkbox(ui, &mut resizable, "Resizable window (experimental)")
            .clicked()
        {
            actions.push(SettingsAction::SetResizableWindow(resizable));
        }
    });
}

pub fn show_feedback(ui: &mut Ui, state: &SettingsState) {
    if let Some((msg, good)) = &state.feedback {
        ui.add_space(6.0);
        let color = if *good { theme::p().accent } else { theme::p().danger };
        ui.label(RichText::new(msg).size(12.0).color(color));
    }
}

pub fn show(
    ui: &mut Ui,
    settings: &mut Settings,
    state: &mut SettingsState,
) -> Vec<SettingsAction> {
    let mut actions = Vec::new();
    show_options(ui, settings, &mut actions);
    show_feedback(ui, state);
    actions
}
