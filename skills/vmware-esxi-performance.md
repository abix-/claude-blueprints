---
name: vmware-esxi-performance
description: VMware ESXi performance troubleshooting for network and storage issues. Use when troubleshooting vmxnet3 TX hangs, NETDEV WATCHDOG errors, ring buffer exhaustion, TSO issues, high storage latency (KAVG/DAVG/GAVG), queue depth tuning, DSNRO configuration, iSCSI performance, PVSCSI optimization, or CPU overcommit symptoms. Covers both guest OS and ESXi host-level diagnostics.
metadata:
  version: "1.2"
  updated: "2026-01-11"
---

# VMware ESXi Performance Troubleshooting

## Quick Diagnostic Flow

**Network issues (TX hangs, watchdog errors):**
1. Check `dmesg -T | grep -i watchdog` for timeout messages
2. Check ring buffers: `ethtool -g <interface>`
3. Check adapter stats: `ethtool -S <interface> | grep -E "ring.full|tx_timeout"`
4. Check offload settings: `ethtool -k <interface>`

**Storage issues (high latency):**
1. Run `esxtop`, press `u` for device view
2. Check DAVG (array latency), KAVG (kernel latency), GAVG (guest latency)
3. Check DQLEN, ACTV, QUED for queue saturation
4. Check device properties: `esxcli storage core device list -d <naa.id>`

## Network Troubleshooting (vmxnet3)

### Symptoms
- `NETDEV WATCHDOG: <interface> (vmxnet3): transmit queue X timed out` in dmesg
- `tx hang` messages
- Network stalls lasting 5+ seconds

### Root Causes (in order of likelihood)
1. TX ring buffer exhaustion (default 512, max 4096)
2. TSO generating large packet batches that flood small rings
3. CPU contention on ESXi host (vCPU overcommit)
4. Driver bugs (less common in modern kernels)

### Diagnostic Commands (Guest)

```bash
# Driver and version
ethtool -i <interface>

# Ring buffer sizes
ethtool -g <interface>

# Adapter statistics - look for "ring full" and "tx timeout count"
ethtool -S <interface>

# Offload features
ethtool -k <interface> | grep -E 'tcp-segmentation|generic-segmentation'

# Kernel messages with timestamps
dmesg -T | grep -i -E 'watchdog|tx.hang|timeout'

# Current uptime in seconds (compare to dmesg timestamps)
awk '{print $1}' /proc/uptime
```

### Fixes

```bash
# Increase ring buffers (immediate, non-persistent)
ethtool -G <interface> rx 4096 tx 4096

# Disable TSO if hangs continue (temporary test)
ethtool -K <interface> tso off

# Make ring buffer change persistent (RHEL/AlmaLinux)
# Add to /etc/NetworkManager/dispatcher.d/ or udev rule
```

### Key Metrics from ethtool -S

| Metric | Meaning | Action if high |
|--------|---------|----------------|
| ring full | TX ring was completely full | Increase TX ring buffer |
| tx timeout count | Watchdog fired | Indicates sustained ring exhaustion |
| pkts tx err | Hardware TX errors | Check physical/virtual NIC |

## CPU Overcommit Diagnosis

### Symptoms
- Network TX hangs even with tuned ring buffers
- High KAVG with low QAVG
- Intermittent storage timeouts

### ESXi Checks

```bash
# In esxtop, press 'c' for CPU view
# Look at %RDY (ready time) and %CSTP (co-stop)
# > 5% is concerning, > 10% is problematic
```

### Guidelines
- vCPUs should not exceed physical cores for latency-sensitive workloads
- 2:1 overcommit causes scheduling delays that trigger watchdogs
- Network watchdog (5s) is more sensitive than iSCSI timeout (30s)

## iSCSI Troubleshooting

### Check for Errors

```bash
# On ESXi host
grep -E "H:0x5|H:0x7|H:0x8" /var/log/vmkernel.log
grep -i -E "APD|PDL|lost.access" /var/log/vmkernel.log

# NIC statistics
esxcli network nic stats get -n vmnicX
```

