use std::time::{Duration, Instant};

use egui::{Align, DragValue, Layout, RichText, TextEdit, Ui};

use crate::engine::HotkeyBinding;
use crate::engine::macros::Macro;
use crate::ui::{theme, widgets};

pub struct MacrosState {
    pub list: Vec<String>,
    pub new_name: String,
    pub loaded: Option<String>,
    pub loaded_event_count: usize,
    pub loaded_duration_ms: u64,
    pub loops: u32,
    pub recording: bool,
    pub record_started_at: Option<Instant>,
    pub record_event_count: u32,
    pub feedback: Option<(String, bool)>,
    /// Capture state for the rebindable record hotkey.
    pub capturing_record_hotkey: bool,
    /// Set when recording just stopped and is waiting for the user to pick
    /// a name. Save/Discard buttons resolve it.
    pub pending: Option<Macro>,
}

impl Default for MacrosState {
    fn default() -> Self {
        Self {
            list: Vec::new(),
            new_name: String::new(),
            loaded: None,
            loaded_event_count: 0,
            loaded_duration_ms: 0,
            loops: 1,
            recording: false,
            record_started_at: None,
            record_event_count: 0,
            feedback: None,
            capturing_record_hotkey: false,
            pending: None,
        }
    }
}

#[derive(Debug)]
pub enum MacroAction {
    /// Start recording. Name is picked after stop via `SavePendingMacro`.
    StartRecording,
    StopRecording,
    SavePendingMacro(String),
    DiscardPendingMacro,
    LoadMacro(String),
    DeleteMacro(String),
    SetLoops(u32),
    Play,
    Stop,
    SetRecordHotkey(HotkeyBinding),
}

pub fn show(
    ui: &mut Ui,
    state: &mut MacrosState,
    is_running: bool,
    record_hotkey: &mut HotkeyBinding,
) -> Vec<MacroAction> {
    let mut actions = Vec::new();

    // RECORD
    widgets::surface_card(ui, |ui| {
        let rebind_changed = widgets::card_header_with(ui, "RECORD", |ui| {
            widgets::rebindable_hotkey_chip(
                ui,
                record_hotkey,
                &mut state.capturing_record_hotkey,
            )
        });
        if rebind_changed {
            actions.push(MacroAction::SetRecordHotkey(*record_hotkey));
        }
        ui.add_space(6.0);

        if state.recording {
            ui.horizontal(|ui| {
                let size = egui::vec2(12.0, 12.0);
                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                let p = ui.painter();
                p.circle_filled(rect.center(), 9.0, theme::p().danger.gamma_multiply(0.2));
                p.circle_filled(rect.center(), 5.5, theme::p().danger.gamma_multiply(0.4));
                p.circle_filled(rect.center(), 3.5, theme::p().danger);
                ui.add_space(6.0);
                ui.label(
                    RichText::new("RECORDING")
                        .size(15.5)
                        .strong()
                        .color(theme::p().danger)
                        .monospace()
                        .extra_letter_spacing(1.1),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::ghost_button(ui, "Stop").clicked() {
                        actions.push(MacroAction::StopRecording);
                    }
                });
            });

            let dur = state
                .record_started_at
                .map(|t| Instant::now().duration_since(t))
                .unwrap_or(Duration::ZERO);
            ui.add_space(2.0);
            ui.label(
                RichText::new(format!(
                    "{:.1}s  ·  {} events",
                    dur.as_secs_f32(),
                    state.record_event_count
                ))
                .size(12.0)
                .color(theme::p().text_muted),
            );
        } else if let Some(pending) = state.pending.as_ref() {
            // Recording just finished — ask for a name.
            ui.label(
                RichText::new(format!(
                    "{} events  ·  {:.1}s",
                    pending.frames.len(),
                    (pending.total_duration_ms as f32) / 1000.0,
                ))
                .size(12.0)
                .color(theme::p().text_muted),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let _ = ui.add(
                    TextEdit::singleline(&mut state.new_name)
                        .hint_text("Name this recording")
                        .desired_width(180.0),
                );
                let ok = !state.new_name.trim().is_empty();
                ui.add_enabled_ui(ok, |ui| {
                    if widgets::ghost_button(ui, "Save").clicked() {
                        actions.push(MacroAction::SavePendingMacro(
                            state.new_name.trim().to_string(),
                        ));
                    }
                });
                ui.add_space(4.0);
                if widgets::ghost_button(ui, "Discard").clicked() {
                    actions.push(MacroAction::DiscardPendingMacro);
                }
            });
        } else {
            ui.horizontal(|ui| {
                if widgets::ghost_button(ui, "Record").clicked() {
                    actions.push(MacroAction::StartRecording);
                }
            });
            ui.add_space(4.0);
            ui.label(
                RichText::new("Captures keyboard + mouse events; name it after you stop")
                    .size(11.5)
                    .color(theme::p().text_faint),
            );
        }
    });

    ui.add_space(6.0);

    // LIBRARY
    widgets::surface_card(ui, |ui| {
        widgets::card_header(ui, "LIBRARY");
        ui.add_space(6.0);

        if state.list.is_empty() {
            ui.label(
                RichText::new("No macros saved yet")
                    .size(12.0)
                    .color(theme::p().text_muted),
            );
        } else {
            for name in state.list.clone() {
                let is_loaded = state.loaded.as_deref() == Some(&name);
                ui.horizontal(|ui| {
                    let label = if is_loaded {
                        RichText::new(&name).size(13.0).color(theme::p().accent).strong()
                    } else {
                        RichText::new(&name).size(13.0).color(theme::p().text)
                    };
                    ui.label(label);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::ghost_button(ui, "Delete").clicked() {
                            actions.push(MacroAction::DeleteMacro(name.clone()));
                        }
                        ui.add_space(4.0);
                        if widgets::ghost_button(ui, "Load").clicked() {
                            actions.push(MacroAction::LoadMacro(name.clone()));
                        }
                    });
                });
            }
        }
    });

    // PLAYBACK
    if state.loaded.is_some() {
        ui.add_space(6.0);
        widgets::surface_card(ui, |ui| {
            widgets::card_header(ui, "PLAYBACK");
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!(
                    "{} events  ·  {:.1}s per loop",
                    state.loaded_event_count,
                    (state.loaded_duration_ms as f32) / 1000.0,
                ))
                .size(12.0)
                .color(theme::p().text_muted),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                widgets::row_label(ui, "Loops");
                let mut loops = state.loops;
                if ui
                    .add(DragValue::new(&mut loops).range(0..=100_000).speed(1.0))
                    .changed()
                {
                    state.loops = loops;
                    actions.push(MacroAction::SetLoops(loops));
                }
                ui.label(
                    RichText::new("(0 = ∞)")
                        .size(11.5)
                        .color(theme::p().text_faint),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !is_running {
                        if widgets::ghost_button(ui, "Play").clicked() {
                            actions.push(MacroAction::Play);
                        }
                    } else {
                        if widgets::ghost_button(ui, "Stop").clicked() {
                            actions.push(MacroAction::Stop);
                        }
                    }
                });
            });
        });
    }

    if let Some((msg, good)) = &state.feedback {
        ui.add_space(6.0);
        let color = if *good { theme::p().accent } else { theme::p().danger };
        ui.label(RichText::new(msg).size(12.0).color(color));
    }

    actions
}
