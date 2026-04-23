<p align="center">
  <img src="assets/app.png" alt="MKAC" width="180">
</p>

<h1 align="center">MKAC</h1>

<p align="center">
  <em>A lightweight mouse &amp; keyboard auto-clicker and key-holder for Windows.</em>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/platform-Windows%2010%2F11-111?style=flat-square" alt="platform">
  <img src="https://img.shields.io/badge/built_with-Rust-dea584?style=flat-square" alt="rust">
  <img src="https://img.shields.io/badge/UI-egui-22C55E?style=flat-square" alt="egui">
</p>

---

## Why ?

I wanted a very lightweight and optimized autoclicker that still feels modern for my personal use. The whole app is around 5.5 MB, and it uses less than 50 MB of RAM when running. It should also be more precise and packs more features than most autoclickers.

## What it does

- **Autoclick** — left / right / middle, 1× / 2× / 3×, at a fixed interval or a target CPS.
- **Autopress** — any key (or combo with Ctrl / Shift / Alt / Win), same rate controls. Pick the key by clicking the chip and pressing it on the keyboard.
- **Hold mode** — same hotkey, different behavior: instead of looping, it latches the key or mouse button down until you press the hotkey again.
- **Macros** — record keyboard + mouse events with timing, name the recording, replay any number of loops. Start/stop recording with a dedicated hotkey (default `F8`); record-hotkey presses register only while MKAC isn't the focused window.
- **Rate shaping** — constant cadence, uniform jitter, or a gaussian curve for human-ish timing. Optional random per-event hold-duration range.
- **Stop conditions** — run forever, stop after N events, or stop after a duration.
- **Tray + autostart** — minimize-to-tray, auto-launch with Windows, compact fixed window or experimental resizable mode.
- **Rebindable hotkeys** — two footer chips: one for toggle (default `F6`), one for macro record (default `F8`). Right-click the chip, or click the small **Rebind** button beside it, then press the key you want.
- **Light & dark themes** — native Windows title bar color follows the app theme.

## Install

1. Grab the latest `mkac.exe` from releases (or build from source, see below).
2. Run it. That's it — no installer, no admin required.

Config lives in `%APPDATA%\MKAC\MKAC\config\`:
- `settings.json` — app settings + hotkey bindings
- `macros/` — saved macro JSONs

## Usage

1. Pick a tab: **Mouse**, **Keyboard**, **Macros**, **Settings**.
2. Set **Mode** to `Autoclick`/`Autopress` (loop) or `Hold` (latch).
3. Configure button/key, pattern, target, rate, stop condition.
4. Focus the target window, press **F6** to start; **F6** again to stop.

## Build

Requires the MSVC toolchain (VS Build Tools: "Desktop development with C++").

```powershell
cargo build --release
```

`release` is already the shipping profile: fat LTO, single codegen unit, `opt-level="s"`, symbols stripped, `panic = "abort"`. Typical output is ~5.5 MB.

The project ships a `build.bat` wrapper that sources `vcvars64.bat` automatically:

```powershell
./build.bat build --release
```

## Stack

- **UI**: [`eframe`](https://github.com/emilk/egui) + [`egui`](https://github.com/emilk/egui) on the glow backend (no bundled default fonts — we ship Inter instead)
- **Input**: raw Win32 `SendInput` for all mouse/keyboard output; `WH_KEYBOARD_LL` + `WH_MOUSE_LL` for hotkey capture and macro recording
- **Threads**: UI thread ⇄ engine thread ⇄ hook thread, coordinated with `crossbeam-channel`
- **Fonts**: Inter (SemiBold + Medium), embedded
- **Tray**: [`tray-icon`](https://github.com/tauri-apps/tray-icon)

## Honest caveats

MKAC uses `SendInput`, which tags every event with the `LLMHF_INJECTED` / `LLKHF_INJECTED` flag. Kernel-level anti-cheats (EAC, BattlEye, Vanguard, …) can see this and may ban you. **Don't use MKAC in games with active anti-cheat.** It will never spoof the injected flag or integrate kernel drivers.

## License

MIT
