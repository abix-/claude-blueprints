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
    'endgame automation goal' = 'reach the Factorio endgame with 100% automation'
    'goal progress measure' = 'measurable progress toward that goal'
    'crash recovery' = 'Recover current state after a crash or interruption'
    'current file authority' = 'instruction copied from an old transcript'
    'routine continuation' = 'Continue without routine confirmation'
    'state action table' = '| Current state | Required action |'
    'single diagnostic attempt' = 'Use one diagnostic attempt for a newly observed deterministic failure'
    'reliability attempts' = 'Use repeated attempts only to prove reliability after the first pass'
    'one Windows entry point' = 'Use `restart.ps1` as the one Windows entry point'
    'status report' = 'Unattended status report'
    'verified progress' = 'Verified progress'
    'milestones reached' = 'Milestones reached'
    'gameplay milestones only' = 'Milestones reached contains only actual gameplay progress'
    'support work is not milestones' = 'Code, tests, builds, commits, skills, and docs are not milestones'
    'support work reporting' = 'Report those under Verified progress and Checks'
    'authority score' = 'Authority score: <number and name>, <current> -> <target>'
    'active attempt' = 'Active attempt'
    'latest blocker' = 'Latest blocker'
    'next action' = 'Next action'
    'verification labels' = 'Code-verified or Live-verified'
    'durable status' = 'docs/status.md'
    'progress stall guard' = 'Progress stall guard'
    'no measurable progress' = 'No measurable engineering or gameplay progress'
    'tested build' = 'Tested build'
    'attempt provenance' = 'Attempt provenance'
    'source changes after attempt' = 'Source changes made after an attempt starts'
    'end checkpoint' = 'End-of-window checkpoint'
    'priority contract' = 'Work priority'
    'direct operator priority' = 'Newest explicit operator direction'
    'active queue priority' = 'current consolidation queue'
    'authority row priority' = 'lowest numbered open authority row'
    'score is not priority' = 'Score measures maturity and never determines priority by itself'
    'new findings wait' = 'New findings enter the todo under their owning authority'
    'priority status' = 'Priority basis'
    'repo changelog authority' = 'one repo-wide `docs/changelog.md`'
    'no per-doc changelogs' = 'Never add changelog sections to individual design docs'
    'documentation contract' = 'Documentation contract'
    'self-learning loop' = 'Self-learning loop'
    'shared skill learning' = 'applicable shared skill'
    'owning project docs learning' = 'owning project docs'
    'same verified learning batch' = 'same verified batch'
    'two repository learning gate' = 'Validate, commit, and push both repositories'
    'prototype filter array contract' = 'prototype filter methods require an array of typed filter tables'
    'prototype filter array example' = 'get_entity_filtered{{filter="type",type="mining-drill"}}'
    'canonical design statements' = 'one canonical design statement in one owning design doc'
    'committed proof per statement' = 'Every design statement names its committed proof'
    'proof state per statement' = 'Every design statement records its current proof state'
    'proof state vocabulary' = '`RED`, `CODE-VERIFIED`, `LIVE-VERIFIED`, or `PROCESS`'
    'no duplicated design prose' = 'Never repeat a design statement in another doc'
    'todo only unfinished' = 'Unfinished implementation and failed proof belong in `docs/todo.md`'
    'rationale stays with owner' = 'Keep only the rationale needed to understand the owning design statement'
    'test references are exact' = 'Use the exact committed test name'
    'docs updated from proof' = 'Update proof state only from current test or gameplay evidence'
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
