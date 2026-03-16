local wezterm = require 'wezterm'
local config = wezterm.config_builder()

-- GPU-accelerated rendering (the whole point of switching)
config.front_end = "WebGpu"

-- shell: powershell for interactive use; Claude Code still uses bash internally
config.default_prog = { 'pwsh.exe' }
config.default_cwd = 'C:/code/endless'

-- font
config.font = wezterm.font('Cascadia Code', { weight = 'Regular' })
config.font_size = 12.0

-- color scheme
config.color_scheme = 'Catppuccin Mocha'

-- window
config.window_padding = { left = 4, right = 4, top = 4, bottom = 4 }
config.window_decorations = "RESIZE"
config.initial_cols = 140
config.initial_rows = 40

-- tabs
config.hide_tab_bar_if_only_one_tab = false
config.tab_bar_at_bottom = true
config.use_fancy_tab_bar = false
config.tab_max_width = 500
config.window_frame = {
  font_size = 14.0,
}

local function pane_label(pane_info)
  local cwd = pane_info.current_working_dir
  if not cwd then return "" end

  local path = cwd.file_path
  -- strip leading / from URL-style paths on Windows (/C:/foo -> C:/foo)
  path = path:gsub("^/(%a:)", "%1")
  -- strip trailing slash
  path = path:gsub("[/\\]$", "")
  -- normalize to backslashes for io.open on Windows
  path = path:gsub("/", "\\")

  local folder = path:match("([^\\]+)$") or ""
  local branch = ""

  -- walk up to find .git/HEAD (no process spawn)
  local check = path
  for _ = 1, 8 do
    local head_path = check .. "\\.git\\HEAD"
    local f = io.open(head_path, "r")
    if f then
      local head = f:read("*l") or ""
      f:close()
      branch = head:match("ref: refs/heads/(.+)") or head:sub(1, 8)
      folder = check:match("([^\\]+)$") or folder
      break
    end
    local parent = check:match("^(.+)\\[^\\]+$")
    if not parent then break end
    check = parent
  end

  if branch ~= "" then
    return folder .. ":" .. branch
  end
  return folder
end

wezterm.on("format-tab-title", function(tab)
  local parts = {}
  for _, p in ipairs(tab.panes) do
    local label = pane_label(p)
    parts[#parts + 1] = label ~= "" and label or "shell"
  end
  local icon = tab.is_active and " * " or "   "
  local text = #parts > 0 and table.concat(parts, " | ") or "shell"
  return { { Text = icon .. text .. " " } }
end)

-- panes
config.pane_focus_follows_mouse = true
config.inactive_pane_hsb = { brightness = 0.7 }

-- scrollback
config.scrollback_lines = 10000

-- disable update nag (we manage via winget)
config.check_for_updates = false

-- keybinds for split panes (great for cargo build + run side by side)
config.keys = {
  { key = '|', mods = 'CTRL|SHIFT', action = wezterm.action.SplitHorizontal { domain = 'CurrentPaneDomain' } },
  { key = '_', mods = 'CTRL|SHIFT', action = wezterm.action.SplitVertical { domain = 'CurrentPaneDomain' } },
  { key = 'w', mods = 'CTRL|SHIFT', action = wezterm.action.CloseCurrentPane { confirm = true } },
  { key = 'q', mods = 'CTRL|SHIFT', action = wezterm.action.QuitApplication },
  { key = 'n', mods = 'CTRL|SHIFT', action = wezterm.action.SplitHorizontal {
    args = { 'pwsh.exe', '-NoExit', '-File', 'C:/code/endless/scripts/claude-next.ps1' },
  }},
}

return config
