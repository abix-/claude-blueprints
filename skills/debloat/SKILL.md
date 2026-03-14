---
name: debloat
description: Strip Windows of junk services, AppX packages, scheduled tasks, startup entries, and telemetry. Use when the user wants to clean up Windows, disable bloatware, or optimize for performance.
disable-model-invocation: true
version: "1.0"
updated: "2026-03-13"
---
# Windows Debloat

Audit and remove Windows bloat. Run as admin PowerShell via scripts (bash mangles `$_`).

## Workflow

1. **Audit** -- scan for junk in each category
2. **Present** -- show the user what to kill vs keep, with brief explanations
3. **Confirm** -- get approval before disabling
4. **Verify** -- confirm each target is dead

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

**Disable method** -- registry is authoritative, `Set-Service` often doesn't stick:
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

**Stop with timeout** -- some services hang on stop:
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
- WbioSrvc (biometric -- unless fingerprint reader)
- TabletInputService (touch keyboard -- desktop)
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
- cbdhsvc (clipboard history -- if unused)
- SysMain (Superfetch -- debatable on SSD, re-enable if cold starts slow)
- LanmanServer (SMB sharing -- unless sharing folders)
- ShellHWDetection (autoplay)
- TokenBroker (web account manager for Store apps)
- WinHttpAutoProxySvc (WPAD proxy detection)
- seclogon (secondary logon / runas)
- WebClient (WebDAV)
- QWAVE (QoS streaming)
- RasMan, SstpSvc (VPN client -- unless using Windows VPN)
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

**Remove method** -- deprovision + per-user removal (works even with InstallService disabled):
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

UpdateOrchestrator tasks are TrustedInstaller-protected -- disable via registry workaround or accept they're neutered once their parent services/apps are removed.

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

## Presentation

Present findings as tables with columns: Service/Package | What it does | Verdict. Group into "kill" and "keep". Always explain what the user would lose. Ask before disabling anything that could affect hardware (webcam, Bluetooth, audio).
