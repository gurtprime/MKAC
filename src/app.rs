use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::{RichText, ViewportCommand};
use tray_icon::TrayIconEvent;

use crate::autostart;
use crate::config::{macros as macros_cfg, settings::Settings};
use crate::engine::macros::{Macro, MacroFrame};
use crate::engine::{
    Action, Command, EngineEvent, EngineHandle, MouseButton, RateConfig, StopAfter,
};
use crate::hooks::hotkey as hk_state;
use crate::hooks::recording::{RecEvent, set_recording};
use crate::hooks::HookHandle;
use crate::tray::{self, TrayHandle};
use crate::ui::panel_clicker::MouseConfig;
use crate::ui::panel_keypress::KeyPressConfig;
use crate::ui::panel_macros::{MacroAction, MacrosState};
use crate::ui::panel_settings::{SettingsAction, SettingsState};
use crate::ui::{
    nav, panel_clicker, panel_keypress, panel_macros, panel_rate, panel_settings, theme, widgets,
};

#[derive(PartialEq, Eq, Copy, Clone, Default, Debug)]
pub enum Tab {
    #[default]
    Mouse,
    Keyboard,
    Macros,
    Settings,
}

pub struct AppState {
    pub tab: Tab,
    pub rate: RateConfig,
    pub stop: StopAfter,
    pub mouse: MouseConfig,
    pub keypress: KeyPressConfig,
    pub macros: MacrosState,
    pub running: bool,
    pub held_keys: Vec<u16>,
    pub held_mouse: Vec<MouseButton>,
    pub settings: Settings,
    pub settings_panel: SettingsState,
    pub want_exit: bool,
    pub recording_buffer: Vec<RecEvent>,
    /// Last action sent to the engine. Lets the Settings tab remember the
    /// prior action while it isn't owning one of its own.
    pub last_action: Action,
    /// Counter value last seen from the macro-record hotkey pulse. We diff
    /// against `hk_state` each frame to detect new presses.
    pub macro_record_last_seen: u32,
    /// Previous-frame focus state. Used to detect focus-gain transitions so
    /// a focus-steal click can't silently leave the app in rebind mode.
    pub was_focused_last_frame: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            tab: Tab::Mouse,
            rate: RateConfig::default(),
            stop: StopAfter::default(),
            mouse: MouseConfig::default(),
            keypress: KeyPressConfig::default(),
            macros: MacrosState::default(),
            running: false,
            held_keys: Vec::new(),
            held_mouse: Vec::new(),
            settings: Settings::default(),
            settings_panel: SettingsState::default(),
            want_exit: false,
            recording_buffer: Vec::new(),
            last_action: Action::default(),
            macro_record_last_seen: 0,
            was_focused_last_frame: false,
        }
    }
}

pub struct App {
    engine: Option<EngineHandle>,
    hooks: Option<HookHandle>,
    tray: Option<TrayHandle>,
    state: AppState,
    cmd_tx: crossbeam_channel::Sender<Command>,
    evt_rx: crossbeam_channel::Receiver<EngineEvent>,
    record_rx: crossbeam_channel::Receiver<RecEvent>,
    /// Set after the first frame when the native title bar has been themed.
    /// FindWindowW needs the window to actually exist, so we defer.
    titlebar_themed: bool,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        engine: EngineHandle,
        hooks: HookHandle,
        tray: Option<TrayHandle>,
        settings: Settings,
    ) -> Self {
        theme::install(&cc.egui_ctx, settings.theme);
        // Give the LL hook thread a handle so it can wake the UI when a
        // hotkey fires while MKAC isn't focused.
        hk_state::install_egui_ctx(cc.egui_ctx.clone());

        // Force the intended size every launch so no persisted/stale size wins.
        cc.egui_ctx
            .send_viewport_cmd(ViewportCommand::InnerSize(egui::vec2(669.0, 547.0)));

        let mut state = AppState::default();
        state.rate.interval_ms = settings.interval_ms.max(1);
        state.settings = settings;
        state.macros.list = macros_cfg::list_macros();

        // Reconcile autostart with the actual registry entry — the user
        // (or another tool) may have toggled it outside the app.
        let actual_autostart = autostart::is_enabled();
        if state.settings.autostart != actual_autostart {
            state.settings.autostart = actual_autostart;
            let _ = state.settings.save();
        }

        // Install hotkey bindings from persisted settings
        hk_state::set_toggle_binding(state.settings.toggle_hotkey);
        hk_state::set_macro_record_binding(state.settings.macro_record_hotkey);

        let cmd_tx = engine.cmd_tx.clone();
        let evt_rx = engine.evt_rx.clone();
        let record_rx = hooks.record_rx.clone();

        state.last_action = build_action(&state);
        let _ = cmd_tx.send(Command::SetRate(state.rate));
        let _ = cmd_tx.send(Command::SetStopAfter(state.stop));
        let _ = cmd_tx.send(Command::SetAction(state.last_action));

        Self {
            engine: Some(engine),
            hooks: Some(hooks),
            tray,
            state,
            cmd_tx,
            evt_rx,
            record_rx,
            titlebar_themed: false,
        }
    }
}

