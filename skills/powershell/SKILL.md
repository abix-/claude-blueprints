---
name: powershell
description: PowerShell standards covering function shape, VMware PowerCLI, modules, and Pester testing. Sourced from abix-/powershell-practical (Aluminium module, 10k LOC vSphere automation). Use when writing PowerShell.
user-invocable: false
version: "2.0"
updated: "2026-05-11"
---
# PowerShell

Source repo: `abix-/powershell-practical/Aluminium` (vSphere /
Windows automation module, ~10k LOC). Style is **pragmatic
PowerCLI**: function-per-file in flat layout, comment-based help on
every cmdlet, `PSCustomObject` outputs that pipe cleanly.

## Edition and platform

- **PowerShell 7+ (`pwsh`)** for new work. Cross-platform, pipeline
  chain operators (`&&` / `||`), ternary, null-coalescing.
- **Windows PowerShell 5.1** required for older VMware PowerCLI / on-prem
  AD modules. Aluminium runs on 5.1.
- Test on both if the module ships to others. Avoid `$IsWindows` /
  `$PSVersionTable.PSVersion.Major -ge 7` branching unless required.

## Function shape

Standard cmdlet:

```powershell
function Get-DatastoreDetails {
    <#
    .SYNOPSIS
    Returns capacity, provisioned, and overcommit metrics for datastores.
    .DESCRIPTION
    Filters by name pattern and optional parent. Uses Get-View for
    server-side filtering rather than client-side Where-Object.
    .PARAMETER Name
    Datastore name pattern (wildcards allowed).
    .PARAMETER Parent
    Optional parent container (cluster, folder, datacenter).
    .EXAMPLE
    Get-DatastoreDetails -Name "prod-*" -Parent "cluster01"
    .NOTES
    Returns PSCustomObject with formatted GB values.
    #>
    [CmdletBinding()]
    param (
        [string]$Name = "",
        [string]$Parent
    )

    # ...
}
```

- **Verb-Noun naming**, approved verbs only (`Get-Verb` lists them).
  `Get-`, `Set-`, `New-`, `Remove-`, `Test-`, `Invoke-`, `Find-`.
- `[CmdletBinding()]` always. Gives `-Verbose`, `-Debug`,
  `-ErrorAction`, `-WarningAction` for free.
- `param ( ... )` block follows `[CmdletBinding()]`. Type-annotate
  parameters with `[string]`, `[int]`, `[switch]`, `[hashtable]`.
- `[Parameter(Mandatory=$true)]` for required args.
- Default values in the param signature: `[string]$Name = ""`.
- Comment-based help: `.SYNOPSIS`, `.DESCRIPTION`, `.PARAMETER X`
  (one per param), `.EXAMPLE`, `.NOTES`. Help is part of the API.

## Output: PSCustomObject

Build typed objects for pipeline-friendly output:

```powershell
$results = @()
foreach ($_d in ($datastores | Select-Object -ExpandProperty Summary)) {
    $capgb = (($_d.Capacity) / 1GB)
    $results += [pscustomobject][ordered]@{
        Name              = $_d.Name
        "CapacityGB"      = "{0:N2}" -f $capgb
        "MaxUsableGB"     = "{0:N2}" -f ($capgb * 0.80)
        "FreeGB"          = "{0:N2}" -f ($_d.FreeSpace / 1GB)
        "ProvisionedGB"   = "{0:N2}" -f $provgb
    }
}
return $results | Sort-Object Name
```

- `[pscustomobject][ordered]@{}` keeps key order. Without `[ordered]`,
  the hashtable randomizes display order.
- Quoted keys (`"FreeGB"`) when the column name has special chars or
  needs to display as-is.
- `"{0:N2}" -f $val` for formatted strings. Standard .NET format codes.
- Numeric suffixes: `1GB`, `1MB`, `1KB`. Built-in constants.
- Return as objects, not strings. Caller pipes into
  `Format-Table`, `Export-Csv`, `ConvertTo-Json` as they need.

