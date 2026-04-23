use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::macros::Macro;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyMods {
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub win: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MouseButton {
    #[default]
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ClickPattern {
    #[default]
    Single,
    Double,
    Triple,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind")]
pub enum Target {
    #[default]
    Cursor,
    FixedPoint {
        x: i32,
        y: i32,
    },
}

/// What the hotkey does for a mouse/keyboard action.
/// - `Auto`: toggle the autoclick/autopress loop on hotkey press.
/// - `Hold`: toggle a virtual latch (button/key stays down) on hotkey press.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TriggerMode {
    #[default]
    Auto,
    Hold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Action {
    MouseClick {
        #[serde(default)]
        button: MouseButton,
        #[serde(default)]
        pattern: ClickPattern,
        #[serde(default)]
        target: Target,
        #[serde(default)]
        mode: TriggerMode,
    },
    KeyTap {
        vk: u16,
        #[serde(default)]
        mods: KeyMods,
        #[serde(default)]
        mode: TriggerMode,
    },
    PlayMacro,
}

impl Default for Action {
    fn default() -> Self {
        Self::MouseClick {
            button: MouseButton::Left,
            pattern: ClickPattern::Single,
            target: Target::Cursor,
            mode: TriggerMode::Auto,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum JitterCurve {
    #[default]
    Uniform,
    Gaussian,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RateConfig {
    pub interval_ms: u64,
    #[serde(default = "default_use_cps")]
    pub use_cps: bool,
    #[serde(default = "default_cps")]
    pub cps: f32,
    #[serde(default)]
    pub jitter_enabled: bool,
    #[serde(default)]
    pub jitter_max_ms: u32,
    #[serde(default)]
    pub jitter_curve: JitterCurve,
    /// Minimum key-down → key-up duration (ms) per event when jitter is on.
    #[serde(default)]
    pub hold_min_ms: u32,
    /// Maximum key-down → key-up duration (ms) per event when jitter is on.
    /// Set equal to `hold_min_ms` for a constant hold duration.
    #[serde(default = "default_hold_max_ms")]
    pub hold_max_ms: u32,
}

fn default_cps() -> f32 {
    10.0
}
fn default_use_cps() -> bool {
    true
}
fn default_hold_max_ms() -> u32 {
    20
}

impl Default for RateConfig {
    fn default() -> Self {
        Self {
            interval_ms: 100,
            use_cps: true,
            cps: 10.0,
            jitter_enabled: false,
            jitter_max_ms: 0,
            jitter_curve: JitterCurve::Uniform,
            hold_min_ms: 0,
            hold_max_ms: 20,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind")]
pub enum StopAfter {
    #[default]
    Never,
    Count {
        n: u64,
    },
    Duration {
        ms: u64,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotkeyBinding {
    #[serde(default)]
    pub vk: u16,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub win: bool,
}

impl HotkeyBinding {
    pub fn new(vk: u16) -> Self {
        Self { vk, ctrl: false, shift: false, alt: false, win: false }
    }

    pub fn is_set(&self) -> bool {
        self.vk != 0
    }

    pub fn pack(&self) -> u32 {
        if !self.is_set() {
            return 0;
        }
        let mut v = self.vk as u32;
        if self.ctrl {
            v |= 1 << 16;
        }
        if self.shift {
            v |= 1 << 17;
        }
        if self.alt {
            v |= 1 << 18;
        }
        if self.win {
            v |= 1 << 19;
        }
        v
    }

    pub fn unpack(v: u32) -> Option<Self> {
        let vk = (v & 0xFFFF) as u16;
        if vk == 0 {
            return None;
        }
        Some(Self {
            vk,
            ctrl: (v & (1 << 16)) != 0,
            shift: (v & (1 << 17)) != 0,
            alt: (v & (1 << 18)) != 0,
            win: (v & (1 << 19)) != 0,
        })
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    Start,
    Stop,
    Toggle,
    SetAction(Action),
    SetRate(RateConfig),
    SetStopAfter(StopAfter),
    LoadMacro(Arc<Macro>),
    SetMacroLoops(u32),
    ClearMacro,
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum EngineEvent {
    Started,
    Stopped,
    HeldChanged(Vec<u16>),
    MouseHeldChanged(Vec<MouseButton>),
}

impl RateConfig {
    pub fn base_interval(&self) -> Duration {
        let ms = if self.use_cps {
            (1000.0 / self.cps.max(0.1)) as u64
        } else {
            self.interval_ms
        };
        Duration::from_millis(ms.max(1))
    }
}
