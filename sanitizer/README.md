# Claude Code Secret Sanitizer

Prevent sensitive identifiers (server names, IPs, domains) from being sent to Anthropic.

## Performance

| Hook | PowerShell | Go | Speedup |
|------|------------|-----|---------|
| `hook-file-access` | 211ms | 13ms | **16x** |
| `hook-bash` (SANITIZED) | 351ms | 10ms | **35x** |
| `hook-bash` (UNSANITIZED) | 357ms | 197ms | **1.8x** |
| `hook-session-start` | 566ms | 64ms | **9x** |
| `hook-session-stop` | ~500ms | ~60ms | **~9x** |

Most Bash commands are SANITIZED (ls, git, npm, etc.) → **35x faster** per command.

## How It Works

```
1. SESSION START - Sanitize Working Tree
═══════════════════════════════════════════════════════════════════════════

When Claude Code launches, BEFORE Claude sees anything:

    Your Project Files                      Working Tree
    ┌─────────────────────┐                 ┌─────────────────────┐
    │ inventory.yml       │    scan &       │ inventory.yml       │
    │ ─────────────────── │    replace      │ ─────────────────── │
    │ host: 111.55.104.65 │ ──────────────► │ host: 111.52.117.80 │
    │ name: prod.internal │   (in place)    │ name: host-a1b.test │
    └─────────────────────┘                 └─────────────────────┘
                                                      ▲
    - Finds IPs matching patterns                     │
    - Finds hostnames matching patterns         Claude reads
    - Generates random sanitized replacements       and edits this
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
    │   BLOCK   │               │ SANITIZED │               │UNSANITIZED│
    └─────┬─────┘               └─────┬─────┘               └─────┬─────┘
          │                           │                           │
          ▼                           ▼                           ▼

    Command tries to         Command runs in            Command runs in
    access sanitizer.json    WORKING TREE               UNSANITIZED DIR
    or unsanitized/**        (sanitized values)         (real values)

    Examples:                Examples:                  Examples:
    - cat sanitizer.json     - git status               - powershell script.ps1
    - ls ~/.claude/uns...    - python, npm, etc         - pwsh ./deploy.ps1
                             - everything else          - & $command

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
│     │ 111.52.117.80   │   unsanitize     │ 111.55.104.65   │            │
│     │ host-a1b.test   │ ───────────────► │ prod.internal   │            │
│     └─────────────────┘   (changed       └─────────────────┘            │
│                            files only)                                  │
└─────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 3: Execute command in unsanitized directory                        │
│         (command string is also unsanitized)                            │
│                                                                         │
│         "Deploying to prod.internal..."                                 │
│         "Connected to 111.55.104.65"                                    │
└─────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 4: Sanitize output (real → sanitized)                              │
│                                                                         │
│         "Deploying to host-a1b.test..."   ◄── Claude sees this          │
│         "Connected to 111.52.117.80"                                    │
└─────────────────────────────────────────────────────────────────────────┘


WHERE REAL VALUES EXIST
═══════════════════════════════════════════════════════════════════════════

    Location                     Contains Real Values?
    ─────────────────────────    ─────────────────────────────────────────
    Working tree                 ✗ NO  - always sanitized
    Claude's view                ✗ NO  - only sees sanitized
    Anthropic servers            ✗ NO  - only receives sanitized
    Unsanitized directory        ✓ YES - for command execution & deployment
```

## Setup

### Prerequisites

- Go 1.21+ installed
- Windows (paths assume Windows; adaptable for Linux/Mac)

### 1. Build the binary

```powershell
cd C:/code/claude-blueprints/sanitizer
go build -o sanitizer.exe ./cmd/sanitizer
```

### 2. Install to ~/.claude/sanitizer

```powershell
mkdir "$env:USERPROFILE/.claude/sanitizer" -Force
Copy-Item sanitizer.exe "$env:USERPROFILE/.claude/sanitizer/"
```

### 3. Create sanitizer.json

Create `~/.claude/sanitizer/sanitizer.json`:

```json
{
    "hostnamePatterns": ["[A-Za-z]{7}[0-9]{2}L?", "\\.domain\\.local$"],
    "mappingsAuto": {},
    "mappingsManual": {
        "server.example.test": "server.example.test",
        "111.55.104.65": "111.50.100.1",
        "C:\\Users\\realuser": "C:\\Users\\sanitizeduser",
        "projectname": "projectname"
    },
    "skipPaths": [".git", "node_modules", ".venv", "__pycache__"],
    "unsanitizedPath": "~/.claude/unsanitized/{project}",
    "blockedPaths": [
        "\\.claude/sanitizer/sanitizer\\.json$",
        "\\.claude/unsanitized/"
    ]
}
```

