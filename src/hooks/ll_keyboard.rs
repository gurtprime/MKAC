use std::sync::atomic::Ordering;

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HHOOK, KBDLLHOOKSTRUCT, LLKHF_INJECTED, SetWindowsHookExW, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use crate::engine::macros::MacroEvent;
use crate::hooks::hotkey;
use crate::hooks::recording::{IS_RECORDING, push};

/// LL keyboard hook. Hotkey dispatch is handled by RegisterHotKey on the
/// hook thread now — this proc only captures key events for macro recording.
/// (Windows 11 suppresses LL hook delivery for events destined to the hook
/// owner's own foreground window, so relying on it for hotkeys broke when
/// MKAC was focused. RegisterHotKey takes a separate input path that isn't
/// subject to that suppression.)
pub unsafe extern "system" fn proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kbd = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let injected = (kbd.flags.0 & LLKHF_INJECTED.0) != 0;
        if !injected {
            let vk = kbd.vkCode as u16;
            let wp = wparam.0 as u32;
            let is_down = matches!(wp, WM_KEYDOWN | WM_SYSKEYDOWN);
            let is_up = matches!(wp, WM_KEYUP | WM_SYSKEYUP);

            // Skip bound hotkey keys so the hotkey press that toggles
            // recording doesn't get captured as a recorded key.
            if IS_RECORDING.load(Ordering::Relaxed) && !hotkey::is_bound_vk(vk) {
                let event = if is_down {
                    Some(MacroEvent::KeyDown { vk })
                } else if is_up {
                    Some(MacroEvent::KeyUp { vk })
                } else {
                    None
                };
                if let Some(e) = event {
                    push(e);
                }
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

pub fn install() -> Option<HHOOK> {
    unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(proc), None, 0).ok() }
}
