---
name: vsphere-influxdb
description: vSphere VM performance investigation via InfluxDB MCP server. Use when investigating slow VMs, high latency, CPU contention, memory pressure, disk performance, or network issues in vSphere. Requires the influxdb MCP server to be running.
user-invocable: false
version: "1.0"
updated: "2026-03-02"
---
# vSphere Performance Investigation via InfluxDB

Use the `influxdb` MCP server's `query-data` tool to investigate VM performance.
ALWAYS read `~/.claude/skills/vmware-esxi-performance.md` alongside this skill for threshold interpretation and root cause analysis.

## MCP Tool Usage

All queries use the `query-data` tool with parameters: `org` and `query` (Flux string).
Replace `BUCKET` with the configured bucket name in all queries.

## Investigation Workflow

### Step 1: Identify the VM

If the user gives a partial name, search for it:

```flux
import "influxdata/influxdb/schema"
schema.tagValues(bucket: "BUCKET", tag: "vmname")
```

### Step 2: VM Overview

Get host placement, cluster, OS, uptime:

```flux
from(bucket: "BUCKET")
  |> range(start: -5m)
  |> filter(fn: (r) => r["_measurement"] == "vsphere_vm_sys")
  |> filter(fn: (r) => r["_field"] == "osUptime_latest")
  |> filter(fn: (r) => r["vmname"] == "VM_NAME")
  |> last()
```

Returns tags: `esxhostname`, `clustername`, `vcenter`, `guesthostname`, `guest` (OS).

### Step 3: CPU

**Usage (%)** -- overall CPU utilization:

```flux
from(bucket: "BUCKET")
  |> range(start: -1h)
  |> filter(fn: (r) => r["_measurement"] == "vsphere_vm_cpu")
  |> filter(fn: (r) => r["_field"] == "usage_average")
  |> filter(fn: (r) => r["cpu"] == "instance-total")
  |> filter(fn: (r) => r["vmname"] == "VM_NAME")
  |> aggregateWindow(every: 1m, fn: max, createEmpty: false)
```

**Readiness (%)** -- vCPU waiting for physical CPU. >5% = contention, >10% = problem:

```flux
from(bucket: "BUCKET")
  |> range(start: -1h)
  |> filter(fn: (r) => r["_measurement"] == "vsphere_vm_cpu")
  |> filter(fn: (r) => r["_field"] == "readiness_average")
  |> filter(fn: (r) => r["cpu"] == "instance-total")
  |> filter(fn: (r) => r["vmname"] == "VM_NAME")
  |> aggregateWindow(every: 1m, fn: max, createEmpty: false)
```

**Per-vCPU usage (MHz)** -- identify hot vCPUs:

```flux
from(bucket: "BUCKET")
  |> range(start: -1h)
  |> filter(fn: (r) => r["_measurement"] == "vsphere_vm_cpu")
  |> filter(fn: (r) => r["_field"] == "usagemhz_average")
  |> filter(fn: (r) => r["vmname"] == "VM_NAME")
  |> filter(fn: (r) => r["cpu"] != "instance-total")
  |> group(columns: ["cpu"])
  |> aggregateWindow(every: 1m, fn: max, createEmpty: false)
  |> keep(columns: ["_time", "_value", "cpu"])
```

### Step 4: Host Correlation

If CPU readiness is high, check the ESXi host. This query auto-resolves the VM's host:

```flux
hostFilter = from(bucket: "BUCKET")
  |> range(start: -1h)
  |> filter(fn: (r) => r["_measurement"] == "vsphere_vm_cpu")
  |> filter(fn: (r) => r["vmname"] == "VM_NAME")
  |> keep(columns: ["esxhostname"])
  |> distinct(column: "esxhostname")
  |> findColumn(fn: (key) => true, column: "esxhostname")

from(bucket: "BUCKET")
  |> range(start: -1h)
  |> filter(fn: (r) => r["_measurement"] == "vsphere_host_cpu")
  |> filter(fn: (r) => r["_field"] == "usage_average")
  |> filter(fn: (r) => r["cpu"] == "instance-total")
  |> filter(fn: (r) => contains(value: r["esxhostname"], set: hostFilter))
  |> group(columns: ["esxhostname"])
  |> aggregateWindow(every: 1m, fn: max, createEmpty: false)
  |> keep(columns: ["_time", "_value", "esxhostname"])
```

Host CPU readiness (same pattern, change field to `readiness_average`).

### Step 5: Memory

**Consumed (GB):**

