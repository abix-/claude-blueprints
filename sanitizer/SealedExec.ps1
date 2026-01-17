<#
.SYNOPSIS
    Executes a command in a sealed environment with real values.

.DESCRIPTION
    1. Creates temp directory
    2. Copies working tree to temp
    3. Renders real values in temp copy (fake -> real)
    4. Executes command in temp directory
    5. Captures output
    6. DELETES temp directory (always, even on error)
    7. Sanitizes output before returning

    The working tree is NEVER modified. Real values only exist in ephemeral temp dir.

.PARAMETER Command
    The command to execute.

.PARAMETER WorkingDir
    The working directory (fake version). Defaults to current directory.

.PARAMETER TimeoutSeconds
    Command timeout in seconds. Defaults to 300 (5 minutes).

.EXAMPLE
    .\SealedExec.ps1 -Command "ansible-playbook site.yml"
#>

param(
    [Parameter(Mandatory)]
    [string]$Command,

    [string]$WorkingDir = (Get-Location).Path,
    [string]$SanitizerDir = "$env:USERPROFILE\.claude\sanitizer",
    [int]$TimeoutSeconds = 300
)

$ErrorActionPreference = "Stop"

Import-Module "$SanitizerDir\Sanitizer.psm1" -Force

$paths = Get-SanitizerPaths -SanitizerDir $SanitizerDir
$config = Get-SanitizerConfig -SecretsPath $paths.Secrets
$reverseMappings = Get-ReverseMappings -SecretsPath $paths.Secrets -AutoMappingsPath $paths.AutoMappings
$forwardMappings = Get-SanitizerMappings -SecretsPath $paths.Secrets -AutoMappingsPath $paths.AutoMappings

# === CREATE SEALED ENVIRONMENT ===

$sealedId = [guid]::NewGuid().ToString("N").Substring(0, 8)
$sealedDir = "$env:TEMP\claude-sealed-$sealedId"

$output = ""
$exitCode = 0

try {
    New-Item -Path $sealedDir -ItemType Directory -Force | Out-Null

    # === COPY AND RENDER ===

    foreach ($file in Get-ChildItem -Path $WorkingDir -Recurse -File -ErrorAction SilentlyContinue) {
        $relativePath = $file.FullName.Substring($WorkingDir.Length).TrimStart('\', '/')

        if (Test-ExcludedPath -RelativePath $relativePath -ExcludePaths $config.excludePaths) { continue }
        if ($file.Length -gt 10MB) { continue }

        $destPath = Join-Path $sealedDir $relativePath
        $destDir = Split-Path $destPath -Parent

        if (-not (Test-Path $destDir)) {
            New-Item -Path $destDir -ItemType Directory -Force | Out-Null
        }

        $isBinary = (Test-ExcludedExtension -Extension $file.Extension -ExcludeExtensions $config.excludeExtensions) -or
                    (Test-BinaryFile -Path $file.FullName)

        if ($isBinary) {
            Copy-Item -Path $file.FullName -Destination $destPath -Force
            continue
        }

        # Process text file - render real values
        try {
            $content = [System.IO.File]::ReadAllText($file.FullName)
            if ([string]::IsNullOrEmpty($content)) {
                Copy-Item -Path $file.FullName -Destination $destPath -Force
                continue
            }

            $content = ConvertTo-RenderedText -Text $content -ReverseMappings $reverseMappings
            [System.IO.File]::WriteAllText($destPath, $content)
        }
        catch {
            Copy-Item -Path $file.FullName -Destination $destPath -Force
        }
    }

    # === EXECUTE COMMAND ===

    try {
        $psi = [System.Diagnostics.ProcessStartInfo]::new()
        $psi.FileName = "cmd.exe"
        $psi.Arguments = "/c cd /d `"$sealedDir`" && $Command 2>&1"
        $psi.RedirectStandardOutput = $true
        $psi.RedirectStandardError = $true
        $psi.UseShellExecute = $false
        $psi.CreateNoWindow = $true
        $psi.WorkingDirectory = $sealedDir

        $process = [System.Diagnostics.Process]::Start($psi)

        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()

        $completed = $process.WaitForExit($TimeoutSeconds * 1000)

        if (-not $completed) {
            $process.Kill()
            $output = "ERROR: Command timed out after $TimeoutSeconds seconds"
            $exitCode = 124
        }
        else {
            [System.Threading.Tasks.Task]::WaitAll($stdout, $stderr)
            $output = $stdout.Result
            if (-not [string]::IsNullOrEmpty($stderr.Result)) {
                $output += "`n" + $stderr.Result
            }
            $exitCode = $process.ExitCode
        }

        $process.Dispose()
    }
    catch {
        $output = "ERROR: Failed to execute command: $_"
        $exitCode = 1
    }
}
finally {
    # === ALWAYS DELETE TEMP DIRECTORY ===
    if (Test-Path $sealedDir) {
        Remove-Item -Path $sealedDir -Recurse -Force -ErrorAction SilentlyContinue
        if (Test-Path $sealedDir) {
            Start-Sleep -Milliseconds 100
            Remove-Item -Path $sealedDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

# === SANITIZE OUTPUT (directly, no subprocess) ===

if (-not [string]::IsNullOrEmpty($output)) {
    $output = ConvertTo-ScrubbedText -Text $output -Mappings $forwardMappings
}

# === RETURN RESULT AS JSON ===

@{
    output   = $output
    exitCode = $exitCode
} | ConvertTo-Json -Compress
