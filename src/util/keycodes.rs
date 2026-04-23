#[derive(Debug, Clone, Copy)]
pub struct KeyDef {
    pub vk: u16,
    pub name: &'static str,
}

pub const KEYS: &[KeyDef] = &[
    // Letters
    KeyDef { vk: 0x41, name: "A" },
    KeyDef { vk: 0x42, name: "B" },
    KeyDef { vk: 0x43, name: "C" },
    KeyDef { vk: 0x44, name: "D" },
    KeyDef { vk: 0x45, name: "E" },
    KeyDef { vk: 0x46, name: "F" },
    KeyDef { vk: 0x47, name: "G" },
    KeyDef { vk: 0x48, name: "H" },
    KeyDef { vk: 0x49, name: "I" },
    KeyDef { vk: 0x4A, name: "J" },
    KeyDef { vk: 0x4B, name: "K" },
    KeyDef { vk: 0x4C, name: "L" },
    KeyDef { vk: 0x4D, name: "M" },
    KeyDef { vk: 0x4E, name: "N" },
    KeyDef { vk: 0x4F, name: "O" },
    KeyDef { vk: 0x50, name: "P" },
    KeyDef { vk: 0x51, name: "Q" },
    KeyDef { vk: 0x52, name: "R" },
    KeyDef { vk: 0x53, name: "S" },
    KeyDef { vk: 0x54, name: "T" },
    KeyDef { vk: 0x55, name: "U" },
    KeyDef { vk: 0x56, name: "V" },
    KeyDef { vk: 0x57, name: "W" },
    KeyDef { vk: 0x58, name: "X" },
    KeyDef { vk: 0x59, name: "Y" },
    KeyDef { vk: 0x5A, name: "Z" },
    // Digits
    KeyDef { vk: 0x30, name: "0" },
    KeyDef { vk: 0x31, name: "1" },
    KeyDef { vk: 0x32, name: "2" },
    KeyDef { vk: 0x33, name: "3" },
    KeyDef { vk: 0x34, name: "4" },
    KeyDef { vk: 0x35, name: "5" },
    KeyDef { vk: 0x36, name: "6" },
    KeyDef { vk: 0x37, name: "7" },
    KeyDef { vk: 0x38, name: "8" },
    KeyDef { vk: 0x39, name: "9" },
    // Function
    KeyDef { vk: 0x70, name: "F1" },
    KeyDef { vk: 0x71, name: "F2" },
    KeyDef { vk: 0x72, name: "F3" },
    KeyDef { vk: 0x73, name: "F4" },
    KeyDef { vk: 0x74, name: "F5" },
    KeyDef { vk: 0x75, name: "F6" },
    KeyDef { vk: 0x76, name: "F7" },
    KeyDef { vk: 0x77, name: "F8" },
    KeyDef { vk: 0x78, name: "F9" },
    KeyDef { vk: 0x79, name: "F10" },
    KeyDef { vk: 0x7A, name: "F11" },
    KeyDef { vk: 0x7B, name: "F12" },
    // Navigation
    KeyDef { vk: 0x25, name: "Left" },
    KeyDef { vk: 0x26, name: "Up" },
    KeyDef { vk: 0x27, name: "Right" },
    KeyDef { vk: 0x28, name: "Down" },
    KeyDef { vk: 0x21, name: "Page Up" },
    KeyDef { vk: 0x22, name: "Page Down" },
    KeyDef { vk: 0x23, name: "End" },
    KeyDef { vk: 0x24, name: "Home" },
    KeyDef { vk: 0x2D, name: "Insert" },
    KeyDef { vk: 0x2E, name: "Delete" },
    // Special
    KeyDef { vk: 0x20, name: "Space" },
    KeyDef { vk: 0x0D, name: "Enter" },
    KeyDef { vk: 0x09, name: "Tab" },
    KeyDef { vk: 0x1B, name: "Escape" },
    KeyDef { vk: 0x08, name: "Backspace" },
    KeyDef { vk: 0x14, name: "Caps Lock" },
    KeyDef { vk: 0x90, name: "Num Lock" },
    KeyDef { vk: 0x91, name: "Scroll Lock" },
    KeyDef { vk: 0x2C, name: "Print Screen" },
    KeyDef { vk: 0x13, name: "Pause" },
    KeyDef { vk: 0x5D, name: "Menu" },
    // Numpad
    KeyDef { vk: 0x60, name: "Num 0" },
    KeyDef { vk: 0x61, name: "Num 1" },
    KeyDef { vk: 0x62, name: "Num 2" },
    KeyDef { vk: 0x63, name: "Num 3" },
    KeyDef { vk: 0x64, name: "Num 4" },
    KeyDef { vk: 0x65, name: "Num 5" },
    KeyDef { vk: 0x66, name: "Num 6" },
    KeyDef { vk: 0x67, name: "Num 7" },
    KeyDef { vk: 0x68, name: "Num 8" },
    KeyDef { vk: 0x69, name: "Num 9" },
    KeyDef { vk: 0x6A, name: "Num *" },
    KeyDef { vk: 0x6B, name: "Num +" },
    KeyDef { vk: 0x6D, name: "Num -" },
    KeyDef { vk: 0x6E, name: "Num ." },
    KeyDef { vk: 0x6F, name: "Num /" },
    // Punctuation (US layout)
    KeyDef { vk: 0xBA, name: ";" },
    KeyDef { vk: 0xBB, name: "=" },
    KeyDef { vk: 0xBC, name: "," },
    KeyDef { vk: 0xBD, name: "-" },
    KeyDef { vk: 0xBE, name: "." },
    KeyDef { vk: 0xBF, name: "/" },
    KeyDef { vk: 0xC0, name: "`" },
    KeyDef { vk: 0xDB, name: "[" },
    KeyDef { vk: 0xDC, name: "\\" },
    KeyDef { vk: 0xDD, name: "]" },
    KeyDef { vk: 0xDE, name: "'" },
];

