use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use egui::{
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke,
    TextStyle, Visuals, style::Spacing,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

/// Runtime color palette. All UI code reads through `theme::p()` so the same
/// rendering code paths work for both dark and light mode without touching
/// the font atlases (which are color-agnostic).
pub struct Palette {
    pub bg: Color32,
    pub bg_lift: Color32,
    pub surface: Color32,
    pub surface_soft: Color32,
    /// Elevated surface. In both modes this is the "raised" color for active
    /// segmented items vs. the container (bg_lift): brighter than bg_lift in
    /// dark, whiter than bg_lift in light.
    pub surface_hi: Color32,
    pub border: Color32,
    pub border_hi: Color32,
    /// Background used when a button/widget is hovered. Kept separate from
    /// `surface_hi` because on light cards the "white" active color would
    /// make hover invisible.
    pub hover: Color32,
    pub text: Color32,
    pub text_muted: Color32,
    pub text_faint: Color32,
    pub accent: Color32,
    pub danger: Color32,
    /// Dark text color appropriate for painting on top of the accent (e.g.
    /// toggle-pill "ON" label). Flips between dark-on-light and near-white
    /// depending on the palette.
    pub on_accent: Color32,
}

static PALETTES: [Palette; 2] = [
    // Dark
    Palette {
        bg: Color32::from_rgb(0x07, 0x08, 0x0B),
        bg_lift: Color32::from_rgb(0x0B, 0x0D, 0x12),
        surface: Color32::from_rgb(0x11, 0x13, 0x18),
        surface_soft: Color32::from_rgb(0x14, 0x16, 0x1C),
        surface_hi: Color32::from_rgb(0x1C, 0x1E, 0x25),
        border: Color32::from_rgb(0x23, 0x25, 0x2C),
        border_hi: Color32::from_rgb(0x3B, 0x3D, 0x47),
        hover: Color32::from_rgb(0x22, 0x24, 0x2C),
        text: Color32::from_rgb(0xF0, 0xF0, 0xF3),
        text_muted: Color32::from_rgb(0x9A, 0x9B, 0xA4),
        text_faint: Color32::from_rgb(0x62, 0x63, 0x6B),
        accent: Color32::from_rgb(0x22, 0xC5, 0x5E),
        danger: Color32::from_rgb(0xEF, 0x44, 0x44),
        on_accent: Color32::from_rgb(0x08, 0x0C, 0x0A),
    },
    // Light
    Palette {
        bg: Color32::from_rgb(0xF4, 0xF5, 0xF7),
        // Container bg — darker gray so the active white segment stands out.
        bg_lift: Color32::from_rgb(0xDF, 0xE1, 0xE7),
        surface: Color32::from_rgb(0xFF, 0xFF, 0xFF),
        surface_soft: Color32::from_rgb(0xF8, 0xF9, 0xFB),
        // Active elevated surface — white, clearly brighter than the gray
        // bg_lift container.
        surface_hi: Color32::from_rgb(0xFF, 0xFF, 0xFF),
        border: Color32::from_rgb(0xD6, 0xD7, 0xDD),
        border_hi: Color32::from_rgb(0xB4, 0xB5, 0xBD),
        // Gray for hovering buttons on white cards — has to be noticeably
        // darker than #FFFFFF or the state change disappears.
        hover: Color32::from_rgb(0xC2, 0xC6, 0xD0),
        text: Color32::from_rgb(0x18, 0x19, 0x1C),
        text_muted: Color32::from_rgb(0x5A, 0x5B, 0x63),
        text_faint: Color32::from_rgb(0x9A, 0x9B, 0xA3),
        accent: Color32::from_rgb(0x16, 0xA3, 0x4A),
        danger: Color32::from_rgb(0xDC, 0x26, 0x26),
        on_accent: Color32::from_rgb(0xFA, 0xFA, 0xFC),
    },
];

static CURRENT: AtomicU8 = AtomicU8::new(0);

#[inline]
pub fn p() -> &'static Palette {
    &PALETTES[CURRENT.load(Ordering::Relaxed) as usize]
}

pub fn current() -> Theme {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Theme::Light,
        _ => Theme::Dark,
    }
}

