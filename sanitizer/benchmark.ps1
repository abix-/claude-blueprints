Import-Module C:/Users/Abix/.claude/sanitizer/Sanitizer.psm1 -Force

$testData = @'
Server 111.148.25.233 connected
Database at 111.83.13.238 responding
Gateway 111.42.149.75 routing to 111.54.21.207
Backup server 111.129.88.154 online
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
