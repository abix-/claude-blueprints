# WezTerm + Claude Code Multi-Agent Setup

Terminal config for running multiple Claude Code agents in parallel, each in its own git worktree.

## What it does

- **Ctrl+Shift+N**: spawns a new Claude agent in the next free `endless-claude-{N}` worktree
- **Tab titles**: show `folder:branch` by reading `.git/HEAD` directly (no subprocess)
- **Pane splits**: Ctrl+Shift+| (horizontal), Ctrl+Shift+_ (vertical)
- GPU-accelerated rendering via WebGpu
- Catppuccin Mocha theme, Cascadia Code font

## Files

- `.wezterm.lua` -- main config, goes to `~/.wezterm.lua`
- `claude-next.ps1` -- agent launcher, goes to your project's `scripts/` dir

## Setup

```powershell
# install wezterm
winget install wez.wezterm

# copy config
copy wezterm\.wezterm.lua $HOME\.wezterm.lua

# copy launcher to your project
copy wezterm\claude-next.ps1 C:\code\endless\scripts\claude-next.ps1
```

## How it works

1. Press `Ctrl+Shift+N` in WezTerm
2. `claude-next.ps1` queries `wezterm cli list` for occupied pane cwds
3. Finds the first unused slot (1-10)
4. Creates a git worktree at `C:\code\endless-claude-{N}` if needed
5. Launches `claude` in that worktree
6. Tab title auto-updates to show `endless-claude-3:issue-42` etc.

Each agent derives its identity from its worktree path -- no config files or registration needed.

## Customization

Edit `.wezterm.lua`:
- `config.default_cwd` -- your project root
- `config.default_prog` -- shell (pwsh, bash, etc.)
- `claude-next.ps1` path in the `Ctrl+Shift+N` keybind
- `$base` and project name in `claude-next.ps1`
