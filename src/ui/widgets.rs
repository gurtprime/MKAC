use egui::{
    Button, Color32, CornerRadius, Frame, Margin, Response, RichText, Sense, Shadow, Stroke, Ui,
    vec2,
};

use crate::engine::HotkeyBinding;
use crate::ui::theme::{self, Theme};
use crate::util::keycodes;

pub fn format_binding(b: &HotkeyBinding) -> String {
    if !b.is_set() {
        return "—".into();
    }
    let mut parts: Vec<&str> = Vec::new();
    if b.ctrl {
        parts.push("Ctrl");
    }
    if b.shift {
        parts.push("Shift");
    }
    if b.alt {
        parts.push("Alt");
    }
    if b.win {
        parts.push("Win");
    }
    let name = keycodes::lookup_name(b.vk).unwrap_or("?");
    if parts.is_empty() {
        name.to_string()
    } else {
        format!("{}+{}", parts.join("+"), name)
    }
}

/// Checkbox rendered as a solid accent-filled square when on, hollow box
/// when off. Replacement for `ui.checkbox` — clearer at a glance than the
/// tiny stock checkmark.
pub fn checkbox(ui: &mut Ui, value: &mut bool, label: &str) -> Response {
    const BOX: f32 = 16.0;
    const GAP: f32 = 7.0;
    const ROW_H: f32 = 24.0;

    let font_id = egui::FontId::new(14.0, egui::FontFamily::Proportional);
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_string(), font_id, theme::p().text);
    let size = vec2(BOX + GAP + galley.size().x, ROW_H);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.clicked() {
        *value = !*value;
    }

    let box_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left(), rect.center().y - BOX * 0.5),
        vec2(BOX, BOX),
    );
    let p = ui.painter();
    let radius = CornerRadius::same(4);
    if *value {
        let fill = if resp.hovered() {
            theme::p().accent.gamma_multiply(0.85)
        } else {
            theme::p().accent
        };
        p.rect_filled(box_rect, radius, fill);
        // Subtle darker outline so the fill reads as a distinct button and
        // not a solid block touching its neighbors.
        p.rect_stroke(
            box_rect,
            radius,
            Stroke::new(1.0, theme::p().accent.gamma_multiply(0.55)),
            egui::StrokeKind::Inside,
        );
    } else {
        let fill = if resp.hovered() {
            theme::p().hover
        } else {
            theme::p().surface
        };
        p.rect_filled(box_rect, radius, fill);
        p.rect_stroke(
            box_rect,
            radius,
            Stroke::new(1.0, theme::p().border_hi),
            egui::StrokeKind::Inside,
        );
    }

    let text_pos = egui::pos2(
        box_rect.right() + GAP,
        rect.center().y - galley.size().y * 0.5,
    );
    p.galley(text_pos, galley, theme::p().text);
    resp
}

/// Row label that occupies the same vertical cell-height as an adjacent
/// segmented control / button, with the text manually centered. Fixes the
/// "label sits slightly higher than the buttons" bug you get from plain
/// `ui.label()` in a `ui.horizontal` next to a taller widget.
pub fn row_label(ui: &mut Ui, text: &str) {
    row_label_colored(ui, text, theme::p().text_muted);
}

fn row_label_colored(ui: &mut Ui, text: &str, color: Color32) {
    // 30 = segmented control's outer height (inner button 24 + margin 3 + 3).
    const ROW_H: f32 = 30.0;
    let font_id = egui::FontId::new(13.0, egui::FontFamily::Proportional);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font_id, color);
    let size = vec2(galley.size().x, ROW_H);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let pos = egui::pos2(
        rect.left(),
        rect.center().y - galley.size().y * 0.5,
    );
    ui.painter().galley(pos, galley, color);
}

/// Chip that captures a plain keypress — no modifiers tracked. Used for
/// picking the target of an autopress action ("which key should MKAC
/// spam"). Single-click enters capture because this lives inside a panel
/// where the user is actively configuring, not in the footer where a
/// focus-steal click could hit it.
pub fn rebindable_key_chip(ui: &mut Ui, vk: &mut u16, capturing: &mut bool) -> bool {
    let mut changed = false;

    if *capturing {
        let captured: Option<u16> = ui.ctx().input(|i| {
            for ev in &i.events {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    ..
                } = ev
                {
                    if *key == egui::Key::Escape {
                        return Some(0); // sentinel: cancel without changing
                    }
                    if let Some(v) = keycodes::egui_key_to_vk(*key) {
                        return Some(v);
                    }
                }
            }
            None
        });
        if let Some(v) = captured {
            if v != 0 && *vk != v {
                *vk = v;
                changed = true;
            }
            *capturing = false;
        }

        let btn = Button::new(
            RichText::new("Press a key…")
                .size(12.5)
                .monospace()
                .color(theme::p().accent)
                .strong(),
        )
        .fill(theme::p().hover)
        .stroke(Stroke::new(1.0, theme::p().accent))
        .corner_radius(CornerRadius::same(4))
        .min_size(vec2(110.0, 24.0));
        let resp = ui.add(btn);
        if resp.clicked_elsewhere() {
            *capturing = false;
        }
    } else {
        let name = keycodes::lookup_name(*vk).unwrap_or("?");
        let btn = Button::new(
            RichText::new(name)
                .size(13.0)
                .monospace()
                .color(theme::p().text)
                .strong(),
        )
        .fill(theme::p().surface)
        .stroke(Stroke::new(1.0, theme::p().border_hi))
        .corner_radius(CornerRadius::same(4))
        .min_size(vec2(110.0, 24.0));
        let resp = ui.add(btn).on_hover_text("Click to rebind");
        if resp.clicked() {
            *capturing = true;
        }
    }

    changed
}

