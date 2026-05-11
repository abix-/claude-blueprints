---
name: ahk
description: AutoHotkey v2 scripting standards for Windows automation, hotkeys, and game macros. Built from the official AHK v2 docs and the AHK community conventions. v1 reached EOL in March 2024.
user-invocable: false
version: "2.0"
updated: "2026-05-11"
---
# AutoHotkey v2

Current stable: **v2.0.26** (May 2026). v1 reached EOL March 2024 and
should be ported. There is no v3.

## File header

Every script starts with:

```ahk
#Requires AutoHotkey v2.0
#SingleInstance Force        ; reload kills the previous instance
#Warn All, MsgBox             ; lint catches typos, undeclared vars

; SetTitleMatchMode 2          ; substring match by default; explicit only if needed
; CoordMode "Mouse", "Screen"  ; declare coordinate frame ONCE at the top
```

- `#Requires` is non-optional. Without it, a v1 user double-clicking
  the script gets a cryptic error.
- `#SingleInstance Force` means re-running replaces; the alternative
  `Off` allows multiple, `Prompt` asks. `Force` is the right default.
- `#Warn` catches typos before runtime. Production scripts keep
  warnings on.

## Syntax fundamentals

- **Expressions everywhere.** No legacy command syntax.
  - v1: `Send, hello` -> v2: `Send("hello")`
  - v1: `MsgBox, hi` -> v2: `MsgBox("hi")`
- **Assignment:** `:=` for expressions, `=` is comparison (legacy `=`
  assignment is gone).
- **Strings:** double quotes. Escape with backtick:
  `` `n`` newline, `` `t`` tab, `` `"`` literal quote, `` ` `` ``backtick.
- **Comments:** `;` line, `/* ... */` block.
- **Concatenation:** dot operator with spaces: `name . " says hi"`.
  Implicit concat (`name " says hi"`) works for adjacent literals
  but the dot reads better.
- **Continuation:** trailing `,` or operator. Or wrap in parentheses.

## Hotkeys

```ahk
^!j::MyFunction()                      ; Ctrl+Alt+J
^!k:: {                                ; multi-line hotkey
    WinActivate("ahk_exe code.exe")
    Send("^p")
}

#HotIf WinActive("ahk_exe Atlas.exe")  ; scope to Atlas window
F8::Reload()
#HotIf                                  ; end scope
```

- Modifiers: `^` Ctrl, `!` Alt, `+` Shift, `#` Win.
- `*` wildcard: `*F8::` matches F8 regardless of modifiers.
- `~` passthrough: `~LButton::` fires without blocking the click.
- `$` block self-trigger: `$F8::Send("F8")` won't loop.
- `Up` suffix for release: `F8 Up::`.
- Combine with `&`: `LControl & RShift::`.
- `#HotIf <expr>` scopes everything until the next `#HotIf` (with no
  arg = end scope). The expression evaluates every time the key
  fires.

## Hotstrings

```ahk
::btw::by the way
:*:btn::button             ; * = no end char required
:?:abc::xyz                ; ? = trigger inside other words
:c:NASA::NASA              ; c = case-sensitive
:b0:omw::On my way         ; b0 = don't auto-backspace
```

- Order of options inside `:opts:` doesn't matter.
- For replacements with expressions, use a function:
  ```ahk
  ::dt::{
      SendInput(FormatTime(, "yyyy-MM-dd"))
  }
  ```

## Sending input

Three send modes, in increasing reliability and decreasing speed:

| Function | Use when |
|----------|----------|
| `SendInput()` | Default. Atomic, fast, undisturbed by user input. |
| `Send()` | Routes via `SendMode`. Use `SendMode "Input"` for the same effect as `SendInput()`. |
| `SendEvent()` | Slow, plays nicely with games and finicky targets. |
| `SendPlay()` | Legacy. Avoid unless `SendEvent` fails. |
| `ControlSend()` | Targets a specific control regardless of focus. Best for background ops. |

```ahk
SendInput("hello{Enter}")
Send("^c")                         ; default SendInput mode
SendEvent("{Tab 5}")               ; 5x Tab
ControlSend("{Enter}", , "ahk_exe game.exe")  ; background
```

- `SetKeyDelay(-1, -1)` for instant keystrokes globally. Default
  delay is 10ms which adds up.
- `SetMouseDelay(-1)` for instant mouse moves.
- `BlockInput("On")` to lock user input during a critical sequence;
  always pair with `BlockInput("Off")` (use `try ... finally`).

## Coordinates