/// Two-column layout with the inter-column gap set equal to the outer window
/// padding, so the three horizontal gutters (left edge · between cols · right
/// edge) are visually symmetric. Inner-column item_spacing is restored so
/// widgets inside cards still sit tight.
fn symmetric_columns<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut [egui::Ui]) -> R,
) -> R {
    // 22 = CentralPanel default margin (8) + outer Frame inner_margin (14).
    const OUTER_GAP: f32 = 22.0;
    let inner_gap = ui.spacing().item_spacing.x;
    ui.spacing_mut().item_spacing.x = OUTER_GAP;
    let r = ui.columns(2, |cols| {
        for col in cols.iter_mut() {
            col.spacing_mut().item_spacing.x = inner_gap;
        }
        add_contents(cols)
    });
    ui.spacing_mut().item_spacing.x = inner_gap;
    r
}

fn build_action(state: &AppState) -> Action {
    match state.tab {
        Tab::Mouse => Action::MouseClick {
            button: state.mouse.button,
            pattern: state.mouse.pattern,
            target: state.mouse.target(),
            mode: state.mouse.mode,
        },
        Tab::Keyboard => Action::KeyTap {
            vk: state.keypress.selected_vk,
            mods: state.keypress.mods,
            mode: state.keypress.mode,
        },
        Tab::Macros => Action::PlayMacro,
        // Settings tab has no action of its own; use whatever was last active.
        Tab::Settings => state.last_action,
    }
}

fn handle_settings_actions(
    state: &mut AppState,
    _cmd_tx: &crossbeam_channel::Sender<Command>,
    ctx: &egui::Context,
    actions: Vec<SettingsAction>,
) {
    for action in actions {
        match action {
            SettingsAction::SetAutostart(v) => match autostart::set(v) {
                Ok(()) => {
                    state.settings.autostart = v;
                    let _ = state.settings.save();
                    state.settings_panel.feedback = Some((
                        if v {
                            "Autostart enabled".into()
                        } else {
                            "Autostart disabled".into()
                        },
                        true,
                    ));
                }
                Err(e) => {
                    state.settings_panel.feedback =
                        Some((format!("Autostart: {e}"), false));
                }
            },
            SettingsAction::SetCloseToTray(v) => {
                state.settings.close_to_tray = v;
                let _ = state.settings.save();
            }
            SettingsAction::SetStartMinimized(v) => {
                state.settings.start_minimized = v;
                let _ = state.settings.save();
            }
            SettingsAction::SetResizableWindow(v) => {
                state.settings.resizable_window = v;
                let _ = state.settings.save();
                ctx.send_viewport_cmd(ViewportCommand::Resizable(v));
                if v {
                    ctx.send_viewport_cmd(ViewportCommand::MinInnerSize(egui::vec2(
                        520.0, 460.0,
                    )));
                }
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(egui::vec2(
                    669.0, 547.0,
                )));
                state.settings_panel.feedback = Some((
                    if v {
                        "Resize mode on".into()
                    } else {
                        "Compact mode on".into()
                    },
                    true,
                ));
            }
        }
    }
}

fn drain_recording(state: &mut AppState, record_rx: &crossbeam_channel::Receiver<RecEvent>) {
    while let Ok(ev) = record_rx.try_recv() {
        state.recording_buffer.push(ev);
    }
    state.macros.record_event_count = state.recording_buffer.len() as u32;
}

fn discard_recording(record_rx: &crossbeam_channel::Receiver<RecEvent>) {
    while record_rx.try_recv().is_ok() {}
}

