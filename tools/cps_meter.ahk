; MKAC CPS Meter — AutoHotkey v2
;
; Low-level mouse + keyboard hook listener with an always-on-top live readout.
; Catches events regardless of source (real clicks or SendInput from MKAC),
; so it works as a ground-truth OS-level CPS measurement.
;
;   F1  — start / stop monitoring
;   F2  — reset counters
;   F10 — quit

#Requires AutoHotkey v2.0
#SingleInstance Force

; Force low-level hook installation so we catch injected events from
; SendInput (what MKAC uses), not just real hardware input.
InstallMouseHook(true, true)
InstallKeybdHook(true, true)

; -------- state --------
monitoring     := false
count          := 0
startTick      := 0
lastSampleTick := 0
lastSampleCount := 0
peakCps        := 0.0

; -------- GUI --------
g := Gui("+AlwaysOnTop -MinimizeBox", "MKAC CPS Meter")
g.BackColor := "0A0B0E"
g.MarginX   := 18
g.MarginY   := 14

g.SetFont("s10 cAAAAAA", "Segoe UI")
lblStatus := g.Add("Text", "w260", "idle · F1 to start")

g.SetFont("s44 Bold c22C55E", "Segoe UI")
lblCps := g.Add("Text", "xm w260 Center", "0.0")

g.SetFont("s10 Bold c888888", "Segoe UI")
g.Add("Text", "xm w260 Center", "events / sec")

g.SetFont("s9 cAAAAAA", "Segoe UI")
lblStats := g.Add("Text", "xm w280", "—")

g.SetFont("s8 c555555", "Segoe UI")
g.Add("Text", "xm w280", "F1 start/stop · F2 reset · F10 quit")

g.Show("w300")

; -------- control hotkeys --------
F1::ToggleMonitoring()
F2::ResetStats()
F10::ExitApp()

; -------- click/key listeners --------
; `~`  = don't swallow the event, forward to the focused window
; `*`  = match regardless of modifier state
; `#UseHook true` is implicit via `#InstallMouseHook`
~*LButton::Incr()
~*RButton::Incr()
~*MButton::Incr()

; Broad keyboard coverage so autopress tests also register.
; Letters + digits + common keys. Modifiers (Ctrl/Shift/etc.) deliberately
; skipped so held modifiers during combos don't count as extra events.
keysToHook := [
  "a","b","c","d","e","f","g","h","i","j","k","l","m",
  "n","o","p","q","r","s","t","u","v","w","x","y","z",
  "0","1","2","3","4","5","6","7","8","9",
  "Space","Enter","Tab","Backspace",
  "F1","F2","F3","F4","F5","F6","F7","F8","F9","F11","F12",
  "-","=","[","]","\",";","'",",",".","/","``"
]
for key in keysToHook {
    ; Skip keys already used as control hotkeys above.
    if (key = "F1" || key = "F2" || key = "F10")
        continue
    HotKey("~*" key, (*) => Incr())
}

Incr(*) {
    global count, monitoring
    if (monitoring)
        count++
}

ToggleMonitoring() {
    global monitoring, count, startTick, peakCps, lastSampleTick, lastSampleCount
    monitoring := !monitoring
    if (monitoring) {
        count           := 0
        peakCps         := 0.0
        startTick       := A_TickCount
        lastSampleTick  := startTick
        lastSampleCount := 0
        lblCps.Text     := "0.0"
        lblStats.Text   := "—"
        lblStatus.Text  := "RECORDING · F1 to stop"
        SetTimer(UpdateUI, 100)
    } else {
        SetTimer(UpdateUI, 0)
        UpdateUI()
        lblStatus.Text := "STOPPED · F1 to resume · F2 to reset"
    }
}

ResetStats() {
    global count, peakCps, startTick, lastSampleTick, lastSampleCount
    count           := 0
    peakCps         := 0.0
    startTick       := A_TickCount
    lastSampleTick  := startTick
    lastSampleCount := 0
    lblCps.Text     := "0.0"
    lblStats.Text   := "—"
}

UpdateUI() {
    global count, startTick, lastSampleTick, lastSampleCount, peakCps
    now        := A_TickCount
    elapsed    := (now - startTick) / 1000.0
    deltaMs    := now - lastSampleTick
    deltaCount := count - lastSampleCount
    instant    := deltaMs > 0 ? (deltaCount * 1000.0 / deltaMs) : 0.0
    if (instant > peakCps)
        peakCps := instant
    avg := elapsed > 0 ? (count / elapsed) : 0.0
    lblCps.Text   := Format("{:.1f}", instant)
    lblStats.Text := Format("total {} · {:.1f}s · avg {:.1f} · peak {:.1f}", count, elapsed, avg, peakCps)
    lastSampleTick  := now
    lastSampleCount := count
}
