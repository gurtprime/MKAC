use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};

use super::command::{
    Action, ClickPattern, Command, EngineEvent, JitterCurve, RateConfig, StopAfter, TriggerMode,
};
use super::macros::{Macro, MacroEvent};
use super::{keyboard, macros, mouse, virtual_hold::VirtualHold};

/// Minimum gap between macro loops so the last event of loop N doesn't fire
/// simultaneously with the first event of loop N+1 (recordings start at
/// delta=0 by construction).
const MACRO_LOOP_GAP: Duration = Duration::from_millis(50);

/// RAII wrapper that raises and restores the Windows multimedia timer
/// resolution for this process. `timeBeginPeriod(1)` drops sleep granularity
/// from ~15.6 ms to ~1 ms which lets the scheduler hit CPS rates above ~64.
struct HiResTimer;

impl HiResTimer {
    fn acquire() -> Self {
        unsafe {
            let _ = windows::Win32::Media::timeBeginPeriod(1);
        }
        Self
    }
}

impl Drop for HiResTimer {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Media::timeEndPeriod(1);
        }
    }
}

#[derive(Default)]
pub struct EngineConfig {
    pub rate: RateConfig,
    pub stop_after: StopAfter,
}

fn compute_interval(rate: &RateConfig) -> Duration {
    let base = rate.base_interval().as_millis() as i64;
    if rate.jitter_enabled && rate.jitter_max_ms > 0 {
        let max = rate.jitter_max_ms as i64;
        let offset = match rate.jitter_curve {
            JitterCurve::Uniform => fastrand::i64(-max..=max),
            JitterCurve::Gaussian => {
                let u1 = fastrand::f64().max(1e-10);
                let u2 = fastrand::f64();
                let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                let sigma = (max as f64) / 3.0;
                ((z * sigma).round() as i64).clamp(-max, max)
            }
        };
        Duration::from_millis((base + offset).max(1) as u64)
    } else {
        Duration::from_millis(base.max(1) as u64)
    }
}

struct MacroPlayState {
    macro_ref: Arc<Macro>,
    frame_idx: usize,
    loops_done: u32,
    loops_target: u32,
    held_keys: HashSet<u16>,
    held_mouse_buttons: HashSet<super::command::MouseButton>,
}

impl MacroPlayState {
    fn track_event(&mut self, event: &MacroEvent) {
        match *event {
            MacroEvent::KeyDown { vk } => {
                self.held_keys.insert(vk);
            }
            MacroEvent::KeyUp { vk } => {
                self.held_keys.remove(&vk);
            }
            MacroEvent::MouseDown { button, .. } => {
                self.held_mouse_buttons.insert(button);
            }
            MacroEvent::MouseUp { button, .. } => {
                self.held_mouse_buttons.remove(&button);
            }
            MacroEvent::MouseMove { .. } => {}
        }
    }

    fn release_all(&mut self) {
        for vk in self.held_keys.drain() {
            keyboard::key_up(vk);
        }
        for button in self.held_mouse_buttons.drain() {
            mouse::button_up(button);
        }
    }
}

struct RunState {
    is_running: bool,
    tick_count: u64,
    start_time: Instant,
    next_tick: Instant,
    macro_state: Option<MacroPlayState>,
}

impl RunState {
    fn new() -> Self {
        Self {
            is_running: false,
            tick_count: 0,
            start_time: Instant::now(),
            next_tick: Instant::now(),
            macro_state: None,
        }
    }

    fn stop(&mut self, evt_tx: &Sender<EngineEvent>) {
        let was_active = self.is_running || self.macro_state.is_some();
        self.is_running = false;
        if let Some(mut macro_state) = self.macro_state.take() {
            macro_state.release_all();
        }
        if was_active {
            let _ = evt_tx.try_send(EngineEvent::Stopped);
        }
    }

    fn stop_all(&mut self, held: &mut VirtualHold, evt_tx: &Sender<EngineEvent>) {
        self.stop(evt_tx);
        let had_keys = !held.held_keys().is_empty();
        let had_mouse = !held.held_mouse_buttons().is_empty();
        held.release_all();
        if had_keys {
            let _ = evt_tx.try_send(EngineEvent::HeldChanged(Vec::new()));
        }
        if had_mouse {
            let _ = evt_tx.try_send(EngineEvent::MouseHeldChanged(Vec::new()));
        }
    }
}

