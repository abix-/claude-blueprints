#Requires -Modules Pester
<#
.SYNOPSIS
    Comprehensive Pester tests for sanitizer CLI tool.
.DESCRIPTION
    Run with: Invoke-Pester -Path ./sanitizer.tests.ps1 -Output Detailed
#>

BeforeAll {
    $script:sanitizer = "$PSScriptRoot/sanitizer.exe"

    # Build sanitizer
    Push-Location $PSScriptRoot
    go build -o sanitizer.exe ./cmd/sanitizer
    if ($LASTEXITCODE -ne 0) { throw "Build failed" }
    Pop-Location

    # Helper to create test config - use .NET to bypass hooks
    function New-TestConfig {
        param(
            [string[]]$Patterns = @(),
            [hashtable]$ManualMappings = @{},
            [hashtable]$AutoMappings = @{},
            [string[]]$SkipPaths = @(".git"),
            [string[]]$BlockedPaths = @()
        )
        $cfg = @{
            hostnamePatterns = $Patterns
            mappingsAuto     = $AutoMappings
            mappingsManual   = $ManualMappings
            skipPaths        = $SkipPaths
            unsanitizedPath  = "~/.claude/unsanitized/{project}"
            blockedPaths     = $BlockedPaths
        }
        return ($cfg | ConvertTo-Json -Depth 10)
    }

    # Helper to create isolated test environment
    function New-TestEnvironment {
        param([string]$Name, [string]$Config)

        $testDir = "$env:TEMP/sanitizer-$Name-$([guid]::NewGuid().ToString('N').Substring(0,8))"
        if (Test-Path $testDir) { Remove-Item $testDir -Recurse -Force }
        [System.IO.Directory]::CreateDirectory("$testDir/.claude/sanitizer") | Out-Null

        if ($Config) {
            [System.IO.File]::WriteAllText("$testDir/.claude/sanitizer/sanitizer.json", $Config)
        }

        return $testDir
    }

    # Helper to write file bypassing hooks
    function Write-TestFile {
        param([string]$Path, [string]$Content)
        $dir = Split-Path $Path -Parent
        if (-not (Test-Path $dir)) {
            [System.IO.Directory]::CreateDirectory($dir) | Out-Null
        }
        [System.IO.File]::WriteAllText($Path, $Content)
    }

    # Helper to read file bypassing hooks
    function Read-TestFile {
        param([string]$Path)
        return [System.IO.File]::ReadAllText($Path)
    }

    # Helper to run sanitizer with custom USERPROFILE
    function Invoke-SanitizerSession {
        param(
            [string]$TestDir,
            [string]$Command = 'hook-session-start'
        )

        $originalProfile = $env:USERPROFILE
        try {
            $env:USERPROFILE = $TestDir
            Push-Location $TestDir
            $null = '{"hook_event_name":"SessionStart"}' | & $script:sanitizer $Command 2>&1
        }
        finally {
            Pop-Location
            $env:USERPROFILE = $originalProfile
        }
    }

    # Helper to invoke hook-bash
    function Invoke-HookBash {
        param([string]$Command)
        $input = @{ hook_event_name = "PreToolUse"; tool_input = @{ command = $Command } } | ConvertTo-Json -Compress
        return $input | & $script:sanitizer hook-bash
    }

    # Helper to invoke hook-file-access
    function Invoke-HookFileAccess {
        param([string]$FilePath, [string]$ToolName = "Read")
        $input = @{ hook_event_name = "PreToolUse"; tool_name = $ToolName; tool_input = @{ file_path = $FilePath } } | ConvertTo-Json -Compress
        return $input | & $script:sanitizer hook-file-access
    }

    # Helper to invoke hook-post
    function Invoke-HookPost {
        param([string]$Output, [string]$ToolName = "Grep")
        $input = @{ hook_event_name = "PostToolUse"; tool_name = $ToolName; tool_output = $Output } | ConvertTo-Json -Compress
        return $input | & $script:sanitizer hook-post
    }
}

# ============================================================================
# IP SANITIZATION
# ============================================================================