```flux
from(bucket: "BUCKET")
  |> range(start: -1h)
  |> filter(fn: (r) => r["_measurement"] == "vsphere_vm_mem")
  |> filter(fn: (r) => r["_field"] == "consumed_average")
  |> filter(fn: (r) => r["vmname"] == "VM_NAME")
  |> map(fn: (r) => ({r with _value: float(v: r._value) / 1048576.0}))
  |> aggregateWindow(every: 1m, fn: max, createEmpty: false)
```

**Active (GB)** -- same query, field = `active_average`, same divisor.

**Swapped (MB)** -- any value > 0 is concerning:

```flux
from(bucket: "BUCKET")
  |> range(start: -1h)
  |> filter(fn: (r) => r["_measurement"] == "vsphere_vm_mem")
  |> filter(fn: (r) => r["_field"] == "swapped_average")
  |> filter(fn: (r) => r["vmname"] == "VM_NAME")
  |> map(fn: (r) => ({r with _value: float(v: r._value) / 1024.0}))
  |> aggregateWindow(every: 1m, fn: max, createEmpty: false)
```

**Swap rate (MB/s)** -- active swapping, fields: `swapinRate_average`, `swapoutRate_average`. Same pattern, divide by 1024.0.

### Step 6: Disk

**Read/Write latency (ms)** -- most important disk metric:

```flux
from(bucket: "BUCKET")
  |> range(start: -1h)
  |> filter(fn: (r) => r["_measurement"] == "vsphere_vm_virtualDisk")
  |> filter(fn: (r) => r["_field"] == "readLatencyUS_latest" or r["_field"] == "writeLatencyUS_latest")
  |> filter(fn: (r) => r["vmname"] == "VM_NAME")
  |> group(columns: ["vmname", "disk", "_field"])
  |> map(fn: (r) => ({r with _value: float(v: r._value) / 1000.0}))
  |> aggregateWindow(every: 1m, fn: max, createEmpty: false)
  |> keep(columns: ["_time", "_value", "disk", "_field"])
```

Latency thresholds: <5ms healthy, 5-15ms warning, >15ms critical.

**IOPS** -- fields: `numberReadAveraged_average`, `numberWriteAveraged_average`. Same pattern, no unit conversion.

**Throughput (MBps)** -- fields: `read_average`, `write_average`. Divide by 1024.0. Filter `disk != "instance-total"`.

**Outstanding IO** -- fields: `readOIO_latest`, `writeOIO_latest`. High values indicate queue saturation.

### Step 7: Network

**Throughput (MBps):**

```flux
from(bucket: "BUCKET")
  |> range(start: -1h)
  |> filter(fn: (r) => r["_measurement"] == "vsphere_vm_net")
  |> filter(fn: (r) => r["_field"] == "bytesRx_average" or r["_field"] == "bytesTx_average")
  |> filter(fn: (r) => r["vmname"] == "VM_NAME")
  |> filter(fn: (r) => r["interface"] =~ /^4/)
  |> group(columns: ["vmname", "interface", "_field"])
  |> map(fn: (r) => ({r with _value: float(v: r._value) / 1024.0}))
  |> aggregateWindow(every: 1m, fn: max, createEmpty: false)
  |> keep(columns: ["_time", "interface", "_value", "_field"])
```

**Dropped packets** -- fields: `droppedRx_summation`, `droppedTx_summation`. Same pattern, no unit conversion. Any non-zero value warrants investigation.

## Diagnosis Patterns

| Symptom | Metrics to Check | Likely Cause |
|---------|-------------------|--------------|
| VM slow, high CPU usage | `usage_average` near 100% | Undersized CPU, runaway process |
| VM slow, low CPU usage | `readiness_average` > 5% | Host overcommit -- check host CPU |
| VM slow, memory swapping | `swapped_average` > 0, swap rates active | Memory overcommit, balloon driver |
| High disk latency | `readLatencyUS_latest` or `writeLatencyUS_latest` > 15ms | Storage array slow, queue saturation, DSNRO throttling |
| High outstanding IO | `readOIO_latest` or `writeOIO_latest` climbing | Queue depth limit hit -- check DSNRO/DQLEN per ESXi perf skill |
| Network drops | `droppedRx_summation` or `droppedTx_summation` > 0 | Ring buffer exhaustion, TX hangs -- check vmxnet3 per ESXi perf skill |

## Tips

- Adjust `range(start: -1h)` based on when the issue occurred. Use `-6h`, `-1d`, `-7d` as needed.
- For time-series trends, use `aggregateWindow(every: 5m, ...)` for longer ranges to reduce data volume.
- To compare multiple VMs, change the vmname filter to regex: `r["vmname"] =~ /pattern/`.
- The `interface =~ /^4/` filter selects vmxnet3 adapters. Adjust if the environment uses different NIC numbering.
- When disk latency is high, correlate with the ESXi performance skill's KAVG/DAVG/QAVG guidance for root cause.
