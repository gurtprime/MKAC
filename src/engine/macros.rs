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
        let total_duration_ms = frames.iter().fold(0u64, |total, frame| {
            total.saturating_add(frame.delta_ms as u64)
        });
        Self {
            schema: 1,
            name: name.into(),
            frames,
            total_duration_ms,
        }
    }

    pub fn recompute_duration(&mut self) {
        self.total_duration_ms = self.frames.iter().fold(0u64, |total, frame| {
            total.saturating_add(frame.delta_ms as u64)
        });
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

#[cfg(test)]
mod tests {
    use super::{Macro, MacroEvent, MacroFrame};
    use crate::engine::MouseButton;

    #[test]
    fn duration_is_saturating_and_recomputable() {
        let mut m = Macro::new(
            "test",
            vec![
                MacroFrame {
                    delta_ms: u32::MAX,
                    event: MacroEvent::MouseMove { x: 0, y: 0 },
                },
                MacroFrame {
                    delta_ms: 10,
                    event: MacroEvent::MouseDown {
                        button: MouseButton::Left,
                        x: 0,
                        y: 0,
                    },
                },
            ],
        );
        assert_eq!(m.total_duration_ms, u32::MAX as u64 + 10);
        m.total_duration_ms = 0;
        m.recompute_duration();
        assert_eq!(m.total_duration_ms, u32::MAX as u64 + 10);
    }
}