fn start_running(
    rs: &mut RunState,
    action: &Action,
    loaded_macro: &Option<Arc<Macro>>,
    macro_loops: u32,
    cfg: &EngineConfig,
    evt_tx: &Sender<EngineEvent>,
) -> bool {
    if rs.is_running {
        return false;
    }
    // Hold mode is a latch, not a loop — starting the tick loop for it would
    // fire spurious clicks. Refuse.
    if matches!(
        action,
        Action::MouseClick {
            mode: TriggerMode::Hold,
            repeat_while_toggled: false,
            ..
        } | Action::KeyTap {
            mode: TriggerMode::Hold,
            ..
        }
    ) {
        return false;
    }
    // For macro playback require a non-empty loaded macro.
    if matches!(action, Action::PlayMacro) {
        match loaded_macro.as_ref() {
            Some(m) if !m.frames.is_empty() => {
                rs.macro_state = Some(MacroPlayState {
                    macro_ref: m.clone(),
                    frame_idx: 0,
                    loops_done: 0,
                    loops_target: macro_loops,
                    held_keys: HashSet::new(),
                    held_mouse_buttons: HashSet::new(),
                });
                rs.next_tick = Instant::now() + Duration::from_millis(m.frames[0].delta_ms as u64);
            }
            _ => {
                // No valid macro — refuse to start so the UI doesn't see a
                // transient running state.
                return false;
            }
        }
    } else {
        // Honor the configured interval for the first tick so pressing the
        // hotkey doesn't instantly fire one click before respecting the rate.
        rs.next_tick = Instant::now() + compute_interval(&cfg.rate);
    }
    rs.is_running = true;
    rs.tick_count = 0;
    rs.start_time = Instant::now();
    let _ = evt_tx.try_send(EngineEvent::Started);
    true
}