fn build_macro_from_buffer(buffer: &[RecEvent], name: &str, stop_at: Instant) -> Macro {
    // Trim events captured within 150ms before stop (likely the Stop-button approach path).
    let grace = Duration::from_millis(150);
    let cutoff = stop_at - grace;
    let kept: Vec<&RecEvent> = buffer.iter().filter(|e| e.at <= cutoff).collect();

    let mut frames: Vec<MacroFrame> = Vec::with_capacity(kept.len());
    let mut prev: Option<Instant> = None;
    for ev in kept {
        let delta = match prev {
            Some(p) => ev.at.duration_since(p).as_millis().min(u32::MAX as u128) as u32,
            None => 0,
        };
        prev = Some(ev.at);
        frames.push(MacroFrame { delta_ms: delta, event: ev.event });
    }
    Macro::new(name, frames)
}

fn handle_macro_actions(
    state: &mut AppState,
    cmd_tx: &crossbeam_channel::Sender<Command>,
    record_rx: &crossbeam_channel::Receiver<RecEvent>,
    actions: Vec<MacroAction>,
) {
    for action in actions {
        match action {
            MacroAction::StartRecording => {
                // Stop any running action first.
                let _ = cmd_tx.send(Command::Stop);
                state.recording_buffer.clear();
                discard_recording(record_rx);
                state.macros.recording = true;
                state.macros.record_started_at = Some(Instant::now());
                state.macros.record_event_count = 0;
                state.macros.pending = None;
                state.macros.feedback = None;
                set_recording(true);
            }
            MacroAction::StopRecording => {
                let stop_at = Instant::now();
                set_recording(false);
                drain_recording(state, record_rx);
                state.macros.recording = false;

                // Build with empty name — the UI prompts the user to name
                // the recording before it's saved to disk.
                let m = build_macro_from_buffer(&state.recording_buffer, "", stop_at);
                state.recording_buffer.clear();
                if m.frames.is_empty() {
                    state.macros.feedback =
                        Some(("Nothing recorded".into(), false));
                } else {
                    state.macros.pending = Some(m);
                    state.macros.feedback = None;
                }
            }
            MacroAction::SavePendingMacro(name) => {
                if let Some(mut m) = state.macros.pending.take() {
                    m.name = name.clone();
                    let count = m.frames.len();
                    let duration_ms = m.total_duration_ms;
                    match macros_cfg::save_macro(&m) {
                        Ok(()) => {
                            state.macros.list = macros_cfg::list_macros();
                            state.macros.loaded = Some(name.clone());
                            state.macros.loaded_event_count = count;
                            state.macros.loaded_duration_ms = duration_ms;
                            state.macros.new_name.clear();
                            state.macros.feedback =
                                Some((format!("Saved '{name}' · {count} events"), true));
                            let _ = cmd_tx.send(Command::LoadMacro(Arc::new(m)));
                        }
                        Err(e) => {
                            // Put it back so the user can retry with a
                            // different name / after fixing the error.
                            state.macros.pending = Some(m);
                            state.macros.feedback =
                                Some((format!("Save failed: {e}"), false));
                        }
                    }
                }
            }
            MacroAction::DiscardPendingMacro => {
                state.macros.pending = None;
                state.macros.new_name.clear();
                state.macros.feedback = Some(("Recording discarded".into(), true));
            }
            MacroAction::LoadMacro(name) => match macros_cfg::load_macro(&name) {
                Ok(m) => {
                    state.macros.loaded = Some(name.clone());
                    state.macros.loaded_event_count = m.frames.len();
                    state.macros.loaded_duration_ms = m.total_duration_ms;
                    let _ = cmd_tx.send(Command::LoadMacro(Arc::new(m)));
                    state.macros.feedback = Some((format!("Loaded '{name}'"), true));
                }
                Err(e) => {
                    state.macros.feedback = Some((format!("Load failed: {e}"), false));
                }
            },
            MacroAction::DeleteMacro(name) => {
                match macros_cfg::delete_macro(&name) {
                    Ok(()) => {
                        if state.macros.loaded.as_deref() == Some(&name) {
                            state.macros.loaded = None;
                            state.macros.loaded_event_count = 0;
                            state.macros.loaded_duration_ms = 0;
                            let _ = cmd_tx.send(Command::ClearMacro);
                        }
                        state.macros.list = macros_cfg::list_macros();
                        state.macros.feedback = Some((format!("Deleted '{name}'"), true));
                    }
                    Err(e) => {
                        state.macros.feedback = Some((format!("Delete failed: {e}"), false));
                    }
                }
            }
            MacroAction::SetLoops(n) => {
                state.macros.loops = n;
                let _ = cmd_tx.send(Command::SetMacroLoops(n));
            }
            MacroAction::Play => {
                if state.macros.loaded.is_none() || state.macros.loaded_event_count == 0 {
                    state.macros.feedback =
                        Some(("No macro loaded — record or load one first".into(), false));
                } else {
                    state.last_action = Action::PlayMacro;
                    let _ = cmd_tx.send(Command::SetAction(Action::PlayMacro));
                    let _ = cmd_tx.send(Command::SetMacroLoops(state.macros.loops));
                    let _ = cmd_tx.send(Command::Start);
                }
            }
            MacroAction::Stop => {
                let _ = cmd_tx.send(Command::Stop);
            }
            MacroAction::SetRecordHotkey(b) => {
                state.settings.macro_record_hotkey = b;
                hk_state::set_macro_record_binding(b);
                let _ = state.settings.save();
                state.macros.feedback = Some((
                    format!("Record hotkey set to {}", widgets::format_binding(&b)),
                    true,
                ));
            }
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        // eframe's default clear is transparent → shows through as black.
        // Use the theme's panel fill so the framebuffer edges match the UI.
        let c = visuals.panel_fill;
        [
            c.r() as f32 / 255.0,
            c.g() as f32 / 255.0,
            c.b() as f32 / 255.0,
            1.0,
        ]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // First-frame native title bar theming. Deferred because
        // FindWindowW only succeeds once winit has actually created the HWND.
        if !self.titlebar_themed {
            crate::platform::apply_titlebar_theme(
                self.state.settings.theme,
                theme::p(),
            );
            self.titlebar_themed = true;
        }

        // Auto-cancel any rebind capture when (a) the window loses focus or
        // (b) the window just gained focus this frame. (b) is crucial — a
        // click that focuses MKAC often lands on a footer chip in the same
        // gesture, which would silently flip the app into capture mode and
        // make the hook ignore the next keypress.
        let focused = ctx.input(|i| i.focused);
        let just_focused = focused && !self.state.was_focused_last_frame;
        if !focused || just_focused {
            self.state.settings_panel.capturing_toggle = false;
            self.state.macros.capturing_record_hotkey = false;
            self.state.keypress.capturing_key = false;
        }
        self.state.was_focused_last_frame = focused;

        // Keep LL hook's rebind flag in sync with UI capture state so the
        // currently-bound hotkey doesn't fire while the user is rebinding
        // or selecting an autopress target.
        hk_state::set_rebind_active(
            self.state.settings_panel.capturing_toggle
                || self.state.macros.capturing_record_hotkey
                || self.state.keypress.capturing_key,
        );

        // Pump tray events
        while let Some(evt) = tray::poll_menu() {
            match evt.id.0.as_str() {
                tray::ID_SHOW => {
                    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(ViewportCommand::Focus);
                }
                tray::ID_HIDE => {
                    ctx.send_viewport_cmd(ViewportCommand::Visible(false));
                }
                tray::ID_QUIT => {
                    self.state.want_exit = true;
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
                _ => {}
            }
        }
        while let Some(evt) = tray::poll_tray() {
            if matches!(evt, TrayIconEvent::DoubleClick { .. }) {
                ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(ViewportCommand::Focus);
            }
        }

        if ctx.input(|i| i.viewport().close_requested())
            && !self.state.want_exit
            && self.state.settings.close_to_tray
            && self.tray.is_some()
        {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(ViewportCommand::Visible(false));
        }

        while let Ok(evt) = self.evt_rx.try_recv() {
            match evt {
                EngineEvent::Started => self.state.running = true,
                EngineEvent::Stopped => self.state.running = false,
                EngineEvent::HeldChanged(v) => self.state.held_keys = v,
                EngineEvent::MouseHeldChanged(v) => self.state.held_mouse = v,
            }
        }

        // Macro-record hotkey pulses from the LL hook.
        let pulses =
            hk_state::take_macro_record_requests(&mut self.state.macro_record_last_seen);
        for _ in 0..pulses {
            // While a pending recording is waiting to be named, the hotkey
            // is a no-op — user should click Save / Discard first.
            if self.state.macros.pending.is_some() {
                continue;
            }
            let action = if self.state.macros.recording {
                MacroAction::StopRecording
            } else {
                MacroAction::StartRecording
            };
            let cmd_tx = self.cmd_tx.clone();
            let rx = self.record_rx.clone();
            handle_macro_actions(&mut self.state, &cmd_tx, &rx, vec![action]);
        }

        // Drain recording channel while recording; discard otherwise so it
        // doesn't grow while hooks keep firing.
        if self.state.macros.recording {
            drain_recording(&mut self.state, &self.record_rx);
        } else {
            discard_recording(&self.record_rx);
        }

        // Outer margin so cards don't kiss the window border.
        let mut toggle_changed = false;
        let mut record_hotkey_changed = false;
        let mut theme_clicked = false;
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    nav::segmented(
                        ui,
                        &mut self.state.tab,
                        &[
                            (Tab::Mouse, "Mouse"),
                            (Tab::Keyboard, "Keyboard"),
                            (Tab::Macros, "Macros"),
                            (Tab::Settings, "Settings"),
                        ],
                    );
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if widgets::theme_toggle(ui, self.state.settings.theme).clicked() {
                                theme_clicked = true;
                            }
                        },
                    );
                });
                ui.add_space(10.0);

                // Reserve space for the bottom-docked footer.
                let scroll_h = (ui.available_height() - 44.0).max(100.0);

                egui::ScrollArea::vertical()
                    .max_height(scroll_h)
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.render_content(ui));

                ui.add_space(10.0);

                // Footer — click-to-rebind hotkey chips + held indicator.
                // Labels are put into same-height cells as the chips so their
                // baselines line up.
                const ROW_H: f32 = 24.0;
                let footer_label = |ui: &mut egui::Ui, text: &str, color: egui::Color32| {
                    ui.add_sized(
                        egui::vec2(0.0, ROW_H),
                        egui::Label::new(
                            RichText::new(text).size(13.0).color(color),
                        ),
                    );
                };
                ui.horizontal(|ui| {
                    footer_label(ui, "Toggle", theme::p().text_faint);
                    toggle_changed = widgets::rebindable_hotkey_chip(
                        ui,
                        &mut self.state.settings.toggle_hotkey,
                        &mut self.state.settings_panel.capturing_toggle,
                    );
                    ui.add_space(10.0);
                    footer_label(ui, "Record", theme::p().text_faint);
                    record_hotkey_changed = widgets::rebindable_hotkey_chip(
                        ui,
                        &mut self.state.settings.macro_record_hotkey,
                        &mut self.state.macros.capturing_record_hotkey,
                    );
                    if !self.state.held_keys.is_empty()
                        || !self.state.held_mouse.is_empty()
                    {
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                let total = self.state.held_keys.len()
                                    + self.state.held_mouse.len();
                                footer_label(
                                    ui,
                                    &format!("{} held", total),
                                    theme::p().accent,
                                );
                            },
                        );
                    }
                });
            });
        if theme_clicked {
            let next = widgets::next_theme(self.state.settings.theme);
            self.state.settings.theme = next;
            theme::set_theme(&ctx, next);
            crate::platform::apply_titlebar_theme(next, theme::p());
            let _ = self.state.settings.save();
        }
        if toggle_changed {
            hk_state::set_toggle_binding(self.state.settings.toggle_hotkey);
            let _ = self.state.settings.save();
        }
        if record_hotkey_changed {
            hk_state::set_macro_record_binding(self.state.settings.macro_record_hotkey);
            let _ = self.state.settings.save();
        }

        // 30fps repaints while running or recording — but only when the
        // window is actually visible. Skipping when hidden keeps idle CPU
        // near 0% on tray-minimized sessions.
        let visible = ctx.input(|i| i.viewport().inner_rect.is_some());
        if visible {
            if self.state.running || self.state.macros.recording {
                // 30 fps while something is actively happening.
                ctx.request_repaint_after(Duration::from_millis(33));
            } else {
                // Slow idle tick so hotkey pulses + diagnostic counters
                // update even if request_repaint from the LL hook thread
                // somehow fails to wake egui.
                ctx.request_repaint_after(Duration::from_millis(200));
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        set_recording(false);
        self.state.settings.interval_ms = self.state.rate.interval_ms;
        let _ = self.state.settings.save();
        if let Some(hooks) = self.hooks.take() {
            hooks.shutdown();
        }
        if let Some(engine) = self.engine.take() {
            engine.shutdown();
        }
        self.tray = None;
    }
}

