---
name: "debloat"
description: "Strip Windows of junk services, AppX packages, scheduled tasks, startup entries, and telemetry. Use when the user wants to clean up Windows, disable bloatware, or optimize for performance."
---
# Windows Debloat

Audit and remove Windows bloat. Run as admin PowerShell via scripts (bash mangles `$_`).

## Workflow

1. **Audit**. Scan for junk in each category
2. **Present**. Show the user what to kill vs keep, with brief explanations
3. **Confirm**. Get approval before disabling
4. **Verify**. Confirm each target is dead

## PowerShell via Bash

Bash mangles `$_`, `$p`, and other PS variables. ALWAYS write `.ps1` scripts to `C:\code\endless\` and run via:
```
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "C:\code\endless\scriptname.ps1"
```
Delete scripts after use. Never use `$PID` (reserved). Use `$procId` instead.

## Categories

### 1. Services
```powershell
# Audit running services
Get-Service | Where-Object Status -eq Running | Select-Object Name, DisplayName, StartType | Sort-Object DisplayName
```

**Disable method**. Registry is authoritative, `Set-Service` often doesn't stick:
```powershell
Set-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Services\$svc" -Name 'Start' -Value 4
Stop-Service -Name $svc -Force -ErrorAction SilentlyContinue
```

**Per-user services** (ending in `_xxxxx`): disable BOTH the instance AND the template:
```powershell
# Template (prevents respawn on new sessions)
Set-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Services\OneSyncSvc" -Name 'Start' -Value 4
# Instance
Set-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Services\OneSyncSvc_89f2f" -Name 'Start' -Value 4
```

**Stop with timeout**. Some services hang on stop:
```powershell
$s = Get-Service -Name $svc -ErrorAction SilentlyContinue
if ($s -and $s.Status -eq 'Running') {
    try {
        $s.Stop()
        $s.WaitForStatus('Stopped', [TimeSpan]::FromSeconds(5))
    } catch { }
}
```

**Known safe-to-disable services:**
- DiagTrack (telemetry)
- jhi_service, LMS, WMIRegistrationService (Intel ME)
- GamingServices, GamingServicesNet, GameInputRedistService, GameInputSvc (Xbox)
- PhoneSvc (Phone Link)
- lfsvc (geolocation)
- RmSvc (radio management)
- SEMgrSvc (NFC payments)
- WbioSrvc (biometric. Unless fingerprint reader)
- TabletInputService (touch keyboard. Desktop)
- SSDPSRV (UPnP discovery)
- iphlpsvc (IPv6 transition)
- DusmSvc (data usage tracking)
- WerSvc (error reporting telemetry)
- DPS, WdiServiceHost, WdiSystemHost (diagnostics/troubleshooter)
- DsSvc (data sharing / Timeline)
- TrkWks (distributed link tracking)
- PcaSvc (compatibility assistant nags)
- CDPSvc + CDPUserSvc (cross-device platform)
- DevicesFlowUserSvc (device discovery UI)
- OneSyncSvc (Microsoft account sync)
- PimIndexMaintenanceSvc (contact indexing)
- UserDataSvc, UnistoreSvc (Mail/Calendar backend)
- WpnService + WpnUserService (Store push notifications)
- DoSvc (P2P update delivery)
- cbdhsvc (clipboard history. If unused)
- SysMain (Superfetch. Debatable on SSD, re-enable if cold starts slow)
- LanmanServer (SMB sharing. Unless sharing folders)
- ShellHWDetection (autoplay)
- TokenBroker (web account manager for Store apps)
- WinHttpAutoProxySvc (WPAD proxy detection)
- seclogon (secondary logon / runas)
- WebClient (WebDAV)
- QWAVE (QoS streaming)
- RasMan, SstpSvc (VPN client. Unless using Windows VPN)
- ClickToRunSvc (Office updates)
- InstallService (Store app installs)
- OptionsPlusUpdaterService (Logi updater)

**GamingServices special case:** backed by AppX package. `Set-Service` gets overridden. Must remove package:
```powershell
Get-AppxPackage -AllUsers -Name 'Microsoft.GamingServices' | Remove-AppxPackage -AllUsers
```

### 2. AppX Packages
```powershell
# Audit
Get-AppxPackage | Select-Object Name | Sort-Object Name
```

**Remove method**. Deprovision + per-user removal (works even with InstallService disabled):
```powershell
Get-AppxProvisionedPackage -Online | Where-Object DisplayName -eq $pkg | Remove-AppxProvisionedPackage -Online -ErrorAction SilentlyContinue
Get-AppxPackage -Name $pkg | Remove-AppxPackage -ErrorAction SilentlyContinue
```

If `-AllUsers` fails with 0x80070002, use the above pattern instead.

**SystemApps** (0x80073CFA) can't be removed via AppX. Use binary rename:
```powershell
takeown /f "$exePath" /a
icacls "$exePath" /grant Administrators:F
Rename-Item "$exePath" "$exeName.disabled"
```
Note: this does NOT work in `C:\Program Files\WindowsApps\` (integrity level lock). Only works in `C:\Windows\SystemApps\`.

**Known safe-to-remove packages:**
- Microsoft.549981C3F5F10 (Cortana)
- Microsoft.BingSearch, Microsoft.BingWeather
- Microsoft.Copilot
- Microsoft.GetHelp, Microsoft.Getstarted
- Microsoft.Microsoft3DViewer
- Microsoft.MicrosoftOfficeHub
- Microsoft.MicrosoftSolitaireCollection
- Microsoft.MicrosoftStickyNotes
- Microsoft.Office.OneNote (Store version)
- Microsoft.OutlookForWindows
- Microsoft.People
- Microsoft.Wallet
- Microsoft.WindowsAlarms, Microsoft.WindowsCamera
- microsoft.windowscommunicationsapps (Mail & Calendar)
- Microsoft.WindowsFeedbackHub, Microsoft.WindowsMaps
- Microsoft.XboxApp, Microsoft.XboxGameOverlay, Microsoft.XboxGamingOverlay
- Microsoft.XboxIdentityProvider, Microsoft.XboxSpeechToTextOverlay
- MicrosoftWindows.CrossDevice
- Microsoft.Windows.DevHome
- Microsoft.YourPhone

**Keep:** Claude, WindowsTerminal, WindowsStore, DesktopAppInstaller, Winget.Source, WindowsCalculator, ScreenSketch, MSPaint, Photos, NVIDIA, Realtek, Edge, all runtimes (.NET, VCLibs, UI.Xaml, WindowsAppRuntime, DirectX)

### 3. Scheduled Tasks
```powershell
# Audit
Get-ScheduledTask | Where-Object State -eq Ready | Select-Object TaskName, TaskPath | Sort-Object TaskPath
```

**Known safe-to-disable:**
- Office Feature Updates, Office Feature Updates Logon
- OobeDiscovery
- Any Logi/Logitech updater tasks
- \Mozilla\ Firefox Background Update (user), Firefox Default Browser Agent (admin)
- \GoogleUserPEH\ RunPlatformExperienceHelper_Daily, _Metrics (Chrome telemetry)
- \Microsoft\Windows\Application Experience\ PcaPatchDbTask, PcaWallpaperAppDetect, StartupAppTask, MareBackup (compat telemetry)
- \Microsoft\Windows\Device Information\ Device, Device User (hardware census)
- \Microsoft\Windows\Flighting\FeatureConfig\ UsageDataReporting, UsageDataFlushing, ReconcileFeatures; \Flighting\OneSettings\ RefreshCache
- \Microsoft\Windows\Power Efficiency Diagnostics\ AnalyzeSystem
- \Microsoft\Windows\Diagnosis\ Scheduled, RecommendedTroubleshootingScanner
- \Microsoft\Windows\ConsentUX\UnifiedConsent\ UnifiedConsentSyncTask
- \Microsoft\Windows\CloudRestore\ Backup; \AppListBackup\ Backup, BackupNonMaintenance
- \Microsoft\Windows\Shell\ ThemesSyncedImageDownload, FamilySafetyMonitor, FamilySafetyRefreshTask
- \Microsoft\Windows\InstallService\ ScanForUpdates, ScanForUpdatesAsUser
- \Microsoft\Windows\Windows Media Sharing\ UpdateLibrary
- \Microsoft\Windows\Maintenance\ WinSAT
- \Microsoft\Windows\Sysmain\ WsSwapAssessmentTask, ResPriStaticDbSync
- \Microsoft\Windows\Work Folders\ Work Folders Logon Synchronization, Work Folders Maintenance Work

**Cannot disable even as admin (TrustedInstaller):** \Microsoft\Windows\SettingSync\ BackgroundUploadTask.
Neuter it by turning off Settings > Accounts > Sync your settings.

**Keep:** Defrag ScheduledDefrag (TRIM on SSD), Chkdsk ProactiveScan, SystemRestore SR,
Time Synchronization, WindowsUpdate Scheduled Start, .NET NGEN, DiskCleanup SilentCleanup,
Registry RegIdleBackup, Servicing StartComponentCleanup.

Revert any task: `Enable-ScheduledTask -TaskPath '<path>' -TaskName '<name>'` (admin).

UpdateOrchestrator tasks are TrustedInstaller-protected. Disable via registry workaround or accept they're neutered once their parent services/apps are removed.

### 4. Startup / Registry
```powershell
# HKLM Run (all users)
Get-ItemProperty -Path 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Run'
# HKCU Run (current user)
Get-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
```

Remove entries with `Remove-ItemProperty`.

### 5. Telemetry Registry
```powershell
# Office telemetry
New-Item -Path "HKCU:\Software\Microsoft\Office\16.0\Common\Feedback" -Force
Set-ItemProperty -Path "HKCU:\Software\Microsoft\Office\16.0\Common\Feedback" -Name "Enabled" -Value 0 -Type DWord
Set-ItemProperty -Path "HKCU:\Software\Microsoft\Office\16.0\Common\Feedback" -Name "SurveyEnabled" -Value 0 -Type DWord