## Pipeline composition

PowerShell's pipeline is the calling convention. Write functions that
both **accept** and **emit** pipeline objects.

```powershell
$datastores | Where-Object { $_.FreeGB -lt 100 } | Sort-Object FreeGB | Format-Table
```

- `ValueFromPipeline=$true` / `ValueFromPipelineByPropertyName=$true`
  on parameters to accept piped input.
- `process { ... }` block for per-item handling.
- `Select-Object -ExpandProperty X` to flatten one level. Common
  Aluminium pattern: `$datastores | Select-Object -ExpandProperty Summary`.
- Avoid `+=` on large arrays. Use `[System.Collections.ArrayList]` or
  assign the pipeline directly: `$results = foreach ($x in $items) { ... }`.

## Error handling

```powershell
try {
    $vm = Get-VMHost $_v -ErrorAction Stop
}
catch {
    Write-Host "$($_v): VMHost not found" -ForegroundColor Red
    continue
}
```

- `-ErrorAction Stop` forces a non-terminating cmdlet error into a
  terminating one that `try/catch` can see. Without it, the catch
  block won't fire.
- `$_` inside catch is the `ErrorRecord`. `$_.Exception.Message` for
  the message, `$_.ScriptStackTrace` for the trace.
- `throw "message"` to raise; `Write-Error` for non-terminating.
- Catch and `continue` inside a loop to keep going past one bad host;
  catch and `return` at the function level to abort.

## Logging output

- `Write-Verbose "[$Name] starting"`: diagnostic. Only shown with
  `-Verbose`. Default for tracing / progress.
- `Write-Host "$($_v): done" -ForegroundColor Green`: user-facing,
  ignores `-Verbose`. Use for status, prompts, and colored markers.
- `Write-Output` (or bare emission): goes down the pipeline. NEVER
  use it for status; it pollutes the function's return value.
- `Write-Warning` for "completed but degraded".
- `Write-Debug` requires `-Debug` and pauses execution; rarely useful.

Bracketed prefix in messages (`[$($_v)]`, `[$Name]`) is the
Aluminium convention. Greppable across long logs.

## Variable expansion in strings

```powershell
"$Name: $($obj.Count) items"
"Result: $($vm.Name) is $($vm.PowerState)"
```

- Single-`$var` expansion works for simple variables.
- `$()` subexpression for property access, method calls, indices, or
  expressions.
