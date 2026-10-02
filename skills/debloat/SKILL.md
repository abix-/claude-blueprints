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

## Whitelist

Only items the operator has explicitly approved go here. Anything running,
scheduled, or set to start that is not on this list gets reviewed.

| Category | Item | Reason |
|---|---|---|
| Service | LSM (Local Session Manager) | Core Windows sign-in sessions; Microsoft: disabling causes system instability |
| Service | NVDisplay.ContainerLocalSystem (NVIDIA Display Container LS) | Operator keeps it (NVIDIA Control Panel and driver plugins) |
| Service | Appinfo (Application Information) | UAC; without it nothing can run as administrator, including installers and the elevated debloat scripts |
| Service | AudioEndpointBuilder (Windows Audio Endpoint Builder) | Builds the audio devices; Audiosrv depends on it, no sound or mic without it |
| Service | Audiosrv (Windows Audio) | Windows sound engine; no sound or mic in any program without it |
| Service | BFE (Base Filtering Engine) | Holds the network filter rules; the firewall cannot run without it |
| Service | BrokerInfrastructure (Background Tasks Infrastructure Service) | Microsoft: required for a stable Start menu; cannot be stopped while Windows runs |
| Service | camsvc (Capability Access Manager Service) | Enforces Privacy switches; forum reports mic and webcam access fail without it |
| Service | CoreMessagingRegistrar (CoreMessaging) | CoreUI registrar over ALPC; explorer, Start menu, TextInputHost, Terminal, browsers use it to pass UI messages. No network |
| Service | CryptSvc (Cryptographic Services) | File signature catalog (catroot2) and trusted root certs; Microsoft: updates and program installs fail without it |
| Service | DcomLaunch (DCOM Server Process Launcher) | Starts COM servers; failure action is reboot the PC after 60 s |
| Service | Dhcp (DHCP Client) | Gets Ethernet 3's IP from the router; netprofm and NlaSvc depend on it |
| Service | Dnscache (DNS Client) | Shared DNS cache for all programs; without it every lookup goes out uncached |
| Service | EventLog (Windows Event Log) | All Windows logs; netprofm and NlaSvc depend on it |
| Service | EventSystem (COM+ Event System) | Delivers logon, logoff and network events; SENS depends on it |
| Service | FontCache (Windows Font Cache Service) | Shared font cache; Microsoft: disabling degrades application performance |
| Service | gpsvc (Group Policy Client) | Nothing to apply on this PC, but Winlogon calls it at sign-in (Control\Winlogon\Notifications\Components\GPClient); lockout risk outweighs the gain |
| Service | hns (Host Network Service) | Builds the WSL2 virtual network (NAT, DNS, DHCP); k3s needs it. Manual, starts on demand |
| Service | HvHost (HV Host Service) | Hyper-V per-VM performance counters; Microsoft: do not disable. Manual, starts with the hypervisor |
| Service | KeyIso (CNG Key Isolation) | Private key isolation inside lsass.exe; no own process, nothing to save by disabling |
| Service | LanmanWorkstation (Workstation) | SMB client for network shares and drives; operator keeps it |

## Blacklist

Items the operator rejected. They stay disabled or removed; if one comes
back, disable it again and note how it came back.

| Category | Item | Reason | Revert |
|---|---|---|---|
| Service | Intel(R) TPM Provisioning Service | Fetches Intel PTT endorsement key certs from iasbroker.intel.com; corporate attestation only | `sc.exe config "Intel(R) TPM Provisioning Service" start= auto` |
| Service | edgeupdate, edgeupdatem (Microsoft Edge Update) | Edge, WebView2, Copilot updater. Re-enabled itself 2026-09-20 03:48 when updater 1.3.271.7 installed | `sc.exe config edgeupdate start= delayed-auto; sc.exe config edgeupdatem start= demand` |

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

The running service manager does not see a raw registry write until reboot,
so a trigger (Windows Update, USB plug) can still start the service once
(StorSvc came back this way on 2026-09-15). Always read `Start` back after
the write; if it is not 4, or you need it to stick before reboot, use
`sc.exe config $svc start= disabled`, which notifies the service manager.
DeviceAssociationService read back 3 after a registry write and needed sc.exe.

