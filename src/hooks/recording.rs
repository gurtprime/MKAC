use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crossbeam_channel::Sender;

use crate::engine::macros::MacroEvent;

pub static IS_RECORDING: AtomicBool = AtomicBool::new(false);

pub fn set_recording(on: bool) {
    IS_RECORDING.store(on, Ordering::Release);
}

#[derive(Debug, Clone)]
pub struct RecEvent {
    pub at: Instant,
    pub event: MacroEvent,
}

static EVENT_SENDER: OnceLock<Sender<RecEvent>> = OnceLock::new();

pub fn install_channel(tx: Sender<RecEvent>) {
    let _ = EVENT_SENDER.set(tx);
}

pub fn push(event: MacroEvent) {
    if let Some(tx) = EVENT_SENDER.get() {
        let _ = tx.try_send(RecEvent { at: Instant::now(), event });
    }
}
