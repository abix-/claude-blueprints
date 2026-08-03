<#
.SYNOPSIS
    Install and compare this repository against the live Claude and Codex
    directories.

.DESCRIPTION
    The one sync tool. See sync.md for the full contract.

    Covers both runtimes and every file, not just skills:

        skills/<name>/          -> ~/.claude/skills/ and ~/.agents/skills/
        claude/skills/<name>/   -> ~/.claude/skills/
        codex/skills/<name>/    -> ~/.agents/skills/
        anything else in claude/ -> ~/.claude/ at the same relative path
        anything else in codex/  -> ~/.codex/ at the same relative path

    Content decides whether a file differs. Timestamps are read only once the
    contents already differ, to say which side was edited more recently.

    Nothing on disk is ever deleted, including files that exist only there.

.PARAMETER Action
    check    Compare and print. Changes nothing. Exits 1 if anything differs.
    install  Copy the repository over the live files, skipping identical ones.
    resolve  Walk each differing file, show the diff, ask what to do with it.

.PARAMETER Runtime
    claude, codex, or all. Defaults to all.

.PARAMETER Include
    Which kinds of file to act on. Any of:

        skills        the skill folders, shared and runtime-specific
        hooks         everything under hooks/
        settings      settings.json and any other settings file
        instructions  CLAUDE.md, failures.md, AGENTS.md and the like

    Defaults to all of them.

.PARAMETER HomePath
    Install somewhere other than the real profile, for a dry run.

.EXAMPLE
    .\sync.ps1
    Compare everything and print the table.

.EXAMPLE
    .\sync.ps1 -Action install -Runtime claude
    Write the repository's Claude files over the live ones.

.EXAMPLE
    .\sync.ps1 -Action resolve
    Decide each differing file one at a time, in either direction.
#>
[CmdletBinding()]
param(
    [ValidateSet('check', 'install', 'resolve')]
    [string]$Action = 'check',

    [ValidateSet('claude', 'codex', 'all')]
    [string]$Runtime = 'all',

    [ValidateSet('all', 'skills', 'hooks', 'settings', 'instructions')]
    [string[]]$Include = @('all'),

    [string]$HomePath = $env:USERPROFILE
)

$ErrorActionPreference = 'Stop'
$Repo = $PSScriptRoot

# Exit codes, per sync.md.
$EXIT_OK = 0
$EXIT_DIFFERS = 1
$EXIT_REFUSED = 2

function Get-SkillNames {
    <#
        Skill directories are the ones holding a SKILL.md. Anything else in a
        skills folder is not a skill and is carried as an ordinary file.
    #>
    param([string]$Root)

    if (-not (Test-Path -LiteralPath $Root -PathType Container)) { return @() }
    Get-ChildItem -LiteralPath $Root -Directory |
        Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName 'SKILL.md') -PathType Leaf } |
        Select-Object -ExpandProperty Name
}

