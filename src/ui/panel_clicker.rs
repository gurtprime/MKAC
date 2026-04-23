use egui::{DragValue, RichText, Ui};

use crate::engine::{ClickPattern, MouseButton, Target, TriggerMode, mouse};
use crate::ui::{nav, theme, widgets};

pub struct MouseConfig {
    pub button: MouseButton,
    pub pattern: ClickPattern,
    pub use_fixed_point: bool,
    pub fixed_x: i32,
    pub fixed_y: i32,
    pub mode: TriggerMode,
}

impl Default for MouseConfig {
    fn default() -> Self {
        Self {
            button: MouseButton::Left,
            pattern: ClickPattern::Single,
            use_fixed_point: false,
            fixed_x: 0,
            fixed_y: 0,
            mode: TriggerMode::Auto,
        }
    }
}

impl MouseConfig {
    pub fn target(&self) -> Target {
        if self.use_fixed_point {
            Target::FixedPoint { x: self.fixed_x, y: self.fixed_y }
        } else {
            Target::Cursor
        }
    }
}

pub fn show(ui: &mut Ui, cfg: &mut MouseConfig) {
    // EXECUTION — mode + button + pattern
    widgets::surface_card(ui, |ui| {
        widgets::card_header(ui, "EXECUTION");
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            widgets::row_label(ui, "Mode");
            ui.add_space(4.0);
            nav::segmented(
                ui,
                &mut cfg.mode,
                &[
                    (TriggerMode::Auto, "Autoclick"),
                    (TriggerMode::Hold, "Hold"),
                ],
            );
        });

        ui.add_space(6.0);

        ui.horizontal(|ui| {
            widgets::row_label(ui, "Button");
            ui.add_space(4.0);
            nav::segmented(
                ui,
                &mut cfg.button,
                &[
                    (MouseButton::Left, "Left"),
                    (MouseButton::Right, "Right"),
                    (MouseButton::Middle, "Middle"),
                ],
            );
        });

        // Pattern only applies to autoclick — hide it in hold mode.
        if matches!(cfg.mode, TriggerMode::Auto) {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                widgets::row_label(ui, "Pattern");
                ui.add_space(4.0);
                nav::segmented(
                    ui,
                    &mut cfg.pattern,
                    &[
                        (ClickPattern::Single, "1×"),
                        (ClickPattern::Double, "2×"),
                        (ClickPattern::Triple, "3×"),
                    ],
                );
            });
        }
    });

    ui.add_space(6.0);

    // POSITIONING
    widgets::surface_card(ui, |ui| {
        widgets::card_header(ui, "POSITIONING");
        ui.add_space(6.0);

        #[derive(Copy, Clone, PartialEq, Eq)]
        enum Mode {
            Cursor,
            Fixed,
        }
        let mut mode = if cfg.use_fixed_point {
            Mode::Fixed
        } else {
            Mode::Cursor
        };
        ui.horizontal(|ui| {
            widgets::row_label(ui, "Target");
            ui.add_space(4.0);
            nav::segmented(
                ui,
                &mut mode,
                &[(Mode::Cursor, "Cursor"), (Mode::Fixed, "Fixed")],
            );
        });
        cfg.use_fixed_point = matches!(mode, Mode::Fixed);

        if cfg.use_fixed_point {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("X").size(12.5).color(theme::p().text_muted));
                ui.add(DragValue::new(&mut cfg.fixed_x).speed(1.0));
                ui.add_space(6.0);
                ui.label(RichText::new("Y").size(12.5).color(theme::p().text_muted));
                ui.add(DragValue::new(&mut cfg.fixed_y).speed(1.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::ghost_button(ui, "Use cursor").clicked() {
                        if let Some((x, y)) = mouse::get_cursor_pos() {
                            cfg.fixed_x = x;
                            cfg.fixed_y = y;
                        }
                    }
                });
            });
        }
    });

}