### Error Codes
- `H:0x5` - Host adapter timeout
- `H:0x7` - Command aborted
- `H:0x8` - Host adapter reset

---

# Storage Latency Deep Dive

## Understanding the Latency Stack

```
VM issues I/O
    ↓
Guest OS disk scheduler
    ↓
Virtual SCSI adapter (PVSCSI/LSI)
    ↓
VMkernel I/O scheduler ← KAVG measured here (includes QAVG)
    ↓
Device queue (DQLEN)   ← QAVG measured here
    ↓
HBA/iSCSI initiator
    ↓
Network/Fabric
    ↓
Storage Array          ← DAVG measured here
```

**GAVG = KAVG + DAVG** (what the guest actually experiences)

## KAVG vs QAVG Relationship

QAVG is a subset of KAVG. Both being high and equal indicates queue depth saturation.

| KAVG | QAVG | Interpretation |
|------|------|----------------|
| High | High (equal) | Queue depth limit reached |
| High | High (QAVG higher) | Array overwhelmed or QoS throttling |
| High | Low/Zero | DSNRO throttling or CPU contention |
| Low | Low | Healthy |

### Why QAVG Can Be Higher Than KAVG

KAVG measures VM I/O only. QAVG includes all I/O (VM + hypervisor metadata). During contention, hypervisor I/O gets deprioritized, inflating QAVG average while VM I/O (KAVG) stays lower.

## Healthy Latency Thresholds

| Metric | Healthy | Warning | Critical |
|--------|---------|---------|----------|
| DAVG | < 5ms | 5-15ms | > 15ms |
| KAVG | < 1ms | 1-2ms | > 2ms |
| GAVG | < 10ms | 10-20ms | > 20ms |

For databases (SQL, SingleStore, etc.):
- Data files: < 5ms GAVG
- Log files: < 1ms GAVG

## Key Metrics (esxtop u view)

| Metric | Meaning | Healthy Value |
|--------|---------|---------------|
| DAVG | Device/array latency | < 10ms |
| KAVG | Kernel latency | < 1ms |
| QAVG | Queue wait time | < 1ms |
| GAVG | Guest observed (DAVG+KAVG) | < 15ms |
| ACTV | Active commands in flight | < DQLEN |
| QUED | Commands waiting in queue | 0 |
| DQLEN | Device queue depth limit | Varies |

### Quick Diagnosis Table

| DAVG | KAVG | QAVG | Likely Cause |
|------|------|------|--------------|
| High | Low | Low | Array/storage problem |
| Low | High | High | Queue depth limit hit |
| Low | High | Low | DSNRO throttling or CPU contention |
| High | High | High | Array slow + queue backing up |

## Diagnosing High DAVG

High DAVG = storage array is slow. Check:

1. **Array-side metrics** - Controller CPU, cache hit ratio, disk latency
2. **RAID configuration** - RAID 6 write penalty is 6x, RAID 10 is 2x
3. **Disk tier** - Data may have tiered to HDD (e.g., Compellent Data Progression)
4. **SSD type** - Read-intensive SSDs have poor sustained write performance
5. **Network path** (iSCSI) - Congestion, MTU mismatch, errors

## Diagnosing High KAVG with Low QAVG

This is the tricky case. I/O is being held in the kernel but not in the device queue.

### Cause 1: DSNRO Throttling

Check `esxcli storage core device list -d <naa.id>`:
```
No of outstanding IOs with competing worlds: 32
```

If you have multiple VMDKs on the same datastore, each VMDK counts as a "competing world." With 8 VMDKs and DSNRO=32, each is limited to 32 outstanding I/Os.

**Fix:** `esxcli storage core device set -d <naa.id> -O 256`

### Cause 2: Clustered Datastore Assumption

`Is Shared Clusterwide: true` triggers defensive throttling even if only one host is active. ESXi doesn't know other hosts are idle.

**Fix:** Same DSNRO increase.

### Cause 3: SSD Misdetection

`Is SSD: false` causes ESXi to apply HDD scheduling policies.