```ahk
CoordMode "Mouse", "Screen"        ; absolute pixel coords
CoordMode "Mouse", "Window"        ; relative to active window
CoordMode "Mouse", "Client"        ; relative to client area (no title bar)
```

- Declare ONCE at the top of the script. Don't switch mid-flow.
- Always set explicitly. Default is "Client" which surprises everyone.
- For games, "Client" coords scale with the game window if DPI is
  consistent.
- **DPI awareness:** add `#DllCall("SetThreadDpiAwarenessContext", "ptr", -4, "ptr")`
  at script top if running on 125%/150% Windows scaling. Otherwise
  pixel coords lie.

## Windows and processes

```ahk
WinActivate("ahk_exe code.exe")    ; identify by exe (most reliable)
WinActivate("ahk_class CabinetWClass")   ; or by class (Explorer)
WinWait("ahk_exe game.exe", , 10)  ; wait up to 10s for it to exist
WinWaitActive("ahk_exe game.exe")  ; wait until it's focused
WinClose("Untitled - Notepad")     ; title match

if WinExist("ahk_exe game.exe") {
    pid := WinGetPID()              ; uses the matched window
    title := WinGetTitle()
}

ProcessExist("game.exe")           ; returns PID or 0
```

- `ahk_exe` over title matching. Titles change with content.
- `ahk_class` for system windows (Explorer, dialog boxes).
- `WinWait` / `WinWaitActive` before sending input to a window that
  may not be focused yet. Without it, your keys go to the wrong app.
- `WinExist` returns 0 (falsy) or the HWND; subsequent `Win*` calls
  operate on the "last found" window.

## Variables and scope

- **Global by default:** assignments at script level are global.
- **Local in functions:** function vars are local unless declared
  `global var` or via `static var`.
- **`static var`:** persists across calls. Initialized once.
- **Auto-declare locals:** v2 will warn if you write to an undeclared
  variable in strict mode. Use `local count := 0` at the function
  top for clarity.
- **Object literals:** `{key: value}`. Access with `obj.key` or
  `obj["key"]`.
- **Array literals:** `[1, 2, 3]`. 1-indexed.
- **Maps:** `m := Map(); m["k"] := "v"`. Order-preserving, beats
  objects for arbitrary keys.

## Objects and classes

```ahk
class HotkeyManager {
    __New(name) {
        this.name := name
        this.count := 0
    }
    Fire() {
        this.count++
        ToolTip(this.name . ": " . this.count)
        SetTimer(() => ToolTip(), -1500)   ; clear after 1.5s
    }
}

mgr := HotkeyManager("paste")
^!v::mgr.Fire()
```

- `__New` is the constructor.
- `this` inside methods. Arrow functions `() =>` capture `this` from
  enclosing scope.
- `static` members shared across instances.
- Use a class when state is shared across multiple hotkeys/handlers;
  free functions otherwise.

## GUI

```ahk
g := Gui("+AlwaysOnTop", "My Tool")
g.Add("Text", , "Name:")
nameEdit := g.Add("Edit", "w200", "default")
g.Add("Button", "Default", "OK").OnEvent("Click", OnOK)
g.OnEvent("Close", (*) => ExitApp())
g.Show()

OnOK(*) {
    MsgBox("hi " . nameEdit.Value)
}
```

- v2 GUI is object-based. `Gui()` constructor, `.Add()` for controls.
- `(*) =>` catches all args (event handlers pass extras).
- `OnEvent("Close", ...)` is essential; without it, the script
  keeps running after the window closes.
- For complex layouts, prefer a tabbed or split layout over a single
  cluttered window.

## Persistence

```ahk
configFile := A_ScriptDir . "\config.ini"
IniWrite(value, configFile, "Section", "Key")
value := IniRead(configFile, "Section", "Key", "default")

; JSON: use cJson.ahk or write a small encoder
FileAppend(jsonStr, A_ScriptDir . "\state.json", "UTF-8")
content := FileRead(A_ScriptDir . "\state.json", "UTF-8")
```

- `A_ScriptDir` for the script's directory. Never hardcode `C:\...`.
- INI for small config (sections + keys, no nesting).
- JSON for structured state; community library `cJson.ahk` is the
  standard.
- `FileEncoding "UTF-8"` at script top to default all File*
  operations to UTF-8.

## Timers

```ahk
SetTimer(CheckSomething, 5000)        ; every 5s
SetTimer(CheckSomething, -2000)       ; once after 2s (negative = one-shot)
SetTimer(CheckSomething, 0)           ; disable

CheckSomething() {
    if SomeCondition()
        ToolTip("hit")
}
```