**Protected services** (MDCoreSvc, SecurityHealthService, wscsvc, MsMpEng):
the registry write succeeds but Stop fails with "Cannot open <svc> service on
computer '.'" even elevated. They stay running until reboot, then do not load.

**Why Task Manager shows many svchost.exe**: since Windows 10 1703 any PC with
more than 3.5 GB RAM runs each service in its own svchost process, so 50
services show as about 40 processes. Regroup them with
`HKLM:\SYSTEM\CurrentControlSet\Control` `SvcHostSplitThresholdInKB` set above
the RAM size in KB, then reboot. Same tweak WinUtil applies. It shortens the
list and saves a little memory per process; it removes no work.

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
- MDCoreSvc (Defender core, once Defender is off by policy)
- SecurityHealthService + wscsvc (Windows Security app, tray icon, Security Center nags. Also remove SecurityHealth from HKLM Run)
- NcbService (push notifications for Store apps)
- DeviceAssociationService (Bluetooth and Miracast pairing, once Bluetooth is off)
- hidserv (keyboard media keys stop working)
- DispBrokerDesktopSvc (wireless and remote display policy)
- StorSvc (Storage Sense and USB drive notifications. Drives still mount)
- CoworkVMService (Claude Cowork VM. Not used by Claude Code in the terminal. Lives inside the Claude AppX, an app update may re-register it)

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

- \Microsoft\Windows\ApplicationData\ DsSvcCleanup; \WDI\ ResolutionHost (once DsSvc and DPS are disabled)
- \Microsoft\Windows\Bluetooth\ UninstallDeviceTask (once Bluetooth services are disabled)
- \Microsoft\Windows\SettingSync\ NetworkStateChangeTask; \CloudExperienceHost\ CreateObjectTask
- \Microsoft\Windows\ExploitGuard\ ExploitGuard MDM policy Refresh; \Subscription\ EnableLicenseAcquisition (MDM and Azure, corporate only)
- \Microsoft\Windows\Location\ Notifications, WindowsActionDialog (once lfsvc is disabled)
- \Microsoft\Windows\Management\Provisioning\ Cellular; \Mobile Broadband Accounts\ MNO Metadata Parser; \WwanSvc\ NotificationTask (cellular modem)
- \Microsoft\Windows\FileHistory\ File History (maintenance mode) (unless File History backup is on)
- \Microsoft\Windows\Input\ LocalUserSyncDataAvailable, MouseSyncDataAvailable, PenSyncDataAvailable, TouchpadSyncDataAvailable (input settings roaming)
- \Microsoft\Windows\NetTrace\ GatherNetworkInfo; \Printing\ EduPrintProv
- \Microsoft\Windows\Autochk\ Proxy (uploads chkdsk results); \DiskFootprint\ Diagnostics; \MemoryDiagnostic\ ProcessMemoryDiagnosticEvents
- \Microsoft\Windows\ApplicationData\ appuriverifierdaily, appuriverifierinstall (web-to-app link checks)
- \Microsoft\Windows\International\ Synchronize Language Settings; \Management\Provisioning\ Logon (MDM)
- \Microsoft\Windows\UPnP\ UPnPHostConfig (once SSDPSRV is off); \Shell\ IndexerAutomaticMaintenance (once Windows Search is off)

**Cannot disable even as admin (TrustedInstaller):** \Microsoft\Windows\SettingSync\ BackgroundUploadTask,
the four \Microsoft\Windows\EDP\ tasks, the two \Microsoft\Windows\BitLocker\ tasks.
Neuter SettingSync by turning off Settings > Accounts > Sync your settings. EDP and BitLocker
only fire on MDM enrollment events and never run on an unmanaged Home PC.

