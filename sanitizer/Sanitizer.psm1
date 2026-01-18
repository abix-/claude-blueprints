<#
.SYNOPSIS
    Shared functions for the Claude Code sanitizer system.
#>

# === DEFAULTS ===

$script:DefaultExcludePaths = @(".git", "node_modules")

$script:ExcludedIpPatterns = @(
    '^127\.', '^0\.0\.0\.0$', '^255\.255\.255\.255$',
    '^169\.254\.', '^224\.', '^239\.', '^11\.\d+\.\d+\.\d+$'
)

$script:Ipv4Regex = '\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b'

# === CONFIGURATION ===

function Get-SanitizerPaths {
    <#
    .SYNOPSIS
        Returns standard sanitizer file paths.

    .EXAMPLE
        $paths = Get-SanitizerPaths
        Get-Content $paths.Secrets
    #>
    [CmdletBinding()]
    param(
        [string]$SanitizerDir = "$env:USERPROFILE\.claude\sanitizer"
    )

    [PSCustomObject]@{
        SanitizerDir = $SanitizerDir
        Secrets      = "$SanitizerDir\sanitizer.json"
        RenderedBase = "$env:USERPROFILE\.claude\rendered"
    }
}

function Get-SanitizerConfig {
    <#
    .SYNOPSIS
        Loads sanitizer configuration from sanitizer.json with defaults.

    .EXAMPLE
        $config = Get-SanitizerConfig
        $config.excludePaths
    #>
    [CmdletBinding()]
    param(
        [string]$SecretsPath = "$env:USERPROFILE\.claude\sanitizer\sanitizer.json"
    )

    $config = [PSCustomObject]@{
        mappings              = @{}
        autoMappings          = @{}
        excludePaths          = $script:DefaultExcludePaths
        patterns              = @{ ipv4 = $true; hostnames = @() }
        renderPath            = "$env:USERPROFILE\.claude\rendered\{project}"
        openExplorerOnRender  = $false
    }

    if (Test-Path $SecretsPath) {
        try {
            $loaded = Get-Content $SecretsPath -Raw | ConvertFrom-Json

            if ($loaded.mappings) {
                $config.mappings = @{}
                foreach ($prop in $loaded.mappings.PSObject.Properties) {
                    $config.mappings[$prop.Name] = $prop.Value
                }
            }
            if ($loaded.autoMappings) {
                $config.autoMappings = @{}
                foreach ($prop in $loaded.autoMappings.PSObject.Properties) {
                    $config.autoMappings[$prop.Name] = $prop.Value
                }
            }
            if ($loaded.excludePaths) { $config.excludePaths = $loaded.excludePaths }
            if ($loaded.patterns) { $config.patterns = $loaded.patterns }
            if ($loaded.renderPath) { $config.renderPath = $loaded.renderPath }
            if ($null -ne $loaded.openExplorerOnRender) { $config.openExplorerOnRender = $loaded.openExplorerOnRender }
        }
        catch {
            Write-Verbose "Failed to parse sanitizer.json: $_"
        }
    }

    $config
}

function Get-SanitizerMappings {
    <#
    .SYNOPSIS
        Loads all mappings (manual + auto) as real->fake hashtable.

    .EXAMPLE
        $mappings = Get-SanitizerMappings
        $fake = $mappings['11.139.237.229']
    #>
    [CmdletBinding()]
    param(
        [string]$SecretsPath = "$env:USERPROFILE\.claude\sanitizer\sanitizer.json"
    )

    $mappings = @{}
    $config = Get-SanitizerConfig -SecretsPath $SecretsPath

    foreach ($key in $config.mappings.Keys) {
        $mappings[$key] = $config.mappings[$key]
    }
    foreach ($key in $config.autoMappings.Keys) {
        if (-not $mappings.ContainsKey($key)) {
            $mappings[$key] = $config.autoMappings[$key]
        }
    }

    $mappings
}

function Get-ReverseMappings {
    <#
    .SYNOPSIS
        Loads all mappings as fake->real hashtable (for rendering).

    .EXAMPLE
        $reverse = Get-ReverseMappings
        $real = $reverse['11.22.33.44']
    #>
    [CmdletBinding()]
    param(
        [string]$SecretsPath = "$env:USERPROFILE\.claude\sanitizer\sanitizer.json"
    )

    $reverse = @{}
    $forward = Get-SanitizerMappings -SecretsPath $SecretsPath

    foreach ($real in $forward.Keys) {
        $fake = $forward[$real]
        if (-not $reverse.ContainsKey($fake)) {
            $reverse[$fake] = $real
        }
    }

    $reverse
}

