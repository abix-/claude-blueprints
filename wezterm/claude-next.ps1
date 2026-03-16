# claude-next.ps1 -- find next available endless-claude-N worktree and launch claude there
# usage: claude-next            (finds next free slot)
#        claude-next 3          (uses slot 3 specifically)

param([int]$Slot = 0)

$base = "C:\code"
$maxSlots = 10

# get cwds already in use by wezterm panes
$used = @()
try {
    $panes = wezterm cli list --format json | ConvertFrom-Json
    $used = $panes | ForEach-Object {
        $_.cwd -replace '^file:///', '' -replace '/$', '' -replace '/', '\'
    }
} catch {
    Write-Host "warn: could not query wezterm panes" -ForegroundColor Yellow
}

if ($Slot -gt 0) {
    # user picked a specific slot
    $dir = Join-Path $base "endless-claude-$Slot"
    $match = $used | Where-Object { $_ -like "$dir*" }
    if ($match) {
        Write-Host "slot $Slot is in use: $match" -ForegroundColor Red
        exit 1
    }
} else {
    # find first free slot
    for ($i = 1; $i -le $maxSlots; $i++) {
        $dir = Join-Path $base "endless-claude-$i"
        $match = $used | Where-Object { $_ -like "$dir*" }
        if (-not $match) {
            $Slot = $i
            break
        }
    }
    if ($Slot -eq 0) {
        Write-Host "no free slots (1-$maxSlots all in use)" -ForegroundColor Red
        exit 1
    }
}

$dir = Join-Path $base "endless-claude-$Slot"

# create worktree if it doesn't exist
if (-not (Test-Path $dir)) {
    Write-Host "creating worktree: endless-claude-$Slot" -ForegroundColor Cyan
    git -C "$base\endless" worktree add $dir
}

Write-Host "launching claude in: endless-claude-$Slot" -ForegroundColor Green
Set-Location $dir
claude