- Single quotes: literal, no expansion.
- Backtick (`` ` ``) is PowerShell's escape character, not the shell's
  command substitution.

## Loops and iteration

```powershell
foreach ($_v in $vmhosts) { ... }
$vmhosts | ForEach-Object { ... }
```

- `foreach (... in ...)` for straight iteration. Faster, more
  readable for a single loop.
- `| ForEach-Object { ... }` when chaining further down the pipeline.
  Has `$_` as the current item.
- Loop variable naming: Aluminium uses `$_v` (host), `$_d` (datastore),
  `$_T` (table), short lowercase prefixed with `_`. Pick consistent
  short names; the body is short enough that this works.

## Modules

```
Aluminium/
  Aluminium.psd1       # manifest
  Aluminium.psm1       # entry module: dot-sources function files
  vmware.ps1           # one functional area per file
  vmware-reporting.ps1
  vmware-tests.ps1
  windows-server.ps1
```

- `.psd1` (manifest): `ModuleVersion`, `RequiredModules`, `GUID`,
  exported function list, dependencies.
- `.psm1` (root): typically dot-sources the `.ps1` files in the
  directory or holds module-level functions.
- Functions in flat `.ps1` files, named by area (`vmware-reporting.ps1`,
  `vmware-monitoring.ps1`). Search-friendly.
- `Export-ModuleMember -Function *-*` to expose all Verb-Noun
  functions automatically. Set `FunctionsToExport` in the manifest
  for stricter control.

## VMware PowerCLI patterns

```powershell
# server-side filtering with Get-View is FAST
$datastores = @(Get-View -ViewType Datastore -Filter @{"Name"="$Name"})

# UID parsing to find the originating vCenter
$ParentvCenter = $obj.UID |
    Where-Object { $_ -match "@(?<vcenter>.*):443" } |
    ForEach-Object { $matches['vcenter'] }

# bulk operations
Get-VMHost $hosts | ForEach-Object {
    Set-AdvancedSetting -AdvancedSetting (Get-AdvancedSetting -Entity $_ -Name $Name) -Value $value
}
```

- `Get-View` over `Get-VM` / `Get-VMHost` for large fleets. Returns
  view objects you can navigate by `MoRef`; orders of magnitude
  faster than the full PowerCLI cmdlet on 1000+ VMs.
- Use the `-Server` parameter explicitly when connected to multiple
  vCenters.
- Wrap connections: `Connect-VIServer $vc -ErrorAction Stop` then
  finally `Disconnect-VIServer -Confirm:$false`.
- Output to CSV: `Export-Csv -NoTypeInformation -Path $path`. Without
  `-NoTypeInformation`, the first line is a `#TYPE` header that breaks
  most consumers.

## Pester testing

Aluminium does not currently ship Pester tests, but they are the
standard. Layout:

```powershell
# Get-DatastoreDetails.Tests.ps1
Describe "Get-DatastoreDetails" {
    BeforeAll {
        . $PSScriptRoot/Get-DatastoreDetails.ps1
        Mock Get-View { ... }
    }

    It "returns one row per datastore" {
        $result = Get-DatastoreDetails -Name "prod-*"
        $result.Count | Should -Be 3
    }

    It "rounds to 2 decimals" {
        $result = Get-DatastoreDetails -Name "prod-*"
        $result[0].CapacityGB | Should -Match '^\d+\.\d{2}$'
    }
}
```

- Pester 5 syntax (`Should -Be`, not `Should Be`).
- `BeforeAll` / `BeforeEach` / `AfterAll` for setup.
- `Mock` replaces cmdlets; pin with `-ParameterFilter`.
- One `*.Tests.ps1` file per function. Co-locate or put under
  `tests/`.
- Run: `Invoke-Pester ./tests` or `Invoke-Pester -Output Detailed`.

## Performance

- `Get-View` over object-mode PowerCLI cmdlets for large queries.
- Avoid `+=` on arrays in loops; rebuilds the array each time
  (`O(n^2)`). Use `ArrayList` or `foreach` returning into a variable.
- `Where-Object` server-side via `-Filter @{}` (Get-View) beats
  client-side `| Where-Object {}`.
- Pipeline can be slower than a `foreach` loop for very large
  collections due to per-item dispatch overhead.
- `Measure-Command { ... }` to benchmark. Real numbers beat guesses.

## Style

- PascalCase for functions, variables, parameters.
- `$Verbose` switches named like `[switch]$Verbose`.
- 4-space indent. Opening brace on the same line as `function`,
  `if`, `foreach`, `try`.
- UTF-8 with BOM is the Windows PowerShell 5.1 default; PS 7 prefers
  UTF-8 without BOM. Pick one per file; never mix.
- Avoid aliases in scripts (`gci`, `?`, `%`). Spell out the cmdlet.
  Aliases are fine in the REPL.

## Avoid

- `Write-Host` for data. It bypasses the pipeline.
- `+=` on large arrays.
- `Invoke-Expression`. Almost always wrong; injection-prone.
- `$ErrorActionPreference = "SilentlyContinue"` at script scope.
  Use `-ErrorAction` per-call when you mean it.
- Wildcards in restore / delete paths without a `-WhatIf` run first.
- `Read-Host` in non-interactive runs. The harness errors out.
- Hardcoded paths like `C:\Scripts`. Use `$PSScriptRoot` or
  `Join-Path`.