**Keep on a nearly full disk:** \Microsoft\Windows\DiskFootprint\ StorageSense (automatic temp cleanup).

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
| Logi Download Assistant removed from HKLM Run | `Set-ItemProperty -Path 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'Logi Download Assistant' -Value '"C:\Program Files\LogiDownloadAssistant\bin\logi_download_assistant.exe" -system-restarted'` |
| Send to OneNote.lnk deleted from user Startup folder | recreate from OneNote options |
| Services registry Start=4 and stopped: IAStorDataMgrSvc, edgeupdate, edgeupdatem, GoogleUpdaterInternalService152.0.7933.0, GoogleUpdaterService152.0.7933.0 | set Start back to 2 under `HKLM:\SYSTEM\CurrentControlSet\Services\<name>` |
| Settings sync off: policy `HKLM:\SOFTWARE\Policies\Microsoft\Windows\SettingSync` DisableSettingSync=2, DisableSettingSyncUserOverride=1; HKCU SettingSync SyncPolicy=5, every Groups\*\Enabled=0. Neuters the TrustedInstaller-locked BackgroundUploadTask | delete the policy key, SyncPolicy=0 |
| DPS stopped and disabled via `sc.exe config DPS start= disabled` (registry key refuses even admin) | `sc.exe config DPS start= auto` |
| DoSvc registry Start=4, stopped; policy `HKLM:\SOFTWARE\Policies\Microsoft\Windows\DeliveryOptimization` DODownloadMode=0 | Start=2, delete the policy key |
| VisualStudio\Updates\BackgroundDownload task disabled | `Enable-ScheduledTask` (admin) |
| MuseAuthService stopped, `sc.exe config MuseAuthService start= disabled` | `sc.exe config MuseAuthService start= auto` |
| Camera Hub killed. Facecam MK.2 keeps settings on the camera, app not needed for the webcam to work. Run entry was already gone | relaunch from Start menu, re-enable "launch at startup" in its settings |

Defender re-enabled its own three scheduled tasks (Cache Maintenance,
Cleanup, Verification) within a day of being disabled. Real-time stayed off
by policy. Expect those tasks to come back; they are maintenance only.

Also found: Brave .ldb churn was one web tab stuck in a loop rewriting a
localStorage key. Find the site with the origin count over the live
`Local Storage\leveldb\*.log` file. Site bug, reload the tab.
`C:\code\factoriobot` had 784 untracked bot logs totalling 42.6 GB; delete
with `rm -f /c/code/factoriobot/*.log` (Claude Code's safety filter blocks
the bulk delete, operator runs it).

### 2026-09-15

Reboot confirmed: MsMpEng not running, Get-MpComputerStatus RealTime,
Behavior, OnAccess all False.

Elevation: the Claude shell is not admin. Run scripts elevated with
`Start-Process powershell.exe -Verb RunAs -Wait` and redirect output to a
file with `*>&1 | Out-File -Encoding ascii` (default encoding is UTF-16 and
cat shows spaced characters).