pub fn lookup_name(vk: u16) -> Option<&'static str> {
    // Linear scan over ~110 entries — only called when the UI renders a
    // keybinding chip, so the cache-friendly sweep beats paying for a
    // HashMap (and its hashbrown + ahash pull-in) that would also need a
    // OnceLock init for a single lookup path.
    KEYS.iter().find(|k| k.vk == vk).map(|k| k.name)
}

pub fn egui_key_to_vk(k: egui::Key) -> Option<u16> {
    use egui::Key::*;
    Some(match k {
        A => 0x41, B => 0x42, C => 0x43, D => 0x44, E => 0x45, F => 0x46, G => 0x47,
        H => 0x48, I => 0x49, J => 0x4A, K => 0x4B, L => 0x4C, M => 0x4D, N => 0x4E,
        O => 0x4F, P => 0x50, Q => 0x51, R => 0x52, S => 0x53, T => 0x54, U => 0x55,
        V => 0x56, W => 0x57, X => 0x58, Y => 0x59, Z => 0x5A,
        Num0 => 0x30, Num1 => 0x31, Num2 => 0x32, Num3 => 0x33, Num4 => 0x34,
        Num5 => 0x35, Num6 => 0x36, Num7 => 0x37, Num8 => 0x38, Num9 => 0x39,
        F1 => 0x70, F2 => 0x71, F3 => 0x72, F4 => 0x73, F5 => 0x74,
        F6 => 0x75, F7 => 0x76, F8 => 0x77, F9 => 0x78,
        F10 => 0x79, F11 => 0x7A, F12 => 0x7B,
        ArrowLeft => 0x25, ArrowUp => 0x26, ArrowRight => 0x27, ArrowDown => 0x28,
        PageUp => 0x21, PageDown => 0x22, End => 0x23, Home => 0x24,
        Insert => 0x2D, Delete => 0x2E,
        Space => 0x20, Enter => 0x0D, Tab => 0x09, Escape => 0x1B, Backspace => 0x08,
        Semicolon => 0xBA, Equals => 0xBB, Comma => 0xBC, Minus => 0xBD, Period => 0xBE,
        Slash => 0xBF, Backtick => 0xC0, OpenBracket => 0xDB, Backslash => 0xDC,
        CloseBracket => 0xDD, Quote => 0xDE,
        _ => return None,
    })
}

pub fn is_extended(vk: u16) -> bool {
    // Arrow keys, Insert, Delete, Home, End, PageUp, PageDown, NumLock,
    // right-side modifiers, numpad divide, Win keys, PrintScreen, etc.
    matches!(
        vk,
        0x21..=0x28      // PageUp/PageDown/End/Home/Left/Up/Right/Down
            | 0x2C        // Print Screen
            | 0x2D | 0x2E // Insert / Delete
            | 0x5B | 0x5C // VK_LWIN / VK_RWIN
            | 0x5D        // Apps/Menu
            | 0x6F        // Numpad Divide
            | 0x90        // NumLock
            | 0xA3        // VK_RCONTROL
            | 0xA5        // VK_RMENU (right Alt)
    )
}
