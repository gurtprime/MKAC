use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::thread::{self, Thread};

use crossbeam_channel::Sender;
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, RegisterHotKey,
    UnregisterHotKey,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MSG, PostThreadMessageW, TranslateMessage, WM_APP, WM_HOTKEY,
    WM_QUIT,
};

use crate::engine::{Command, HotkeyBinding};

/// Registered-hotkey IDs used with `RegisterHotKey`. Arbitrary but stable.
const HOTKEY_ID_TOGGLE: i32 = 1;
const HOTKEY_ID_RECORD: i32 = 2;
/// Custom thread message: re-register hotkeys using the current atomic
/// binding values. Posted by `set_*_binding` when the UI changes a binding.
const WM_REREGISTER_HOTKEYS: u32 = WM_APP + 1;

static TOGGLE_BINDING: AtomicU32 = AtomicU32::new(0);
static MACRO_RECORD_BINDING: AtomicU32 = AtomicU32::new(0);
static REBIND_ACTIVE: AtomicBool = AtomicBool::new(false);
static CMD_SENDER: OnceLock<Sender<Command>> = OnceLock::new();
/// OS thread id of the hook thread, captured in `pump_messages` at startup.
/// Set once; read by `set_*_binding` to post `WM_REREGISTER_HOTKEYS` across
/// threads (UnregisterHotKey must run on the thread that called
/// RegisterHotKey, so we can't touch the registration directly from the UI
/// thread — we signal the hook thread to do it).
static HOOK_THREAD_ID: OnceLock<u32> = OnceLock::new();

/// Handle to the dedicated waker thread. LL hook callbacks unpark it when
/// a hotkey fires; it then calls `request_repaint` on the egui context from
/// a non-hook context. Keeping `request_repaint` out of the hook callback
/// is critical — egui's Context is an `Arc<RwLock<..>>`, and acquiring the
/// write lock mid-frame can stall for tens of ms. An LL hook callback that
/// stalls risks Windows silently unregistering the hook (300ms timeout on
/// Win7+), and we suspect the "works unfocused, fails focused" bug is
/// exactly this: a focused MKAC renders at 30fps while the hook is trying
/// to wake it, races on the lock, and gets dropped.
static WAKER_THREAD: OnceLock<Thread> = OnceLock::new();
static WAKE_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Counter incremented each time the macro-record hotkey fires. The UI
/// thread diffs against its last-seen value each frame to drive the
/// start/stop-recording action.
static MACRO_RECORD_REQUESTS: AtomicU32 = AtomicU32::new(0);

pub fn install_cmd_sender(tx: Sender<Command>) {
    let _ = CMD_SENDER.set(tx);
}

pub fn install_egui_ctx(ctx: egui::Context) {
    // Spawn a dedicated waker thread that parks until unparked by the LL
    // hook. Only this thread calls `request_repaint` — so the hook callback
    // is fast (just an atomic store + unpark) and never contends on egui's
    // Context RwLock.
    let handle = thread::Builder::new()
        .name("mkac-ui-waker".into())
        .spawn(move || loop {
            thread::park();
            // Drain any coalesced wake requests; spurious wakeups are fine
            // — the loop just calls request_repaint once more.
            if WAKE_REQUESTED.swap(false, Ordering::Acquire) {
                ctx.request_repaint();
            }
        })
        .expect("spawn waker thread");
    let _ = WAKER_THREAD.set(handle.thread().clone());
}

fn wake_ui() {
    WAKE_REQUESTED.store(true, Ordering::Release);
    if let Some(t) = WAKER_THREAD.get() {
        t.unpark();
    }
}

pub fn set_toggle_binding(b: HotkeyBinding) {
    TOGGLE_BINDING.store(b.pack(), Ordering::Release);
    poke_hook_thread_to_reregister();
}

pub fn set_macro_record_binding(b: HotkeyBinding) {
    MACRO_RECORD_BINDING.store(b.pack(), Ordering::Release);
    poke_hook_thread_to_reregister();
}

fn poke_hook_thread_to_reregister() {
    if let Some(&tid) = HOOK_THREAD_ID.get() {
        unsafe {
            let _ = PostThreadMessageW(
                tid,
                WM_REREGISTER_HOTKEYS,
                WPARAM(0),
                LPARAM(0),
            );
        }
    }
}

fn hotkey_binding_to_mods(b: &HotkeyBinding) -> HOT_KEY_MODIFIERS {
    let mut m = MOD_NOREPEAT;
    if b.ctrl {
        m |= MOD_CONTROL;
    }
    if b.shift {
        m |= MOD_SHIFT;
    }
    if b.alt {
        m |= MOD_ALT;
    }
    if b.win {
        m |= MOD_WIN;
    }
    m
}

