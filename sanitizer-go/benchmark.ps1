Import-Module C:/Users/Abix/.claude/sanitizer/Sanitizer.psm1 -Force

$testData = @'
Server 192.168.1.100 connected
Database at 10.0.0.50 responding
Gateway 172.16.0.1 routing to 10.20.30.40
Backup server 192.168.100.200 online
'@

# Warm up
$null = ConvertTo-ScrubbedText -Text $testData
$null = $testData | C:/code/claude-blueprints/sanitizer-go/sanitizer.exe scrub-ips

# Benchmark PowerShell (10 iterations)
$ps = Measure-Command { 1..10 | ForEach-Object { ConvertTo-ScrubbedText -Text $testData } }

# Benchmark Go (10 iterations)
$go = Measure-Command { 1..10 | ForEach-Object { $testData | C:/code/claude-blueprints/sanitizer-go/sanitizer.exe scrub-ips } }

Write-Host "PowerShell: $([math]::Round($ps.TotalMilliseconds))ms (10 runs)"
Write-Host "Go:         $([math]::Round($go.TotalMilliseconds))ms (10 runs)"
Write-Host "Ratio:      $([math]::Round($ps.TotalMilliseconds / $go.TotalMilliseconds, 1))x faster"
