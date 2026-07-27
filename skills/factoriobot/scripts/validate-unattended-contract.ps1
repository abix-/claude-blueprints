[CmdletBinding()]
param(
    [string] $SkillPath = (Join-Path $PSScriptRoot '..\SKILL.md'),
    [string] $InterfacePath = (Join-Path $PSScriptRoot '..\agents\openai.yaml')
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$skill = Get-Content -Raw -LiteralPath $SkillPath
$interface = Get-Content -Raw -LiteralPath $InterfacePath

$requirements = [ordered]@{
    'bare invocation trigger' = 'Bare invocation starts unattended work'
    'crash recovery' = 'Recover current state after a crash or interruption'
    'current file authority' = 'instruction copied from an old transcript'
    'routine continuation' = 'Continue without routine confirmation'
    'state action table' = '| Current state | Required action |'
    'single diagnostic attempt' = 'Use one diagnostic attempt for a newly observed deterministic failure'
    'reliability attempts' = 'Use repeated attempts only to prove reliability after the first pass'
    'one Windows entry point' = 'Use `restart.ps1` as the one Windows entry point'
    'stop conditions' = 'Stop unattended work only when'
    'completion conditions' = 'Bare invocation is complete only when'
    'continue after batch' = 'select it and continue'
}

$missing = foreach ($entry in $requirements.GetEnumerator()) {
    if (-not $skill.Contains($entry.Value)) {
        $entry.Key
    }
}

if ($missing) {
    throw "Factoriobot unattended contract is missing: $($missing -join ', ')"
}

if (-not $interface.Contains('Use $factoriobot')) {
    throw 'Factoriobot default prompt must explicitly invoke $factoriobot'
}

if (-not $interface.Contains('continue the highest-priority documented authority work unattended')) {
    throw 'Factoriobot default prompt must start the unattended authority path'
}

$nonAscii = [regex]::Match($skill + $interface, '[^\x00-\x7F]')
if ($nonAscii.Success) {
    throw "Factoriobot skill package contains non-ASCII text: $($nonAscii.Value)"
}

Write-Output 'Factoriobot unattended contract is valid.'