function Add-Tree {
    <#
        Map every file under a source root onto a destination root, keeping the
        relative path. Refuses on a duplicate destination: two sources landing
        on one file means one of them silently wins.
    #>
    param(
        [hashtable]$Manifest,
        [string]$SourceRoot,
        [string]$DestinationRoot
    )

    if (-not (Test-Path -LiteralPath $SourceRoot -PathType Container)) { return }

    foreach ($file in Get-ChildItem -LiteralPath $SourceRoot -File -Recurse) {
        $relative = $file.FullName.Substring($SourceRoot.Length).TrimStart('\', '/')
        $destination = Join-Path $DestinationRoot $relative
        if ($Manifest.ContainsKey($destination)) {
            [Console]::Error.WriteLine("REFUSED: duplicate destination $destination, from $($Manifest[$destination]) and $($file.FullName)")
            exit $EXIT_REFUSED
        }
        $Manifest[$destination] = $file.FullName
    }
}

function Build-Manifest {
    <#
        Everything this runtime owns, as destination -> source. Refuses when a
        runtime skill shares a name with a shared skill, because one would
        overwrite the other on disk with no sign of it.
    #>
    param([string]$RuntimeName)

    $sharedSkills = Join-Path $Repo 'skills'
    $runtimeRoot = Join-Path $Repo $RuntimeName
    $runtimeSkills = Join-Path $runtimeRoot 'skills'

    $shared = @(Get-SkillNames -Root $sharedSkills)
    $own = @(Get-SkillNames -Root $runtimeSkills)
    $clash = @($shared | Where-Object { $own -contains $_ })
    if ($clash.Count -gt 0) {
        [Console]::Error.WriteLine("REFUSED: runtime skill conflicts with shared skill: $($clash -join ', ')")
        exit $EXIT_REFUSED
    }

    if ($RuntimeName -eq 'claude') {
        $skillDestination = Join-Path $HomePath '.claude\skills'
        $configDestination = Join-Path $HomePath '.claude'
    }
    else {
        $skillDestination = Join-Path $HomePath '.agents\skills'
        $configDestination = Join-Path $HomePath '.codex'
    }

    $manifest = @{}
    Add-Tree -Manifest $manifest -SourceRoot $sharedSkills -DestinationRoot $skillDestination
    Add-Tree -Manifest $manifest -SourceRoot $runtimeSkills -DestinationRoot $skillDestination

    # Everything under the runtime folder that is not a skill is config, and
    # keeps its relative path: claude/hooks/x.py -> ~/.claude/hooks/x.py
    if (Test-Path -LiteralPath $runtimeRoot -PathType Container) {
        foreach ($file in Get-ChildItem -LiteralPath $runtimeRoot -File -Recurse) {
            $relative = $file.FullName.Substring($runtimeRoot.Length).TrimStart('\', '/')
            if ($relative -like 'skills\*' -or $relative -like 'skills/*') { continue }
            $destination = Join-Path $configDestination $relative
            if ($manifest.ContainsKey($destination)) {
                [Console]::Error.WriteLine("REFUSED: duplicate destination $destination, from $($manifest[$destination]) and $($file.FullName)")
                exit $EXIT_REFUSED
            }
            $manifest[$destination] = $file.FullName
        }
    }

    return $manifest
}

function Get-Kind {
    <#
        Which kind of file this is, decided by where it lands. Used only by
        -Include, so a run can be narrowed to skills, hooks, settings or
        instructions.
    #>
    param([string]$Relative)

    if ($Relative -match '(^|[\\/])skills[\\/]') { return 'skills' }
    if ($Relative -match '(^|[\\/])hooks[\\/]') { return 'hooks' }
    if ($Relative -match 'settings.*\.json$') { return 'settings' }
    return 'instructions'
}

function Test-Included {
    param([string]$Kind)

    if ($Include -contains 'all') { return $true }
    return $Include -contains $Kind
}

function Test-SameContent {
    param([string]$Left, [string]$Right)

    $a = [System.IO.File]::ReadAllBytes($Left)
    $b = [System.IO.File]::ReadAllBytes($Right)
    if ($a.Length -ne $b.Length) { return $false }
    for ($i = 0; $i -lt $a.Length; $i++) {
        if ($a[$i] -ne $b[$i]) { return $false }
    }
    return $true
}

function Get-Status {
    <#
        Content decides identical or not. Only once they differ do timestamps
        speak, and only to say which side moved last.
    #>
    param([string]$Source, [string]$Destination)

    if (-not (Test-Path -LiteralPath $Destination -PathType Leaf)) { return 'MISSING' }
    if (Test-SameContent -Left $Source -Right $Destination) { return 'OK' }

    $repoTime = (Get-Item -LiteralPath $Source).LastWriteTimeUtc
    $localTime = (Get-Item -LiteralPath $Destination).LastWriteTimeUtc
    if ($repoTime -gt $localTime) { return 'REPO-NEWER' }
    if ($localTime -gt $repoTime) { return 'LOCAL-NEWER' }
    return 'DIFF'
}

function Get-Rows {
    param([string]$RuntimeName)

    $manifest = Build-Manifest -RuntimeName $RuntimeName
    $rows = foreach ($destination in ($manifest.Keys | Sort-Object)) {
        $source = $manifest[$destination]
        $relative = $destination.Substring($HomePath.Length).TrimStart('\', '/')
        $kind = Get-Kind -Relative $relative
        if (-not (Test-Included -Kind $kind)) { continue }
        [pscustomobject]@{
            Runtime     = $RuntimeName
            Kind        = $kind
            Status      = Get-Status -Source $source -Destination $destination
            Source      = $source
            Destination = $destination
            Relative    = $relative
        }
    }

    # A skill on disk that the repository does not know about is reported and
    # never touched. It is somebody's work until they say otherwise.
    #
    # Only skills. The config directories are the runtime's own: caches, daemon
    # state and credentials live there, none of it ours to report on, and the
    # names alone are noise.
    $skillRoot = if ($RuntimeName -eq 'claude') {
        Join-Path $HomePath '.claude\skills'
    }
    else {
        Join-Path $HomePath '.agents\skills'
    }
    $roots = $manifest.Keys |
        Where-Object { $_.StartsWith($skillRoot, [System.StringComparison]::OrdinalIgnoreCase) } |
        ForEach-Object { Split-Path -Path $_ -Parent } |
        Sort-Object -Unique
    $known = @{}
    foreach ($k in $manifest.Keys) { $known[$k] = $true }
    foreach ($root in $roots) {
        if (-not (Test-Path -LiteralPath $root -PathType Container)) { continue }
        foreach ($file in Get-ChildItem -LiteralPath $root -File) {
            if ($known.ContainsKey($file.FullName)) { continue }
            $rows += [pscustomobject]@{
                Runtime     = $RuntimeName
                Status      = 'LOCAL-ONLY'
                Source      = $null
                Destination = $file.FullName
                Relative    = $file.FullName.Substring($HomePath.Length).TrimStart('\', '/')
            }
        }
    }

    return $rows
}

function Get-StatusColour {
    <#
        The colour each status prints in. Its own function so the mapping can
        be checked without a terminal: colour does not survive a pipe.
    #>
    param([string]$Status)

    switch ($Status) {
        'OK' { 'DarkGray' }
        'MISSING' { 'Yellow' }
        'LOCAL-ONLY' { 'Yellow' }
        'REPO-NEWER' { 'Red' }
        'LOCAL-NEWER' { 'Green' }
        default { 'Yellow' }
    }
}

function Write-Rows {
    param([object[]]$Rows, [string]$RuntimeName)

    Write-Host ""
    Write-Host $RuntimeName
    foreach ($row in $Rows) {
        Write-Host ("{0,-12} {1}" -f $row.Status, $row.Relative) `
            -ForegroundColor (Get-StatusColour -Status $row.Status)
    }
}

function Copy-Into {
    param([string]$Source, [string]$Destination)

    $parent = Split-Path -Path $Destination -Parent
    if (-not (Test-Path -LiteralPath $parent -PathType Container)) {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
    }
    Copy-Item -LiteralPath $Source -Destination $Destination -Force
}

function Invoke-Install {
    param([object[]]$Rows)

    $copied = 0
    foreach ($row in $Rows) {
        if ($row.Status -eq 'OK' -or $row.Status -eq 'LOCAL-ONLY') { continue }
        Copy-Into -Source $row.Source -Destination $row.Destination
        $copied++
    }
    return $copied
}

function Show-Diff {
    param([object[]]$Rows, [object]$Row)

    $left = if (Test-Path -LiteralPath $Row.Destination -PathType Leaf) { $Row.Destination } else { $null }
    if ($null -eq $left) {
        Write-Host "  (no local copy yet)" -ForegroundColor Yellow
        return
    }
    Compare-Object -ReferenceObject (Get-Content -LiteralPath $left) `
                   -DifferenceObject (Get-Content -LiteralPath $Row.Source) |
        ForEach-Object {
            $mark = if ($_.SideIndicator -eq '<=') { 'local ' } else { 'repo  ' }
            $colour = if ($_.SideIndicator -eq '<=') { 'Green' } else { 'Red' }
            Write-Host ("  {0} {1}" -f $mark, $_.InputObject) -ForegroundColor $colour
        }
}

function Invoke-Resolve {
    <#
        One file at a time, both directions, and nothing moves without an
        answer for that exact file.
    #>
    param([object[]]$Rows)

    foreach ($row in $Rows) {
        if ($row.Status -eq 'OK' -or $row.Status -eq 'LOCAL-ONLY') { continue }

        Write-Host ""
        Write-Host ("{0}  {1}" -f $row.Status, $row.Relative) -ForegroundColor Cyan
        Show-Diff -Rows $Rows -Row $row

        while ($true) {
            $answer = Read-Host "  [l]ocal to repo  [r]epo to local  [d]iff  [v]iew  [s]kip  [q]uit"
            switch ($answer.ToLower()) {
                'l' {
                    if (-not (Test-Path -LiteralPath $row.Destination -PathType Leaf)) {
                        Write-Host "  nothing local to promote" -ForegroundColor Yellow
                        continue
                    }
                    Copy-Into -Source $row.Destination -Destination $row.Source
                    Write-Host "  promoted into the repository" -ForegroundColor Green
                    break
                }
                'r' {
                    Copy-Into -Source $row.Source -Destination $row.Destination
                    Write-Host "  accepted the repository version" -ForegroundColor Green
                    break
                }
                'd' { Show-Diff -Rows $Rows -Row $row; continue }
                'v' {
                    if (Test-Path -LiteralPath $row.Destination -PathType Leaf) {
                        Get-Content -LiteralPath $row.Destination | Out-Host -Paging
                    }
                    Get-Content -LiteralPath $row.Source | Out-Host -Paging
                    continue
                }
                's' { break }
                'q' { return }
                default { Write-Host "  l, r, d, v, s or q" -ForegroundColor Yellow; continue }
            }
            break
        }
    }
}

$runtimes = if ($Runtime -eq 'all') { @('claude', 'codex') } else { @($Runtime) }
$differs = $false

foreach ($name in $runtimes) {
    $rows = @(Get-Rows -RuntimeName $name)

    switch ($Action) {
        'install' {
            $copied = Invoke-Install -Rows $rows
            Write-Host ("{0}: copied {1} file(s)" -f $name, $copied)
            $rows = @(Get-Rows -RuntimeName $name)
        }
        'resolve' {
            Invoke-Resolve -Rows $rows
            $rows = @(Get-Rows -RuntimeName $name)
        }
    }

    Write-Rows -Rows $rows -RuntimeName $name
    if ($rows | Where-Object { $_.Status -ne 'OK' -and $_.Status -ne 'LOCAL-ONLY' }) {
        $differs = $true
    }
}

if ($differs) { exit $EXIT_DIFFERS }
exit $EXIT_OK