/// Keycap chip with an inline "rebind" affordance. The chip itself is a
/// plain label — no click-to-capture. Rebinding is opt-in:
///   - Right-click the chip, OR
///   - Click the adjacent small "rebind" ghost button.
/// Both require deliberate intent, so a focus-steal click on MKAC cannot
/// flip the app into capture mode. (That was the root cause of the
/// "hotkey works unfocused, not focused" bug: the click that gave MKAC
/// focus would land on/near the chip and silently enter rebind, making
/// `REBIND_ACTIVE` true and suppressing the LL hook dispatch on the next
/// press of the bound key.)
pub fn rebindable_hotkey_chip(
    ui: &mut Ui,
    binding: &mut HotkeyBinding,
    capturing: &mut bool,
) -> bool {
    let mut changed = false;

    if *capturing {
        let captured: Option<HotkeyBinding> = ui.ctx().input(|i| {
            for ev in &i.events {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    repeat: false,
                    ..
                } = ev
                {
                    if *key == egui::Key::Escape
                        && !modifiers.ctrl
                        && !modifiers.shift
                        && !modifiers.alt
                    {
                        return Some(HotkeyBinding::default());
                    }
                    if let Some(vk) = keycodes::egui_key_to_vk(*key) {
                        return Some(HotkeyBinding {
                            vk,
                            ctrl: modifiers.ctrl,
                            shift: modifiers.shift,
                            alt: modifiers.alt,
                            win: false,
                        });
                    }
                }
            }
            None
        });
        if let Some(b) = captured {
            if b.is_set() {
                *binding = b;
                changed = true;
            }
            *capturing = false;
        }

        let btn = Button::new(
            RichText::new("Press a key…")
                .size(12.0)
                .monospace()
                .color(theme::p().accent)
                .strong(),
        )
        .fill(theme::p().hover)
        .stroke(Stroke::new(1.0, theme::p().accent))
        .corner_radius(CornerRadius::same(4))
        .min_size(vec2(0.0, 24.0));
        let resp = ui.add(btn);
        if resp.clicked_elsewhere() {
            *capturing = false;
        }
        ui.add_space(4.0);
        if ghost_button(ui, "Cancel").clicked() {
            *capturing = false;
        }
    } else {
        let btn = Button::new(
            RichText::new(format_binding(binding))
                .size(12.5)
                .monospace()
                .color(theme::p().text)
                .strong(),
        )
        .fill(theme::p().hover)
        .stroke(Stroke::new(1.0, theme::p().border_hi))
        .corner_radius(CornerRadius::same(4))
        .min_size(vec2(0.0, 24.0));
        let resp = ui
            .add(btn)
            .on_hover_text("Right-click or hit the rebind button to change");
        if resp.secondary_clicked() {
            *capturing = true;
        }
        ui.add_space(4.0);
        let rebind_btn = Button::new(
            RichText::new("Rebind")
                .size(11.5)
                .color(theme::p().text_muted),
        )
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0, theme::p().border))
        .corner_radius(CornerRadius::same(4))
        .min_size(vec2(0.0, 24.0));
        if ui.add(rebind_btn).clicked() {
            *capturing = true;
        }
    }

    changed
}

/// Accent bar + ALL CAPS title, followed by a thin hairline divider.
pub fn card_header(ui: &mut Ui, title: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(3.0, 12.0), Sense::hover());
        ui.painter()
            .rect_filled(rect, CornerRadius::same(1), theme::p().accent);
        ui.add_space(7.0);
        ui.label(
            RichText::new(title)
                .size(12.0)
                .color(theme::p().text)
                .strong()
                .extra_letter_spacing(1.8),
        );
    });
    header_divider(ui);
}

