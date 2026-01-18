---
name: infrastructure-troubleshooting
description: Systematic methodology for diagnosing infrastructure problems across compute, storage, network, and virtualization layers. Use as the starting framework for any troubleshooting scenario, then reference platform-specific skills (vmware-esxi-performance, etc.) for deep dives.
metadata:
  version: "1.1"
  updated: "2026-01-18"
---
# Infrastructure Troubleshooting Methodology

## Skill Hierarchy

This is the **parent methodology skill**. For platform-specific deep dives:

| Platform | Skill | Covers |
|----------|-------|--------|
| VMware ESXi | `vmware-esxi-performance` | esxtop, KAVG/DAVG, DSNRO, vmxnet3, PVSCSI |
| OpenShift Virt | *(future)* | KubeVirt, Portworx, OVN networking |
| Pure Storage | *(future)* | FlashArray diagnostics, replication |

Start here for methodology, go to child skills for commands and thresholds.

---

## Core Principle: Layered Isolation

Work from the bottom of the stack up. Symptoms at the application layer often have root causes in infrastructure.

```
Application Layer     ← Symptoms appear here
    ↓
Operating System      ← Guest drivers, kernel, services
    ↓
Virtualization        ← VM config, hypervisor scheduling, virtual hardware
    ↓
Infrastructure        ← Physical compute, storage, network
```

**Rule:** Don't chase symptoms. Isolate which layer owns the problem before diving deep.

---

## The Troubleshooting Loop

1. **Observe** — What exactly is happening? Quantify it.
2. **Hypothesize** — Based on the layer model, where could this originate?
3. **Test** — Gather data that proves or disproves the hypothesis.
4. **Isolate** — Narrow to a single layer, then a single component.
5. **Fix/Escalate** — Resolve if within scope, or package data for vendor.

---

## Quick Decision Tree

```
Performance problem reported
    │
    ├── Affecting multiple VMs/apps?
    │       YES → Infrastructure layer (storage, network, host)
    │       NO  → VM-specific or app-specific
    │
    ├── Correlates with time of day?
    │       YES → Contention, scheduled jobs, backups
    │       NO  → Configuration or hardware issue
    │
    ├── Did anything change recently?
    │       YES → Start there (patches, config, new workloads)
    │       NO  → Gradual degradation or intermittent failure
    │
    └── Can you reproduce it?
            YES → Capture data during reproduction
            NO  → Set up monitoring to catch next occurrence
```

---

## Layer-Specific Quick Checks

### Storage Latency

**First question:** Is latency from the array (DAVG) or the hypervisor (KAVG)?

- High DAVG, low KAVG → Array problem
- Low DAVG, high KAVG → Hypervisor throttling or queue saturation

**Deep dive:** See `vmware-esxi-performance` skill for metrics, thresholds, esxtop commands, and DSNRO tuning.

### Network Performance

**First question:** Is it latency-bound or throughput-bound?

**Test raw throughput (bypass protocols):**
```bash
iperf3 -c <target> -t 30 -P 4
```

- iperf fast, application slow → Protocol issue (SMB, iSCSI tuning)
- iperf slow → Network path issue

**Bandwidth Delay Product (BDP):**
```
Max throughput = TCP Window Size / RTT
```
- 64KB window + 50ms latency = 1.28 MB/s max
- High latency links need large TCP windows or parallel streams

**Check for packet loss:**
```powershell
pathping <target>
```
Even 1-2% loss destroys TCP throughput.

### CPU Contention

**First question:** Is the workload starved for CPU? Check %RDY (ready time) and %CSTP (co-stop) in esxtop.

**Deep dive:** See `vmware-esxi-performance` skill for thresholds and analysis.

### Firewall/Security Devices

**Symptoms:** Traffic slow through firewall, fast same-segment.

**Quick isolation:**
1. Create test rule with no threat profiles
2. Compare performance
3. If faster → tune profiles, not disable

**Palo Alto quick checks:**
```
show session id <id>
show counter global filter delta yes severity drop
```

Look for: threat inspection latency, session limits, QoS throttling.

---

## Network Troubleshooting (Non-VMware)

### SMB/Windows File Copies