Describe "sanitize-ips" {
    Context "Private IP ranges" {
        It "sanitizes 10.x.x.x range" {
            $result = "111.247.206.175" | & $sanitizer sanitize-ips
            $result | Should -Not -Match "^10\."
            $result | Should -Match "^111\.\d+\.\d+\.\d+$"
        }

        It "sanitizes 172.16-31.x.x range" {
            $result = "111.235.144.217 and 111.135.181.235" | & $sanitizer sanitize-ips
            $result | Should -Not -Match "172\.(1[6-9]|2[0-9]|3[01])\."
            $result | Should -Match "111\.\d+\.\d+\.\d+"
        }

        It "sanitizes 192.168.x.x range" {
            $result = "111.64.135.196" | & $sanitizer sanitize-ips
            $result | Should -Not -Match "192\.168\."
            $result | Should -Match "^111\.\d+\.\d+\.\d+$"
        }
    }

    Context "Excluded IPs (should NOT be sanitized)" {
        It "preserves loopback 127.x.x.x" {
            $result = "127.0.0.1 and 127.255.255.255" | & $sanitizer sanitize-ips
            $result | Should -Match "127\.0\.0\.1"
            $result | Should -Match "127\.255\.255\.255"
        }

        It "preserves 0.0.0.0" {
            $result = "bind to 0.0.0.0" | & $sanitizer sanitize-ips
            $result | Should -Match "0\.0\.0\.0"
        }

        It "preserves broadcast/netmask 255.x.x.x" {
            $result = "255.255.255.0 and 255.255.255.255" | & $sanitizer sanitize-ips
            $result | Should -Match "255\.255\.255\.0"
            $result | Should -Match "255\.255\.255\.255"
        }

        It "preserves link-local 169.254.x.x" {
            $result = "APIPA 169.254.1.1" | & $sanitizer sanitize-ips
            $result | Should -Match "169\.254\.1\.1"
        }

        It "preserves multicast 224-239.x.x.x" {
            $result = "multicast 224.0.0.1 and 239.255.255.255" | & $sanitizer sanitize-ips
            $result | Should -Match "224\.0\.0\.1"
            $result | Should -Match "239\.255\.255\.255"
        }

        It "preserves already-sanitized 111.x.x.x" {
            $result = "111.50.100.200" | & $sanitizer sanitize-ips
            $result | Should -Be "111.50.100.200"
        }
    }

    Context "Determinism" {
        It "produces identical output for same input" {
            $r1 = "111.38.230.69" | & $sanitizer sanitize-ips
            $r2 = "111.38.230.69" | & $sanitizer sanitize-ips
            $r1 | Should -Be $r2
        }

        It "produces different output for different inputs" {
            $r1 = "111.38.230.69" | & $sanitizer sanitize-ips
            $r2 = "111.68.155.57" | & $sanitizer sanitize-ips
            $r1 | Should -Not -Be $r2
        }
    }

    Context "Edge cases" {
        It "handles multiple IPs on one line" {
            $result = "111.38.230.69, 111.247.206.175, 111.235.144.217" | & $sanitizer sanitize-ips
            $result | Should -Not -Match "192\.168\."
            $result | Should -Not -Match "^10\."
            $result | Should -Not -Match "172\.16\."
            ([regex]::Matches($result, "111\.\d+\.\d+\.\d+")).Count | Should -Be 3
        }

        It "does not match version numbers like 111.213.77.138" {
            # Version numbers within valid IP range could match - this tests boundary
            $result = "version 111.213.77.138" | & $sanitizer sanitize-ips
            # 111.213.77.138 is a valid IP and will be sanitized - this is expected behavior
            $result | Should -Match "111\.\d+\.\d+\.\d+"
        }

        It "handles IP at start/end of string" {
            $result = "111.38.230.69" | & $sanitizer sanitize-ips
            $result | Should -Match "^111\.\d+\.\d+\.\d+$"
        }
    }
}

# ============================================================================
# HOOK-BASH (Command Routing)
# ============================================================================

