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
#>

param(
    [Parameter(Mandatory=$true)]
    [string]$Command,

    [string]$WorkingDir = (Get-Location).Path,
    [string]$SanitizerDir = "$env:USERPROFILE\.claude\sanitizer"
)

$ErrorActionPreference = "Stop"

$secretsPath = "$SanitizerDir\secrets.json"
$autoMappingsPath = "$SanitizerDir\auto_mappings.json"
$sanitizeOutputScript = "$SanitizerDir\SanitizeOutput.ps1"

# === CREATE SEALED ENVIRONMENT ===

$sealedId = [guid]::NewGuid().ToString("N").Substring(0, 8)
$sealedDir = "$env:TEMP\claude-sealed-$sealedId"

$output = ""
$exitCode = 0

try {
    # Create temp directory
    New-Item -Path $sealedDir -ItemType Directory -Force | Out-Null

    # === LOAD REVERSE MAPPINGS (fake -> real) ===

    $reverseMappings = @{}

    if (Test-Path $secretsPath) {
        try {
            $config = Get-Content $secretsPath -Raw | ConvertFrom-Json
            if ($config.mappings) {
                foreach ($prop in $config.mappings.PSObject.Properties) {
                    # fake -> real (value -> name)
                    $reverseMappings[$prop.Value] = $prop.Name
                }
            }
        }
        catch { }
    }

    if (Test-Path $autoMappingsPath) {
        try {
            $auto = Get-Content $autoMappingsPath -Raw | ConvertFrom-Json
            if ($auto.mappings) {
                foreach ($prop in $auto.mappings.PSObject.Properties) {
                    if (-not $reverseMappings.ContainsKey($prop.Value)) {
                        $reverseMappings[$prop.Value] = $prop.Name
                    }
                }
            }
        }
        catch { }
    }

    # Sort by fake value length descending
    $sortedFakes = @($reverseMappings.Keys | Sort-Object { $_.Length } -Descending)

    # === GET EXCLUSIONS ===

    $excludePaths = @(".git", "node_modules", ".claude", "bin", "obj", "__pycache__", "venv", ".venv")
    $excludeExtensions = @(".exe", ".dll", ".pdb", ".png", ".jpg", ".jpeg", ".gif", ".ico")

    if (Test-Path $secretsPath) {
        try {
            $config = Get-Content $secretsPath -Raw | ConvertFrom-Json
            if ($config.excludePaths) { $excludePaths = $config.excludePaths }
            if ($config.excludeExtensions) { $excludeExtensions = $config.excludeExtensions }
        }
        catch { }
    }

    # === COPY AND RENDER ===

    Get-ChildItem -Path $WorkingDir -Recurse -File -ErrorAction SilentlyContinue | ForEach-Object {
        $file = $_
        $relativePath = $file.FullName.Substring($WorkingDir.Length).TrimStart('\', '/')

        # Check path exclusions
        $excluded = $false
        foreach ($exclude in $excludePaths) {
            if ($relativePath -like "$exclude\*" -or $relativePath -like "$exclude/*" -or $relativePath -like "*\$exclude\*") {
                $excluded = $true
                break
            }
        }
        if ($excluded) { return }

        # Skip huge files
        if ($file.Length -gt 10MB) { return }

        # Determine destination
        $destPath = Join-Path $sealedDir $relativePath
        $destDir = Split-Path $destPath -Parent

        if (-not (Test-Path $destDir)) {
            New-Item -Path $destDir -ItemType Directory -Force | Out-Null
        }

        # Check if binary
        $isBinary = $false
        try {
            $stream = [System.IO.File]::OpenRead($file.FullName)
            $buffer = [byte[]]::new([Math]::Min(8192, $stream.Length))
            $bytesRead = $stream.Read($buffer, 0, $buffer.Length)
            $stream.Close()
            $stream.Dispose()
            for ($i = 0; $i -lt $bytesRead; $i++) {
                if ($buffer[$i] -eq 0) { $isBinary = $true; break }
            }
        }
        catch { $isBinary = $true }

        # Check extension exclusions
        foreach ($ext in $excludeExtensions) {
            if ($file.Extension -ieq $ext) { $isBinary = $true; break }
        }

        if ($isBinary) {
            # Just copy binary files without modification
            Copy-Item -Path $file.FullName -Destination $destPath -Force
            return
        }

        # Process text file - render real values
        try {
            $content = [System.IO.File]::ReadAllText($file.FullName)
            if ([string]::IsNullOrEmpty($content)) {
                Copy-Item -Path $file.FullName -Destination $destPath -Force
                return
            }

            # Apply reverse mappings (fake -> real)
            foreach ($fake in $sortedFakes) {
                $real = $reverseMappings[$fake]
                $content = $content -replace [regex]::Escape($fake), $real
            }

            [System.IO.File]::WriteAllText($destPath, $content)
        }
        catch {
            # Fallback: just copy
            Copy-Item -Path $file.FullName -Destination $destPath -Force
        }
    }

    # === EXECUTE COMMAND ===

    try {
        $psi = New-Object System.Diagnostics.ProcessStartInfo
        $psi.FileName = "cmd.exe"
        $psi.Arguments = "/c cd /d `"$sealedDir`" && $Command 2>&1"
        $psi.RedirectStandardOutput = $true
        $psi.RedirectStandardError = $true
        $psi.UseShellExecute = $false
        $psi.CreateNoWindow = $true
        $psi.WorkingDirectory = $sealedDir

        $process = [System.Diagnostics.Process]::Start($psi)

        # Read output with timeout
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()

        $completed = $process.WaitForExit(300000)  # 5 minute timeout

        if (-not $completed) {
            $process.Kill()
            $output = "ERROR: Command timed out after 5 minutes"
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
        try {
            Remove-Item -Path $sealedDir -Recurse -Force -ErrorAction SilentlyContinue
        }
        catch {
            # Try harder
            Start-Sleep -Milliseconds 100
            Remove-Item -Path $sealedDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

# === SANITIZE OUTPUT ===

if (-not [string]::IsNullOrEmpty($output)) {
    $output = $output | & powershell.exe -ExecutionPolicy Bypass -NoProfile -File $sanitizeOutputScript
}

# === RETURN RESULT AS JSON ===

@{
    output = $output
    exitCode = $exitCode
} | ConvertTo-Json -Compress
