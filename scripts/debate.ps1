# debate.ps1 - structured debate loop for multiple AI agents
# one file, no dependencies, replaces debate.py + debate.cmd

param(
    [Parameter(Position=0)]
    [string]$Command,
    [Parameter(Position=1, ValueFromRemainingArguments)]
    [string[]]$Args
)

$ErrorActionPreference = 'Stop'

$DebateDir = Join-Path $env:USERPROFILE '.debate'
$ActiveDir = Join-Path $DebateDir 'active'
$ArchiveDir = Join-Path $DebateDir 'archive'
$StateFile = Join-Path $ActiveDir 'state.json'
$MessagesFile = Join-Path $ActiveDir 'messages.jsonl'
$GoalFile = Join-Path $ActiveDir 'goal.md'
$LockFile = Join-Path $ActiveDir '.lock'

$Phases = @('GOAL','PROPOSE','DISCUSS','CONSENSUS','IMPLEMENT','VERIFY','DONE')

function Now-Iso { [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ') }

function With-Lock {
    param([scriptblock]$Action)
    if (-not (Test-Path $ActiveDir)) { New-Item -ItemType Directory -Path $ActiveDir -Force | Out-Null }
    $stream = [System.IO.File]::Open($LockFile, 'OpenOrCreate', 'ReadWrite', 'None')
    try { & $Action }
    finally { $stream.Close() }
}

function Load-State {
    if (-not (Test-Path $StateFile)) { return $null }
    Get-Content $StateFile -Raw | ConvertFrom-Json
}

function Save-State {
    param($state)
    $state.updated = Now-Iso
    $state | ConvertTo-Json -Depth 10 | Set-Content $StateFile -Encoding UTF8
}

function Append-Message {
    param($msg)
    $msg | Add-Member -NotePropertyName 'ts' -NotePropertyValue (Now-Iso) -Force
    ($msg | ConvertTo-Json -Depth 10 -Compress) | Add-Content $MessagesFile -Encoding UTF8
}

function Read-Messages {
    param([int]$Last = 0)
    if (-not (Test-Path $MessagesFile)) { return @() }
    $lines = Get-Content $MessagesFile -Encoding UTF8 | Where-Object { $_.Trim() }
    $msgs = $lines | ForEach-Object { $_ | ConvertFrom-Json }
    if ($Last -gt 0 -and $msgs.Count -gt $Last) {
        $msgs = $msgs[($msgs.Count - $Last)..($msgs.Count - 1)]
    }
    $msgs
}

function Require-State {
    $s = Load-State
    if ($null -eq $s) { Write-Host 'no active debate. run: debate new "goal"'; exit 1 }
    $s
}

function Get-SessionId {
    $sid = $env:CLAUDE_CODE_SESSION_ID
    if (-not $sid) { Write-Host 'CLAUDE_CODE_SESSION_ID not set. run this from inside a Claude Code session.'; exit 1 }
    $sid
}

function Get-Agent {
    param($state)
    $sid = Get-SessionId
    foreach ($k in $state.participants.PSObject.Properties.Name) {
        $v = $state.participants.$k
        if ($v.session_id -eq $sid) { return $k }
    }
    Write-Host 'this session has not joined the debate. run: debate join'; exit 1
}

function Resolve-Body {
    param($msg)
    if ($msg.body_file -and (Test-Path $msg.body_file)) {
        return (Get-Content $msg.body_file -Raw -Encoding UTF8).Trim()
    }
    if ($msg.body_file) { return "(file missing: $($msg.body_file))" }
    $msg.body
}

function Get-Body {
    param([string[]]$remaining)
    $fileIdx = [Array]::IndexOf($remaining, '--file')
    if ($fileIdx -ge 0 -and $fileIdx -lt $remaining.Count - 1) {
        $path = $remaining[$fileIdx + 1]
        if (-not (Test-Path $path)) { Write-Host "file not found: $path"; exit 1 }
        return (Get-Content $path -Raw -Encoding UTF8).Trim()
    }
    $textParts = $remaining | Where-Object { $_ -ne '--file' }
    if (-not $textParts) { Write-Host 'provide a message or use --file <path>'; exit 1 }
    $textParts -join ' '
}

function Save-BodyFile {
    param([string]$body, [string]$label)
    $bodiesDir = Join-Path $ActiveDir 'bodies'
    if (-not (Test-Path $bodiesDir)) { New-Item -ItemType Directory -Path $bodiesDir -Force | Out-Null }
    $ts = [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss')
    $path = Join-Path $bodiesDir "$label-$ts.md"
    Set-Content $path -Value "$body`n" -Encoding UTF8
    $path
}

function Build-MessageFields {
    param([string]$body, [string]$label)
    if ($body.Length -gt 500) {
        $path = Save-BodyFile $body $label
        return @{ body = $body.Substring(0, 200) + '...'; body_file = $path }
    }
    @{ body = $body }
}

function Format-Message {
    param($msg)
    $ts = ''
    if ($msg.ts) {
        if ($msg.ts -is [DateTime]) { $ts = $msg.ts.ToString('HH:mm:ss') }
        elseif ($msg.ts -is [string] -and $msg.ts.Length -ge 19) { $ts = $msg.ts.Substring(11, 8) }
    }
    $sender = if ($msg.from) { $msg.from } else { '?' }
    $body = Resolve-Body $msg
    $header = "[$ts] $sender"
    if ($msg.phase) {
        $header += " ($($msg.phase)"
        if ($msg.round) { $header += " r$($msg.round)" }
        if ($msg.verdict) { $header += " $($msg.verdict)" }
        $header += ')'
    }
    Write-Host "$header`: $body"
}

# --- commands ---

function Cmd-New {
    param([string[]]$remaining)
    With-Lock {
        if (Test-Path $StateFile) { Write-Host 'a debate is already active. run: debate done'; exit 1 }
        if (-not (Test-Path $ActiveDir)) { New-Item -ItemType Directory -Path $ActiveDir -Force | Out-Null }
        $ts = Now-Iso
        $id = 'debate-' + [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss')
        $state = [PSCustomObject]@{
            id = $id
            phase = 'GOAL'
            round = 0
            last_action_by = $null
            participants = [PSCustomObject]@{
                human = [PSCustomObject]@{ name = 'human'; type = 'human' }
            }
            created = $ts
            updated = $ts
        }
        Save-State $state
        $goal = $remaining -join ' '
        Set-Content $GoalFile -Value "$goal`n" -Encoding UTF8
        Append-Message ([PSCustomObject]@{ from = 'human'; phase = 'GOAL'; body = $goal })
        Write-Host "debate started: $id"
        Write-Host "goal: $goal"
        Write-Host ''
        Write-Host '=== terminal 1 (your claude) ==='
        Write-Host '  claude'
        Write-Host '  then say: /debate'
        Write-Host ''
        Write-Host '=== terminal 2 (other claude) ==='
        Write-Host '  claude'
        Write-Host '  then say: /debate'
    }
}

function Cmd-Join {
    param([string[]]$remaining)
    if ($null -eq $remaining) { $remaining = @() }
    With-Lock {
        $state = Require-State
        $sid = Get-SessionId
        foreach ($k in $state.participants.PSObject.Properties.Name) {
            if ($state.participants.$k.session_id -eq $sid) {
                Write-Host "already joined as $k"; return
            }
        }
        $agents = $state.participants.PSObject.Properties.Name | Where-Object { $state.participants.$_.type -ne 'human' }
        $name = if ($remaining.Count -gt 0 -and $remaining[0] -and $remaining[0] -notlike '--*') { $remaining[0] } else { "agent-$([char]([int][char]'a' + @($agents).Count))" }
        if ($state.participants.PSObject.Properties.Name -contains $name) { Write-Host "$name already taken"; exit 1 }
        $agentType = 'claude-code'
        $typeIdx = [Array]::IndexOf($remaining, '--type')
        if ($typeIdx -ge 0 -and $typeIdx -lt $remaining.Count - 1) { $agentType = $remaining[$typeIdx + 1] }
        $state.participants | Add-Member -NotePropertyName $name -NotePropertyValue ([PSCustomObject]@{
            name = $name; type = $agentType; session_id = $sid
        }) -Force
        Append-Message ([PSCustomObject]@{ from = 'system'; phase = $state.phase; body = "$name joined as $agentType" })
        if ($state.phase -eq 'GOAL') {
            $state.phase = 'PROPOSE'
            $state.round = 1
            Append-Message ([PSCustomObject]@{ from = 'system'; phase = 'PROPOSE'; body = 'round 1. any agent may propose.' })
        }
        Save-State $state
        Write-Host "joined as $name ($agentType)"
    }
}

function Cmd-Status {
    $state = Require-State
    $agents = ($state.participants.PSObject.Properties.Name | Where-Object { $state.participants.$_.type -ne 'human' }) -join ', '
    $last = if ($state.last_action_by) { $state.last_action_by } else { '(none)' }
    $goal = if (Test-Path $GoalFile) { (Get-Content $GoalFile -Raw -Encoding UTF8).Trim() } else { '(none)' }
    Write-Host "debate:   $($state.id)"
    Write-Host "phase:    $($state.phase)"
    Write-Host "round:    $($state.round)"
    Write-Host "last:     $last"
    Write-Host "agents:   $agents"
    Write-Host "goal:     $goal"
}

function Cmd-Read {
    param([string[]]$remaining)
    if ($null -eq $remaining) { $remaining = @() }
    $last = 0
    $lastIdx = if ($remaining.Count -gt 0) { [Array]::IndexOf($remaining, '--last') } else { -1 }
    if ($lastIdx -ge 0 -and $lastIdx -lt $remaining.Count - 1) { $last = [int]$remaining[$lastIdx + 1] }
    $msgs = Read-Messages -Last $last
    if (-not $msgs -or $msgs.Count -eq 0) { Write-Host 'no messages yet'; return }
    foreach ($m in $msgs) { Format-Message $m }
}

function Cmd-Propose {
    param([string[]]$remaining)
    With-Lock {
        $state = Require-State
        $agent = Get-Agent $state
        $body = Get-Body $remaining
        if ($state.phase -ne 'PROPOSE') { Write-Host "cannot propose in phase $($state.phase)"; exit 1 }
        $state.last_action_by = $agent
        $fields = Build-MessageFields $body "propose-$agent-r$($state.round)"
        $msg = [PSCustomObject]@{ from = $agent; phase = 'PROPOSE'; round = $state.round }
        foreach ($k in $fields.Keys) { $msg | Add-Member -NotePropertyName $k -NotePropertyValue $fields[$k] -Force }
        Append-Message $msg
        $state.phase = 'DISCUSS'
        Save-State $state
        Write-Host 'proposal submitted. waiting for discussion.'
    }
}

function Cmd-Discuss {
    param([string[]]$remaining)
    With-Lock {
        $state = Require-State
        $agent = Get-Agent $state
        if ($state.phase -ne 'DISCUSS') { Write-Host "cannot discuss in phase $($state.phase)"; exit 1 }
        $verdict = $remaining[0]
        if ($verdict -notin @('agree','disagree','revise')) { Write-Host "verdict must be: agree, disagree, or revise"; exit 1 }
        $bodyArgs = $remaining[1..($remaining.Count - 1)]
        $agents = @($state.participants.PSObject.Properties.Name | Where-Object { $state.participants.$_.type -ne 'human' })
        $proposer = $state.last_action_by
        if ($agent -eq $proposer) {
            if ($agents.Count -ge 2) {
                Write-Host 'another agent is available to discuss. let them do it.'
            } else {
                Write-Host 'solo agent cannot discuss their own proposal. a second agent must join.'
            }
            exit 1
        }
        $body = Get-Body $bodyArgs
        $state.last_action_by = $agent
        $fields = Build-MessageFields $body "discuss-$agent-r$($state.round)"
        $msg = [PSCustomObject]@{ from = $agent; phase = 'DISCUSS'; round = $state.round; verdict = $verdict }
        foreach ($k in $fields.Keys) { $msg | Add-Member -NotePropertyName $k -NotePropertyValue $fields[$k] -Force }
        Append-Message $msg
        if ($verdict -eq 'agree') {
            $state.phase = 'CONSENSUS'
            Append-Message ([PSCustomObject]@{ from = 'system'; phase = 'CONSENSUS'; body = 'agreement reached. waiting for human to advance to IMPLEMENT or send back.' })
        } else {
            $state.round += 1
            $state.phase = 'PROPOSE'
            Append-Message ([PSCustomObject]@{ from = 'system'; phase = 'PROPOSE'; body = "disagreement. round $($state.round)." })
        }
        Save-State $state
    }
}

function Cmd-Say {
    param([string[]]$remaining)
    With-Lock {
        $state = Require-State
        $body = Get-Body $remaining
        $fields = Build-MessageFields $body 'say-human'
        $msg = [PSCustomObject]@{ from = 'human'; phase = $state.phase }
        foreach ($k in $fields.Keys) { $msg | Add-Member -NotePropertyName $k -NotePropertyValue $fields[$k] -Force }
        Append-Message $msg
        Write-Host 'message sent'
    }
}

function Cmd-Force {
    param([string[]]$remaining)
    With-Lock {
        $state = Require-State
        $phase = $remaining[0].ToUpper()
        if ($phase -notin $Phases) { Write-Host "unknown phase: $phase. valid: $($Phases -join ', ')"; exit 1 }
        $old = $state.phase
        $state.phase = $phase
        if ($phase -eq 'PROPOSE') { $state.round += 1 }
        Save-State $state
        Append-Message ([PSCustomObject]@{ from = 'human'; phase = $phase; body = "human forced phase: $old -> $phase" })
        Write-Host "phase changed: $old -> $phase"
    }
}

function Cmd-Implement {
    param([string[]]$remaining)
    With-Lock {
        $state = Require-State
        $agent = Get-Agent $state
        $body = Get-Body $remaining
        if ($state.phase -notin @('CONSENSUS','IMPLEMENT')) { Write-Host "cannot implement in phase $($state.phase)"; exit 1 }
        $state.phase = 'IMPLEMENT'
        $state.last_action_by = $agent
        Save-State $state
        $fields = Build-MessageFields $body "implement-$agent-r$($state.round)"
        $msg = [PSCustomObject]@{ from = $agent; phase = 'IMPLEMENT'; round = $state.round }
        foreach ($k in $fields.Keys) { $msg | Add-Member -NotePropertyName $k -NotePropertyValue $fields[$k] -Force }
        Append-Message $msg
        Write-Host 'implementation recorded.'
    }
}

function Cmd-Verify {
    param([string[]]$remaining)
    With-Lock {
        $state = Require-State
        $agent = Get-Agent $state
        if ($state.phase -ne 'IMPLEMENT') { Write-Host "cannot verify in phase $($state.phase)"; exit 1 }
        $verdict = $remaining[0]
        if ($verdict -notin @('accept','reject')) { Write-Host "verdict must be: accept or reject"; exit 1 }
        $bodyArgs = $remaining[1..($remaining.Count - 1)]
        $body = Get-Body $bodyArgs
        $fields = Build-MessageFields $body "verify-$agent-r$($state.round)"
        $msg = [PSCustomObject]@{ from = $agent; phase = 'VERIFY'; round = $state.round; verdict = $verdict }
        foreach ($k in $fields.Keys) { $msg | Add-Member -NotePropertyName $k -NotePropertyValue $fields[$k] -Force }
        Append-Message $msg
        $state.last_action_by = $agent
        if ($verdict -eq 'accept') {
            $state.phase = 'DONE'
            Append-Message ([PSCustomObject]@{ from = 'system'; phase = 'DONE'; body = 'implementation accepted. debate complete.' })
        } else {
            $state.round += 1
            $state.phase = 'PROPOSE'
            Append-Message ([PSCustomObject]@{ from = 'system'; phase = 'PROPOSE'; body = "implementation rejected. back to debate. round $($state.round)." })
        }
        Save-State $state
    }
}

function Cmd-Done {
    $state = Require-State
    if (-not (Test-Path $ArchiveDir)) { New-Item -ItemType Directory -Path $ArchiveDir -Force | Out-Null }
    $dest = Join-Path $ArchiveDir $state.id
    Copy-Item $ActiveDir $dest -Recurse -Force
    Remove-Item $ActiveDir -Recurse -Force
    Write-Host "debate archived: $dest"
}

function Cmd-Watch {
    if (-not (Test-Path $MessagesFile)) { Write-Host 'no active debate'; exit 1 }
    $seen = 0
    Write-Host "watching debate (ctrl-c to stop)`n"
    if (Test-Path $GoalFile) {
        $goal = (Get-Content $GoalFile -Raw -Encoding UTF8).Trim()
        if ($goal) { Write-Host "GOAL: $goal`n" }
    }
    while ($true) {
        $msgs = Read-Messages
        if ($msgs.Count -gt $seen) {
            foreach ($m in $msgs[$seen..($msgs.Count - 1)]) { Format-Message $m }
            $seen = $msgs.Count
        }
        Start-Sleep -Seconds 1
    }
}

function Show-Help {
    Write-Host @"
debate - structured agent debate

commands:
  new <goal>                      start a new debate
  join [name] [--type <type>]     join the debate
  status                          show debate status
  read [--last N]                 read messages
  propose <msg> | --file <path>   submit a proposal
  discuss <agree|disagree|revise> <msg> | --file <path>
  say <msg> | --file <path>       human sends a message
  force <phase> [agent]           human forces a phase change
  implement <msg> | --file <path> record implementation
  verify <accept|reject> <msg> | --file <path>
  done                            archive the debate
  watch                           watch the debate live
"@
}

# --- dispatch ---

switch ($Command) {
    'new'       { Cmd-New $Args }
    'join'      { Cmd-Join $Args }
    'status'    { Cmd-Status }
    'read'      { Cmd-Read $Args }
    'propose'   { Cmd-Propose $Args }
    'discuss'   { Cmd-Discuss $Args }
    'say'       { Cmd-Say $Args }
    'force'     { Cmd-Force $Args }
    'implement' { Cmd-Implement $Args }
    'verify'    { Cmd-Verify $Args }
    'done'      { Cmd-Done }
    'watch'     { Cmd-Watch }
    default     { Show-Help }
}
