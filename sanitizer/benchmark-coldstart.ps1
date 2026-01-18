$testData = @'
Server 111.148.25.233 connected to 111.83.13.238
'@

# Benchmark PowerShell cold start (spawn powershell + load module + run)
$ps = Measure-Command {
    1..5 | ForEach-Object {
        $testData | powershell.exe -NoProfile -Command "Import-Module C:/Users/Abix/.claude/sanitizer/Sanitizer.psm1; ConvertTo-ScrubbedText -Text ([Console]::In.ReadToEnd())"
    }
}

# Benchmark Go cold start (spawn binary)
$go = Measure-Command {
    1..5 | ForEach-Object {
        $testData | C:/code/claude-blueprints/sanitizer/sanitizer.exe sanitize-ips
    }
}

Write-Host "PowerShell cold start: $([math]::Round($ps.TotalMilliseconds))ms (5 runs)"
Write-Host "Go cold start:         $([math]::Round($go.TotalMilliseconds))ms (5 runs)"
Write-Host "Go is $([math]::Round($ps.TotalMilliseconds / $go.TotalMilliseconds, 1))x faster"