**Fix:** `esxcli storage core device set -d <naa.id> -m true`

### Cause 4: CPU Contention

Even on dedicated hosts, software iSCSI threads or storage driver processing can get delayed.

**Check:** `esxtop` → press `c` → look at %RDY for VMkernel threads.

## Diagnosing High QAVG and KAVG (Equal)

Queue depth is saturated. Commands are waiting in line.

**Check in esxtop:**
- ACTV = DQLEN → queue is full
- QUED > 0 → commands waiting

**Fixes:**
1. Increase device queue depth (HBA-dependent)
2. Increase DSNRO if multi-VM/multi-VMDK
3. Spread I/O across more LUNs/datastores
4. Add PVSCSI controllers in guest

## esxtop Storage Views

### Device View (press 'u')

Key columns:
- DEVICE - NAA identifier
- DQLEN - Device queue length (limit)
- ACTV - Active commands
- QUED - Queued commands
- %USD - Queue utilization percentage
- DAVG/KAVG/GAVG/QAVG - Latencies

### Adapter View (press 'd')

Shows HBA-level statistics:
- AQLEN - Adapter queue length
- Useful for FC HBA bottlenecks

### VM View (press 'v')

Shows per-VM disk statistics:
- LAT/rd, LAT/wr - Read/write latency
- Identifies which VM is causing load

## Capture Performance Data

```bash
# Batch mode capture for 60 seconds, 2-second intervals
esxtop -b -d 2 -n 30 > /tmp/esxtop_capture.csv

# Then analyze with Excel or perfmon
```

---

# Queue Depth and DSNRO Configuration

## Queue Depth Stack

```
┌─────────────────────────────────────────────────────────────────┐
│ Guest VM                                                        │
│   PVSCSI queue: 64 default, 254 max per device                 │
│   PVSCSI adapter: 256 default, 1024 max total                  │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ VMkernel                                                        │
│   ┌─────────────────────┐      ┌──────────────────────┐        │
│   │ Scheduler Queue     │      │ Device Queue         │        │
│   │ (DSNRO throttle)    │ ──→  │ (DQLEN)             │        │
│   │                     │      │                      │        │
│   │ Per-world limit     │      │ Per-device limit     │        │
│   │ Default: 32         │      │ HBA-dependent        │        │
│   └─────────────────────┘      └──────────────────────┘        │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ HBA / iSCSI Initiator                                           │
│   Queue depth: 32-255 depending on vendor                       │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ Storage Array                                                   │
│   Port queue depth: 1600-2048 typical                          │
└─────────────────────────────────────────────────────────────────┘
```

## DSNRO (Disk.SchedNumReqOutstanding)

### What It Does

DSNRO limits outstanding I/Os per "world" to a shared device. A "world" is:
- A VMDK (virtual disk)
- NOT a VM - each VMDK in a VM counts separately

### When It Kicks In

DSNRO throttling activates when:
1. Multiple worlds share a LUN (multiple VMDKs or multiple VMs)
2. VMkernel detects I/O switching between worlds (Disk.SchedQControlVMSwitches threshold, default 6)

### Critical Misconception

**Wrong:** "One VM per datastore means no DSNRO throttling"
**Right:** "One VMDK per datastore means no DSNRO throttling"

A single VM with 8 VMDKs on one datastore = 8 competing worlds = DSNRO applies.

### Configuration

```bash
# Check current setting per device
esxcli storage core device list -d <naa.id> | grep "outstanding"

# Output: No of outstanding IOs with competing worlds: 32

# Change per device (immediate, persists until reboot)
esxcli storage core device set -d <naa.id> -O 256

# Valid range: 1-256
```

### Persistence

Per-device DSNRO changes do not persist across reboots. Options:

1. **Host profile** - For vCenter-managed environments
2. **PowerCLI script** - Run at boot via /etc/rc.local.d/
3. **Claim rule** - Cannot set DSNRO directly, but can set PSP options

### Scripted Application (All Devices)