Describe "hook-bash" {
    Context "BLOCK - Sensitive path access" {
        It "blocks cat of sanitizer.json" {
            $result = Invoke-HookBash "cat ~/.claude/sanitizer/sanitizer.json" | ConvertFrom-Json
            $result.hookSpecificOutput.permissionDecision | Should -Be "deny"
        }

        It "blocks with backslash path" {
            $result = Invoke-HookBash 'cat ~\.claude\sanitizer\sanitizer.json' | ConvertFrom-Json
            $result.hookSpecificOutput.permissionDecision | Should -Be "deny"
        }

        It "blocks ls of unsanitized directory" {
            $result = Invoke-HookBash "ls ~/.claude/unsanitized/" | ConvertFrom-Json
            $result.hookSpecificOutput.permissionDecision | Should -Be "deny"
        }

        It "blocks access via full Windows path" {
            $result = Invoke-HookBash "cat C:/Users/test/.claude/sanitizer/sanitizer.json" | ConvertFrom-Json
            $result.hookSpecificOutput.permissionDecision | Should -Be "deny"
        }
    }

    Context "SANITIZED - Normal commands (allow as-is)" {
        It "allows ls" {
            $result = Invoke-HookBash "ls -la"
            $result | Should -BeNullOrEmpty
        }

        It "allows git" {
            $result = Invoke-HookBash "git status"
            $result | Should -BeNullOrEmpty
        }

        It "allows npm" {
            $result = Invoke-HookBash "npm install"
            $result | Should -BeNullOrEmpty
        }

        It "allows python" {
            $result = Invoke-HookBash "python script.py"
            $result | Should -BeNullOrEmpty
        }

        It "allows go" {
            $result = Invoke-HookBash "go build ./..."
            $result | Should -BeNullOrEmpty
        }
    }

    Context "UNSANITIZED - PowerShell commands (wrap for real values)" {
        It "wraps powershell.exe" {
            $result = Invoke-HookBash "powershell.exe -Command Get-Date" | ConvertFrom-Json
            $result.hookSpecificOutput.updatedInput.command | Should -Not -BeNullOrEmpty
            $result.hookSpecificOutput.permissionDecision | Should -Be "allow"
        }

        It "wraps pwsh" {
            $result = Invoke-HookBash "pwsh -Command Get-Date" | ConvertFrom-Json
            $result.hookSpecificOutput.updatedInput.command | Should -Not -BeNullOrEmpty
        }

        It "wraps .ps1 scripts" {
            $result = Invoke-HookBash "./Deploy-App.ps1" | ConvertFrom-Json
            $result.hookSpecificOutput.updatedInput.command | Should -Not -BeNullOrEmpty
        }

        It "wraps & call operator" {
            $result = Invoke-HookBash '& $script' | ConvertFrom-Json
            $result.hookSpecificOutput.updatedInput.command | Should -Not -BeNullOrEmpty
        }

        It "wraps case-insensitive POWERSHELL" {
            $result = Invoke-HookBash "POWERSHELL -Command test" | ConvertFrom-Json
            $result.hookSpecificOutput.updatedInput.command | Should -Not -BeNullOrEmpty
        }
    }
}

# ============================================================================
# HOOK-FILE-ACCESS (File Access Control)
# ============================================================================

Describe "hook-file-access" {
    Context "BLOCK - Sensitive files" {
        It "blocks Read of sanitizer.json" {
            $result = Invoke-HookFileAccess "C:/Users/test/.claude/sanitizer/sanitizer.json" "Read" | ConvertFrom-Json
            $result.hookSpecificOutput.permissionDecision | Should -Be "deny"
        }

        It "blocks Edit of sanitizer.json" {
            $result = Invoke-HookFileAccess "C:/Users/test/.claude/sanitizer/sanitizer.json" "Edit" | ConvertFrom-Json
            $result.hookSpecificOutput.permissionDecision | Should -Be "deny"
        }

        It "blocks Write to sanitizer.json" {
            $result = Invoke-HookFileAccess "C:/Users/test/.claude/sanitizer/sanitizer.json" "Write" | ConvertFrom-Json
            $result.hookSpecificOutput.permissionDecision | Should -Be "deny"
        }

        It "blocks Read of unsanitized directory" {
            $result = Invoke-HookFileAccess "C:/Users/test/.claude/unsanitized/project/file.txt" "Read" | ConvertFrom-Json
            $result.hookSpecificOutput.permissionDecision | Should -Be "deny"
        }

        It "blocks with backslash paths" {
            $result = Invoke-HookFileAccess 'C:\Users\test\.claude\sanitizer\sanitizer.json' "Read" | ConvertFrom-Json
            $result.hookSpecificOutput.permissionDecision | Should -Be "deny"
        }

        It "blocks with mixed separators" {
            $result = Invoke-HookFileAccess 'C:/Users/test\.claude/sanitizer\sanitizer.json' "Read" | ConvertFrom-Json
            $result.hookSpecificOutput.permissionDecision | Should -Be "deny"
        }
    }

    Context "ALLOW - Normal files" {
        It "allows Read of project files" {
            $result = Invoke-HookFileAccess "C:/code/project/main.go" "Read"
            $result | Should -BeNullOrEmpty
        }

        It "allows Edit of project files" {
            $result = Invoke-HookFileAccess "C:/code/project/main.go" "Edit"
            $result | Should -BeNullOrEmpty
        }

        It "allows Write of project files" {
            $result = Invoke-HookFileAccess "C:/code/project/main.go" "Write"
            $result | Should -BeNullOrEmpty
        }
    }
}