function Save-AutoMappings {
    <#
    .SYNOPSIS
        Saves autoMappings back to sanitizer.json.

    .EXAMPLE
        Save-AutoMappings -AutoMappings $autoMappings
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [hashtable]$AutoMappings,

        [string]$SecretsPath = "$env:USERPROFILE\.claude\sanitizer\sanitizer.json"
    )

    $config = @{}
    if (Test-Path $SecretsPath) {
        try {
            $loaded = Get-Content $SecretsPath -Raw | ConvertFrom-Json
            foreach ($prop in $loaded.PSObject.Properties) {
                $config[$prop.Name] = $prop.Value
            }
        }
        catch { }
    }

    $config['autoMappings'] = $AutoMappings
    $config | ConvertTo-Json -Depth 5 | Set-Content -Path $SecretsPath -Encoding UTF8
}

# === FILE DETECTION ===

function Test-BinaryFile {
    <#
    .SYNOPSIS
        Checks if a file contains binary content (null bytes).

    .EXAMPLE
        if (Test-BinaryFile -Path "file.dat") { "Binary" }
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [string]$Path
    )

    try {
        $stream = [System.IO.File]::OpenRead($Path)
        $buffer = [byte[]]::new([Math]::Min(8192, $stream.Length))
        $bytesRead = $stream.Read($buffer, 0, $buffer.Length)
        $stream.Close()
        $stream.Dispose()

        for ($i = 0; $i -lt $bytesRead; $i++) {
            if ($buffer[$i] -eq 0) { return $true }
        }
        return $false
    }
    catch {
        return $true
    }
}

function Get-FileEncoding {
    <#
    .SYNOPSIS
        Detects file encoding from BOM.

    .EXAMPLE
        $enc = Get-FileEncoding -Path "file.txt"
        [System.IO.File]::ReadAllText($path, $enc)
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [string]$Path
    )

    $bytes = [System.IO.File]::ReadAllBytes($Path)

    if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
        return [System.Text.UTF8Encoding]::new($true)
    }
    elseif ($bytes.Length -ge 2 -and $bytes[0] -eq 0xFF -and $bytes[1] -eq 0xFE) {
        return [System.Text.UnicodeEncoding]::new($false, $true)
    }

    [System.Text.UTF8Encoding]::new($false)
}

# === EXCLUSION CHECKS ===

function Test-ExcludedPath {
    <#
    .SYNOPSIS
        Checks if a relative path matches exclusion patterns.

    .EXAMPLE
        Test-ExcludedPath -RelativePath "node_modules/pkg/index.js" -ExcludePaths @("node_modules")
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [string]$RelativePath,

        [string[]]$ExcludePaths = $script:DefaultExcludePaths
    )

    foreach ($exclude in $ExcludePaths) {
        if ($RelativePath -like "$exclude\*" -or
            $RelativePath -like "$exclude/*" -or
            $RelativePath -like "*\$exclude\*" -or
            $RelativePath -like "*/$exclude/*" -or
            $RelativePath -eq $exclude) {
            return $true
        }
    }
    $false
}

function Test-ExcludedIp {
    <#
    .SYNOPSIS
        Checks if an IP should be excluded from sanitization.

    .EXAMPLE
        Test-ExcludedIp -Ip "127.0.0.1"  # Returns $true
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [string]$Ip
    )

    foreach ($pattern in $script:ExcludedIpPatterns) {
        if ($Ip -match $pattern) { return $true }
    }
    $false
}

# === FAKE VALUE GENERATION ===

function New-FakeIp {
    <#
    .SYNOPSIS
        Generates a random fake IP in the 11.x.x.x range.

    .EXAMPLE
        $fake = New-FakeIp
    #>
    [CmdletBinding()]
    param()

    $b2 = Get-Random -Minimum 1 -Maximum 255
    $b3 = Get-Random -Minimum 1 -Maximum 255
    $b4 = Get-Random -Minimum 1 -Maximum 255
    "11.$b2.$b3.$b4"
}