```bash
# Apply to all Compellent devices on host
for naa in $(esxcli storage core device list | grep -B20 "COMPELNT" | grep "^naa\." | awk '{print $1}'); do
  esxcli storage core device set -d $naa -O 256
  esxcli storage core device set -d $naa -m true
done
```

## Device Queue Depth (DQLEN)

DQLEN is the maximum commands the device queue can hold. Set by:
- HBA driver (FC)
- Software iSCSI initiator settings
- Hardware iSCSI adapter

### Default Values by Adapter Type

| Adapter | Default DQLEN |
|---------|---------------|
| QLogic FC | 64 |
| Emulex FC | 32 |
| Software iSCSI | 128 |
| Hardware iSCSI | Varies |

### Changing HBA Queue Depth

```bash
# Check current HBA module parameters
esxcli system module parameters list -m <driver>

# Example for QLogic
esxcli system module parameters set -m qlnativefc -p "ql2xmaxqdepth=128"

# Requires reboot
```

## Competing Worlds Scenarios

### Scenario 1: Multiple VMs, Same Datastore

```
Datastore A
├── VM1 (1 VMDK)  ← World 1
├── VM2 (1 VMDK)  ← World 2
└── VM3 (1 VMDK)  ← World 3

DSNRO applies: Each VM limited to 32 outstanding I/Os
```

### Scenario 2: Single VM, Multiple VMDKs

```
Datastore A
└── VM1
    ├── disk1.vmdk  ← World 1
    ├── disk2.vmdk  ← World 2
    ├── disk3.vmdk  ← World 3
    └── disk4.vmdk  ← World 4

DSNRO applies: Each VMDK limited to 32 outstanding I/Os
Total VM I/O capped at 4 × 32 = 128, but per-disk only 32
```

### Scenario 3: Single VM, Single VMDK

```
Datastore A
└── VM1
    └── disk1.vmdk  ← Only world

DSNRO does NOT apply: Limited only by device queue depth
```

### Scenario 4: Cluster-Shared Datastore, One Active Host

```
Datastore A (shared by 6 hosts)
└── VM1 on Host1
    └── disk1.vmdk  ← Only active world

DSNRO still applies: ESXi sees "Is Shared Clusterwide: true"
Defensive throttling even though other hosts are idle
```

## RDM Exception

Raw Device Mappings (RDMs) are NOT subject to DSNRO. Each RDM gets full device queue depth.

## SIOC Interaction

When Storage I/O Control (SIOC) is enabled:
- DSNRO is disabled
- SIOC manages queue depth dynamically based on latency
- SIOC cannot increase queue depth beyond DQLEN

## Vendor Recommendations

| Vendor | DSNRO | HBA Queue |
|--------|-------|-----------|
| Dell/EMC SC Series | 64 | 255 |
| Pure Storage | 256 | 256 |
| NetApp | 64-128 | 128 |
| HPE Nimble | 256 | 256 |

Always check vendor best practices documentation.

## Calculating Required Queue Depth

Using Little's Law:
```
Queue Depth = IOPS × Latency (seconds)
```

Example:
- Workload: 5000 IOPS
- Array latency: 2ms (0.002s)
- Required queue: 5000 × 0.002 = 10

But for bursts, multiply by 3-5x headroom: 30-50 queue depth needed.

---

# PVSCSI Adapter Tuning

## Why PVSCSI Matters

PVSCSI (Paravirtualized SCSI) is VMware's high-performance virtual storage adapter.

| Adapter | Queue/Device | Queue/Adapter | CPU Overhead |
|---------|--------------|---------------|--------------|
| LSI Logic | 32 | 128 | Higher |
| LSI Logic SAS | 32 | 128 | Higher |
| PVSCSI | 64 (default) | 256 (default) | Lowest |
| PVSCSI (tuned) | 254 (max) | 1024 (max) | Lowest |

## Check Current Settings

### Linux

```bash
# Per-device queue depth
cat /sys/module/vmw_pvscsi/parameters/cmd_per_lun

# Ring pages (affects adapter queue)
cat /sys/module/vmw_pvscsi/parameters/ring_pages

# Verify PVSCSI is in use
lspci | grep -i vmware
# Should show: VMware PVSCSI SCSI Controller
```

