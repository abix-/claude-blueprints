[CmdletBinding()]
param(
    [ValidateSet("claude", "codex", "all")]
    [string]$Runtime = "all",

    [string]$HomePath = $env:USERPROFILE
)

$scriptPath = Join-Path $PSScriptRoot "sync-check.py"
& python $scriptPath install --runtime $Runtime --home $HomePath
exit $LASTEXITCODE
