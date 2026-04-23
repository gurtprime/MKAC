use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};

use super::command::{
    Action, Command, EngineEvent, JitterCurve, RateConfig, StopAfter, TriggerMode,
};
use super::macros::Macro;
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

pub struct EngineConfig {
    pub rate: RateConfig,
    pub stop_after: StopAfter,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            rate: RateConfig::default(),
            stop_after: StopAfter::default(),
        }
    }
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
        if self.is_running {
            self.is_running = false;
            self.macro_state = None;
            let _ = evt_tx.try_send(EngineEvent::Stopped);
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
        Action::MouseClick { mode: TriggerMode::Hold, .. }
            | Action::KeyTap { mode: TriggerMode::Hold, .. }
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
                });
                rs.next_tick =
                    Instant::now() + Duration::from_millis(m.frames[0].delta_ms as u64);
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
    // Request 1ms timer resolution so sub-16ms tick intervals actually fire
    // on time. Without this Windows caps sleep granularity at ~15.6 ms,
    // which in turn caps our effective CPS at ~64. Released at shutdown.
    let _hi_res_timer = HiResTimer::acquire();

    let mut cfg = EngineConfig::default();
    let mut action = Action::default();
    let mut rs = RunState::new();
    let mut held = VirtualHold::default();
    let mut loaded_macro: Option<Arc<Macro>> = None;
    let mut macro_loops: u32 = 1;

    loop {
        let cmd = if rs.is_running {
            match cmd_rx.recv_deadline(rs.next_tick) {
                Ok(c) => Some(c),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => {
                    held.release_all();
                    return;
                }
            }
        } else {
            match cmd_rx.recv() {
                Ok(c) => Some(c),
                Err(_) => {
                    held.release_all();
                    return;
                }
            }
        };

        if let Some(cmd) = cmd {
            match cmd {
                Command::Start => {
                    start_running(&mut rs, &action, &loaded_macro, macro_loops, &cfg, &evt_tx);
                }
                Command::Stop => rs.stop(&evt_tx),
                Command::Toggle => {
                    // When the configured action is in Hold mode, the hotkey
                    // latches/unlatches the target instead of starting a loop.
                    match action {
                        Action::MouseClick { mode: TriggerMode::Hold, button, .. } => {
                            held.toggle_mouse(button);
                            let _ = evt_tx.try_send(EngineEvent::MouseHeldChanged(
                                held.held_mouse_buttons(),
                            ));
                        }
                        Action::KeyTap { mode: TriggerMode::Hold, vk, .. } => {
                            held.toggle(vk);
                            let _ = evt_tx.try_send(EngineEvent::HeldChanged(held.held_keys()));
                        }
                        _ => {
                            if rs.is_running {
                                rs.stop(&evt_tx);
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
                    if matches!(
                        action,
                        Action::MouseClick { mode: TriggerMode::Hold, .. }
                            | Action::KeyTap { mode: TriggerMode::Hold, .. }
                    ) {
                        rs.stop(&evt_tx);
                    }
                    if !matches!(action, Action::PlayMacro) {
                        rs.macro_state = None;
                    }
                }
                Command::SetRate(r) => cfg.rate = r,
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
                    rs.macro_state = None;
                }
                Command::Shutdown => {
                    held.release_all();
                    return;
                }
            }
            continue;
        }

        // Tick dispatch (no command pending, deadline reached)
        if !rs.is_running || Instant::now() < rs.next_tick {
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
            Action::MouseClick { button, pattern, target, mode: TriggerMode::Auto } => {
                mouse::click(button, pattern, target, hold_ms);
            }
            Action::KeyTap { vk, mods, mode: TriggerMode::Auto } => {
                keyboard::key_tap(vk, mods, hold_ms);
            }
            // Hold mode shouldn't reach the tick loop — but if a late
            // SetAction swapped us, stop safely instead of firing clicks.
            Action::MouseClick { .. } | Action::KeyTap { .. } => {
                rs.stop(&evt_tx);
                continue;
            }
            Action::PlayMacro => {
                if let Some(ref mut ms) = rs.macro_state {
                    let m = ms.macro_ref.clone();
                    if m.frames.is_empty() {
                        rs.stop(&evt_tx);
                        continue;
                    }
                    if ms.frame_idx < m.frames.len() {
                        macros::play_event(&m.frames[ms.frame_idx].event);
                        ms.frame_idx += 1;
                    }
                    let mut wrapped = false;
                    if ms.frame_idx >= m.frames.len() {
                        ms.loops_done += 1;
                        if ms.loops_target > 0 && ms.loops_done >= ms.loops_target {
                            rs.tick_count = rs.tick_count.wrapping_add(1);
                            rs.is_running = false;
                            rs.macro_state = None;
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
                    rs.tick_count = rs.tick_count.wrapping_add(1);
                    rs.next_tick = Instant::now() + delta;
                    continue;
                } else {
                    // No macro loaded: stop.
                    rs.stop(&evt_tx);
                    continue;
                }
            }
        }

        rs.tick_count = rs.tick_count.wrapping_add(1);

        let should_stop = match cfg.stop_after {
            StopAfter::Never => false,
            StopAfter::Count { n } => rs.tick_count >= n,
            StopAfter::Duration { ms } => {
                Instant::now().duration_since(rs.start_time).as_millis() as u64 >= ms
            }
        };
        if should_stop {
            rs.is_running = false;
            let _ = evt_tx.try_send(EngineEvent::Stopped);
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
