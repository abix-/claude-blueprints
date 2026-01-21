#Requires -Modules Pester
<#
.SYNOPSIS
    Pester tests for sanitizer CLI tool.
.DESCRIPTION
    Run with: Invoke-Pester -Path ./sanitizer.tests.ps1 -Output Detailed
#>

BeforeAll {
    $script:sanitizer = "$PSScriptRoot/sanitizer.exe"

    Push-Location $PSScriptRoot
    go build -o sanitizer.exe ./cmd/sanitizer
    if ($LASTEXITCODE -ne 0) { throw "Build failed" }
    Pop-Location

    # -------------------------------------------------------------------------
    # Test Data - IPs and Patterns
    # -------------------------------------------------------------------------

    # Private IPs (will be sanitized)
    $script:IP_10      = "10.0.0.1"
    $script:IP_172     = "172.16.0.1"
    $script:IP_192     = "192.168.1.1"
    $script:IP_192_2   = "192.168.2.2"
    $script:IP_192_99  = "192.168.99.99"
    $script:IP_192_88  = "192.168.88.88"

    # Excluded IPs (should NOT be sanitized)
    $script:IP_LOOP    = "127.0.0.1"
    $script:IP_LOOP2   = "127.255.255.255"
    $script:IP_ZERO    = "0.0.0.0"
    $script:IP_BCAST   = "255.255.255.255"
    $script:IP_MASK    = "255.255.255.0"
    $script:IP_LINK    = "169.254.1.1"
    $script:IP_MCAST   = "224.0.0.1"
    $script:IP_MCAST2  = "239.255.255.255"

    # Already-sanitized IPs (111.x range - pass through unchanged)
    $script:IP_SAN     = "111.50.100.200"

    # Regex pattern matching any sanitized IP
    $script:RX_SAN     = "111\.\d+\.\d+\.\d+"

    # -------------------------------------------------------------------------
    # Test Helpers
    # -------------------------------------------------------------------------

    function New-TestConfig {
        param(
            [string[]]$Patterns = @(),
            [hashtable]$ManualMappings = @{},
            [hashtable]$AutoMappings = @{},
            [string[]]$SkipPaths = @(".git"),
            [string[]]$BlockedPaths = @()
        )
        @{
            hostnamePatterns = $Patterns
            mappingsAuto     = $AutoMappings
            mappingsManual   = $ManualMappings
            skipPaths        = $SkipPaths
            unsanitizedPath  = "~/.claude/unsanitized/{project}"
            blockedPaths     = $BlockedPaths
        } | ConvertTo-Json -Depth 10
    }

    function Write-TestFile([string]$Path, [string]$Content) {
        $dir = Split-Path $Path -Parent
        if (-not (Test-Path $dir)) { [System.IO.Directory]::CreateDirectory($dir) | Out-Null }
        [System.IO.File]::WriteAllText($Path, $Content)
    }

    function Read-TestFile([string]$Path) { [System.IO.File]::ReadAllText($Path) }

    # Main test runner - handles environment setup/teardown and USERPROFILE swap
    function Invoke-SanitizerTest {
        param(
            [string]$Name,
            [string]$Config,
            [scriptblock]$Test
        )

        $testDir = "$env:TEMP/sanitizer-$Name-$([guid]::NewGuid().ToString('N').Substring(0,8))"
        if (Test-Path $testDir) { Remove-Item $testDir -Recurse -Force }
        [System.IO.Directory]::CreateDirectory("$testDir/.claude/sanitizer") | Out-Null

        if ($Config) {
            [System.IO.File]::WriteAllText("$testDir/.claude/sanitizer/sanitizer.json", $Config)
        }

        $originalProfile = $env:USERPROFILE
        try {
            $env:USERPROFILE = $testDir
            Push-Location $testDir
            & $Test $testDir
        }
        finally {
            Pop-Location
            $env:USERPROFILE = $originalProfile
            Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    function Invoke-Session([string]$Cmd = 'hook-session-start') {
        $null = '{"hook_event_name":"SessionStart"}' | & $script:sanitizer $Cmd 2>&1
    }

    function Invoke-HookBash([string]$Command) {
        (@{ hook_event_name = "PreToolUse"; tool_input = @{ command = $Command } } | ConvertTo-Json -Compress) |
            & $script:sanitizer hook-bash
    }

    function Invoke-HookFileAccess([string]$FilePath, [string]$ToolName = "Read") {
        (@{ hook_event_name = "PreToolUse"; tool_name = $ToolName; tool_input = @{ file_path = $FilePath } } | ConvertTo-Json -Compress) |
            & $script:sanitizer hook-file-access
    }

    function Invoke-HookPost([string]$Output, [string]$ToolName = "Grep") {
        (@{ hook_event_name = "PostToolUse"; tool_name = $ToolName; tool_output = $Output } | ConvertTo-Json -Compress) |
            & $script:sanitizer hook-post
    }
}

# ============================================================================
# IP SANITIZATION (sanitize-ips command)
# ============================================================================

Describe "sanitize-ips" {
    It "sanitizes private ranges: 10.x, 172.16-31.x, 192.168.x" {
        $IP_10 | & $sanitizer sanitize-ips | Should -Match "^$RX_SAN$"
        "$IP_172 $IP_172" | & $sanitizer sanitize-ips | Should -Match "$RX_SAN.*$RX_SAN"
        $IP_192 | & $sanitizer sanitize-ips | Should -Match "^$RX_SAN$"
    }

    It "preserves excluded IPs: loopback, broadcast, link-local, multicast, already-sanitized" {
        "$IP_LOOP $IP_LOOP2" | & $sanitizer sanitize-ips | Should -Match "$IP_LOOP.*$IP_LOOP2"
        $IP_ZERO | & $sanitizer sanitize-ips | Should -Match $IP_ZERO
        "$IP_MASK $IP_BCAST" | & $sanitizer sanitize-ips | Should -Match "$IP_MASK.*$IP_BCAST"
        $IP_LINK | & $sanitizer sanitize-ips | Should -Match $IP_LINK
        "$IP_MCAST $IP_MCAST2" | & $sanitizer sanitize-ips | Should -Match "$IP_MCAST.*$IP_MCAST2"
        $IP_SAN | & $sanitizer sanitize-ips | Should -Be $IP_SAN
    }

    It "is deterministic (same input = same output)" {
        $r1 = $IP_192 | & $sanitizer sanitize-ips
        $r2 = $IP_192 | & $sanitizer sanitize-ips
        $r1 | Should -Be $r2
    }

    It "handles multiple IPs on one line" {
        $result = "$IP_192, $IP_10, $IP_172" | & $sanitizer sanitize-ips
        ([regex]::Matches($result, $RX_SAN)).Count | Should -Be 3
    }

    It "sanitizes public IPs" {
        "8.8.8.8 1.1.1.1 208.67.222.222" | & $sanitizer sanitize-ips | Should -Not -Match "8\.8\.8\.8|1\.1\.1\.1|208\.67"
    }
}

# ============================================================================
# HOOK-BASH (Command Routing)
# ============================================================================

Describe "hook-bash" {
    Context "BLOCK - Sensitive paths" {
        It "blocks access to sanitizer.json and unsanitized directory" {
            (Invoke-HookBash "cat ~/.claude/sanitizer/sanitizer.json" | ConvertFrom-Json).hookSpecificOutput.permissionDecision | Should -Be "deny"
            (Invoke-HookBash 'cat ~\.claude\sanitizer\sanitizer.json' | ConvertFrom-Json).hookSpecificOutput.permissionDecision | Should -Be "deny"
            (Invoke-HookBash "ls ~/.claude/unsanitized/" | ConvertFrom-Json).hookSpecificOutput.permissionDecision | Should -Be "deny"
            (Invoke-HookBash "cat C:/Users/test/.claude/sanitizer/sanitizer.json" | ConvertFrom-Json).hookSpecificOutput.permissionDecision | Should -Be "deny"
        }
    }

    Context "SANITIZED - Normal commands (pass through)" {
        It "allows common commands: ls, git, npm, python, go" {
            Invoke-HookBash "ls -la" | Should -BeNullOrEmpty
            Invoke-HookBash "git status" | Should -BeNullOrEmpty
            Invoke-HookBash "npm install" | Should -BeNullOrEmpty
            Invoke-HookBash "python script.py" | Should -BeNullOrEmpty
            Invoke-HookBash "go build ./..." | Should -BeNullOrEmpty
        }
    }

    Context "UNSANITIZED - PowerShell commands (wrap for real values)" {
        It "wraps PowerShell commands for execution in unsanitized dir" {
            (Invoke-HookBash "powershell.exe -Command Get-Date" | ConvertFrom-Json).hookSpecificOutput.updatedInput.command | Should -Not -BeNullOrEmpty
            (Invoke-HookBash "pwsh -Command Get-Date" | ConvertFrom-Json).hookSpecificOutput.updatedInput.command | Should -Not -BeNullOrEmpty
            (Invoke-HookBash "./Deploy-App.ps1" | ConvertFrom-Json).hookSpecificOutput.updatedInput.command | Should -Not -BeNullOrEmpty
            (Invoke-HookBash '& $script' | ConvertFrom-Json).hookSpecificOutput.updatedInput.command | Should -Not -BeNullOrEmpty
            (Invoke-HookBash "POWERSHELL -Command test" | ConvertFrom-Json).hookSpecificOutput.updatedInput.command | Should -Not -BeNullOrEmpty
        }
    }
}

# ============================================================================
# HOOK-FILE-ACCESS (File Access Control)
# ============================================================================

Describe "hook-file-access" {
    It "blocks Read/Edit/Write of sensitive files" {
        @("Read", "Edit", "Write") | ForEach-Object {
            (Invoke-HookFileAccess "C:/Users/test/.claude/sanitizer/sanitizer.json" $_  | ConvertFrom-Json).hookSpecificOutput.permissionDecision | Should -Be "deny"
        }
        (Invoke-HookFileAccess "C:/Users/test/.claude/unsanitized/project/file.txt" "Read" | ConvertFrom-Json).hookSpecificOutput.permissionDecision | Should -Be "deny"
        (Invoke-HookFileAccess 'C:\Users\test\.claude\sanitizer\sanitizer.json' "Read" | ConvertFrom-Json).hookSpecificOutput.permissionDecision | Should -Be "deny"
    }

    It "allows access to normal project files" {
        Invoke-HookFileAccess "C:/code/project/main.go" "Read" | Should -BeNullOrEmpty
        Invoke-HookFileAccess "C:/code/project/main.go" "Edit" | Should -BeNullOrEmpty
        Invoke-HookFileAccess "C:/code/project/main.go" "Write" | Should -BeNullOrEmpty
    }
}

# ============================================================================
# HOOK-POST (Output Sanitization)
# ============================================================================

Describe "hook-post" {
    It "sanitizes IPs in tool output" {
        Invoke-SanitizerTest -Name "hook-post" -Config (New-TestConfig -AutoMappings @{ $IP_192 = "111.50.50.50" }) -Test {
            $result = Invoke-HookPost "Found server at $IP_192" | ConvertFrom-Json
            $result.hookSpecificOutput.updatedOutput | Should -Match "111\.50\.50\.50"
        }
    }

    It "returns null when no changes needed" {
        Invoke-SanitizerTest -Name "hook-post-nochange" -Config (New-TestConfig) -Test {
            Invoke-HookPost "No sensitive data here" | Should -BeNullOrEmpty
        }
    }
}

# ============================================================================
# HOOK-SESSION-START (Project Sanitization)
# ============================================================================

Describe "hook-session-start" {
    It "sanitizes private IP ranges in files" {
        Invoke-SanitizerTest -Name "session-ips" -Config (New-TestConfig) -Test {
            param($dir)
            Write-TestFile "$dir/config.txt" "server1 = $IP_192`nserver2 = $IP_10`nserver3 = $IP_172"
            Invoke-Session
            $sanitized = Read-TestFile "$dir/config.txt"
            $sanitized | Should -Not -Match "192\.168\.|^10\.|172\.16\."
            ([regex]::Matches($sanitized, $RX_SAN)).Count | Should -Be 3
        }
    }

    It "preserves excluded IPs" {
        Invoke-SanitizerTest -Name "session-excluded" -Config (New-TestConfig) -Test {
            param($dir)
            Write-TestFile "$dir/config.txt" "localhost = $IP_LOOP`nbind = $IP_ZERO"
            Invoke-Session
            $content = Read-TestFile "$dir/config.txt"
            $content | Should -Match $IP_LOOP
            $content | Should -Match $IP_ZERO
        }
    }

    It "skips .git and .claude directories" {
        Invoke-SanitizerTest -Name "session-skip" -Config (New-TestConfig) -Test {
            param($dir)
            [System.IO.Directory]::CreateDirectory("$dir/.git") | Out-Null
            $gitContent = "secret = $IP_192"
            Write-TestFile "$dir/.git/config" $gitContent
            Write-TestFile "$dir/main.go" "ip = $IP_192_2"
            Invoke-Session
            Read-TestFile "$dir/.git/config" | Should -Be $gitContent
            Read-TestFile "$dir/main.go" | Should -Match $RX_SAN
        }
    }

    It "skips binary files" {
        Invoke-SanitizerTest -Name "session-binary" -Config (New-TestConfig) -Test {
            param($dir)
            $bytes = [byte[]]@(0x00, 0x50, 0x4B, 0x03, 0x04)
            [System.IO.File]::WriteAllBytes("$dir/archive.zip", $bytes)
            Write-TestFile "$dir/main.go" "ip = $IP_192"
            Invoke-Session
            [System.IO.File]::ReadAllBytes("$dir/archive.zip") | Should -Be $bytes
            Read-TestFile "$dir/main.go" | Should -Match $RX_SAN
        }
    }

    It "processes nested directories" {
        Invoke-SanitizerTest -Name "session-nested" -Config (New-TestConfig) -Test {
            param($dir)
            Write-TestFile "$dir/src/config/db.yaml" "host: $IP_192"
            Write-TestFile "$dir/src/app/settings.json" "{`"ip`": `"$IP_10`"}"
            Invoke-Session
            Read-TestFile "$dir/src/config/db.yaml" | Should -Match $RX_SAN
            Read-TestFile "$dir/src/app/settings.json" | Should -Match $RX_SAN
        }
    }

    It "saves discovered mappings to config" {
        Invoke-SanitizerTest -Name "session-persist" -Config (New-TestConfig) -Test {
            param($dir)
            Write-TestFile "$dir/app.txt" "server = $IP_192_99"
            Invoke-Session
            (Read-TestFile "$dir/.claude/sanitizer/sanitizer.json" | ConvertFrom-Json).mappingsAuto.PSObject.Properties.Name | Should -Contain $IP_192_99
        }
    }
}

# ============================================================================
# HOOK-SESSION-STOP (Sync to Unsanitized)
# ============================================================================

Describe "hook-session-stop" {
    It "creates unsanitized copy with real values restored" {
        Invoke-SanitizerTest -Name "session-stop" -Config (New-TestConfig) -Test {
            param($dir)
            $projectName = Split-Path $dir -Leaf
            Write-TestFile "$dir/deploy.ps1" "Connect-Server -IP `"$IP_192`""

            Invoke-Session 'hook-session-start'
            Read-TestFile "$dir/deploy.ps1" | Should -Match $RX_SAN

            $null = '{"hook_event_name":"Stop"}' | & $script:sanitizer hook-session-stop 2>&1

            "$dir/.claude/unsanitized/$projectName/deploy.ps1" | Should -Exist
            Read-TestFile "$dir/.claude/unsanitized/$projectName/deploy.ps1" | Should -Match $IP_192
        }
    }
}

# ============================================================================
# HOSTNAME PATTERNS
# ============================================================================

Describe "hostname-patterns" {
    It "sanitizes hostnames matching pattern" {
        Invoke-SanitizerTest -Name "host-basic" -Config (New-TestConfig -Patterns @("server\d{2}")) -Test {
            param($dir)
            Write-TestFile "$dir/inventory.yml" "host: server01`nbackup: server99"
            Invoke-Session
            $sanitized = Read-TestFile "$dir/inventory.yml"
            $sanitized | Should -Not -Match "server01|server99"
            $sanitized | Should -Match "host-[a-z0-9]+\.example\.test"
        }
    }

    It "matches case-insensitively and captures FQDN suffix" {
        Invoke-SanitizerTest -Name "host-fqdn" -Config (New-TestConfig -Patterns @("srv[0-9]+")) -Test {
            param($dir)
            Write-TestFile "$dir/dns.txt" "SRV01.internal.corp.local"
            Invoke-Session
            Read-TestFile "$dir/dns.txt" | Should -Not -Match "srv01|internal\.corp\.local"
        }
    }

    It "handles multiple patterns" {
        Invoke-SanitizerTest -Name "host-multi" -Config (New-TestConfig -Patterns @("web\d+", "db\d+", "app\d+")) -Test {
            param($dir)
            Write-TestFile "$dir/arch.txt" "web01 -> app01 -> db01"
            Invoke-Session
            Read-TestFile "$dir/arch.txt" | Should -Not -Match "web01|app01|db01"
        }
    }

    It "preserves values with identity mapping" {
        $config = New-TestConfig -Patterns @("Packed[A-Za-z0-9]+Array") -ManualMappings @{
            "host-hh0o0585.example.testArray"   = "host-hh0o0585.example.testArray"
            "host-3uyaeoky.example.testArray" = "host-3uyaeoky.example.testArray"
        }
        Invoke-SanitizerTest -Name "host-identity" -Config $config -Test {
            param($dir)
            Write-TestFile "$dir/types.gd" "var a: host-hh0o0585.example.testArray`nvar b: host-3uyaeoky.example.testArray"
            Invoke-Session
            $sanitized = Read-TestFile "$dir/types.gd"
            $sanitized | Should -Match "host-hh0o0585.example.testArray"
            $sanitized | Should -Match "host-3uyaeoky.example.testArray"
        }
    }

    It "is deterministic across runs" {
        Invoke-SanitizerTest -Name "host-determ" -Config (New-TestConfig -Patterns @("myhost\d+")) -Test {
            param($dir)
            $original = "connect to myhost01 and myhost02"
            Write-TestFile "$dir/test.txt" $original
            Invoke-Session
            $firstRun = Read-TestFile "$dir/test.txt"

            Write-TestFile "$dir/test.txt" $original
            Invoke-Session
            Read-TestFile "$dir/test.txt" | Should -Be $firstRun
        }
    }

    It "handles invalid regex pattern gracefully" {
        Invoke-SanitizerTest -Name "host-invalid" -Config (New-TestConfig -Patterns @("server(", "valid\d+")) -Test {
            param($dir)
            Write-TestFile "$dir/test.txt" "valid01 and server("
            Invoke-Session
            Read-TestFile "$dir/test.txt" | Should -Match "host-[a-z0-9]+\.example\.test"
        }
    }
}

# ============================================================================
# EXEC FUNCTION
# ============================================================================

Describe "exec" {
    It "runs command in unsanitized dir with real values, sanitizes output" {
        Invoke-SanitizerTest -Name "exec-basic" -Config (New-TestConfig -AutoMappings @{ $IP_192 = "111.77.77.77" }) -Test {
            param($dir)
            Write-TestFile "$dir/show-ip.ps1" "Write-Output `"IP: $IP_192`""
            Invoke-Session
            $result = & $script:sanitizer exec 'powershell -NoProfile -File show-ip.ps1' 2>&1
            $result | Should -Match "111\.77\.77\.77"
            $result | Should -Not -Match "192\.168"
        }
    }

    It "discovers new IPs from command output" {
        Invoke-SanitizerTest -Name "exec-discover" -Config (New-TestConfig) -Test {
            param($dir)
            Write-TestFile "$dir/new-ip.ps1" "Write-Output `"Found: $IP_192_88`""
            Invoke-Session
            $null = & $script:sanitizer exec 'powershell -NoProfile -File new-ip.ps1' 2>&1
            (Read-TestFile "$dir/.claude/sanitizer/sanitizer.json" | ConvertFrom-Json).mappingsAuto.PSObject.Properties.Name | Should -Contain $IP_192_88
        }
    }
}

# ============================================================================
# MANUAL MAPPINGS
# ============================================================================

Describe "manual-mappings" {
    It "takes precedence over auto mappings" {
        $config = New-TestConfig -ManualMappings @{ $IP_192 = "111.99.99.99" } -AutoMappings @{ $IP_192 = "111.11.11.11" }
        Invoke-SanitizerTest -Name "manual-precedence" -Config $config -Test {
            param($dir)
            Write-TestFile "$dir/test.txt" "server = $IP_192"
            Invoke-Session
            $sanitized = Read-TestFile "$dir/test.txt"
            $sanitized | Should -Match "111\.99\.99\.99"
            $sanitized | Should -Not -Match "111\.11\.11\.11"
        }
    }

    It "supports custom string replacement" {
        Invoke-SanitizerTest -Name "manual-custom" -Config (New-TestConfig -ManualMappings @{ 'C:\Users\realuser' = 'C:\Users\testuser' }) -Test {
            param($dir)
            Write-TestFile "$dir/config.txt" 'path = C:\Users\realuser\data'
            Invoke-Session
            Read-TestFile "$dir/config.txt" | Should -Match 'C:\\Users\\testuser\\data'
        }
    }
}

# ============================================================================
# TEXT TRANSFORMATION
# ============================================================================

Describe "text-transformation" {
    It "replaces longest keys first to avoid partial matches" {
        # Two IPs where one is prefix of the other - longer must be replaced first
        $short = "10.0.0.1"
        $long  = "10.0.0.100"
        $config = New-TestConfig -AutoMappings @{ $short = "111.1.1.1"; $long = "111.1.1.100" }
        Invoke-SanitizerTest -Name "text-longest" -Config $config -Test {
            param($dir)
            Write-TestFile "$dir/test.txt" "short: $short`nlong: $long"
            Invoke-Session
            $sanitized = Read-TestFile "$dir/test.txt"
            $sanitized | Should -Match "short: 111\.1\.1\.1"
            $sanitized | Should -Match "long: 111\.1\.1\.100"
            $sanitized | Should -Not -Match "10\.0\.0"
        }
    }
}

# ============================================================================
# FILE HANDLING
# ============================================================================

Describe "file-handling" {
    It "detects null bytes as binary and skips" {
        Invoke-SanitizerTest -Name "file-nullbyte" -Config (New-TestConfig) -Test {
            param($dir)
            $bytes = [System.Text.Encoding]::UTF8.GetBytes("text") + [byte]0x00 + [System.Text.Encoding]::UTF8.GetBytes($IP_192)
            [System.IO.File]::WriteAllBytes("$dir/mixed.bin", $bytes)
            Write-TestFile "$dir/clean.txt" $IP_192_2
            Invoke-Session
            [System.IO.File]::ReadAllBytes("$dir/mixed.bin") | Should -Be $bytes
            Read-TestFile "$dir/clean.txt" | Should -Match $RX_SAN
        }
    }

    It "skips files larger than 10MB" {
        Invoke-SanitizerTest -Name "file-large" -Config (New-TestConfig) -Test {
            param($dir)
            [System.IO.File]::WriteAllText("$dir/large.txt", ("$IP_192`n" * 900000))
            Write-TestFile "$dir/small.txt" $IP_192_2
            Invoke-Session
            # Large file should still have original IP (not sanitized)
            [System.IO.File]::ReadLines("$dir/large.txt") | Select-Object -First 1 | Should -Match $IP_192
            # Small file should be sanitized
            Read-TestFile "$dir/small.txt" | Should -Match $RX_SAN
        }
    }

    It "skips configured paths like node_modules" {
        Invoke-SanitizerTest -Name "file-skip" -Config (New-TestConfig -SkipPaths @(".git", "node_modules", "vendor")) -Test {
            param($dir)
            [System.IO.Directory]::CreateDirectory("$dir/node_modules/pkg") | Out-Null
            [System.IO.Directory]::CreateDirectory("$dir/src/lib/vendor/pkg") | Out-Null
            $originalContent = "ip = $IP_192"
            Write-TestFile "$dir/node_modules/pkg/index.js" $originalContent
            Write-TestFile "$dir/src/lib/vendor/pkg/file.go" $originalContent
            Write-TestFile "$dir/app.js" "ip = $IP_192_2"
            Invoke-Session
            Read-TestFile "$dir/node_modules/pkg/index.js" | Should -Be $originalContent
            Read-TestFile "$dir/src/lib/vendor/pkg/file.go" | Should -Be $originalContent
            Read-TestFile "$dir/app.js" | Should -Match $RX_SAN
        }
    }
}

# ============================================================================
# CONFIG HANDLING
# ============================================================================

Describe "config-handling" {
    It "creates default config if missing" {
        Invoke-SanitizerTest -Name "config-default" -Config $null -Test {
            param($dir)
            Remove-Item "$dir/.claude/sanitizer/sanitizer.json" -ErrorAction SilentlyContinue
            Write-TestFile "$dir/test.txt" $IP_192
            Invoke-Session
            "$dir/.claude/sanitizer/sanitizer.json" | Should -Exist
        }
    }

    It "handles UTF-8 BOM in config" {
        Invoke-SanitizerTest -Name "config-bom" -Config $null -Test {
            param($dir)
            $config = New-TestConfig -Patterns @("test\d+")
            $bom = [byte[]]@(0xEF, 0xBB, 0xBF)
            [System.IO.File]::WriteAllBytes("$dir/.claude/sanitizer/sanitizer.json", $bom + [System.Text.Encoding]::UTF8.GetBytes($config))
            Write-TestFile "$dir/test.txt" "test01"
            Invoke-Session
            Read-TestFile "$dir/test.txt" | Should -Match "host-[a-z0-9]+\.example\.test"
        }
    }
}

# ============================================================================
# REGRESSION TESTS
# ============================================================================

Describe "regression-tests" {
    It "hostname charset generates valid hostnames (no dots in random part)" {
        Invoke-SanitizerTest -Name "reg-charset" -Config (New-TestConfig -Patterns @("testhost\d+")) -Test {
            param($dir)
            Write-TestFile "$dir/test.txt" "testhost01"
            Invoke-Session
            Read-TestFile "$dir/test.txt" | Should -Match "^host-[a-z0-9]{8}\.example\.test$"
        }
    }

    It "config mappings preserve original values as keys" {
        Invoke-SanitizerTest -Name "reg-configkeys" -Config (New-TestConfig -Patterns @("myserver\d+")) -Test {
            param($dir)
            Write-TestFile "$dir/test.txt" "myserver01"
            Invoke-Session
            $config = Read-TestFile "$dir/.claude/sanitizer/sanitizer.json" | ConvertFrom-Json
            $config.mappingsAuto.PSObject.Properties.Name | Should -Contain "myserver01"
            $config.mappingsAuto.PSObject.Properties.Name | Should -Not -Match "host-.*\.example\.test"
        }
    }
}