impl App {
    fn render_content(&mut self, ui: &mut egui::Ui) {
        let cmd_tx = self.cmd_tx.clone();

        let two_col = ui.available_width() >= 540.0
            && matches!(self.state.tab, Tab::Mouse | Tab::Keyboard);

        match self.state.tab {
            Tab::Mouse => {
                let before = (
                    self.state.mouse.pattern,
                    self.state.mouse.button,
                    self.state.mouse.use_fixed_point,
                    self.state.mouse.fixed_x,
                    self.state.mouse.fixed_y,
                    self.state.mouse.mode,
                );

                if two_col {
                    symmetric_columns(ui, |cols| {
                        panel_clicker::show(&mut cols[0], &mut self.state.mouse);
                        panel_rate::show_rate(
                            &mut cols[1],
                            &mut self.state.rate,
                            &cmd_tx,
                        );
                        cols[1].add_space(6.0);
                        panel_rate::show_stop(
                            &mut cols[1],
                            &mut self.state.stop,
                            &cmd_tx,
                        );
                    });
                } else {
                    panel_clicker::show(ui, &mut self.state.mouse);
                    ui.add_space(6.0);
                    panel_rate::show_rate(ui, &mut self.state.rate, &cmd_tx);
                    ui.add_space(6.0);
                    panel_rate::show_stop(ui, &mut self.state.stop, &cmd_tx);
                }

                let after = (
                    self.state.mouse.pattern,
                    self.state.mouse.button,
                    self.state.mouse.use_fixed_point,
                    self.state.mouse.fixed_x,
                    self.state.mouse.fixed_y,
                    self.state.mouse.mode,
                );
                if before != after {
                    let a = build_action(&self.state);
                    self.state.last_action = a;
                    let _ = cmd_tx.send(Command::SetAction(a));
                }
                self.state.settings.interval_ms = self.state.rate.interval_ms;
            }
            Tab::Keyboard => {
                let before_key = self.state.keypress.selected_vk;
                let before_mods = self.state.keypress.mods;
                let before_mode = self.state.keypress.mode;

                if two_col {
                    symmetric_columns(ui, |cols| {
                        panel_keypress::show(&mut cols[0], &mut self.state.keypress);
                        panel_rate::show_rate(
                            &mut cols[1],
                            &mut self.state.rate,
                            &cmd_tx,
                        );
                        cols[1].add_space(6.0);
                        panel_rate::show_stop(
                            &mut cols[1],
                            &mut self.state.stop,
                            &cmd_tx,
                        );
                    });
                } else {
                    panel_keypress::show(ui, &mut self.state.keypress);
                    ui.add_space(6.0);
                    panel_rate::show_rate(ui, &mut self.state.rate, &cmd_tx);
                    ui.add_space(6.0);
                    panel_rate::show_stop(ui, &mut self.state.stop, &cmd_tx);
                }

                if self.state.keypress.selected_vk != before_key
                    || self.state.keypress.mods != before_mods
                    || self.state.keypress.mode != before_mode
                {
                    let a = build_action(&self.state);
                    self.state.last_action = a;
                    let _ = cmd_tx.send(Command::SetAction(a));
                }
                self.state.settings.interval_ms = self.state.rate.interval_ms;
            }
            Tab::Macros => {
                let actions = panel_macros::show(
                    ui,
                    &mut self.state.macros,
                    self.state.running,
                    &mut self.state.settings.macro_record_hotkey,
                );
                if let Some(hooks) = self.hooks.as_ref() {
                    let rx = hooks.record_rx.clone();
                    if !actions.is_empty() {
                        handle_macro_actions(&mut self.state, &cmd_tx, &rx, actions);
                    }
                }
            }
            Tab::Settings => {
                let actions = panel_settings::show(
                    ui,
                    &mut self.state.settings,
                    &mut self.state.settings_panel,
                );
                if !actions.is_empty() {
                    let ctx = ui.ctx().clone();
                    handle_settings_actions(&mut self.state, &cmd_tx, &ctx, actions);
                }
            }
        }
    }
}
