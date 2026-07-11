use std::collections::{HashMap, HashSet};

use super::command::{KeyMods, MouseButton};
use super::{keyboard, mouse};

#[derive(Default)]
pub struct VirtualHold {
    keys: HashSet<u16>,
    mouse_buttons: HashMap<u8, isize>,
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
    pub fn toggle_combo(&mut self, vk: u16, mods: KeyMods) -> bool {
        let mut combo = Vec::with_capacity(5);
        if mods.ctrl {
            combo.push(0x11);
        }
        if mods.shift {
            combo.push(0x10);
        }
        if mods.alt {
            combo.push(0x12);
        }
        if mods.win {
            combo.push(0x5B);
        }
        if !combo.contains(&vk) {
            combo.push(vk);
        }

        if self.keys.contains(&vk) {
            for key in combo.into_iter().rev() {
                if self.keys.remove(&key) {
                    keyboard::key_up(key);
                }
            }
            false
        } else {
            for key in combo {
                if self.keys.insert(key) {
                    keyboard::key_down(key);
                }
            }
            true
        }
    }

    pub fn toggle_mouse(&mut self, button: MouseButton) -> bool {
        let byte = mb_to_byte(button);
        if let Some(target) = self.mouse_buttons.remove(&byte) {
            mouse::hold_button_up(button, target);
            false
        } else {
            let target = mouse::hold_button_down(button);
            self.mouse_buttons.insert(byte, target);
            true
        }
    }

    pub fn release_all(&mut self) {
        for vk in self.keys.drain() {
            keyboard::key_up(vk);
        }
        for (b, target) in self.mouse_buttons.drain() {
            mouse::hold_button_up(byte_to_mb(b), target);
        }
    }

    pub fn held_keys(&self) -> Vec<u16> {
        let mut v: Vec<u16> = self.keys.iter().copied().collect();
        v.sort_unstable();
        v
    }

    pub fn held_mouse_buttons(&self) -> Vec<MouseButton> {
        let mut v: Vec<MouseButton> = self.mouse_buttons.keys().copied().map(byte_to_mb).collect();
        v.sort_unstable_by_key(|b| mb_to_byte(*b));
        v
    }
}

impl Drop for VirtualHold {
    fn drop(&mut self) {
        self.release_all();
    }
}
