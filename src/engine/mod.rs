pub mod command;
pub mod keyboard;
pub mod macros;
pub mod mouse;
pub mod scheduler;
pub mod virtual_hold;

use std::thread::{self, JoinHandle};

use crossbeam_channel::{Receiver, Sender, bounded, unbounded};

pub use command::{
    Action, ClickPattern, Command, EngineEvent, HotkeyBinding, JitterCurve, KeyMods, MouseButton,
    RateConfig, StopAfter, Target, TriggerMode,
};

pub struct EngineHandle {
    pub cmd_tx: Sender<Command>,
    pub evt_rx: Receiver<EngineEvent>,
    thread: Option<JoinHandle<()>>,
}

impl EngineHandle {
    pub fn spawn() -> Self {
        let (cmd_tx, cmd_rx) = unbounded::<Command>();
        let (evt_tx, evt_rx) = bounded::<EngineEvent>(64);

        let thread = thread::Builder::new()
            .name("mkac-engine".into())
            .spawn(move || scheduler::run(cmd_rx, evt_tx))
            .expect("spawn engine thread");

        Self { cmd_tx, evt_rx, thread: Some(thread) }
    }

    pub fn shutdown(mut self) {
        let _ = self.cmd_tx.send(Command::Shutdown);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