# ============================================================================
# HOOK-POST (Output Sanitization)
# ============================================================================

Describe "hook-post" {
    It "sanitizes IPs in tool output" {
        $testDir = New-TestEnvironment -Name "hook-post"
        try {
            # Create config with existing mapping
            $config = New-TestConfig -AutoMappings @{ "111.64.135.196" = "111.50.50.50" }
            Write-TestFile "$testDir/.claude/sanitizer/sanitizer.json" $config

            $originalProfile = $env:USERPROFILE
            $env:USERPROFILE = $testDir
            try {
                $result = Invoke-HookPost "Found server at 111.64.135.196"
                $json = $result | ConvertFrom-Json
                $json.hookSpecificOutput.updatedOutput | Should -Match "111\.50\.50\.50"
                $json.hookSpecificOutput.updatedOutput | Should -Not -Match "192\.168\.1\.100"
            }
            finally {
                $env:USERPROFILE = $originalProfile
            }
        }
        finally {
            Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    It "returns null when no changes needed" {
        $testDir = New-TestEnvironment -Name "hook-post-nochange"
        try {
            $config = New-TestConfig
            Write-TestFile "$testDir/.claude/sanitizer/sanitizer.json" $config

            $originalProfile = $env:USERPROFILE
            $env:USERPROFILE = $testDir
            try {
                $result = Invoke-HookPost "No sensitive data here"
                $result | Should -BeNullOrEmpty
            }
            finally {
                $env:USERPROFILE = $originalProfile
            }
        }
        finally {
            Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

# ============================================================================
# HOOK-SESSION-START (Project Sanitization)
# ============================================================================

Describe "hook-session-start" {
    Context "IP sanitization" {
        It "sanitizes all private IP ranges in files" {
            $testDir = New-TestEnvironment -Name "session-ips"
            try {
                Write-TestFile "$testDir/config.txt" @"
server1 = 111.64.135.196
server2 = 111.210.246.198
server3 = 111.187.8.34
"@
                Invoke-SanitizerSession -TestDir $testDir

                $sanitized = Read-TestFile "$testDir/config.txt"
                $sanitized | Should -Not -Match "192\.168\."
                $sanitized | Should -Not -Match "^10\."
                $sanitized | Should -Not -Match "172\.16\."
                ([regex]::Matches($sanitized, "111\.\d+\.\d+\.\d+")).Count | Should -Be 3
            }
            finally {
                Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
            }
        }

        It "preserves excluded IPs" {
            $testDir = New-TestEnvironment -Name "session-excluded"
            try {
                Write-TestFile "$testDir/config.txt" "localhost = 127.0.0.1`nbind = 0.0.0.0"
                Invoke-SanitizerSession -TestDir $testDir

                $sanitized = Read-TestFile "$testDir/config.txt"
                $sanitized | Should -Match "127\.0\.0\.1"
                $sanitized | Should -Match "0\.0\.0\.0"
            }
            finally {
                Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
            }
        }
    }

    Context "File handling" {
        It "skips .git directory" {
            $testDir = New-TestEnvironment -Name "session-skipgit"
            try {
                [System.IO.Directory]::CreateDirectory("$testDir/.git") | Out-Null
                # Use a unique marker that won't be sanitized
                $gitContent = "secret = 192.168.77.77"
                $mainContent = "ip = 192.168.88.88"
                Write-TestFile "$testDir/.git/config" $gitContent
                Write-TestFile "$testDir/main.go" $mainContent
                Invoke-SanitizerSession -TestDir $testDir

                # .git should be untouched (exact same content)
                Read-TestFile "$testDir/.git/config" | Should -Be $gitContent
                # main.go should be sanitized (different from original)
                Read-TestFile "$testDir/main.go" | Should -Not -Be $mainContent
                Read-TestFile "$testDir/main.go" | Should -Match "111\.\d+\.\d+\.\d+"
            }
            finally {
                Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
            }
        }

        It "skips .claude directory (config protection)" {
            $testDir = New-TestEnvironment -Name "session-skipclaude"
            try {
                $config = New-TestConfig -AutoMappings @{ "111.80.178.23" = "111.1.1.1" }
                Write-TestFile "$testDir/.claude/sanitizer/sanitizer.json" $config
                Write-TestFile "$testDir/app.py" "host = 111.80.178.23"
                Invoke-SanitizerSession -TestDir $testDir

                # Config should NOT have its keys sanitized
                $savedConfig = Read-TestFile "$testDir/.claude/sanitizer/sanitizer.json" | ConvertFrom-Json
                $savedConfig.mappingsAuto.PSObject.Properties.Name | Should -Contain "111.80.178.23"
            }
            finally {
                Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
            }
        }

        It "skips binary files" {
            $testDir = New-TestEnvironment -Name "session-binary"
            try {
                # Create a file with null bytes (binary)
                $bytes = [byte[]]@(0x00, 0x50, 0x4B, 0x03, 0x04)  # ZIP header
                [System.IO.File]::WriteAllBytes("$testDir/archive.zip", $bytes)
                Write-TestFile "$testDir/main.go" "ip = 111.38.230.69"
                Invoke-SanitizerSession -TestDir $testDir

                # Binary should be untouched
                [System.IO.File]::ReadAllBytes("$testDir/archive.zip") | Should -Be $bytes
                # Text file should be sanitized
                Read-TestFile "$testDir/main.go" | Should -Not -Match "192\.168\.1\.1"
            }
            finally {
                Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
            }
        }

        It "processes nested directories" {
            $testDir = New-TestEnvironment -Name "session-nested"
            try {
                Write-TestFile "$testDir/src/config/db.yaml" "host: 111.64.135.196"
                Write-TestFile "$testDir/src/app/settings.json" '{"ip": "111.210.246.198"}'
                Invoke-SanitizerSession -TestDir $testDir

                Read-TestFile "$testDir/src/config/db.yaml" | Should -Not -Match "192\.168\."
                Read-TestFile "$testDir/src/app/settings.json" | Should -Not -Match "10\.0\.0\."
            }
            finally {
                Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
            }
        }
    }

    Context "Mapping persistence" {
        It "saves discovered mappings to config" {
            $testDir = New-TestEnvironment -Name "session-persist"
            try {
                $config = New-TestConfig
                Write-TestFile "$testDir/.claude/sanitizer/sanitizer.json" $config
                # Use a specific IP that will be discovered and saved
                $testIP = "192.168.99.99"
                Write-TestFile "$testDir/app.txt" "server = $testIP"
                Invoke-SanitizerSession -TestDir $testDir

                $savedConfig = Read-TestFile "$testDir/.claude/sanitizer/sanitizer.json" | ConvertFrom-Json
                # The original IP should be saved as a key in mappingsAuto
                $savedConfig.mappingsAuto.PSObject.Properties.Name | Should -Contain $testIP
            }
            finally {
                Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
            }
        }
    }
}

# ============================================================================
# HOOK-SESSION-STOP (Sync to Unsanitized)
# ============================================================================

Describe "hook-session-stop" {
    It "creates unsanitized copy with real values restored" {
        $testDir = New-TestEnvironment -Name "session-stop"
        $projectName = Split-Path $testDir -Leaf
        $originalIP = "192.168.50.123"

        try {
            Write-TestFile "$testDir/deploy.ps1" "Connect-Server -IP `"$originalIP`""

            # Session start sanitizes and saves mapping
            Invoke-SanitizerSession -TestDir $testDir -Command 'hook-session-start'
            $sanitized = Read-TestFile "$testDir/deploy.ps1"
            $sanitized | Should -Match "111\.\d+\.\d+\.\d+"
            $sanitized | Should -Not -Match $originalIP

            # Session stop restores
            $originalProfile = $env:USERPROFILE
            try {
                $env:USERPROFILE = $testDir
                Push-Location $testDir
                $null = '{"hook_event_name":"Stop"}' | & $sanitizer hook-session-stop 2>&1
            }
            finally {
                Pop-Location
                $env:USERPROFILE = $originalProfile
            }

            # Unsanitized copy should have original IP restored
            $unsanitizedFile = "$testDir/.claude/unsanitized/$projectName/deploy.ps1"
            $unsanitizedFile | Should -Exist
            Read-TestFile $unsanitizedFile | Should -Match $originalIP
        }
        finally {
            Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

# ============================================================================
# HOSTNAME PATTERNS
# ============================================================================

Describe "hostname-patterns" {
    Context "Pattern matching" {
        It "sanitizes hostnames matching pattern" {
            $testDir = New-TestEnvironment -Name "host-basic" -Config (New-TestConfig -Patterns @("server\d{2}"))
            try {
                Write-TestFile "$testDir/inventory.yml" "host: server01`nbackup: server99"
                Invoke-SanitizerSession -TestDir $testDir

                $sanitized = Read-TestFile "$testDir/inventory.yml"
                $sanitized | Should -Not -Match "server01"
                $sanitized | Should -Not -Match "server99"
                $sanitized | Should -Match "host-[a-z0-9]+\.example\.test"
            }
            finally {
                Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
            }
        }

        It "matches case-insensitively" {
            $testDir = New-TestEnvironment -Name "host-case" -Config (New-TestConfig -Patterns @("prodweb\d+"))
            try {
                Write-TestFile "$testDir/hosts.txt" "host-4x14bsel.example.test`nhost-0309skeo.example.test`nhost-a3plihqn.example.test"
                Invoke-SanitizerSession -TestDir $testDir

                $sanitized = Read-TestFile "$testDir/hosts.txt"
                $sanitized | Should -Not -Match "(?i)prodweb"
            }
            finally {
                Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
            }
        }

        It "captures domain suffix (FQDN)" {
            $testDir = New-TestEnvironment -Name "host-fqdn" -Config (New-TestConfig -Patterns @("srv[0-9]+"))
            try {
                Write-TestFile "$testDir/dns.txt" "srv01.internal.corp.local"
                Invoke-SanitizerSession -TestDir $testDir

                $sanitized = Read-TestFile "$testDir/dns.txt"
                $sanitized | Should -Not -Match "srv01"
                $sanitized | Should -Not -Match "internal\.corp\.local"
            }
            finally {
                Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
            }
        }

        It "handles multiple patterns" {
            $testDir = New-TestEnvironment -Name "host-multi" -Config (New-TestConfig -Patterns @("web\d+", "db\d+", "app\d+"))
            try {
                Write-TestFile "$testDir/arch.txt" "web01 -> app01 -> db01"
                Invoke-SanitizerSession -TestDir $testDir

                $sanitized = Read-TestFile "$testDir/arch.txt"
                $sanitized | Should -Not -Match "web01"
                $sanitized | Should -Not -Match "app01"
                $sanitized | Should -Not -Match "db01"
            }
            finally {
                Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
            }
        }
    }

    Context "Identity mappings (preservation)" {
        It "preserves values with identity mapping" {
            $config = New-TestConfig -Patterns @("Packed[A-Za-z0-9]+Array") -ManualMappings @{
                "host-hh0o0585.example.testArray"   = "host-hh0o0585.example.testArray"
                "host-3uyaeoky.example.testArray" = "host-3uyaeoky.example.testArray"
            }
            $testDir = New-TestEnvironment -Name "host-identity" -Config $config
            try {
                Write-TestFile "$testDir/types.gd" "var a: host-hh0o0585.example.testArray`nvar b: host-3uyaeoky.example.testArray"
                Invoke-SanitizerSession -TestDir $testDir

                $sanitized = Read-TestFile "$testDir/types.gd"
                $sanitized | Should -Match "host-hh0o0585.example.testArray"
                $sanitized | Should -Match "host-3uyaeoky.example.testArray"
            }
            finally {
                Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
            }
        }
    }

    Context "Determinism" {
        It "produces same output across multiple runs" {
            $testDir = New-TestEnvironment -Name "host-determ" -Config (New-TestConfig -Patterns @("myhost\d+"))
            try {
                $original = "connect to myhost01 and myhost02"
                Write-TestFile "$testDir/test.txt" $original

                # First run
                Invoke-SanitizerSession -TestDir $testDir
                $firstRun = Read-TestFile "$testDir/test.txt"

                # Reset file, keep config
                Write-TestFile "$testDir/test.txt" $original

                # Second run
                Invoke-SanitizerSession -TestDir $testDir
                $secondRun = Read-TestFile "$testDir/test.txt"

                $firstRun | Should -Be $secondRun
            }
            finally {
                Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
            }
        }
    }

    Context "Edge cases" {
        It "leaves non-matching text unchanged" {
            $testDir = New-TestEnvironment -Name "host-nomatch" -Config (New-TestConfig -Patterns @("server\d{2}"))
            try {
                $original = "Hello world, no hostnames here"
                Write-TestFile "$testDir/readme.txt" $original
                Invoke-SanitizerSession -TestDir $testDir

                (Read-TestFile "$testDir/readme.txt").Trim() | Should -Be $original
            }
            finally {
                Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
            }
        }

        It "handles mixed hostnames and IPs" {
            $testDir = New-TestEnvironment -Name "host-mixed" -Config (New-TestConfig -Patterns @("dbserver\d+"))
            try {
                Write-TestFile "$testDir/config.ini" "host = dbserver01`nip = 111.64.135.196"
                Invoke-SanitizerSession -TestDir $testDir

                $sanitized = Read-TestFile "$testDir/config.ini"
                $sanitized | Should -Not -Match "dbserver01"
                $sanitized | Should -Not -Match "192\.168\.1\.100"
                $sanitized | Should -Match "host-[a-z0-9]+\.example\.test"
                $sanitized | Should -Match "111\.\d+\.\d+\.\d+"
            }
            finally {
                Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
            }
        }
    }
}

# ============================================================================
# CONFIG HANDLING
# ============================================================================

Describe "config-handling" {
    It "creates default config if missing" {
        $testDir = New-TestEnvironment -Name "config-default"
        # Don't create config file
        Remove-Item "$testDir/.claude/sanitizer/sanitizer.json" -ErrorAction SilentlyContinue
        try {
            Write-TestFile "$testDir/test.txt" "111.38.230.69"
            Invoke-SanitizerSession -TestDir $testDir

            # Config should be created
            "$testDir/.claude/sanitizer/sanitizer.json" | Should -Exist
        }
        finally {
            Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    It "handles UTF-8 BOM in config" {
        $testDir = New-TestEnvironment -Name "config-bom"
        try {
            # Write config with UTF-8 BOM
            $config = New-TestConfig -Patterns @("test\d+")
            $bom = [byte[]]@(0xEF, 0xBB, 0xBF)
            $content = [System.Text.Encoding]::UTF8.GetBytes($config)
            [System.IO.File]::WriteAllBytes("$testDir/.claude/sanitizer/sanitizer.json", $bom + $content)

            Write-TestFile "$testDir/test.txt" "test01"
            Invoke-SanitizerSession -TestDir $testDir

            # Should still work
            Read-TestFile "$testDir/test.txt" | Should -Match "host-[a-z0-9]+\.example\.test"
        }
        finally {
            Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

# ============================================================================
# REGRESSION TESTS (for bugs we've fixed)
# ============================================================================

Describe "regression-tests" {
    It "hostname charset generates valid hostnames (no dots in random part)" {
        # Regression: ip.go had corrupt charset "host-78qei5ef.example.test23456789"
        $testDir = New-TestEnvironment -Name "reg-charset" -Config (New-TestConfig -Patterns @("testhost\d+"))
        try {
            Write-TestFile "$testDir/test.txt" "testhost01"
            Invoke-SanitizerSession -TestDir $testDir

            $sanitized = Read-TestFile "$testDir/test.txt"
            # Should match host-XXXXXXXX.example.test where X is alphanumeric only
            $sanitized | Should -Match "^host-[a-z0-9]{8}\.example\.test$"
        }
        finally {
            Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    It "config mappings preserve original values as keys (not sanitized keys)" {
        # Regression: .claude directory wasn't skipped, config got sanitized
        $testDir = New-TestEnvironment -Name "reg-configkeys" -Config (New-TestConfig -Patterns @("myserver\d+"))
        try {
            Write-TestFile "$testDir/test.txt" "myserver01"
            Invoke-SanitizerSession -TestDir $testDir

            $config = Read-TestFile "$testDir/.claude/sanitizer/sanitizer.json" | ConvertFrom-Json
            # Key should be original value, not sanitized
            $config.mappingsAuto.PSObject.Properties.Name | Should -Contain "myserver01"
            $config.mappingsAuto.PSObject.Properties.Name | Should -Not -Match "host-.*\.example\.test"
        }
        finally {
            Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}
