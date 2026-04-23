use serde::{Deserialize, Serialize};

use super::command::MouseButton;
use super::{keyboard, mouse};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum MacroEvent {
    KeyDown { vk: u16 },
    KeyUp { vk: u16 },
    MouseDown { button: MouseButton, x: i32, y: i32 },
    MouseUp { button: MouseButton, x: i32, y: i32 },
    MouseMove { x: i32, y: i32 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct MacroFrame {
    pub delta_ms: u32,
    pub event: MacroEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Macro {
    #[serde(default = "default_schema")]
    pub schema: u32,
    pub name: String,
    pub frames: Vec<MacroFrame>,
    #[serde(default)]
    pub total_duration_ms: u64,
}

fn default_schema() -> u32 {
    1
}

impl Macro {
    pub fn new(name: impl Into<String>, frames: Vec<MacroFrame>) -> Self {
        let total_duration_ms = frames.iter().map(|f| f.delta_ms as u64).sum();
        Self { schema: 1, name: name.into(), frames, total_duration_ms }
    }
}

pub fn play_event(event: &MacroEvent) {
    match *event {
        MacroEvent::KeyDown { vk } => keyboard::key_down(vk),
        MacroEvent::KeyUp { vk } => keyboard::key_up(vk),
        MacroEvent::MouseDown { button, x, y } => {
            mouse::set_cursor(x, y);
            mouse::button_down(button);
        }
        MacroEvent::MouseUp { button, x, y } => {
            mouse::set_cursor(x, y);
            mouse::button_up(button);
        }
        MacroEvent::MouseMove { x, y } => {
            mouse::set_cursor(x, y);
        }
    }
}