**Common culprits for slow SMB:**
1. SMB signing (default in Server 2022+)
2. High latency + small TCP windows
3. Packet loss causing retransmissions
4. Firewall deep packet inspection

**Diagnostics:**
```powershell
# Check SMB version and signing
Get-SmbConnection | Select-Object ServerName, Dialect
Get-SmbClientConfiguration | Select-Object RequireSecuritySignature

# Check TCP window scaling
netsh interface tcp show global
```

**Wireshark analysis:**
1. Filter: `tcp.flags.syn == 1` to find handshake
2. Check SYN packet for Window Scale option
3. During transfer: Statistics → TCP Stream Graph → Round Trip Time
4. Look for retransmissions: `tcp.analysis.retransmission`

### WAN Performance

**If same file transfers fast locally, slow over WAN:**

| Check | Command | Looking for |
|-------|---------|-------------|
| Latency | `ping -n 20 <target>` | Baseline RTT |
| Packet loss | `pathping <target>` | Loss at any hop |
| MTU issues | `ping -f -l 1472 <target>` | Fragmentation |
| Raw throughput | `iperf3 -c <target>` | Actual bandwidth |

**If iperf shows good throughput but robocopy is slow:**
- SMB is chatty over high-latency links
- Try `/MT:16` for parallel streams
- Check for SMB signing overhead
- Consider WAN optimizers for persistent issue

---

## Log Locations Quick Reference

### VMware ESXi
| Log | Location | Retention |
|-----|----------|-----------|
| vmkernel | `/var/log/vmkernel.log` | Rotates at 1MB |
| VM logs | `/vmfs/volumes/<ds>/<vm>/vmware.log` | Longer retention |
| hostd | `/var/log/hostd.log` | VM management events |

**Note:** vmkernel rotates quickly. Check `.log.1`, `.log.2` for historical data.

### Linux
```bash
dmesg -T                    # Kernel ring buffer with timestamps
journalctl -u <service>     # Systemd service logs
journalctl -k               # Kernel messages only
journalctl --since "1 hour ago"
```

### Windows
```powershell
Get-EventLog -LogName System -Newest 100
Get-WinEvent -FilterHashtable @{LogName='System'; Level=2} # Errors only
```

### OpenShift/Kubernetes
```bash
oc adm node-logs <node>
oc logs <pod> --previous      # Previous container instance
oc describe node <node>       # Events and conditions
```

---

## Data Collection for Escalation

Collect data **at time of incident**. Logs rotate, metrics age out.

### VMware Support
- [ ] `vm-support` bundle from ESXi host
- [ ] VM's `vmware.log` (from datastore, not guest)
- [ ] vCenter performance charts (screenshot/export)
- [ ] `esxtop` batch capture: `esxtop -b -d 2 -n 30 > capture.csv`

### Storage Vendor
- [ ] Array support bundle
- [ ] ESXi storage errors: `grep -E "H:0x|APD|PDL" /var/log/vmkernel.log`
- [ ] Topology diagram showing paths
- [ ] Timestamps of incidents

### Red Hat/OpenShift
- [ ] `oc adm must-gather`
- [ ] `sosreport` from affected nodes
- [ ] `dmesg` and `journalctl` output
- [ ] Cluster version: `oc get clusterversion`

### Network Vendor
- [ ] Packet captures from both endpoints
- [ ] Switch port statistics
- [ ] Firewall session tables
- [ ] QoS policy configuration

---

## Anti-Patterns

| Don't | Do Instead |
|-------|------------|
| Chase symptoms at app layer | Isolate which layer owns the problem |
| Change multiple variables | One change, measure, repeat |
| Skip data collection | Capture first, analyze later |
| Assume the obvious | Verify with data |
| Escalate without logs | Package data vendors actually need |

---

## Response Documentation Template

When documenting troubleshooting:

```
## Symptom
[What exactly is happening, quantified]

## Timeline
[When started, any correlation with changes/events]

## Scope
[What's affected, what's working normally]

## Data Collected
[Logs, metrics, captures — with timestamps]

## Layer Isolation
[Infrastructure → Virtualization → OS → Application]

## Root Cause
[Which layer, which component, why]

## Resolution
[What fixed it, or escalation path]
```
