pub mod hotkey;
pub mod ll_keyboard;
pub mod ll_mouse;
pub mod recording;

use std::thread::{self, JoinHandle};

use crossbeam_channel::{Receiver, Sender, bounded};
use windows::Win32::System::Threading::GetCurrentThreadId;

use crate::engine::Command;
use crate::hooks::recording::RecEvent;

pub struct HookHandle {
    thread_id: u32,
    thread: Option<JoinHandle<()>>,
    pub record_rx: Receiver<RecEvent>,
}

impl HookHandle {
    pub fn spawn(cmd_tx: Sender<Command>) -> Self {
        hotkey::install_cmd_sender(cmd_tx);

        let (tid_tx, tid_rx) = bounded::<u32>(1);
        // 100K events × ~40 bytes = ~4MB reserved for the ring buffer, enough
        // for many minutes of mixed keyboard+mouse capture at realistic rates
        // (the UI drains continuously while recording, so the channel stays
        // near-empty in steady state). The old 1M cap reserved ~40MB up-front
        // just to idle.
        let (rec_tx, rec_rx) = bounded::<RecEvent>(100_000);
        recording::install_channel(rec_tx);

        let thread = thread::Builder::new()
            .name("mkac-hooks".into())
            .spawn(move || {
                let tid = unsafe { GetCurrentThreadId() };
                let _ = tid_tx.send(tid);
                // LL hooks must be installed on a thread that pumps messages.
                if ll_keyboard::install().is_none() {
                    eprintln!(
                        "WH_KEYBOARD_LL install failed; hotkeys & macro capture are disabled"
                    );
                }
                if ll_mouse::install().is_none() {
                    eprintln!("WH_MOUSE_LL install failed; mouse macro capture is disabled");
                }
                hotkey::pump_messages();
            })
            .expect("spawn hook thread");

        let thread_id = tid_rx.recv().expect("hook thread id");
        Self { thread_id, thread: Some(thread), record_rx: rec_rx }
    }

    pub fn shutdown(mut self) {
        hotkey::post_quit(self.thread_id);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
