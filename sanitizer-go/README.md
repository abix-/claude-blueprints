# Claude Code Secret Sanitizer

Prevent sensitive identifiers (server names, IPs, domains) from being sent to Anthropic.

## Performance

| Hook | PowerShell | Go | Speedup |
|------|------------|-----|---------|
| `hook-file-access` | 211ms | 13ms | **16x** |
| `hook-bash` (FAKE) | 351ms | 10ms | **35x** |
| `hook-bash` (REAL) | 357ms | 197ms | **1.8x** |
| `hook-session-start` | 566ms | 64ms | **9x** |
| `hook-session-stop` | ~500ms | ~60ms | **~9x** |

Most Bash commands are FAKE (ls, git, npm, etc.) → **35x faster** per command.

## How It Works

```
1. SESSION START - Sanitize Working Tree
═══════════════════════════════════════════════════════════════════════════

When Claude Code launches, BEFORE Claude sees anything:

    Your Project Files                      Working Tree
    ┌─────────────────────┐                 ┌─────────────────────┐
    │ inventory.yml       │    scan &       │ inventory.yml       │
    │ ─────────────────── │    replace      │ ─────────────────── │
    │ host: 11.100.201.234 │ ──────────────► │ host: 11.22.33.44   │
    │ name: prod.internal │   (in place)    │ name: host-a1b.test │
    └─────────────────────┘                 └─────────────────────┘
                                                      ▲
    - Finds IPs matching patterns                     │
    - Finds hostnames matching patterns         Claude reads
    - Generates fake replacements               and edits this
    - Saves mappings to sanitizer.json


2. COMMAND EXECUTION - Run in Unsanitized Directory
═══════════════════════════════════════════════════════════════════════════

When Claude runs a command like "powershell ./Deploy-App.ps1":

┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 1: Hook intercepts command                                         │
└─────────────────────────────────────────────────────────────────────────┘
                                       │
          ┌────────────────────────────┼────────────────────────────┐
          ▼                            ▼                            ▼
    ┌───────────┐               ┌───────────┐               ┌───────────┐
    │   DENY    │               │   FAKE    │               │   REAL    │
    └─────┬─────┘               └─────┬─────┘               └─────┬─────┘
          │                           │                           │
          ▼                           ▼                           ▼

    Command tries to         Command runs in            Command runs in
    access sanitizer.json    WORKING TREE               UNSANITIZED DIR
    or unsanitized/**        (fake values)              (real values)

    Examples:                Examples:                  Examples:
    - cat sanitizer.json     - git status               - powershell script.ps1
    - ls ~/.claude/uns...    - python, npm, etc         - ansible-playbook site.yml
                             - everything else          - awx job_templates launch

    ✗ Blocked                Runs directly              Syncs changes, runs
                                                        with real values,
                                                        output sanitized
                                                               │
                                                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 2: Sync changes to unsanitized directory                           │
│                                                                         │
│     Working Tree                         Unsanitized Directory          │
│     ┌─────────────────┐     copy &       ┌─────────────────┐            │
│     │ 11.22.33.44     │   unsanitize     │ 192.168.1.100   │            │
│     │ host-a1b.test   │ ───────────────► │ prod.internal   │            │
│     └─────────────────┘   (changed       └─────────────────┘            │
│                            files only)                                  │
└─────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 3: Execute command in unsanitized directory                        │
│         (command runs with REAL values)                                 │
│                                                                         │
│         "Deploying to prod.internal..."                                 │
│         "Connected to 192.168.1.100"                                    │
└─────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 4: Sanitize output (real → fake)                                   │
│                                                                         │
│         "Deploying to host-a1b.test..."   ◄── Claude sees this          │
│         "Connected to 11.22.33.44"                                      │
└─────────────────────────────────────────────────────────────────────────┘


WHERE REAL VALUES EXIST
═══════════════════════════════════════════════════════════════════════════

    Location                     Contains Real Values?
    ─────────────────────────    ─────────────────────────────────────────
    Working tree                 ✗ NO  - always fake
    Claude's view                ✗ NO  - only sees fake
    Anthropic servers            ✗ NO  - only receives fake
    Unsanitized directory        ✓ YES - for command execution & deployment
```

## Setup

### Prerequisites

- Go 1.21+ installed
- Windows (paths assume Windows; adaptable for Linux/Mac)

### 1. Build the binary

```powershell
cd C:/code/claude-blueprints/sanitizer-go
go build -o sanitizer.exe ./cmd/sanitizer
```

### 2. Install to ~/.claude/bin

```powershell
mkdir "$env:USERPROFILE/.claude/bin" -Force
Copy-Item sanitizer.exe "$env:USERPROFILE/.claude/bin/"
```

### 3. Create sanitizer.json

Create `~/.claude/sanitizer/sanitizer.json`:

```json
{
    "mappings": {
        "real-server.internal.corp": "fake-server.example.test"
    },
    "autoMappings": {},
    "patterns": {
        "ipv4": true,
        "hostnames": ["\\.internal\\.corp$", "\\.local$"]
    },
    "unsanitizedPath": "~/.claude/unsanitized/{project}",
    "excludePaths": [".git", "node_modules", ".venv", "__pycache__"]
}
```