function New-FakeHostname {
    <#
    .SYNOPSIS
        Generates a random fake hostname.

    .EXAMPLE
        $fake = New-FakeHostname
    #>
    [CmdletBinding()]
    param()

    $chars = 'abcdefghijklmnopqrstuvwxyz0123456789'
    $suffix = -join (1..8 | ForEach-Object { $chars[(Get-Random -Maximum $chars.Length)] })
    "host-$suffix.example.test"
}

function Get-DeterministicFakeIp {
    <#
    .SYNOPSIS
        Generates a deterministic fake IP from a real IP (for output scrubbing).

    .EXAMPLE
        $fake = Get-DeterministicFakeIp -RealIp "11.139.237.229"
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [string]$RealIp
    )

    $md5 = [System.Security.Cryptography.MD5]::Create()
    $hash = $md5.ComputeHash([System.Text.Encoding]::UTF8.GetBytes("ip:$RealIp"))
    $md5.Dispose()
    $hex = [BitConverter]::ToString($hash).Replace("-", "").ToLower()

    $b2 = ([Convert]::ToInt32($hex.Substring(0, 2), 16) % 254) + 1
    $b3 = ([Convert]::ToInt32($hex.Substring(2, 2), 16) % 254) + 1
    $b4 = ([Convert]::ToInt32($hex.Substring(4, 2), 16) % 254) + 1
    "11.$b2.$b3.$b4"
}

# === TEXT TRANSFORMATION ===

function ConvertTo-SanitizedText {
    <#
    .SYNOPSIS
        Applies mappings to text (real->fake).

    .EXAMPLE
        $sanitized = ConvertTo-SanitizedText -Text $content -Mappings $mappings
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [string]$Text,

        [Parameter(Mandatory)]
        [hashtable]$Mappings
    )

    $sorted = $Mappings.Keys | Sort-Object { $_.Length } -Descending
    foreach ($real in $sorted) {
        $Text = $Text -replace [regex]::Escape($real), $Mappings[$real]
    }
    $Text
}

function ConvertTo-RenderedText {
    <#
    .SYNOPSIS
        Applies reverse mappings to text (fake->real).

    .EXAMPLE
        $rendered = ConvertTo-RenderedText -Text $content -ReverseMappings $reverse
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [string]$Text,

        [Parameter(Mandatory)]
        [hashtable]$ReverseMappings
    )

    $sorted = $ReverseMappings.Keys | Sort-Object { $_.Length } -Descending
    foreach ($fake in $sorted) {
        $Text = $Text -replace [regex]::Escape($fake), $ReverseMappings[$fake]
    }
    $Text
}

function ConvertTo-ScrubbedText {
    <#
    .SYNOPSIS
        Sanitizes text with fallback IP scrubbing for unknown values.

    .EXAMPLE
        $scrubbed = ConvertTo-ScrubbedText -Text $output -Mappings $mappings
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [string]$Text,

        [hashtable]$Mappings = @{}
    )

    # Apply known mappings first
    if ($Mappings.Count -gt 0) {
        $Text = ConvertTo-SanitizedText -Text $Text -Mappings $Mappings
    }

    # Fallback: scrub any remaining IPs
    $Text = [regex]::Replace($Text, $script:Ipv4Regex, {
        param($m)
        $ip = $m.Value
        if (Test-ExcludedIp -Ip $ip) { return $ip }
        Get-DeterministicFakeIp -RealIp $ip
    })

    $Text
}

# === EXPORTS ===

Export-ModuleMember -Function @(
    'Get-SanitizerPaths'
    'Get-SanitizerConfig'
    'Get-SanitizerMappings'
    'Get-ReverseMappings'
    'Save-AutoMappings'
    'Test-BinaryFile'
    'Get-FileEncoding'
    'Test-ExcludedPath'
    'Test-ExcludedIp'
    'New-FakeIp'
    'New-FakeHostname'
    'Get-DeterministicFakeIp'
    'ConvertTo-SanitizedText'
    'ConvertTo-RenderedText'
    'ConvertTo-ScrubbedText'
)

Export-ModuleMember -Variable @(
    'DefaultExcludePaths'
    'Ipv4Regex'
)
