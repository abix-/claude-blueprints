#Requires -Modules Pester
<#
.SYNOPSIS
    Pester tests for sanitizer CLI tool.
.DESCRIPTION
    Run with: Invoke-Pester -Path ./sanitizer.tests.ps1
#>

BeforeAll {
    $script:sanitizer = "$PSScriptRoot/sanitizer.exe"

    # Build sanitizer
    Push-Location $PSScriptRoot
    go build -o sanitizer.exe ./cmd/sanitizer
    if ($LASTEXITCODE -ne 0) { throw "Build failed" }
    Pop-Location

    # Helper to create test config
    function New-TestConfig {
        param(
            [string[]]$Patterns = @(),
            [hashtable]$ManualMappings = @{}
        )
        @{
            hostnamePatterns = $Patterns
            mappingsAuto     = @{}
            mappingsManual   = $ManualMappings
            skipPaths        = @(".git")
            unsanitizedPath  = "~/.claude/unsanitized/{project}"
            blockedPaths     = @()
        } | ConvertTo-Json -Depth 10
    }

    # Helper to create isolated test environment with custom USERPROFILE
    function New-TestEnvironment {
        param([string]$Name, [string]$Config)

        $testDir = "$env:TEMP/sanitizer-$Name-$([guid]::NewGuid().ToString('N').Substring(0,8))"
        if (Test-Path $testDir) { Remove-Item $testDir -Recurse -Force }
        New-Item -ItemType Directory -Path "$testDir/.claude/sanitizer" -Force | Out-Null

        if ($Config) {
            Set-Content "$testDir/.claude/sanitizer/sanitizer.json" $Config
        }

        return $testDir
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
}

Describe "sanitize-ips" {
    It "replaces private IPs with 111.x.x.x range" {
        $result = "Server at 111.36.128.123 and 111.4.212.124" | & $sanitizer sanitize-ips

        $result | Should -Not -Match "192\.168\."
        $result | Should -Not -Match "10\.0\.0\."
        $result | Should -Match "111\.\d+\.\d+\.\d+"
    }

    It "produces deterministic output for same input" {
        $r1 = "111.36.128.123" | & $sanitizer sanitize-ips
        $r2 = "111.36.128.123" | & $sanitizer sanitize-ips

        $r1 | Should -Be $r2
        $r1 | Should -Not -Match "192\.168\."
    }

    It "preserves excluded IPs (loopback, broadcast)" {
        $result = "localhost 127.0.0.1 and broadcast 255.255.255.255" | & $sanitizer sanitize-ips

        $result | Should -Match "127\.0\.0\.1"
        $result | Should -Match "255\.255\.255\.255"
    }
}

Describe "hook-bash" {
    Context "BLOCK decisions" {
        It "blocks access to sanitizer.json" {
            $input = '{"hook_event_name":"PreToolUse","tool_input":{"command":"cat ~/.claude/sanitizer/sanitizer.json"}}'
            $result = $input | & $sanitizer hook-bash | ConvertFrom-Json

            $result.hookSpecificOutput.permissionDecision | Should -Be "deny"
        }

        It "blocks access to unsanitized directory" {
            $input = '{"hook_event_name":"PreToolUse","tool_input":{"command":"ls ~/.claude/unsanitized/"}}'
            $result = $input | & $sanitizer hook-bash | ConvertFrom-Json

            $result.hookSpecificOutput.permissionDecision | Should -Be "deny"
        }
    }

    Context "SANITIZED decisions (allow as-is)" {
        It "allows ls command without modification" {
            $input = '{"hook_event_name":"PreToolUse","tool_input":{"command":"ls -la"}}'
            $result = $input | & $sanitizer hook-bash

            $result | Should -BeNullOrEmpty
        }

        It "allows git command without modification" {
            $input = '{"hook_event_name":"PreToolUse","tool_input":{"command":"git status"}}'
            $result = $input | & $sanitizer hook-bash

            $result | Should -BeNullOrEmpty
        }
    }

    Context "UNSANITIZED decisions (wrap command)" {
        It "wraps powershell commands for unsanitized execution" {
            $input = '{"hook_event_name":"PreToolUse","tool_input":{"command":"powershell.exe -Command Get-Date"}}'
            $result = $input | & $sanitizer hook-bash | ConvertFrom-Json

            $result.hookSpecificOutput.updatedInput.command | Should -Not -BeNullOrEmpty
        }
    }
}

Describe "hook-file-access" {
    Context "BLOCK decisions" {
        It "blocks read of sanitizer.json" {
            $input = '{"hook_event_name":"PreToolUse","tool_name":"Read","tool_input":{"file_path":"C:/Users/Test/.claude/sanitizer/sanitizer.json"}}'
            $result = $input | & $sanitizer hook-file-access | ConvertFrom-Json

            $result.hookSpecificOutput.permissionDecision | Should -Be "deny"
        }

        It "blocks edit of unsanitized path" {
            $input = '{"hook_event_name":"PreToolUse","tool_name":"Edit","tool_input":{"file_path":"C:/Users/Test/.claude/unsanitized/project/file.txt"}}'
            $result = $input | & $sanitizer hook-file-access | ConvertFrom-Json

            $result.hookSpecificOutput.permissionDecision | Should -Be "deny"
        }
    }

    Context "ALLOW decisions" {
        It "allows access to normal project files" {
            $input = '{"hook_event_name":"PreToolUse","tool_name":"Read","tool_input":{"file_path":"C:/code/project/main.go"}}'
            $result = $input | & $sanitizer hook-file-access

            $result | Should -BeNullOrEmpty
        }
    }
}

Describe "hook-session-start" {
    It "sanitizes IPs in project files" {
        $testDir = New-TestEnvironment -Name "session-ips"
        try {
            Set-Content "$testDir/config.txt" @"
# Config file
server = 111.36.128.123
backup = 111.4.212.124
gateway = 111.167.157.20
"@
            Invoke-SanitizerSession -TestDir $testDir

            $sanitized = Get-Content "$testDir/config.txt" -Raw
            $sanitized | Should -Not -Match "192\.168\.50\.100"
            $sanitized | Should -Match "111\.\d+\.\d+\.\d+"
        }
        finally {
            Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

Describe "hook-session-stop" {
    It "syncs sanitized files to unsanitized directory with restored values" {
        $testDir = New-TestEnvironment -Name "session-stop"
        $projectName = Split-Path $testDir -Leaf

        try {
            # Use .NET to write file directly, bypassing any hooks
            [System.IO.File]::WriteAllText("$testDir/test.txt", "server = 192.168.1.50")

            # Session start sanitizes and saves mapping
            Invoke-SanitizerSession -TestDir $testDir -Command 'hook-session-start'
            $sanitized = [System.IO.File]::ReadAllText("$testDir/test.txt")
            $sanitized | Should -Match "111\.\d+\.\d+\.\d+"

            # Session stop restores to unsanitized
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
            $unsanitizedFile = "$testDir/.claude/unsanitized/$projectName/test.txt"
            $unsanitizedFile | Should -Exist
            [System.IO.File]::ReadAllText($unsanitizedFile) | Should -Match "192\.168\.1\.50"
        }
        finally {
            Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

Describe "hostname-patterns" {
    It "sanitizes hostnames matching pattern" {
        $testDir = New-TestEnvironment -Name "hostname-basic" -Config (New-TestConfig -Patterns @("server\d{2}"))
        try {
            Set-Content "$testDir/config.txt" "Connecting to server01 and server99"
            Invoke-SanitizerSession -TestDir $testDir

            $sanitized = Get-Content "$testDir/config.txt" -Raw
            $sanitized | Should -Not -Match "server01"
            $sanitized | Should -Not -Match "server99"
            $sanitized | Should -Match "host-.*\.example\.test"
        }
        finally {
            Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    It "matches hostnames case-insensitively" {
        $testDir = New-TestEnvironment -Name "hostname-case" -Config (New-TestConfig -Patterns @("prodweb\d+"))
        try {
            Set-Content "$testDir/config.txt" "Servers: host-4x14bsel.example.test, host-0309skeo.example.test, host-a3plihqn.example.test"
            Invoke-SanitizerSession -TestDir $testDir

            $sanitized = Get-Content "$testDir/config.txt" -Raw
            $sanitized | Should -Not -Match "(?i)prodweb"
            ([regex]::Matches($sanitized, "host-.*?\.example\.test")).Count | Should -BeGreaterOrEqual 3
        }
        finally {
            Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    It "sanitizes hostname with domain suffix (FQDN)" {
        $testDir = New-TestEnvironment -Name "hostname-fqdn" -Config (New-TestConfig -Patterns @("srv[0-9]+"))
        try {
            Set-Content "$testDir/config.txt" "Host: srv01.internal.corp"
            Invoke-SanitizerSession -TestDir $testDir

            $sanitized = Get-Content "$testDir/config.txt" -Raw
            $sanitized | Should -Not -Match "srv01"
            $sanitized | Should -Not -Match "internal\.corp"
        }
        finally {
            Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    It "handles multiple hostname patterns" {
        $testDir = New-TestEnvironment -Name "hostname-multi" -Config (New-TestConfig -Patterns @("web\d+", "db\d+", "app\d+"))
        try {
            Set-Content "$testDir/config.txt" "web01 connects to db01, app01 is frontend"
            Invoke-SanitizerSession -TestDir $testDir

            $sanitized = Get-Content "$testDir/config.txt" -Raw
            $sanitized | Should -Not -Match "web01"
            $sanitized | Should -Not -Match "db01"
            $sanitized | Should -Not -Match "app01"
            ([regex]::Matches($sanitized, "host-.*?\.example\.test")).Count | Should -BeGreaterOrEqual 3
        }
        finally {
            Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    It "preserves values with identity mappings" {
        $config = New-TestConfig -Patterns @("Packed[A-Za-z0-9]+Array") -ManualMappings @{
            "host-hh0o0585.example.testArray" = "host-hh0o0585.example.testArray"
        }
        $testDir = New-TestEnvironment -Name "hostname-identity" -Config $config
        try {
            Set-Content "$testDir/config.txt" "var arr: host-hh0o0585.example.testArray = host-hh0o0585.example.testArray()"
            Invoke-SanitizerSession -TestDir $testDir

            $sanitized = Get-Content "$testDir/config.txt" -Raw
            $sanitized | Should -Match "host-hh0o0585.example.testArray"
        }
        finally {
            Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    It "leaves non-matching text unchanged" {
        $testDir = New-TestEnvironment -Name "hostname-nomatch" -Config (New-TestConfig -Patterns @("server\d{2}"))
        try {
            $original = "Hello world, this has no hostnames"
            Set-Content "$testDir/config.txt" $original
            Invoke-SanitizerSession -TestDir $testDir

            $sanitized = (Get-Content "$testDir/config.txt" -Raw).Trim()
            $sanitized | Should -Be $original
        }
        finally {
            Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    It "produces deterministic output across runs" {
        $testDir = New-TestEnvironment -Name "hostname-deterministic" -Config (New-TestConfig -Patterns @("myhost\d+"))
        try {
            $original = "Connect to myhost01"
            Set-Content "$testDir/config.txt" $original

            # First run
            Invoke-SanitizerSession -TestDir $testDir
            $firstRun = (Get-Content "$testDir/config.txt" -Raw).Trim()

            # Reset file, keep config with saved mappings
            Set-Content "$testDir/config.txt" $original

            # Second run
            Invoke-SanitizerSession -TestDir $testDir
            $secondRun = (Get-Content "$testDir/config.txt" -Raw).Trim()

            $firstRun | Should -Be $secondRun
        }
        finally {
            Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    It "sanitizes both hostnames and IPs in same file" {
        $testDir = New-TestEnvironment -Name "hostname-mixed" -Config (New-TestConfig -Patterns @("dbserver\d+"))
        try {
            Set-Content "$testDir/config.txt" "dbserver01 at 111.64.135.196, backup at 111.210.246.198"
            Invoke-SanitizerSession -TestDir $testDir

            $sanitized = Get-Content "$testDir/config.txt" -Raw
            $sanitized | Should -Not -Match "dbserver01"
            $sanitized | Should -Not -Match "192\.168\.1\.100"
            $sanitized | Should -Not -Match "10\.0\.0\.50"
            $sanitized | Should -Match "host-.*\.example\.test"
            $sanitized | Should -Match "111\.\d+\.\d+\.\d+"
        }
        finally {
            Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}
