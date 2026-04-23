//! Thin Win32 shims used by the UI layer. Kept here so the rest of the code
//! stays platform-agnostic at the surface.

use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Dwm::{
    DWMWA_CAPTION_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute,
};
use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
use windows::core::PCWSTR;

use crate::ui::theme::{Palette, Theme};

/// Make the native title bar match the in-app theme.
///
/// - `DWMWA_USE_IMMERSIVE_DARK_MODE` (attr 20) toggles dark caption/controls on
///   Win10 1809+ and Win11 — so the min/maximize/close button glyphs flip
///   color appropriately.
/// - `DWMWA_CAPTION_COLOR` (attr 35) paints the caption bar with a specific
///   color. Win11 22000+ only; older builds ignore it silently.
///
/// Called every time the theme changes and once on the first frame. Failures
/// are swallowed — this is cosmetic.
pub fn apply_titlebar_theme(theme: Theme, pal: &Palette) {
    let title: Vec<u16> = "MKAC".encode_utf16().chain(std::iter::once(0)).collect();
    let hwnd = unsafe { FindWindowW(PCWSTR::null(), PCWSTR(title.as_ptr())) };
    let hwnd = match hwnd {
        Ok(h) if !h.0.is_null() => h,
        _ => return,
    };

    // DWM expects a Win32 BOOL (4-byte signed int).
    let dark: i32 = matches!(theme, Theme::Dark) as i32;
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark as *const _ as *const _,
            std::mem::size_of::<i32>() as u32,
        );
    }

    // COLORREF is 0x00BBGGRR — Windows expects BGR packing.
    let bg = pal.bg;
    let col = COLORREF(
        (bg.r() as u32) | ((bg.g() as u32) << 8) | ((bg.b() as u32) << 16),
    );
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_COLOR,
            &col as *const _ as *const _,
            std::mem::size_of::<COLORREF>() as u32,
        );
    }
}
