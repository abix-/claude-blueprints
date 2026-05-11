---
name: ahk
description: AutoHotkey v2 scripting standards for Windows automation, hotkeys, and game macros. Use when writing AHK scripts.
user-invocable: false
version: "1.0"
updated: "2026-05-11"
---
# AutoHotkey v2

## Core
- v2 only. v1 syntax is dead. If editing legacy v1, port to v2 first.
- File header: `#Requires AutoHotkey v2.0` and `#SingleInstance Force`.
- Expressions everywhere. No more legacy command syntax (`Send, hello`). Use `Send("hello")`.
- Strings use double quotes. Escape with backtick: `` `n`` (newline), `` `t`` (tab), `` `"`` (literal quote).
- Comments: `;` line, `/* ... */` block.

## Hotkeys and hotstrings
- Hotkey definition: `^!j::MyFunction()` (Ctrl+Alt+J).
- Modifiers: `^` Ctrl, `!` Alt, `+` Shift, `#` Win.
- Hotstrings: `::btw::by the way`. Add `:*:` for no-end-char, `:?:` for inside-word.
- Bind to a function for anything non-trivial:
  ```ahk
  ^!j:: {
      WinActivate("ahk_exe code.exe")
      Send("^p")
  }
  ```

## Sending input
- `Send("...")` for default mode. `SendInput("...")` for fast/atomic. `SendEvent("...")` for slow/reliable in games.
- `SetKeyDelay(-1, -1)` for instant keystrokes; raise for finicky targets.
- For games, `SendInput` often gets blocked. Use `SendEvent` or `ControlSend` against a specific window.
- `Click()`, `MouseMove(x, y)`. Always set `CoordMode("Mouse", "Screen")` or `"Window"` explicitly.

## Windows and processes
- `WinActivate("ahk_exe notepad.exe")` is more reliable than title matching.
- `WinWait`, `WinWaitActive` before sending input to a window that may not be focused yet.
- `ProcessExist("name.exe")` returns PID or 0.

## State
- Globals: declare with `global var` inside a function, or use a class for grouped state.
- Persist across runs: `IniWrite` / `IniRead` to `A_ScriptDir . "\config.ini"`.

## GUI
- v2 Gui is object-based: `g := Gui(); g.Add("Edit", "w200"); g.Show()`.
- Event handlers: `g.OnEvent("Close", (*) => ExitApp())`.

## Game macros
- Detect window with `WinActive("ahk_exe Atlas.exe")` to scope hotkeys.
- Use `#HotIf WinActive("ahk_exe game.exe")` blocks to make hotkeys game-only.
- Anti-cheat: many games detect `SendInput`. `ControlSend` is sometimes safer. Hardware-level (Interception driver) is the last resort and risks bans.

## Avoid
- v1 syntax (`Send, text`, `IfWinActive`). Port and delete.
- Tight `Loop` with no `Sleep`. Burns a CPU core.
- Hardcoded screen coordinates without `CoordMode`. Breaks on different DPI/resolution.
- `WinGetTitle` for matching. Use `ahk_exe` / `ahk_class` instead.