| Field | Description |
|-------|-------------|
| `mappingsManual` | Manual real → sanitized mappings (takes precedence) |
| `mappingsAuto` | Auto-discovered IPs/hostnames (populated automatically) |
| `hostnamePatterns` | Regex patterns for hostname discovery (see [Hostname Patterns](#hostname-patterns)) |
| `skipPaths` | Paths to skip during sanitization |
| `unsanitizedPath` | Where to write unsanitized version (`{project}` expands to project folder name) |
| `blockedPaths` | Regex patterns for paths Claude cannot access (blocks Read/Edit/Write/Bash) |

### 4. Configure settings.json

Add to `~/.claude/settings.json`:

```json
{
    "hooks": {
        "SessionStart": [
            {
                "matcher": "",
                "hooks": [{
                    "type": "command",
                    "command": "%USERPROFILE%/.claude/sanitizer/sanitizer.exe hook-session-start"
                }]
            }
        ],
        "PreToolUse": [
            {
                "matcher": "Bash",
                "hooks": [{
                    "type": "command",
                    "command": "%USERPROFILE%/.claude/sanitizer/sanitizer.exe hook-bash"
                }]
            },
            {
                "matcher": "Read|Edit|Write",
                "hooks": [{
                    "type": "command",
                    "command": "%USERPROFILE%/.claude/sanitizer/sanitizer.exe hook-file-access"
                }]
            }
        ],
        "PostToolUse": [
            {
                "matcher": "Grep|Glob",
                "hooks": [{
                    "type": "command",
                    "command": "%USERPROFILE%/.claude/sanitizer/sanitizer.exe hook-post"
                }]
            }
        ],
        "Stop": [
            {
                "matcher": "",
                "hooks": [{
                    "type": "command",
                    "command": "%USERPROFILE%/.claude/sanitizer/sanitizer.exe hook-session-stop"
                }]
            }
        ]
    }
}
```

### 5. Restart Claude Code

## Usage

1. Start Claude - files get sanitized, mappings saved
2. Work normally - Claude sees sanitized values, commands run with real values
3. Deploy from `~/.claude/unsanitized/{project}/`

### If Claude crashes

Working tree stays sanitized (safe). Unsanitized directory already has real values.

## CLI Commands

```
sanitizer.exe <command>

Commands:
  hook-session-start   Sanitize project at session start
  hook-session-stop    Sync to unsanitized directory at session end
  hook-bash            Route Bash commands (BLOCK/SANITIZED/UNSANITIZED)
  hook-file-access     Block access to sensitive files, sanitize on read/write
  hook-post            Sanitize tool output (for Grep/Glob)
  sanitize-ips         Stdin→stdout IP sanitization
  exec                 Run command in unsanitized dir, sanitize output
```

### Standalone usage

```powershell
# Sanitize text (discovers new IPs, saves to mappings)
echo "Server at 111.55.104.65" | sanitizer.exe sanitize-ips
# Output: Server at 111.52.117.80

# Manually run session start (sanitize current directory)
sanitizer.exe hook-session-start

# Manually sync to unsanitized directory
sanitizer.exe hook-session-stop
```

## Command Routing

### BLOCK - Blocked entirely

Commands accessing sensitive paths:
- `*/sanitizer.json`
- `~/.claude/unsanitized/*`

### UNSANITIZED - Run with real values

| Pattern | Examples |
|---------|----------|
| `powershell` / `pwsh` | `powershell ./script.ps1` |
| `*.ps1` | `./Deploy-App.ps1` |
| `& ...` | `& $command` |

Command string is unsanitized before execution. Output is sanitized before Claude sees it.

### SANITIZED - Run with sanitized values (default)

Everything else: `git`, `ls`, `npm`, `python`, etc.

## Hostname Patterns

Patterns in `hostnamePatterns` are wrapped before matching:

```
(?i)\b + YOUR_PATTERN + (?:\.[a-zA-Z0-9-]+)*
```

- `(?i)` - case insensitive
- `\b` - must start at word boundary
- `(?:\.[a-zA-Z0-9-]+)*` - captures optional domain suffixes

### Pattern Examples

| Use Case | Pattern | Matches |
|----------|---------|---------|
| Domain suffix | `[a-z0-9-]+\.corp\.local$` | `server01.corp.local` |
| Server naming convention | `[A-Za-z]{7}[0-9]{2}L?` | `YOURSVR01`, `yoursvr01L` |
| Prefix-based | `prod-[a-z0-9]+` | `prod-web01`, `prod-db` |

### Identity Mappings

To protect values from being sanitized, add them to `mappingsManual` with identical key/value:

```json
"mappingsManual": {
    "PackedInt32Array": "PackedInt32Array",
    "PackedFloat32Array": "PackedFloat32Array"
}
```

Use this when patterns accidentally match programming type names (e.g., Godot's `Packed*Array` types) or other strings you want preserved.

## IP Handling

### Auto-discovered (sanitized)
- Private ranges: `10.x.x.x`, `172.16-31.x.x`, `192.168.x.x`
- Public IPs found in project files

### Excluded (not sanitized)
- Loopback: `127.x.x.x`
- Broadcast: `0.0.0.0`, `255.255.255.255`
- Link-local: `169.254.x.x`
- Multicast: `224.x.x.x` - `239.x.x.x`
- Subnet masks: `255.x.x.x`
- Already sanitized: `111.x.x.x` (our sanitized range)

### Sanitized IP generation

All sanitized IPs use the `111.x.x.x` range with random octets (1-254).

- **First discovery**: Random IP generated, saved to `mappingsAuto`
- **Subsequent encounters**: Looked up from saved mappings (consistent)
- **Collision detection**: Regenerates if random value already used

This approach is not reversible - someone with sanitized output cannot determine the original IP.

## Testing

38 tests covering all functionality. Requires [Pester](https://pester.dev/) v5+:

```powershell
# Install Pester 5 (if needed)
Install-Module -Name Pester -Force -SkipPublisherCheck -Scope CurrentUser

# Run tests
Invoke-Pester ./sanitizer.tests.ps1 -Output Detailed
```

### Test Coverage

| Category | Tests | What's Tested |
|----------|-------|---------------|
| sanitize-ips | 5 | Private/public/excluded IP ranges, determinism |
| hook-bash | 3 | BLOCK/SANITIZED/UNSANITIZED routing |
| hook-file-access | 3 | Blocking sensitive files, Write content sanitization |
| hook-post | 2 | Output sanitization for Grep/Glob |
| hook-session-start | 6 | File sanitization, skip paths, binary detection |
| hook-session-stop | 1 | Unsanitized directory sync |
| hostname-patterns | 6 | Regex matching, FQDN capture, identity mappings |
| exec | 2 | Command execution with real values, output sanitization |
| manual-mappings | 2 | Precedence over auto, custom replacements |
| text-transformation | 1 | Longest-key-first replacement |
| file-handling | 3 | Binary detection, 10MB limit, skip paths |
| config-handling | 2 | Default creation, UTF-8 BOM |
| regression-tests | 2 | Hostname charset, config key preservation |

## Project Structure

```
sanitizer/
├── cmd/sanitizer/main.go    # CLI entry point
├── internal/
│   ├── config.go            # Load/save sanitizer.json
│   ├── exec.go              # Run command with real values
│   ├── file.go              # File operations, binary detection
│   ├── hook_bash.go         # Bash command routing
│   ├── hook_fileaccess.go   # File access blocking/sanitization
│   ├── hook_post.go         # Post-tool output sanitization
│   ├── hook_session.go      # Session start/stop hooks
│   ├── ip.go                # IP detection/generation
│   └── text.go              # Text transformation
├── go.mod
├── sanitizer.tests.ps1      # Pester test suite
└── README.md
```

## Files Blocked from Claude

Configured via `blockedPaths` in sanitizer.json (regex patterns matched against normalized `/` paths):

| Default Pattern | Blocks | Reason |
|-----------------|--------|--------|
| `\.claude/sanitizer/sanitizer\.json$` | Config file | Contains real→sanitized mappings |
| `\.claude/unsanitized/` | Unsanitized directory | Contains real values |

## Troubleshooting

### Hook not running

Check `settings.json` paths point to the correct binary location.

### Files not sanitized

1. Check `sanitizer.json` exists and is valid JSON
2. Check `hostnamePatterns` has patterns for hostname discovery (IPv4 is always enabled)
3. Check file isn't in `skipPaths`
4. Check file isn't binary or >10MB

### Command runs with sanitized values when it shouldn't

Add the command pattern to `unsanitizedCmdPatterns` in `internal/hook_bash.go` and rebuild.

### UTF-8 BOM issues

The Go sanitizer strips UTF-8 BOM from `sanitizer.json`. If you see JSON parse errors, check for BOM with `xxd sanitizer.json | head -1`.
