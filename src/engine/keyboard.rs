use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC, MapVirtualKeyW, SendInput, VIRTUAL_KEY,
};

use crate::engine::command::KeyMods;
use crate::util::keycodes;

const VK_SHIFT: u16 = 0x10;
const VK_CONTROL: u16 = 0x11;
const VK_MENU: u16 = 0x12;
const VK_LWIN: u16 = 0x5B;

fn make_input(vk: u16, up: bool) -> INPUT {
    let scan = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) } as u16;
    let mut flags = KEYBD_EVENT_FLAGS(0);
    if keycodes::is_extended(vk) {
        flags = KEYBD_EVENT_FLAGS(flags.0 | KEYEVENTF_EXTENDEDKEY.0);
    }
    if up {
        flags = KEYBD_EVENT_FLAGS(flags.0 | KEYEVENTF_KEYUP.0);
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

pub fn key_down(vk: u16) {
    let i = [make_input(vk, false)];
    unsafe {
        SendInput(&i, std::mem::size_of::<INPUT>() as i32);
    }
}

pub fn key_up(vk: u16) {
    let i = [make_input(vk, true)];
    unsafe {
        SendInput(&i, std::mem::size_of::<INPUT>() as i32);
    }
}

pub fn key_tap(vk: u16, mods: KeyMods, hold_ms: u32) {
    // Phase 1 — press modifiers + the key.
    let mut down_buf: [std::mem::MaybeUninit<INPUT>; 5] =
        [const { std::mem::MaybeUninit::uninit() }; 5];
    let mut n = 0usize;
    let mut push_down = |i: INPUT| {
        down_buf[n].write(i);
        n += 1;
    };
    if mods.ctrl {
        push_down(make_input(VK_CONTROL, false));
    }
    if mods.shift {
        push_down(make_input(VK_SHIFT, false));
    }
    if mods.alt {
        push_down(make_input(VK_MENU, false));
    }
    if mods.win {
        push_down(make_input(VK_LWIN, false));
    }
    push_down(make_input(vk, false));
    let down: &[INPUT] =
        unsafe { std::slice::from_raw_parts(down_buf.as_ptr() as *const INPUT, n) };
    unsafe {
        SendInput(down, std::mem::size_of::<INPUT>() as i32);
    }

    if hold_ms > 0 {
        spin_sleep::sleep(std::time::Duration::from_millis(hold_ms as u64));
    }

    // Phase 2 — release the key first, then the modifiers in reverse order.
    let mut up_buf: [std::mem::MaybeUninit<INPUT>; 5] =
        [const { std::mem::MaybeUninit::uninit() }; 5];
    let mut m = 0usize;
    let mut push_up = |i: INPUT| {
        up_buf[m].write(i);
        m += 1;
    };
    push_up(make_input(vk, true));
    if mods.win {
        push_up(make_input(VK_LWIN, true));
    }
    if mods.alt {
        push_up(make_input(VK_MENU, true));
    }
    if mods.shift {
        push_up(make_input(VK_SHIFT, true));
    }
    if mods.ctrl {
        push_up(make_input(VK_CONTROL, true));
    }
    let up: &[INPUT] =
        unsafe { std::slice::from_raw_parts(up_buf.as_ptr() as *const INPUT, m) };
    unsafe {
        SendInput(up, std::mem::size_of::<INPUT>() as i32);
    }
}
