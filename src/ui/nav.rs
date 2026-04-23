use egui::{Button, Color32, CornerRadius, Frame, Margin, RichText, Stroke, Ui, vec2};

use crate::ui::theme;

pub fn segmented<T: Copy + PartialEq>(ui: &mut Ui, current: &mut T, tabs: &[(T, &str)]) {
    Frame::NONE
        .fill(theme::p().bg_lift)
        .stroke(Stroke::new(1.0, theme::p().border))
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::same(3))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.horizontal(|ui| {
                for (tab, name) in tabs {
                    let active = *current == *tab;
                    let btn = Button::new(
                        RichText::new(*name)
                            .size(13.0)
                            .color(if active {
                                theme::p().text
                            } else {
                                theme::p().text_muted
                            })
                            .strong()
                            .extra_letter_spacing(0.4),
                    )
                    .fill(if active {
                        theme::p().surface_hi
                    } else {
                        Color32::TRANSPARENT
                    })
                    .stroke(Stroke::NONE)
                    .corner_radius(CornerRadius::same(5))
                    .min_size(vec2(70.0, 24.0));
                    let resp = ui.add(btn);
                    if resp.clicked() {
                        *current = *tab;
                    }
                    if active {
                        // Accent underline within the pill for a crisp active state.
                        let r = resp.rect;
                        let bar = egui::Rect::from_min_max(
                            egui::pos2(r.center().x - 9.0, r.max.y - 4.0),
                            egui::pos2(r.center().x + 9.0, r.max.y - 2.5),
                        );
                        ui.painter()
                            .rect_filled(bar, CornerRadius::same(1), theme::p().accent);
                    }
                }
            });
        });
}