fn set_current(theme: Theme) {
    CURRENT.store(
        match theme {
            Theme::Dark => 0,
            Theme::Light => 1,
        },
        Ordering::Relaxed,
    );
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let inter_semibold: &'static [u8] =
        include_bytes!("../../assets/fonts/Inter-SemiBold.ttf");
    let inter_medium: &'static [u8] =
        include_bytes!("../../assets/fonts/Inter-Medium.ttf");

    fonts.font_data.insert(
        "inter-semibold".into(),
        Arc::new(FontData::from_static(inter_semibold)),
    );
    fonts.font_data.insert(
        "inter-medium".into(),
        Arc::new(FontData::from_static(inter_medium)),
    );

    // Inter for everything — proportional AND monospace slots. The app has no
    // actual monospace content; we just reuse Inter for any `.monospace()`
    // RichText calls so they don't fall back to egui's default Hack font.
    {
        let fam = fonts.families.entry(FontFamily::Proportional).or_default();
        fam.insert(0, "inter-semibold".into());
        fam.insert(1, "inter-medium".into());
    }
    {
        let fam = fonts.families.entry(FontFamily::Monospace).or_default();
        fam.insert(0, "inter-semibold".into());
        fam.insert(1, "inter-medium".into());
    }

    ctx.set_fonts(fonts);
}

/// Build the `Visuals` block from the currently-selected palette.
fn make_visuals() -> Visuals {
    let pal = p();
    let mut v = match current() {
        Theme::Dark => Visuals::dark(),
        Theme::Light => Visuals::light(),
    };
    v.override_text_color = Some(pal.text);
    v.window_fill = pal.bg;
    v.panel_fill = pal.bg;
    v.extreme_bg_color = pal.surface;
    v.faint_bg_color = pal.surface_soft;
    v.code_bg_color = pal.surface_hi;
    v.window_stroke = Stroke::new(1.0, pal.border);
    v.hyperlink_color = pal.accent;

    let r = CornerRadius::same(5);
    let r_lg = CornerRadius::same(10);

    v.widgets.noninteractive.bg_fill = pal.bg;
    v.widgets.noninteractive.weak_bg_fill = pal.bg;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, pal.text);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, pal.border);
    v.widgets.noninteractive.corner_radius = r;

    v.widgets.inactive.bg_fill = pal.surface;
    v.widgets.inactive.weak_bg_fill = pal.surface;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, pal.text);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, pal.border);
    v.widgets.inactive.corner_radius = r;

    v.widgets.hovered.bg_fill = pal.hover;
    v.widgets.hovered.weak_bg_fill = pal.hover;
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, pal.text);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, pal.border_hi);
    v.widgets.hovered.corner_radius = r;

    v.widgets.active.bg_fill = pal.hover;
    v.widgets.active.weak_bg_fill = pal.hover;
    v.widgets.active.fg_stroke = Stroke::new(1.0, pal.text);
    v.widgets.active.bg_stroke = Stroke::new(1.5, pal.accent);
    v.widgets.active.corner_radius = r;

    v.widgets.open.bg_fill = pal.surface_hi;
    v.widgets.open.weak_bg_fill = pal.surface_hi;
    v.widgets.open.fg_stroke = Stroke::new(1.0, pal.text);
    v.widgets.open.bg_stroke = Stroke::new(1.0, pal.border_hi);
    v.widgets.open.corner_radius = r;

    v.selection.bg_fill = pal.accent.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, pal.accent);

    v.window_corner_radius = r_lg;
    v.menu_corner_radius = CornerRadius::same(6);

    v
}

fn apply_style(ctx: &egui::Context) {
    let mut style = (*ctx.global_style()).clone();
    style.visuals = make_visuals();

    let mut spacing = Spacing::default();
    spacing.item_spacing = egui::vec2(9.0, 7.0);
    spacing.button_padding = egui::vec2(11.0, 6.0);
    spacing.interact_size = egui::vec2(32.0, 22.0);
    spacing.window_margin = Margin::same(14);
    spacing.menu_margin = Margin::same(8);
    spacing.indent = 12.0;
    spacing.slider_width = 150.0;
    style.spacing = spacing;

    style.text_styles = [
        (TextStyle::Heading, FontId::new(17.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
        (TextStyle::Button, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
    ]
    .into();

    ctx.set_global_style(style);
}

/// One-time setup at app launch.
pub fn install(ctx: &egui::Context, theme: Theme) {
    install_fonts(ctx);
    set_current(theme);
    apply_style(ctx);
}

/// Cheap runtime theme swap — no font reload, just re-applies the style.
pub fn set_theme(ctx: &egui::Context, theme: Theme) {
    set_current(theme);
    apply_style(ctx);
}