/// Register both hotkeys from current atomic binding values. Called on the
/// hook thread only. Any previous registration is torn down first so this
/// is safe to call repeatedly.
///
/// If `REBIND_ACTIVE` is true, we still tear down but skip re-registering —
/// the UI needs raw `WM_KEYDOWN`s to reach the window while the user is
/// picking a new key, and RegisterHotKey would intercept them.
fn register_hotkeys_on_this_thread() {
    unsafe {
        // Best-effort unregister — fails silently if not previously registered.
        let _ = UnregisterHotKey(None, HOTKEY_ID_TOGGLE);
        let _ = UnregisterHotKey(None, HOTKEY_ID_RECORD);
    }
    if REBIND_ACTIVE.load(Ordering::Relaxed) {
        return;
    }
    if let Some(b) = HotkeyBinding::unpack(TOGGLE_BINDING.load(Ordering::Relaxed)) {
        if b.is_set() {
            unsafe {
                let _ = RegisterHotKey(
                    None,
                    HOTKEY_ID_TOGGLE,
                    hotkey_binding_to_mods(&b),
                    b.vk as u32,
                );
            }
        }
    }
    if let Some(b) = HotkeyBinding::unpack(MACRO_RECORD_BINDING.load(Ordering::Relaxed)) {
        if b.is_set() {
            unsafe {
                let _ = RegisterHotKey(
                    None,
                    HOTKEY_ID_RECORD,
                    hotkey_binding_to_mods(&b),
                    b.vk as u32,
                );
            }
        }
    }
}

pub fn set_rebind_active(on: bool) {
    let prev = REBIND_ACTIVE.swap(on, Ordering::Release);
    if prev != on {
        // The hook thread must unregister while the user is rebinding so the
        // currently-bound key actually reaches MKAC's window (otherwise the
        // OS intercepts it as a hotkey and the capture UI never sees the
        // WM_KEYDOWN). It re-registers when rebind exits.
        poke_hook_thread_to_reregister();
    }
}

fn dispatch_toggle() {
    if let Some(tx) = CMD_SENDER.get() {
        let _ = tx.try_send(Command::Toggle);
    }
    // Make sure the UI sees the running-state change even if MKAC isn't
    // the focused window (so the hero/footer indicators refresh promptly).
    wake_ui();
}

/// Bump the macro-record request counter. UI thread will notice on its
/// next frame and flip recording state.
fn dispatch_macro_record() {
    MACRO_RECORD_REQUESTS.fetch_add(1, Ordering::Release);
    wake_ui();
}

/// Returns how many new macro-record toggles have arrived since the last
/// call, updating `last_seen` in place.
pub fn take_macro_record_requests(last_seen: &mut u32) -> u32 {
    let now = MACRO_RECORD_REQUESTS.load(Ordering::Acquire);
    let delta = now.wrapping_sub(*last_seen);
    *last_seen = now;
    delta
}

pub fn is_bound_vk(vk: u16) -> bool {
    for atom in [&TOGGLE_BINDING, &MACRO_RECORD_BINDING] {
        if let Some(b) = HotkeyBinding::unpack(atom.load(Ordering::Relaxed)) {
            if b.vk == vk {
                return true;
            }
        }
    }
    false
}

pub fn pump_messages() {
    // Capture this thread's id so set_*_binding can PostThreadMessage here.
    let tid = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };
    let _ = HOOK_THREAD_ID.set(tid);
    // Register the hotkeys for the first time with whatever bindings the
    // UI has already loaded from settings.
    register_hotkeys_on_this_thread();

    let mut msg = MSG::default();
    loop {
        let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if ret.0 == 0 || ret.0 == -1 {
            break;
        }
        match msg.message {
            WM_HOTKEY => match msg.wParam.0 as i32 {
                HOTKEY_ID_TOGGLE => {
                    if !REBIND_ACTIVE.load(Ordering::Relaxed) {
                        dispatch_toggle();
                    }
                }
                HOTKEY_ID_RECORD => {
                    if !REBIND_ACTIVE.load(Ordering::Relaxed) {
                        dispatch_macro_record();
                    }
                }
                _ => {}
            },
            WM_REREGISTER_HOTKEYS => {
                register_hotkeys_on_this_thread();
            }
            _ => unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            },
        }
    }
    unsafe {
        let _ = UnregisterHotKey(None, HOTKEY_ID_TOGGLE);
        let _ = UnregisterHotKey(None, HOTKEY_ID_RECORD);
    }
}

pub fn post_quit(thread_id: u32) {
    unsafe {
        let _ = PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
    }
}
