use std::sync::OnceLock;
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::time::Instant;

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HHOOK, LLMHF_INJECTED, MSLLHOOKSTRUCT, SetWindowsHookExW, WH_MOUSE_LL,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONDOWN,
    WM_RBUTTONUP,
};

use crate::engine::command::MouseButton;
use crate::engine::macros::MacroEvent;
use crate::hooks::recording::{IS_RECORDING, push};

/// Max MouseMove sample rate during recording (~100 Hz). Windows fires
/// WM_MOUSEMOVE on every sub-pixel movement; recording all of them would
/// bloat macros and spam the channel.
const MOVE_THROTTLE_MS: u64 = 10;

static MOVE_EPOCH: OnceLock<Instant> = OnceLock::new();
static LAST_MOVE_MS: AtomicU64 = AtomicU64::new(0);
static LAST_MOVE_X: AtomicI32 = AtomicI32::new(i32::MIN);
static LAST_MOVE_Y: AtomicI32 = AtomicI32::new(i32::MIN);

pub unsafe extern "system" fn proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && IS_RECORDING.load(Ordering::Relaxed) {
        let ms = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
        let injected = (ms.flags & LLMHF_INJECTED) != 0;
        if !injected {
            let (x, y) = (ms.pt.x, ms.pt.y);
            let event = match wparam.0 as u32 {
                WM_LBUTTONDOWN => Some(MacroEvent::MouseDown { button: MouseButton::Left, x, y }),
                WM_LBUTTONUP => Some(MacroEvent::MouseUp { button: MouseButton::Left, x, y }),
                WM_RBUTTONDOWN => Some(MacroEvent::MouseDown { button: MouseButton::Right, x, y }),
                WM_RBUTTONUP => Some(MacroEvent::MouseUp { button: MouseButton::Right, x, y }),
                WM_MBUTTONDOWN => Some(MacroEvent::MouseDown { button: MouseButton::Middle, x, y }),
                WM_MBUTTONUP => Some(MacroEvent::MouseUp { button: MouseButton::Middle, x, y }),
                WM_MOUSEMOVE => {
                    let epoch = MOVE_EPOCH.get_or_init(Instant::now);
                    let now_ms = epoch.elapsed().as_millis() as u64;
                    let last_ms = LAST_MOVE_MS.load(Ordering::Relaxed);
                    let dup = LAST_MOVE_X.load(Ordering::Relaxed) == x
                        && LAST_MOVE_Y.load(Ordering::Relaxed) == y;
                    if !dup && now_ms.saturating_sub(last_ms) >= MOVE_THROTTLE_MS {
                        LAST_MOVE_MS.store(now_ms, Ordering::Relaxed);
                        LAST_MOVE_X.store(x, Ordering::Relaxed);
                        LAST_MOVE_Y.store(y, Ordering::Relaxed);
                        Some(MacroEvent::MouseMove { x, y })
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(e) = event {
                push(e);
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

pub fn install() -> Option<HHOOK> {
    unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(proc), None, 0).ok() }
}
