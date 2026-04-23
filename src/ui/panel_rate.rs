use crossbeam_channel::Sender;
use egui::{DragValue, RichText, Ui};

use crate::engine::{Command, JitterCurve, RateConfig, StopAfter};
use crate::ui::{nav, theme, widgets};

#[derive(Copy, Clone, PartialEq, Eq)]
enum RateMode {
    Interval,
    Cps,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum StopMode {
    Never,
    Count,
    Duration,
}

pub fn show_rate(ui: &mut Ui, rate: &mut RateConfig, cmd_tx: &Sender<Command>) {
    let before = *rate;

    widgets::surface_card(ui, |ui| {
        widgets::card_header(ui, "RATE");
        ui.add_space(6.0);

        let mut mode = if rate.use_cps {
            RateMode::Cps
        } else {
            RateMode::Interval
        };
        ui.horizontal(|ui| {
            widgets::row_label(ui, "Mode");
            ui.add_space(4.0);
            nav::segmented(
                ui,
                &mut mode,
                &[
                    (RateMode::Cps, "CPS"),
                    (RateMode::Interval, "Interval"),
                ],
            );
        });
        rate.use_cps = matches!(mode, RateMode::Cps);

        ui.add_space(6.0);

        if rate.use_cps {
            ui.horizontal(|ui| {
                widgets::row_label(ui, "Target");
                ui.add(
                    DragValue::new(&mut rate.cps)
                        .range(0.1..=200.0)
                        .suffix(" cps")
                        .speed(0.2),
                );
            });
        } else {
            ui.horizontal(|ui| {
                widgets::row_label(ui, "Every");
                ui.add(
                    DragValue::new(&mut rate.interval_ms)
                        .range(1..=60_000)
                        .suffix(" ms")
                        .speed(1.0),
                );
            });
        }
    });

    ui.add_space(6.0);

    // VARIATION card with OFF/ON
    widgets::surface_card(ui, |ui| {
        let toggled = widgets::card_header_with(ui, "VARIATION", |ui| {
            widgets::toggle_pill(ui, &mut rate.jitter_enabled)
        });
        let _ = toggled;

        if rate.jitter_enabled {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                widgets::row_label(ui, "Interval  ±");
                ui.add(
                    DragValue::new(&mut rate.jitter_max_ms)
                        .range(0..=5000)
                        .suffix(" ms"),
                );
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                widgets::row_label(ui, "Hold");
                ui.add(
                    DragValue::new(&mut rate.hold_min_ms)
                        .range(0..=500)
                        .suffix(" ms"),
                );
                ui.label(
                    RichText::new("To")
                        .size(12.0)
                        .color(theme::p().text_faint),
                );
                let mut max = rate.hold_max_ms.max(rate.hold_min_ms);
                if ui
                    .add(DragValue::new(&mut max).range(0..=500).suffix(" ms"))
                    .changed()
                {
                    rate.hold_max_ms = max;
                }
                // Keep min ≤ max.
                if rate.hold_min_ms > rate.hold_max_ms {
                    rate.hold_max_ms = rate.hold_min_ms;
                }
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                widgets::row_label(ui, "Curve");
                ui.add_space(4.0);
                nav::segmented(
                    ui,
                    &mut rate.jitter_curve,
                    &[
                        (JitterCurve::Uniform, "Uniform"),
                        (JitterCurve::Gaussian, "Gaussian"),
                    ],
                );
            });
            ui.add_space(2.0);
            ui.label(
                RichText::new(match rate.jitter_curve {
                    JitterCurve::Uniform => "Evenly-distributed random offsets",
                    JitterCurve::Gaussian => "Most offsets near zero, tails at ±max",
                })
                .size(11.5)
                .color(theme::p().text_faint),
            );
        } else {
            ui.add_space(4.0);
            ui.label(
                RichText::new("Events fire instantly on an exact cadence")
                    .size(12.0)
                    .color(theme::p().text_faint),
            );
        }
    });

    if *rate != before {
        let _ = cmd_tx.send(Command::SetRate(*rate));
    }
}

pub fn show_stop(ui: &mut Ui, stop: &mut StopAfter, cmd_tx: &Sender<Command>) {
    let before = *stop;

    widgets::surface_card(ui, |ui| {
        widgets::card_header(ui, "STOP AFTER");
        ui.add_space(6.0);

        let mut mode = match stop {
            StopAfter::Never => StopMode::Never,
            StopAfter::Count { .. } => StopMode::Count,
            StopAfter::Duration { .. } => StopMode::Duration,
        };
        let prev_mode = mode;
        ui.horizontal(|ui| {
            widgets::row_label(ui, "Mode");
            ui.add_space(4.0);
            nav::segmented(
                ui,
                &mut mode,
                &[
                    (StopMode::Never, "Never"),
                    (StopMode::Count, "Count"),
                    (StopMode::Duration, "Duration"),
                ],
            );
        });
        if mode != prev_mode {
            *stop = match mode {
                StopMode::Never => StopAfter::Never,
                StopMode::Count => StopAfter::Count { n: 100 },
                StopMode::Duration => StopAfter::Duration { ms: 30_000 },
            };
        }

        match stop {
            StopAfter::Never => {
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Runs until stopped manually")
                        .size(12.0)
                        .color(theme::p().text_faint),
                );
            }
            StopAfter::Count { n } => {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    widgets::row_label(ui, "Stop at");
                    ui.add(DragValue::new(n).range(1..=1_000_000).speed(1.0));
                    ui.label(RichText::new("Events").size(12.5).color(theme::p().text_muted));
                });
            }
            StopAfter::Duration { ms } => {
                ui.add_space(6.0);
                let mut seconds = (*ms as f32) / 1000.0;
                ui.horizontal(|ui| {
                    widgets::row_label(ui, "Stop at");
                    if ui
                        .add(
                            DragValue::new(&mut seconds)
                                .range(0.1..=3600.0)
                                .suffix(" s")
                                .speed(0.1),
                        )
                        .changed()
                    {
                        *ms = (seconds * 1000.0).round() as u64;
                    }
                });
            }
        }
    });

    if *stop != before {
        let _ = cmd_tx.send(Command::SetStopAfter(*stop));
    }
}
