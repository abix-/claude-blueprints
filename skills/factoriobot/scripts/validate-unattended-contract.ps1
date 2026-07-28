[CmdletBinding()]
param(
    [string] $SkillPath = (Join-Path $PSScriptRoot '..\SKILL.md'),
    [string] $InterfacePath = (Join-Path $PSScriptRoot '..\agents\openai.yaml')
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$skill = Get-Content -Raw -LiteralPath $SkillPath
$interface = Get-Content -Raw -LiteralPath $InterfacePath
$normalizedSkill = [regex]::Replace($skill, '\s+', ' ')

$requirements = [ordered]@{
    'bare invocation trigger' = 'Bare invocation starts unattended work'
    'endgame automation goal' = 'reach the Factorio endgame with 100% automation'
    'goal progress measure' = 'judge the work by whether live game state reached it'
    'operator owns active goal' = 'The operator owns the active goal'
    'exact operator goal wording' = 'Preserve the operator''s exact human-language Factorio outcome as the active goal'
    'no agent substitute goal' = 'Never replace it with an authority, software, test, documentation, batch, blocker, or implementation goal'
    'factorio goal sentence' = 'I am <Factorio action> so <Factorio outcome>'
    'unaligned without factorio sentence' = 'If that sentence cannot be written clearly, the proposed work is not aligned'
    'goal milestone test' = 'Configure the acceptance attempt to stop on that exact gameplay milestone'
    'gameplay progress execution loop' = 'Gameplay-progress execution loop'
    'combined verification gate' = 'one combined verification gate'
    'gameplay is next objective' = 'the next objective is gameplay movement'
    'support actions are not steps' = 'Do not present support actions as separate progress steps'
    'one acceptance attempt' = 'Run one acceptance attempt'
    'no gameplay movement response' = 'If gameplay does not move'
    'locked rules require hooks' = 'Hook enforcement is required for every locked unattended rule'
    'skill hook tests change together' = 'Update the skill, hook, and hook tests in the same change'
    'no instruction only enforcement' = 'Do not ship instruction-only enforcement'
    'recovery triggers' = 'Recover current state after a crash, interruption, or compaction'
    'compaction recovery' = 'After any context compaction'
    'compaction evidence boundary' = 'Carried instruction text is recovery evidence only'
    'compaction current agents' = 'current filesystem `AGENTS.md`'
    'compaction current skills' = 'active matching skills'
    'current phase authority' = 'The current phase is authoritative'
    'resolve current paths' = 'Resolve every path from current files with `rg --files`'
    'resolve current repository details' = 'Resolve every command, API, function, test location, and hook permission'
    'never guess repository details' = 'Never guess repository details'
    'diagnose phase' = 'Diagnose: Find the root cause blocking progress toward the permanent Factorio endgame goal'
    'prove phase' = 'Prove: Resolve an allowed permanent proof location first'
    'implement phase' = 'Implement: Read the current canonical implementation boundary'
    'build phase' = 'Build: Run the focused and required broader checks'
    'push phase' = 'Push: Push the exact tested commit'
    'play phase' = 'Play: Use `restart.ps1 -Gate <milestone>` once'
    'review phase' = 'Review: Use the canonical repository review workflow'
    'resume interrupted phase' = 'resume the interrupted phase automatically'
    'complete design load' = 'Load every authoritative design document before work'
    'complete design files' = 'read every file completely before selecting or changing work'
    'missing context reload' = 'not fully present in the current context'
    'complete disk reload' = 'reread it completely from disk'
    'truncated output recovery' = 'continue reading bounded ranges'
    'goal alignment gate' = 'Goal-alignment gate'
    'goal value stop' = 'Goal-value stop'
    'minimum valid support' = 'minimum valid support artifact'
    'support action budget' = 'one edit, one verification, and one correction'
    'return gameplay' = 'return to gameplay work'
    'operator gameplay goal' = 'Operator-owned gameplay goal'
    'current gameplay acceptance' = 'Current gameplay acceptance'
    'selected authority batch' = 'Selected authority batch'
    'governing design statements' = 'Governing design statements'
    'measured gaps' = 'Measured gaps'
    'acceptance measure action' = 'advances a recorded gameplay acceptance measure'
    'authority bypass action' = 'removes a documented authority bypass required for that'
    'reject unaligned action' = 'If neither statement is true, do not perform the action'
    'support is not progress' = 'supporting evidence, never progress by themselves'
    'complete run first' = 'Review the complete run before choosing a change'
    'review gate' = 'Review gate'
    'review prior work' = 'Review all work since the last gameplay movement'
    'review complete todo' = 'Review every remaining item in `docs/todo.md`'
    'review authoritative design' = 'Review every authoritative design document'
    'review current diffs' = 'Review recent Git history, status, and every relevant uncommitted diff'
    'review attempt evidence' = 'Review every attempt since the last gameplay movement'
    'compare to goal' = 'Compare the reviewed evidence with the permanent gameplay goal'
    'select only after review' = 'Only after that complete review, select one next authority batch'
    'record next batch' = 'Record the selected batch in `docs/todo.md` and the `docs/authority.md` current consolidation queue'
    'hook blocks before review' = 'The tracked project hook blocks production edits, builds, restarts, and acceptance attempts until the review gate passes'
    'proof commit opens implementation' = 'Production implementation remains blocked until the selected batch has committed failing proofs'
    'whole aligned batch' = 'complete the whole design-aligned authority batch'
    'acceptance rerun gate' = 'before another acceptance run'
    'compaction automatic resume' = 'Resume the in-flight batch automatically'
    'current file authority' = 'instruction copied from an old transcript'
    'routine continuation' = 'Continue without routine confirmation'
    'state action table' = '| Current state | Required action |'
    'single diagnostic attempt' = 'Use one diagnostic attempt for a newly observed deterministic failure'
    'reliability attempts' = 'Use repeated attempts only to prove reliability after the first pass'
    'one Windows entry point' = 'Use `restart.ps1` as the one Windows entry point'
    'status report' = 'Unattended status report'
    'endgame automation report' = 'Endgame automation progress:'
    'endgame route denominator' = 'complete prototype-derived route to the Space Age endgame'
    'automated capability evidence' = 'completed and automated count'
    'player help exclusion' = 'Player help does not count as automated'
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
    'gameplay movement gate' = 'Gameplay movement gate'
    'one replacement attempt' = 'one implementation plus its replacement acceptance attempt'
    'zero gameplay delta' = 'zero gameplay delta'
    'technical work cannot reset gameplay gate' = 'Technical progress does not reset this gate'
    'forbid another hotfix loop' = 'Do not make another patch, build, restart, or acceptance attempt'
    'audit attempts since movement' = 'Audit every attempt since the last gameplay milestone moved'
    'group failures by authority' = 'Group every failure under its existing authority'
    'one milestone-closing batch' = 'one coherent authority batch that closes the next gameplay milestone'
    'checkpoint development' = 'Use the nearest verified milestone save for development'
    'fresh start final acceptance' = 'Use a fresh-start run only for final acceptance'
    'time without gameplay movement' = 'Time since last gameplay movement:'
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
    $normalizedRequirement = [regex]::Replace($entry.Value, '\s+', ' ')
    if (-not $normalizedSkill.Contains($normalizedRequirement)) {
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
