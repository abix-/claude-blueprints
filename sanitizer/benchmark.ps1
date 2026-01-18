Import-Module C:/Users/Abix/.claude/sanitizer/Sanitizer.psm1 -Force

$testData = @'
Server 11.100.201.234 connected
Database at 11.178.40.57 responding
Gateway 11.112.65.63 routing to 11.63.167.174
Backup server 11.100.166.98 online
'@

# Warm up
$null = ConvertTo-ScrubbedText -Text $testData
$null = $testData | C:/code/claude-blueprints/sanitizer/sanitizer.exe sanitize-ips

# Benchmark PowerShell (10 iterations)
$ps = Measure-Command { 1..10 | ForEach-Object { ConvertTo-ScrubbedText -Text $testData } }

# Benchmark Go (10 iterations)
$go = Measure-Command { 1..10 | ForEach-Object { $testData | C:/code/claude-blueprints/sanitizer/sanitizer.exe sanitize-ips } }

Write-Host "PowerShell: $([math]::Round($ps.TotalMilliseconds))ms (10 runs)"
Write-Host "Go:         $([math]::Round($go.TotalMilliseconds))ms (10 runs)"
Write-Host "Ratio:      $([math]::Round($ps.TotalMilliseconds / $go.TotalMilliseconds, 1))x faster"
