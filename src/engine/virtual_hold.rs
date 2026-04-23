use std::collections::HashSet;

use super::command::MouseButton;
use super::{keyboard, mouse};

#[derive(Default)]
pub struct VirtualHold {
    keys: HashSet<u16>,
    mouse_buttons: HashSet<u8>,
}

fn mb_to_byte(b: MouseButton) -> u8 {
    match b {
        MouseButton::Left => 1,
        MouseButton::Right => 2,
        MouseButton::Middle => 3,
    }
}

fn byte_to_mb(b: u8) -> MouseButton {
    match b {
        2 => MouseButton::Right,
        3 => MouseButton::Middle,
        _ => MouseButton::Left,
    }
}

impl VirtualHold {
    pub fn toggle(&mut self, vk: u16) -> bool {
        if self.keys.contains(&vk) {
            keyboard::key_up(vk);
            self.keys.remove(&vk);
            false
        } else {
            keyboard::key_down(vk);
            self.keys.insert(vk);
            true
        }
    }

    pub fn toggle_mouse(&mut self, button: MouseButton) -> bool {
        let byte = mb_to_byte(button);
        if self.mouse_buttons.contains(&byte) {
            mouse::button_up(button);
            self.mouse_buttons.remove(&byte);
            false
        } else {
            mouse::button_down(button);
            self.mouse_buttons.insert(byte);
            true
        }
    }

    pub fn release_all(&mut self) {
        for vk in self.keys.drain() {
            keyboard::key_up(vk);
        }
        for b in self.mouse_buttons.drain() {
            mouse::button_up(byte_to_mb(b));
        }
    }

    pub fn held_keys(&self) -> Vec<u16> {
        let mut v: Vec<u16> = self.keys.iter().copied().collect();
        v.sort_unstable();
        v
    }

    pub fn held_mouse_buttons(&self) -> Vec<MouseButton> {
        let mut v: Vec<MouseButton> =
            self.mouse_buttons.iter().copied().map(byte_to_mb).collect();
        v.sort_unstable_by_key(|b| mb_to_byte(*b));
        v
    }
}

impl Drop for VirtualHold {
    fn drop(&mut self) {
        self.release_all();
    }
}