| Field | Description |
|-------|-------------|
| `mappings` | Manual real → fake mappings (takes precedence) |
| `autoMappings` | Auto-discovered IPs/hostnames (populated automatically) |
| `patterns.ipv4` | Auto-discover IPv4 addresses |
| `patterns.hostnames` | Regex patterns for hostname discovery |
| `unsanitizedPath` | Where to write unsanitized version (`{project}` expands to project folder name) |
| `excludePaths` | Paths to skip during sanitization |

### 4. Configure settings.json

Add to `~/.claude/settings.json`:

```json
{
    "permissions": {
        "deny": [
            "~/.claude/sanitizer/sanitizer.json",
            "~/.claude/unsanitized/**"
        ]
    },
    "hooks": {
        "SessionStart": [
            {
                "matcher": "",
                "hooks": [{
                    "type": "command",
                    "command": "%USERPROFILE%/.claude/bin/sanitizer.exe hook-session-start"
                }]
            }
        ],
        "PreToolUse": [
            {
                "matcher": "Bash",
                "hooks": [{
                    "type": "command",
                    "command": "%USERPROFILE%/.claude/bin/sanitizer.exe hook-bash"
                }]
            },
            {
                "matcher": "Read|Edit|Write",
                "hooks": [{
                    "type": "command",
                    "command": "%USERPROFILE%/.claude/bin/sanitizer.exe hook-file-access"
                }]
            }
        ],
        "Stop": [
            {
                "matcher": "",
                "hooks": [{
                    "type": "command",
                    "command": "%USERPROFILE%/.claude/bin/sanitizer.exe hook-session-stop"
                }]
            }
        ]
    }
}
```

### 5. Restart Claude Code

## Usage

1. Start Claude - files get sanitized, unsanitized copy created
2. Work normally - Claude sees fake values, commands run with real values
3. Deploy from `~/.claude/unsanitized/{project}/`

### If Claude crashes

Working tree stays fake (safe). Unsanitized directory already has real values.

## CLI Commands

```
sanitizer.exe <command>

Commands:
  hook-session-start   Sanitize project at session start
  hook-session-stop    Sync to unsanitized directory at session end
  hook-bash            Route Bash commands (DENY/FAKE/REAL)
  hook-file-access     Block access to sensitive files
  sanitize-ips         Stdin→stdout IP sanitization (deterministic)
  exec                 Run command in unsanitized dir, sanitize output
```

### Standalone usage

```powershell
# Sanitize text with deterministic fake IPs
echo "Server at 11.100.201.234" | sanitizer.exe sanitize-ips
# Output: Server at 11.145.240.80

# Manually run session start (sanitize current directory)
sanitizer.exe hook-session-start

# Manually sync to unsanitized directory
sanitizer.exe hook-session-stop
```

## Command Routing

### DENY - Blocked entirely

Commands accessing sensitive paths:
- `*/sanitizer.json`
- `~/.claude/unsanitized/*`

### REAL - Run in unsanitized directory with real values

| Pattern | Examples |
|---------|----------|
| `powershell` / `pwsh` | `powershell ./script.ps1` |
| `*.ps1` | `./Deploy-App.ps1` |
| `& ...` | `& $command` |
| `ansible*` | `ansible-playbook site.yml` |
| `awx` | `awx job_templates launch` |

Output from REAL commands is sanitized before Claude sees it.

### FAKE - Run in working tree with fake values (default)

Everything else: `git`, `ls`, `npm`, `python`, etc.

## IP Handling

### Auto-discovered (sanitized)
- Private ranges: `10.x.x.x`, `172.16-31.x.x`, `192.168.x.x`
- Public IPs found in project files

### Excluded (not sanitized)
- Loopback: `127.x.x.x`
- Broadcast: `0.0.0.0`, `255.255.255.255`
- Link-local: `169.254.x.x`
- Multicast: `224.x.x.x`, `239.x.x.x`
- Already fake: `11.x.x.x` (our fake range)

### Fake IP generation

- **Random**: Used for auto-discovered values (stored in `autoMappings`)
- **Deterministic**: Used for output scrubbing (MD5 hash → consistent fake IP)

All fake IPs use the `11.x.x.x` range.

## Project Structure

```
sanitizer-go/
├── cmd/sanitizer/main.go   # CLI entry point
├── internal/
│   ├── config.go           # Load/save sanitizer.json
│   ├── file.go             # File operations, binary detection
│   ├── hook_bash.go        # Bash command routing
│   ├── hook_fileaccess.go  # File access blocking
│   ├── hook_session.go     # Session start/stop hooks
│   ├── ip.go               # IP detection/generation
│   └── text.go             # Text transformation
├── go.mod
└── README.md
```

## Files Blocked from Claude

| Path | Reason |
|------|--------|
| `~/.claude/sanitizer/sanitizer.json` | Contains real→fake mappings |
| `~/.claude/unsanitized/**` | Contains real values |

## Troubleshooting

### Hook not running

Check `settings.json` paths point to the correct binary location.

### Files not sanitized

1. Check `sanitizer.json` exists and is valid JSON
2. Check `patterns.ipv4` is `true` or `patterns.hostnames` has patterns
3. Check file isn't in `excludePaths`
4. Check file isn't binary or >10MB

### Command runs with fake values when it shouldn't

Add the command pattern to the REAL patterns in `internal/hook/bash.go` and rebuild.

### UTF-8 BOM issues

The Go sanitizer strips UTF-8 BOM from `sanitizer.json`. If you see JSON parse errors, check for BOM with `xxd sanitizer.json | head -1`.