/// Card heading with a trailing right-aligned section (e.g. an OFF/ON toggle).
pub fn card_header_with<R>(
    ui: &mut Ui,
    title: &str,
    trailing: impl FnOnce(&mut Ui) -> R,
) -> R {
    let mut result = None;
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(3.0, 12.0), Sense::hover());
        ui.painter()
            .rect_filled(rect, CornerRadius::same(1), theme::p().accent);
        ui.add_space(7.0);
        ui.label(
            RichText::new(title)
                .size(12.0)
                .color(theme::p().text)
                .strong()
                .extra_letter_spacing(1.8),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            result = Some(trailing(ui));
        });
    });
    header_divider(ui);
    result.unwrap()
}

fn header_divider(ui: &mut Ui) {
    ui.add_space(5.0);
    let (_, rect) = ui.allocate_space(vec2(ui.available_width(), 1.0));
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        Stroke::new(1.0, theme::p().border),
    );
    ui.add_space(2.0);
}

pub fn surface_card(ui: &mut Ui, contents: impl FnOnce(&mut Ui)) {
    Frame::NONE
        .fill(theme::p().surface)
        .stroke(Stroke::new(1.0, theme::p().border))
        .corner_radius(CornerRadius::same(10))
        .shadow(Shadow {
            offset: [0, 2],
            blur: 10,
            spread: 0,
            color: Color32::from_black_alpha(60),
        })
        .inner_margin(Margin::symmetric(14, 12))
        .show(ui, contents);
}

/// Compact icon button that swaps the current theme when clicked. Paints the
/// sun/moon glyph directly so we don't depend on the active font having them.
pub fn theme_toggle(ui: &mut Ui, current: Theme) -> Response {
    let size = vec2(34.0, 26.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());

    let p = ui.painter();
    let bg = if resp.hovered() {
        theme::p().hover
    } else {
        theme::p().surface
    };
    p.rect_filled(rect, CornerRadius::same(6), bg);
    p.rect_stroke(
        rect,
        CornerRadius::same(6),
        Stroke::new(1.0, theme::p().border),
        egui::StrokeKind::Inside,
    );

    let c = rect.center();
    let fg = theme::p().text_muted;
    match current {
        Theme::Dark => {
            // ☀ — small filled disc + 8 rays
            p.circle_filled(c, 3.5, fg);
            let stroke = Stroke::new(1.3, fg);
            for i in 0..8 {
                let a = (i as f32) * std::f32::consts::TAU / 8.0;
                let (sx, sy) = (a.cos(), a.sin());
                let start = c + egui::vec2(sx, sy) * 5.5;
                let end = c + egui::vec2(sx, sy) * 8.0;
                p.line_segment([start, end], stroke);
            }
        }
        Theme::Light => {
            // ☾ — disc with a smaller bg-colored disc carved out
            p.circle_filled(c, 6.5, fg);
            p.circle_filled(c + egui::vec2(2.5, -2.5), 5.5, bg);
        }
    }

    resp.on_hover_text(match current {
        Theme::Dark => "Switch to light mode",
        Theme::Light => "Switch to dark mode",
    })
}

/// Helper: which theme to switch TO, given the current one.
pub fn next_theme(current: Theme) -> Theme {
    match current {
        Theme::Dark => Theme::Light,
        Theme::Light => Theme::Dark,
    }
}

pub fn ghost_button(ui: &mut Ui, label: &str) -> Response {
    let btn = Button::new(RichText::new(label).size(13.0).color(theme::p().text))
        .fill(theme::p().surface)
        .stroke(Stroke::new(1.0, theme::p().border))
        .corner_radius(CornerRadius::same(5))
        .min_size(vec2(0.0, 22.0));
    ui.add(btn)
}

/// OFF/ON pill toggle (miniature segmented control).
pub fn toggle_pill(ui: &mut Ui, on: &mut bool) -> bool {
    let before = *on;
    Frame::NONE
        .fill(theme::p().hover)
        .stroke(Stroke::new(1.0, theme::p().border))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::same(2))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.horizontal(|ui| {
                let off_btn = Button::new(
                    RichText::new("OFF")
                        .size(11.0)
                        .color(if !*on {
                            theme::p().text
                        } else {
                            theme::p().text_faint
                        })
                        .strong(),
                )
                .fill(if !*on {
                    theme::p().surface
                } else {
                    Color32::TRANSPARENT
                })
                .stroke(Stroke::NONE)
                .corner_radius(CornerRadius::same(3))
                .min_size(vec2(26.0, 14.0));
                if ui.add(off_btn).clicked() {
                    *on = false;
                }
                let on_btn = Button::new(
                    RichText::new("ON")
                        .size(11.0)
                        .color(if *on {
                            theme::p().on_accent
                        } else {
                            theme::p().text_faint
                        })
                        .strong(),
                )
                .fill(if *on { theme::p().accent } else { Color32::TRANSPARENT })
                .stroke(Stroke::NONE)
                .corner_radius(CornerRadius::same(3))
                .min_size(vec2(26.0, 14.0));
                if ui.add(on_btn).clicked() {
                    *on = true;
                }
            });
        });
    *on != before
}


