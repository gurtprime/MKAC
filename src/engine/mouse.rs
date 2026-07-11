use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    MOUSEINPUT, MOUSE_EVENT_FLAGS,
};

use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetForegroundWindow, PostMessageW, SetCursorPos, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MBUTTONDOWN, WM_MBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP,
};

use super::command::{ClickPattern, MouseButton, Target};

fn button_flags(button: MouseButton, up: bool) -> MOUSE_EVENT_FLAGS {
    match (button, up) {
        (MouseButton::Left, false) => MOUSEEVENTF_LEFTDOWN,
        (MouseButton::Left, true) => MOUSEEVENTF_LEFTUP,
        (MouseButton::Right, false) => MOUSEEVENTF_RIGHTDOWN,
        (MouseButton::Right, true) => MOUSEEVENTF_RIGHTUP,
        (MouseButton::Middle, false) => MOUSEEVENTF_MIDDLEDOWN,
        (MouseButton::Middle, true) => MOUSEEVENTF_MIDDLEUP,
    }
}

fn make_input(button: MouseButton, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: button_flags(button, up),
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

pub fn click(button: MouseButton, pattern: ClickPattern, target: Target, hold_ms: u32) {
    if let Target::FixedPoint { x, y } = target {
        unsafe {
            let _ = SetCursorPos(x, y);
        }
    }

    let count = match pattern {
        ClickPattern::Single => 1,
        ClickPattern::Double => 2,
        ClickPattern::Triple => 3,
    };

    // When hold_ms > 0 we split down/up so the button actually stays held
    // between them. When zero, we batch the whole sequence into one SendInput
    // for max responsiveness (original behavior).
    if hold_ms == 0 {
        let mut inputs: Vec<INPUT> = Vec::with_capacity(count * 2);
        for _ in 0..count {
            inputs.push(make_input(button, false));
            inputs.push(make_input(button, true));
        }
        unsafe {
            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }
        return;
    }

    let down = [make_input(button, false)];
    let up = [make_input(button, true)];
    for i in 0..count {
        unsafe {
            SendInput(&down, std::mem::size_of::<INPUT>() as i32);
        }
        spin_sleep::sleep(std::time::Duration::from_millis(hold_ms as u64));
        unsafe {
            SendInput(&up, std::mem::size_of::<INPUT>() as i32);
        }
        // Small gap between presses in multi-click so the OS distinguishes them.
        if i + 1 < count {
            spin_sleep::sleep(std::time::Duration::from_millis(5));
        }
    }
}

pub fn button_down(button: MouseButton) {
    let inputs = [make_input(button, false)];
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

pub fn button_up(button: MouseButton) {
    let inputs = [make_input(button, true)];
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Hold-mode mouse injection for games that do not preserve injected mouse
/// state from `SendInput`. Returns the target HWND so release can be delivered
/// to the same game even if focus changes before the toggle-off hotkey.
pub fn hold_button_down(button: MouseButton) -> isize {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        button_down(button);
        return 0;
    }

    let mut point = POINT { x: 0, y: 0 };
    let _ = unsafe { GetCursorPos(&mut point) };
    let _ = unsafe { ScreenToClient(hwnd, &mut point) };
    let message = match button {
        MouseButton::Left => WM_LBUTTONDOWN,
        MouseButton::Right => WM_RBUTTONDOWN,
        MouseButton::Middle => WM_MBUTTONDOWN,
    };
    let wparam = WPARAM(match button {
        MouseButton::Left => 0x0001,
        MouseButton::Right => 0x0002,
        MouseButton::Middle => 0x0010,
    });
    let lparam = pack_mouse_point(point);
    if unsafe { PostMessageW(Some(hwnd), message, wparam, lparam) }.is_ok() {
        hwnd.0 as isize
    } else {
        button_down(button);
        0
    }
}

pub fn hold_button_up(button: MouseButton, target: isize) {
    if target == 0 {
        button_up(button);
        return;
    }
    let hwnd = HWND(target as *mut _);
    let message = match button {
        MouseButton::Left => WM_LBUTTONUP,
        MouseButton::Right => WM_RBUTTONUP,
        MouseButton::Middle => WM_MBUTTONUP,
    };
    let _ = unsafe { PostMessageW(Some(hwnd), message, WPARAM(0), LPARAM(0)) };
}

fn pack_mouse_point(point: POINT) -> LPARAM {
    let x = (point.x as i16 as u16) as u32;
    let y = (point.y as i16 as u16) as u32;
    LPARAM((x | (y << 16)) as isize)
}

pub fn set_cursor(x: i32, y: i32) {
    unsafe {
        let _ = SetCursorPos(x, y);
    }
}

pub fn get_cursor_pos() -> Option<(i32, i32)> {
    let mut p = POINT { x: 0, y: 0 };
    unsafe {
        GetCursorPos(&mut p).ok()?;
    }
    Some((p.x, p.y))
}