pub fn run(cmd_rx: Receiver<Command>, evt_tx: Sender<EngineEvent>) {
    let mut cfg = EngineConfig::default();
    let mut action = Action::default();
    let mut rs = RunState::new();
    let mut held = VirtualHold::default();
    let mut loaded_macro: Option<Arc<Macro>> = None;
    let mut macro_loops: u32 = 1;
    // Keep the process timer resolution at its normal idle value. The 1 ms
    // request is needed only while an auto action or macro is running.
    let mut hi_res_timer: Option<HiResTimer> = None;

    loop {
        if rs.is_running {
            if hi_res_timer.is_none() {
                hi_res_timer = Some(HiResTimer::acquire());
            }
        } else {
            hi_res_timer = None;
        }

        let cmd = if rs.is_running {
            match cmd_rx.recv_deadline(rs.next_tick) {
                Ok(c) => Some(c),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => {
                    rs.stop_all(&mut held, &evt_tx);
                    return;
                }
            }
        } else {
            match cmd_rx.recv() {
                Ok(c) => Some(c),
                Err(_) => {
                    rs.stop_all(&mut held, &evt_tx);
                    return;
                }
            }
        };

        if let Some(cmd) = cmd {
            match cmd {
                Command::Start => {
                    start_running(&mut rs, &action, &loaded_macro, macro_loops, &cfg, &evt_tx);
                }
                Command::Stop => rs.stop_all(&mut held, &evt_tx),
                Command::Toggle => {
                    // When the configured action is in Hold mode, the hotkey
                    // latches/unlatches the target instead of starting a loop.
                    match action {
                        Action::MouseClick {
                            mode: TriggerMode::Hold,
                            repeat_while_toggled: false,
                            button,
                            ..
                        } => {
                            held.toggle_mouse(button);
                            let _ = evt_tx
                                .try_send(EngineEvent::MouseHeldChanged(held.held_mouse_buttons()));
                        }
                        Action::MouseClick {
                            mode: TriggerMode::Hold,
                            repeat_while_toggled: true,
                            ..
                        } => {
                            if rs.is_running {
                                rs.stop_all(&mut held, &evt_tx);
                            } else {
                                start_running(
                                    &mut rs,
                                    &action,
                                    &loaded_macro,
                                    macro_loops,
                                    &cfg,
                                    &evt_tx,
                                );
                            }
                        }
                        Action::KeyTap {
                            mode: TriggerMode::Hold,
                            vk,
                            mods,
                        } => {
                            held.toggle_combo(vk, mods);
                            let _ = evt_tx.try_send(EngineEvent::HeldChanged(held.held_keys()));
                        }
                        _ => {
                            if rs.is_running {
                                rs.stop_all(&mut held, &evt_tx);
                            } else {
                                start_running(
                                    &mut rs,
                                    &action,
                                    &loaded_macro,
                                    macro_loops,
                                    &cfg,
                                    &evt_tx,
                                );
                            }
                        }
                    }
                }
                Command::SetAction(a) => {
                    action = a;
                    // Swapping to a Hold action mid-loop would keep ticking
                    // clicks — stop first.
                    // Any action change invalidates generated held state and
                    // must stop the previous action before adopting the new one.
                    rs.stop_all(&mut held, &evt_tx);
                }
                Command::SetRate(r) => cfg.rate = r.normalized(),
                Command::SetStopAfter(s) => cfg.stop_after = s,
                Command::LoadMacro(m) => loaded_macro = Some(m),
                Command::SetMacroLoops(n) => {
                    macro_loops = n;
                    if let Some(ref mut ms) = rs.macro_state {
                        ms.loops_target = n;
                    }
                }
                Command::ClearMacro => {
                    loaded_macro = None;
                    // Deleting a library item must not stop an unrelated
                    // mouse/keyboard action that happens to be running.
                    if matches!(action, Action::PlayMacro) {
                        rs.stop_all(&mut held, &evt_tx);
                    }
                }
                Command::Shutdown => {
                    rs.stop_all(&mut held, &evt_tx);
                    return;
                }
            }
            continue;
        }

        // Tick dispatch (no command pending, deadline reached)
        if !rs.is_running || Instant::now() < rs.next_tick {
            continue;
        }

        // Check the deadline before dispatching the next event as well as
        // after it. This prevents a long macro gap or slow input call from
        // producing one event after a duration limit has elapsed.
        if should_stop(&cfg.stop_after, &rs) {
            rs.stop_all(&mut held, &evt_tx);
            continue;
        }

        let hold_ms = if cfg.rate.jitter_enabled {
            let min = cfg.rate.hold_min_ms;
            let max = cfg.rate.hold_max_ms.max(min);
            if max > min {
                fastrand::u32(min..=max)
            } else {
                min
            }
        } else {
            0
        };
        match action {
            // Auto mode drives the tick loop.
            Action::MouseClick {
                button,
                pattern,
                target,
                mode: TriggerMode::Auto,
                ..
            } => {
                mouse::click(button, pattern, target, hold_ms);
            }
            Action::MouseClick {
                button,
                target,
                mode: TriggerMode::Hold,
                repeat_while_toggled: true,
                ..
            } => {
                // Some game clients accept discrete clicks but discard a
                // synthetic persistent button-down state.
                mouse::click(button, ClickPattern::Single, target, hold_ms);
            }
            Action::KeyTap {
                vk,
                mods,
                mode: TriggerMode::Auto,
            } => {
                keyboard::key_tap(vk, mods, hold_ms);
            }
            // Hold mode shouldn't reach the tick loop — but if a late
            // SetAction swapped us, stop safely instead of firing clicks.
            Action::MouseClick { .. } | Action::KeyTap { .. } => {
                rs.stop_all(&mut held, &evt_tx);
                continue;
            }
            Action::PlayMacro => {
                let Some(mut ms) = rs.macro_state.take() else {
                    // No macro loaded: stop.
                    rs.stop_all(&mut held, &evt_tx);
                    continue;
                };
                let m = ms.macro_ref.clone();
                if m.frames.is_empty() {
                    ms.release_all();
                    rs.is_running = false;
                    let _ = evt_tx.try_send(EngineEvent::Stopped);
                    continue;
                }
                if ms.frame_idx < m.frames.len() {
                    let event = m.frames[ms.frame_idx].event;
                    macros::play_event(&event);
                    ms.track_event(&event);
                    ms.frame_idx += 1;
                }

                rs.tick_count = rs.tick_count.wrapping_add(1);
                if should_stop(&cfg.stop_after, &rs) {
                    ms.release_all();
                    rs.is_running = false;
                    let _ = evt_tx.try_send(EngineEvent::Stopped);
                    continue;
                }

                let mut wrapped = false;
                if ms.frame_idx >= m.frames.len() {
                    ms.loops_done += 1;
                    if ms.loops_target > 0 && ms.loops_done >= ms.loops_target {
                        ms.release_all();
                        rs.is_running = false;
                        let _ = evt_tx.try_send(EngineEvent::Stopped);
                        continue;
                    }
                    ms.frame_idx = 0;
                    wrapped = true;
                }
                let delta_ms = m.frames[ms.frame_idx].delta_ms.max(1) as u64;
                let delta = if wrapped {
                    // Avoid zero-delay loop restarts that chain recorded
                    // frames back-to-back across loop boundaries.
                    Duration::from_millis(delta_ms).max(MACRO_LOOP_GAP)
                } else {
                    Duration::from_millis(delta_ms)
                };
                rs.next_tick = Instant::now() + delta;
                rs.macro_state = Some(ms);
                continue;
            }
        }

        rs.tick_count = rs.tick_count.wrapping_add(1);

        if should_stop(&cfg.stop_after, &rs) {
            rs.stop_all(&mut held, &evt_tx);
            continue;
        }

        let step = compute_interval(&cfg.rate);
        rs.next_tick += step;
        let now = Instant::now();
        if rs.next_tick < now {
            rs.next_tick = now + step;
        }
    }
}

fn should_stop(stop_after: &StopAfter, rs: &RunState) -> bool {
    match *stop_after {
        StopAfter::Never => false,
        StopAfter::Count { n } => n > 0 && rs.tick_count >= n,
        StopAfter::Duration { ms } => rs.start_time.elapsed().as_millis() as u64 >= ms,
    }
}