- Negative period = one-shot. Positive = repeating.
- `SetTimer(fn, 0)` to disable.
- Avoid timers under 50ms unless you need them; each fires on the
  AHK main thread and competes with hotkey processing.

## Error handling

```ahk
try {
    WinActivate("ahk_exe target.exe")
    Send("hello")
}
catch as e {
    MsgBox("activate failed: " . e.Message)
}
finally {
    BlockInput("Off")
}
```

- `try / catch as e / finally`. Standard structure.
- `e.Message`, `e.What`, `e.Line`, `e.Extra`.
- `throw ValueError("bad arg")` to raise; built-in error classes:
  `Error`, `OSError`, `MemoryError`, `TypeError`, `ValueError`,
  `IndexError`, `KeyError`, `MethodError`, `PropertyError`,
  `TargetError`, `TimeoutError`, `UnsetError`, `ZeroDivisionError`.

## Performance

- **`SendInput` beats `Send`** when atomic input matters. Buffers the
  keystrokes and emits them as one event stream.
- **`SetKeyDelay(-1, -1)`** removes the 10ms per-key delay. For long
  sequences, this is the difference between snappy and laggy.
- **Hoist `WinActive` / `WinExist` checks** out of `Loop` bodies.
  Each call walks the window list.
- **`#HotIf <expr>` re-evaluates on every key press.** Keep the
  expression cheap (`WinActive("ahk_exe game.exe")` is fine; a regex
  walk is not).
- **Avoid `Sleep 0` in tight loops.** Use `Sleep 1` or rework with
  a timer. `Sleep 0` yields once per loop and still burns CPU.
- **`Map` over `Object` for arbitrary keys** -- Maps are
  hash-table-backed with predictable performance; objects pay
  property-lookup overhead.
- **Compile string concatenation in a loop into one expression**, or
  use a `Buffer` for kilobyte-scale output. AHK strings are
  immutable; repeated concat is O(n^2).
- **`SetBatchLines -1`** (legacy) is replaced in v2 by `SetWinDelay -1`,
  `SetKeyDelay -1`, `SetMouseDelay -1`, `SetControlDelay -1`. Set the
  ones you need; don't set all blindly.

## Game macros

```ahk
#HotIf WinActive("ahk_exe Atlas.exe")
F8::Reload()
NumpadAdd::{
    Loop 10 {
        SendEvent("{Space}")
        Sleep 50
    }
}
#HotIf
```

- Scope hotkeys with `#HotIf` so they don't fire in other apps.
- Many games block `SendInput` via cursor capture or hook detection.
  `SendEvent` is sometimes accepted. `ControlSend` works against
  background windows.
- **Anti-cheat awareness:** AHK is detected by VAC, EAC, BattlEye on
  many games. Using it can result in a ban. Single-player and modded
  games are typically safe; competitive multiplayer is not.
- For pixel-color reading: `PixelGetColor(x, y, "Slow")`. The "Slow"
  flag uses a different method that works through DWM compositing;
  the default fails on some setups.
- `ImageSearch` for template matching but slow on large screens;
  prefer pixel checks at known positions.

## Debugging

- `MsgBox("got here: " . val)` is the default REPL.
- `OutputDebug "value=" val` writes to DebugView (sysinternals).
- `ListVars`, `ListHotkeys`, `ListLines` -- runtime introspection
  windows.
- `KeyHistory` shows the last 40 keystrokes; invaluable for hotkey
  conflicts.
- Step debugger: VS Code with the `AutoHotkey v2 Language Support`
  extension supports DBGp. Set breakpoints, inspect variables.

## Style

- 4-space indent, opening brace same line.
- Tabs converted to spaces.
- Hotkey at column 0, body indented.
- One concept per script for simple tools. Multi-tool scripts:
  group by section with comment dividers.
- Library code under `Lib/` next to the script. AHK auto-includes
  `Lib/<func>.ahk` when `<func>` is called and not yet defined.

## Avoid

- v1 syntax (`Send, text`, `IfWinActive`, `=` for assignment). Port
  and delete.
- Tight `Loop` with no `Sleep`. Burns a CPU core; the OS will starve
  other AHK threads.
- Hardcoded screen coordinates without `CoordMode`. Breaks on
  different DPI / resolution / window size.
- `WinGetTitle` for matching. Use `ahk_exe` / `ahk_class` instead.
- Global state when a class would scope it.
- String concat in a loop without a `Buffer`. O(n^2).
- `Run` with a user-supplied string -- shell injection.
- Skipping `#Requires AutoHotkey v2.0`. The next user will run it
  on v1 and waste an hour.