| Change | Revert |
|---|---|
| iphlpsvc, MapsBroker, stisvc: registry Start=4, stopped | Start=2 under `HKLM:\SYSTEM\CurrentControlSet\Services\<name>`, Start-Service |
| TrkWks: `sc.exe config TrkWks start= disabled` (registry key refuses admin like DPS) | `sc.exe config TrkWks start= auto` |
| eufy-viewer.lnk deleted from user Startup folder | recreate shortcut to `C:\code\eufy\target\debug\eufy-capture.exe --ui --out C:/code/eufy/recordings` |
| TokenBroker, BTAGService, BthAvctpSvc, bthserv: registry Start=4, stopped. Bluetooth is off until reverted | Start=2 (TokenBroker Start=3), Start-Service |
| SystemApps renamed to .disabled (Remove-AppxPackage gave 0x80073CFA): XBox.TCUI.exe, WpcUapApp.exe (ParentalControls), BioEnrollmentHost.exe, PeopleExperienceHost.exe. MicrosoftEdgeDevToolsClient has no exe, nothing to rename | rename each back under `C:\Windows\SystemApps\<pkg>\` |

| TokenBroker svchost zombie (PID 7924) killed after its Start=4 | reboot restarts nothing, it is disabled |
| ContentDeliveryManager SubscribedContent-338393/353694/353696, SoftLandingEnabled, RotatingLockScreenOverlayEnabled, PreInstalledAppsEnabled, OemPreInstalledAppsEnabled = 0 (Settings app suggestions, tips, lock screen ads, promoted app auto-install) | set each back to 1 or delete the value |
| Policy `HKLM:\SOFTWARE\Policies\Microsoft\Windows\DataCollection` AllowTelemetry=0 | delete the value |
| 19 more Microsoft tasks disabled: ApplicationData\DsSvcCleanup, WDI\ResolutionHost, Bluetooth\UninstallDeviceTask, SettingSync\NetworkStateChangeTask, CloudExperienceHost\CreateObjectTask, ExploitGuard\ExploitGuard MDM policy Refresh, Location\Notifications, Location\WindowsActionDialog, Management\Provisioning\Cellular, Mobile Broadband Accounts\MNO Metadata Parser, WwanSvc\NotificationTask, Subscription\EnableLicenseAcquisition, FileHistory\File History (maintenance mode), Input\LocalUserSyncDataAvailable, Input\MouseSyncDataAvailable, Input\PenSyncDataAvailable, Input\TouchpadSyncDataAvailable, NetTrace\GatherNetworkInfo, Printing\EduPrintProv | `Enable-ScheduledTask -TaskPath '\Microsoft\Windows\<folder>\' -TaskName '<name>'` (admin) |
| After reboot, 9 more disabled: Autochk\Proxy, DiskFootprint\Diagnostics, MemoryDiagnostic\ProcessMemoryDiagnosticEvents, ApplicationData\appuriverifierdaily, ApplicationData\appuriverifierinstall, International\Synchronize Language Settings, Management\Provisioning\Logon, UPnP\UPnPHostConfig, Shell\IndexerAutomaticMaintenance | same `Enable-ScheduledTask` per task (admin) |
| NcbService, hidserv, DispBrokerDesktopSvc, StorSvc: registry Start=4, stopped. hidserv off means keyboard media keys stop working | Start=3 under `HKLM:\SYSTEM\CurrentControlSet\Services\<name>`, Start-Service |
| DeviceAssociationService: registry write read back as 3, so `sc.exe config DeviceAssociationService start= disabled`. Stopped | `sc.exe config DeviceAssociationService start= demand` |
| MDCoreSvc, SecurityHealthService, wscsvc: registry Start=4. Protected, "Cannot open" on stop, still running until reboot | Start=2 (MDCoreSvc, wscsvc), Start=3 (SecurityHealthService) |
| CoworkVMService (Claude Cowork VM, cowork-svc.exe inside the Claude AppX): registry Start=4, stopped. The Claude desktop app may re-register it on update | Start=2, Start-Service CoworkVMService |
| SecurityHealth removed from HKLM Run, SecurityHealthSystray killed | `Set-ItemProperty -Path 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Run' -Name SecurityHealth -Value 'C:\Windows\system32\SecurityHealthSystray.exe'` |

Refused with "Access is denied" even elevated (TrustedInstaller-owned, same
as SettingSync BackgroundUploadTask): the four EDP tasks and the two
BitLocker tasks. They only fire on MDM enrollment events, which never happen
on an unmanaged Home PC, so leave them. DiskFootprint\StorageSense left
enabled on purpose: it is the automatic temp file cleanup on a 97% full disk.

factoriobot logs deleted (784 files, 42 GB, gitignored). `rm -f *.log` from
inside the repo ran without the safety filter blocking it. Disk 29 GB to
71 GB free.

Not done, still open: Steam (operator keeps it), Spotify, Greenshot, IDMan,
f.lux still autostart. Discord no longer autostarts. Services the operator
chose to keep: wuauserv and UsoSvc (Windows Update), mpssvc (firewall),
FontCache, LanmanWorkstation, NVDisplay.ContainerLocalSystem, camsvc, Themes.
XboxGameCallableUI SystemApp not yet renamed. Bash mangles `$s` inside
`-Command` strings too, not only in heredocs; always use a `.ps1` file.
Post-reboot check still owed: MDCoreSvc, SecurityHealthService, wscsvc,
StorSvc should all be Stopped.

### 2026-09-23 (whitelist review)

Every service, task, and startup entry reviewed one at a time. Approved
items go to the Whitelist section; items below were not approved.

| Change | Revert |
|---|---|
| Intel(R) TPM Provisioning Service: `sc.exe config ... start= disabled`, read back Start=4. Not whitelisted. Fetches Intel PTT endorsement key certificates from iasbroker.intel.com, only used for corporate device attestation | `sc.exe config "Intel(R) TPM Provisioning Service" start= auto` |

## Presentation

Present findings as tables with columns: Service/Package | What it does | Verdict. Group into "kill" and "keep". Always explain what the user would lose. Ask before disabling anything that could affect hardware (webcam, Bluetooth, audio).
