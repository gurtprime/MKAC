//! Shared runtime icon data. `build.rs` decodes `assets/app.png`, resizes it
//! to a fixed square, and dumps the raw RGBA buffer into `OUT_DIR/icon.rgba`.
//! Both the egui viewport and the tray reuse that buffer.

/// Matches `RUNTIME_SIZE` in `build.rs`.
pub const SIZE: u32 = 128;

/// Raw RGBA bytes for a SIZE×SIZE icon.
pub const RGBA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon.rgba"));