### Windows

```powershell
# Check registry
Get-ItemProperty "HKLM:\SYSTEM\CurrentControlSet\Services\pvscsi\Parameters\Device" -Name DriverParameter
```

## Tuning PVSCSI

### Linux - Persistent Configuration

Create `/etc/modprobe.d/pvscsi.conf`:
```
options vmw_pvscsi cmd_per_lun=254 ring_pages=32
```

Rebuild initramfs:
```bash
# RHEL/CentOS/AlmaLinux
dracut -f

# Ubuntu/Debian
update-initramfs -u

# Then reboot
```

### Linux - Runtime (Non-Persistent)

Cannot change at runtime. Requires module reload or reboot.

### Windows - Registry

```
Path: HKLM\SYSTEM\CurrentControlSet\Services\pvscsi\Parameters\Device
Value Name: DriverParameter
Value Type: REG_SZ
Value Data: RequestRingPages=32,MaxQueueDepth=254
```

Then reboot.

## Ring Pages Explained

Ring pages control the PVSCSI adapter's command ring buffer:
- Default: 8 pages × 4KB = 32KB
- Each I/O entry: 128 bytes
- Default capacity: 32KB / 128B = 256 entries
- With 32 pages: 128KB / 128B = 1024 entries

Increasing ring pages allows more concurrent adapter-level I/O.

## Queue Depth Interaction

Three queue depths interact:

1. **PVSCSI device queue** (cmd_per_lun): Per-VMDK limit in guest
2. **PVSCSI adapter queue** (ring_pages): Total for all VMDKs on adapter
3. **ESXi device queue** (DQLEN): Per-LUN limit on host

I/O flows through all three. Lowest limit wins.

### Example Bottleneck

```
Guest PVSCSI: cmd_per_lun=254
ESXi DSNRO: 32
Array: plenty of capacity

Result: Guest sends 254, ESXi throttles to 32 per world
KAVG increases even though PVSCSI is tuned
```

**Must tune both guest and host settings.**

## Multiple PVSCSI Controllers

Maximum 4 PVSCSI controllers per VM, 64 devices per controller.

### Benefits of Multiple Controllers

1. Separate queue pools per controller
2. Parallel I/O processing
3. Isolate workloads (logs vs data)

### Recommended Layout for Databases

| Controller | Devices | Purpose |
|------------|---------|---------|
| SCSI 0 | OS disk | System |
| SCSI 1 | Data disks | Database data files |
| SCSI 2 | Log disks | Transaction logs |
| SCSI 3 | Temp disks | TempDB / scratch |

### Adding Controllers

In vSphere:
1. Edit VM settings
2. Add New Device → SCSI Controller
3. Select PVSCSI type
4. Attach disks to new controller

## VMware KB References

- KB 2053145: Large-scale workloads with intensive I/O patterns
- KB 1017423: Changing PVSCSI queue depth
- KB 1267: Changing HBA queue depth

## Verification After Tuning

### Linux

```bash
# Verify module parameters
cat /sys/module/vmw_pvscsi/parameters/cmd_per_lun
# Should show: 254

cat /sys/module/vmw_pvscsi/parameters/ring_pages
# Should show: 32

# Check disk queue depth visible to block layer
cat /sys/block/sd*/device/queue_depth
```

### Check ESXi Side

In esxtop (v view for VM disks):
- ACTV should be able to exceed 64 now
- QUED should decrease if previously backing up

## Troubleshooting

### Settings Not Applied

1. Verify modprobe.d file syntax
2. Verify initramfs was rebuilt
3. Check `dmesg | grep pvscsi` for driver messages
4. Ensure VMware Tools is current version

### Still Seeing High Latency

1. Check ESXi-side DSNRO (often the actual bottleneck)
2. Check for multiple VMDKs triggering competing worlds
3. Verify storage array isn't the bottleneck (DAVG)
