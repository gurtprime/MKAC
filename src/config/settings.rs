use serde::{Deserialize, Serialize};

use super::{schema::SETTINGS_SCHEMA, settings_path};
use crate::engine::HotkeyBinding;
use crate::ui::theme::Theme;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_schema")]
    pub schema: u32,
    #[serde(default)]
    pub autostart: bool,
    #[serde(default)]
    pub close_to_tray: bool,
    #[serde(default)]
    pub start_minimized: bool,
    #[serde(default)]
    pub resizable_window: bool,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default = "default_interval")]
    pub interval_ms: u64,
    #[serde(default = "default_toggle_hk")]
    pub toggle_hotkey: HotkeyBinding,
    #[serde(default = "default_macro_record_hk")]
    pub macro_record_hotkey: HotkeyBinding,
}

fn default_schema() -> u32 {
    SETTINGS_SCHEMA
}
fn default_interval() -> u64 {
    100
}
fn default_toggle_hk() -> HotkeyBinding {
    HotkeyBinding::new(0x75) // VK_F6
}
fn default_macro_record_hk() -> HotkeyBinding {
    HotkeyBinding::new(0x77) // VK_F8
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema: SETTINGS_SCHEMA,
            autostart: false,
            close_to_tray: false,
            start_minimized: false,
            resizable_window: false,
            theme: Theme::Dark,
            interval_ms: 100,
            toggle_hotkey: default_toggle_hk(),
            macro_record_hotkey: default_macro_record_hk(),
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        if let Some(path) = settings_path() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                match serde_json::from_str::<Settings>(&data) {
                    Ok(mut s) => {
                        s.schema = SETTINGS_SCHEMA;
                        if !s.toggle_hotkey.is_set() {
                            s.toggle_hotkey = default_toggle_hk();
                        }
                        if !s.macro_record_hotkey.is_set() {
                            s.macro_record_hotkey = default_macro_record_hk();
                        }
                        return s;
                    }
                    Err(e) => {
                        eprintln!("settings.json parse failed: {e}; using defaults");
                    }
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let _ = super::ensure_dirs();
        if let Some(path) = settings_path() {
            let data = serde_json::to_string_pretty(self)?;
            std::fs::write(path, data)?;
        }
        Ok(())
    }
}