# ContentDeliveryManager (Start menu ads, silent installs)
Set-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager" -Name "SilentInstalledAppsEnabled" -Value 0 -Type DWord
Set-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager" -Name "SystemPaneSuggestionsEnabled" -Value 0 -Type DWord
Set-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager" -Name "SubscribedContent-338389Enabled" -Value 0 -Type DWord
Set-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager" -Name "SubscribedContent-310093Enabled" -Value 0 -Type DWord
Set-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager" -Name "SubscribedContent-338388Enabled" -Value 0 -Type DWord

# Store auto-updates
New-Item -Path "HKLM:\SOFTWARE\Policies\Microsoft\WindowsStore" -Force
Set-ItemProperty -Path "HKLM:\SOFTWARE\Policies\Microsoft\WindowsStore" -Name "AutoDownload" -Value 2 -Type DWord
```

### 6. Zombie Cleanup
After disabling services, some svchost processes stay alive until reboot. Map and kill them:
```powershell
# Map svchost to services
$allSvcs = Get-CimInstance Win32_Service
Get-Process -Name svchost | Sort-Object WorkingSet64 -Descending | ForEach-Object {
    $procId = $_.Id
    $mb = [math]::Round($_.WorkingSet64 / 1MB, 1)
    $svcs = ($allSvcs | Where-Object { $_.ProcessId -eq $procId } | Select-Object -ExpandProperty Name) -join ', '
    if (-not $svcs) { $svcs = '(no service mapped)' }
    Write-Host ("{0,6} MB  PID {1,6}  {2}" -f $mb, $procId, $svcs)
}
```
Kill zombie PIDs with `Stop-Process -Id $procId -Force`.

### 7. Windows Defender (real-time off, stays off)

The Windows Security toggle is temporary and Tamper Protection undoes scripted
changes. To make it stick: Tamper Protection OFF first (Windows Security >
Virus & threat protection > Manage settings), then Set-MpPreference AND the
policy keys. The policy keys are what survive reboots (same keys WinUtil and
O&O ShutUp10 write). Keep Tamper Protection off or it comes back.

```powershell
Set-MpPreference -DisableRealtimeMonitoring $true
Set-MpPreference -DisableBehaviorMonitoring $true
Set-MpPreference -DisableIOAVProtection $true
Set-MpPreference -DisableScriptScanning $true
Set-MpPreference -DisableCatchupQuickScan $true
Set-MpPreference -DisableCatchupFullScan $true
Set-MpPreference -ScanScheduleDay 8        # 8 = never
Set-MpPreference -RemediationScheduleDay 8
Set-MpPreference -MAPSReporting 0
Set-MpPreference -SubmitSamplesConsent 2
$pol = 'HKLM:\SOFTWARE\Policies\Microsoft\Windows Defender'
New-Item -Path "$pol\Real-Time Protection" -Force
Set-ItemProperty -Path $pol -Name 'DisableAntiSpyware' -Value 1 -Type DWord
Set-ItemProperty -Path $pol -Name 'DisableAntiVirus' -Value 1 -Type DWord
foreach ($n in 'DisableRealtimeMonitoring','DisableBehaviorMonitoring','DisableOnAccessProtection','DisableScanOnRealtimeEnable','DisableIOAVProtection') {
    Set-ItemProperty -Path "$pol\Real-Time Protection" -Name $n -Value 1 -Type DWord
}
Get-ScheduledTask -TaskPath '\Microsoft\Windows\Windows Defender\' | Disable-ScheduledTask
```

Verify: `Get-MpComputerStatus` shows RealTimeProtectionEnabled, BehaviorMonitorEnabled,
OnAccessProtectionEnabled all False. MsMpEng.exe keeps running until reboot;
DisableAntiSpyware stops it loading at next boot.

**Revert Defender:**
```powershell
Remove-Item -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows Defender' -Recurse -Force
Set-MpPreference -DisableRealtimeMonitoring $false
Set-MpPreference -DisableBehaviorMonitoring $false
Set-MpPreference -DisableIOAVProtection $false
Set-MpPreference -DisableScriptScanning $false
Set-MpPreference -DisableCatchupQuickScan $false
Set-MpPreference -DisableCatchupFullScan $false
Set-MpPreference -ScanScheduleDay 0
Set-MpPreference -RemediationScheduleDay 0
Get-ScheduledTask -TaskPath '\Microsoft\Windows\Windows Defender\' | Enable-ScheduledTask
```
Then reboot and turn Tamper Protection back on in Windows Security.

### 8. OneDrive gotcha

`OneDriveSetup.exe /uninstall` run from a non-elevated shell did NOT uninstall
on 2026-09-14. It reinstalled into a `<version>_1` folder and relaunched
OneDrive with `/versionReinstalledUseForTraceOnly`. Check
`Get-Process OneDrive*` after running it. Uninstall via Apps and Features or
`winget uninstall Microsoft.OneDrive` instead.

## Session log

### 2026-09-14 (this PC, i7-9700K, Win10 Home)

Symptom: PC felt slow. Cause found: MsMpEng.exe heavy read IO (Defender
scanning a 2 TB disk at 23 GB free). Services and AppX were already lean.

Done, with revert for each:

| Change | Revert |
|---|---|
| TeamViewer service disabled (by operator, services.msc) | services.msc, set TeamViewer to Automatic and start it |
| Bonjour Service: registry Start=4, stopped | `Set-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Services\Bonjour Service' -Name Start -Value 2; Start-Service 'Bonjour Service'` |
| UserOOBEBroker killed; HKCU UserProfileEngagement ScoobeSystemSettingEnabled=0; ContentDeliveryManager SilentInstalledAppsEnabled, SystemPaneSuggestionsEnabled, SubscribedContent-338389/310093/338388 = 0 | set those values back to 1 |
| cus.exe (CHERRY Utility Software) killed; Run entry removed from `HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run` | `Set-ItemProperty -Path 'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run' -Name 'CHERRY Utility Software' -Value '"C:\Program Files (x86)\CHERRY Utility Software\cus.exe" --minimized'` |
| OneDrive killed; HKCU Run OneDrive entry removed; 3 OneDrive scheduled tasks disabled | `Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name OneDrive -Value '"C:\Users\Abix\AppData\Local\Microsoft\OneDrive\OneDrive.exe" /background'; Get-ScheduledTask | Where-Object TaskName -like 'OneDrive*' | Enable-ScheduledTask` |
| FileCoAuth.exe killed | relaunches itself |
| Defender real-time off via section 7 (Tamper Protection was already off) | section 7 revert |
| OneDrive uninstalled with `winget uninstall Microsoft.OneDrive` (winget printed exit code 2147747483 but exe, uninstall entry and processes were all gone) | `winget install Microsoft.OneDrive` |
| Mozilla tasks disabled (both) | `Get-ScheduledTask -TaskPath '\Mozilla\' \| Enable-ScheduledTask` (admin) |
| GoogleUserPEH tasks disabled (both) | `Get-ScheduledTask -TaskPath '\GoogleUserPEH\' \| Enable-ScheduledTask` |
| 28 Windows telemetry/sync tasks disabled (the section 3 list). SettingSync BackgroundUploadTask refused, access denied | `Enable-ScheduledTask` per task (admin) |

Also found: Brave .ldb churn was one web tab stuck in a loop rewriting a
localStorage key. Find the site with the origin count over the live
`Local Storage\leveldb\*.log` file. Site bug, reload the tab.
`C:\code\factoriobot` had 784 untracked bot logs totalling 42.6 GB; delete
with `rm -f /c/code/factoriobot/*.log` (Claude Code's safety filter blocks
the bulk delete, operator runs it).

Not done, still open: Steam (operator keeps it), Camera Hub, Greenshot,
IDMan, f.lux, Logi Download Assistant still autostart. Intel RST
(IAStorDataMgrSvc) and MuseAuthService still Automatic. VisualStudio
BackgroundDownload task still enabled. Reboot pending to confirm MsMpEng
stays unloaded. Disk at 23 GB free of 1862 GB pending the log delete.

## Presentation

Present findings as tables with columns: Service/Package | What it does | Verdict. Group into "kill" and "keep". Always explain what the user would lose. Ask before disabling anything that could affect hardware (webcam, Bluetooth, audio).
