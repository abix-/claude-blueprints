# Forge: Rust-based Ansible-compatible automation

**Status: design agreed. Engineering not started.**

## The plan in one paragraph

Build a Rust tool, working name **Forge**, that is a drop-in replacement for
`ansible-playbook`. It reads existing Ansible playbooks, roles, inventories,
vaults, and collections unchanged. It executes them via a hybrid transport
layer: a **persistent Rust agent** on Linux/Windows servers where one can be
installed, and **agentless SSH** for network devices, appliances, and
locked-down hosts. Each task runs as a **native Rust module** when one
exists; otherwise it falls back to running the existing **Python module via
an AnsiBallZ-compatible bridge**. The user controls the runtime per-task,
per-play, or via config. Existing AWX/AAP setups adopt Forge by shipping a
custom Execution Environment image; nothing in AWX changes.

## What we are building

### Architecture (three first-class transport modes)

| Mode | Used for | Behavior |
|---|---|---|
| **Persistent agent (mTLS)** | Linux/Windows servers | Static Rust daemon. Controller pushes plans over mTLS; agent executes locally, streams events, holds cached state. Continuous-reconcile capable. |
| **Agentless SSH + native modules** | Network devices, appliances, hardened hosts, embedded | Controller generates idempotent shell or ships a small static helper binary. No Python required. |
| **Agentless SSH + Python bridge** | Long-tail Ansible modules that haven't been ported to native Rust | AnsiBallZ-compatible wrapper. Same behavior as Ansible today. |

Transport is selected per host via inventory variables. A single playbook
freely mixes all three.

### Module runtime: rust or python, with explicit control

The controller maintains a registry of native Rust module implementations.
For each task it picks rust or python and **annotates every output line**
with which runtime executed it:

```
TASK [Install nginx]            [native]
TASK [Configure firewall]       [python:ansible.posix.firewalld]
TASK [Render config]            [native]
```

User overrides, in precedence order:
1. Per-task: `runtime: python` keyword on the task
2. Per-play: `runtime: rust` on the play
3. Config file: `runtime_overrides:` map
4. CLI: `--prefer rust|python`, `--require rust|python`

Operational tooling:
- `--runtime-report` dry-runs a plan and shows what would run where
- `--list-native` shows every FQCN with a Rust implementation
- End-of-play summary: `Tasks: 47 native, 12 python, 0 failed`

### AWX / AAP integration

Forge ships as a CLI binary that emulates `ansible-playbook` at the level of
flags, event-stream format (ansible-runner compatible), artifact layout,
and fact cache layout. Users build a custom Execution Environment image
with Forge bundled and select it on existing job templates. AWX, AAP, the
web UI, RBAC, schedules, workflows, REST API: **no changes.**

## Why this design

Three forces shaped it:

**1. The Python tax keeps getting worse.**
PEP 668 (externally-managed environments), Python version drift across
mixed-OS fleets, module_utils version conflicts, compiled extensions
breaking on OS upgrades. Every OS upgrade breaks half the cloud modules.
Each native Rust module is permanent immunity to that for the tasks using
it. This is the single most concrete ongoing gain.

**2. The per-task ceremony is the real cost, not the language.**
Ansible's slowness is mkdir + scp + python startup + cleanup, repeated per
task per host. Rust + tokio kills this for native modules (in-process
function calls) and softens it for python modules (persistent SSH, no
ControlPersist timeout). The "Rust is faster than Python" microbenchmarks
are a side show; the ceremony cost is the headline.

**3. The world is hybrid; no existing tool admits it.**
Ansible is agentless-only. Salt/Chef/Puppet are agent-only. None lets you
keep your existing Ansible playbooks AND get the agent model on the hosts
that benefit. Forge is the first.

## What you gain

**Day 1 (install binary, change nothing):**
- 5-25x faster playbook runs (controller-side parallelism)
- 3-5x AWX worker density on the same infrastructure
- Pre-flight dependency analysis catches Python dep mismatches before tasks run
- Zero migration cost; existing playbooks/roles/vaults work unchanged

**Accumulating (as native modules ship):**
- Per-task Python startup eliminated for ported modules (~100-200 ms each)
- Permanent immunity to OS Python drift on those tasks
- Targets without Python become reachable (distroless containers, embedded, OpenWRT)
- Typical 60% native state adds ~1.7-2x further speedup

**Agent mode (Linux/Windows servers):**
- Sub-millisecond task dispatch (no SSH ceremony at all)
- Continuous reconciliation and drift detection without controller round-trips
- Real-time event streaming (file changes, service state, log patterns)
- mTLS cert per host, no SSH key sprawl
- NAT/firewall-friendly outbound mode

## Implementation phases

### Tier 1: alpha that runs real playbooks (~20-25 person-months)

Core runtime emulation:
- Playbook + role YAML parser, full syntax (4-6 weeks)
- Variable precedence engine, 22 levels (2-3 weeks)
- Jinja2 + ~80 Ansible filters + lookup shim via minijinja (3-4 weeks)
- Role resolution + loader (2 weeks)
- Handler dispatch (1 week)
- Strategy engine (linear/free/host_pinned) on tokio (3 weeks)
- SSH connection layer via russh + become (3 weeks)
- AnsiBallZ wrapper (Python module shipping protocol) (6-8 weeks)
- Inventory: static + dynamic shell-out (2-3 weeks)
- Vault: AES-256-CTR + PBKDF2 (1-2 weeks)
- Callback plugins: default/json/yaml/minimal (2 weeks)
- CLI compatibility with ansible-playbook flags (2 weeks)

Agent:
- mTLS RPC protocol (3-4 weeks)
- Daemon implementation, systemd unit + Windows Service (2-3 weeks)
- Install/update + SSH bootstrap (2 weeks)
- Local state cache + event streaming (4-5 weeks)
- Security hardening, cert management (2 weeks)

Modules and routing:
- 20 native modules (file, copy, template, command, shell, apt, dnf,
  systemd, service, user, group, lineinfile, blockinfile, get_url, git,
  stat, hostname, mount, cron, raw): ~4 person-months
- Runtime routing system (registry, precedence, --runtime-report,
  output annotation): ~1 person-month

AWX:
- ansible-runner event format compatibility (3-4 weeks)
- Artifact layout compatibility (1 week)
- Custom Execution Environment image (1-2 weeks)

Quality:
- Test harness diffing against vanilla Ansible (2-3 person-months)
- Docs + onboarding (1-2 person-months)

### Tier 2: production-credible (~38-44 person-months total)

Adds on top of Tier 1:
- 30 more native modules (especially cloud: aws/gcp/azure/kubernetes)
  using `aws-sdk-rust`, `kube-rs`, etc. (~6 person-months)
- WinRM, docker, k8s connection plugins (~2 person-months)
- Native dynamic inventory plugins (AWS, GCP, k8s) (~3 person-months)
- All become methods + edge cases (~1 person-month)
- Network device support framework (~2 person-months)
- Performance work to 10k hosts (~2 person-months)
- Python callback plugin bridge (~1 person-month)
- Polish, bug fixes, packaging (~3 person-months)

### Tier 3: ecosystem and parity (~8-10 person-years for a team)

- 200+ more native modules
- Network device modules (Cisco/Juniper/Arista/etc.)
- AWX-equivalent web UI for users who want Rust-native control plane
- Compliance/audit, RBAC, enterprise auth

Tier 3 requires a real team, not solo effort. Out of scope for the initial
build.

## Naming, trademark, and positioning

**Product name: Forge** (working). Fits the Chef/Puppet/Salt workshop-
metaphor naming culture; short, memorable. Backup option: **Rax** (zero
collision, requires explanation).

**Compatibility marketing is permitted.** "Ansible-compatible" /
"drop-in for ansible-playbook" is nominative fair use, same as CockroachDB
saying "PostgreSQL-compatible." Industry precedent is overwhelming. The
only hard rules:

1. Don't put "Ansible" in the product name.
2. Don't use the Ansible logo.
3. Don't imply Red Hat endorsement.
4. Include one disclaimer line in the README footer:
   *"Not affiliated with Red Hat. Ansible is a trademark of Red Hat, Inc."*

**Tagline candidates:**
- "Ansible-compatible automation, in Rust."
- "Drop-in for ansible-playbook. Native modules, persistent agents,
  no Python tax."
- "Runs your Ansible playbooks 10x faster, no rewrite."

## Honest costs

- Engineering surface is large. Tier 1 alone is 20-25 person-months and is
  high-risk for solo founders. JetPorch went this route and died at ~18
  months.
- Two execution paths (native + Python bridge) double the test matrix.
- Behavior drift between native and Python implementations is a real risk,
  mitigated by the `runtime: python` per-task escape hatch.
- AnsiBallZ, variable precedence, and Jinja2-filter coverage are ~40% of
  the core work, unglamorous and mostly invisible to users.
- The Python bridge is permanent; the long tail of obscure modules will
  never be ported to native Rust.

## Strategic verdict

**This is the first design that lets users keep their Ansible investment
while escaping its ceiling.** Salt/Chef/Puppet make you rewrite. Vanilla
Ansible makes you live with the ceiling. Forge gives you both:
- Existing playbooks/roles/vaults/AWX unchanged.
- Agent benefits for the servers that can run it.
- Native Rust modules for performance and Python-dep immunity.
- All in one tool, one binary, one playbook syntax.

The risk is execution: this is 20+ person-months of work before public
release. Funding or a committed two-person team is the difference between
shipping and a JetPorch repeat.

---

# Supporting analysis

Everything below is the research and reasoning that led to the plan above.

## How Ansible's module shipping actually works

This is the key context for understanding both the inefficiency and the
opportunity. Per task, per host, every single time:

### Default flow (no pipelining)

```
TASK [Install nginx] **********
1. SSH to target
2. Run: mkdir -p ~/.ansible/tmp/ansible-tmp-1234-abc/
3. sftp/scp: upload wrapper.py to that tmp dir
4. SSH again: python3 ~/.ansible/tmp/ansible-tmp-1234-abc/AnsiballZ_apt.py
5. Read JSON output from stdout
6. SSH again: rm -rf ~/.ansible/tmp/ansible-tmp-1234-abc/
```

A 50-task play on 100 hosts = 5,000 of these dances.

### What's in the AnsiBallZ wrapper

A single generated Python file containing:
- A base64-encoded zip with the module's source code
- All `module_utils` the module imports (import graph walked)
- Bootstrap code that extracts the zip to a temp dir, sets up `sys.path`,
  and `exec()`s the actual module
- Arguments embedded inline (regenerated per task because args change)

Size: ~100-500 KB simple modules, several MB for complex ones (AWS modules
pull in a lot of `module_utils`).

### SSH multiplexing (ControlPersist) softens the pain

Ansible enables SSH's `ControlMaster auto` + `ControlPersist 60s` by default:
- First SSH does full handshake (expensive)
- Subsequent commands within 60s reuse the same TCP connection via a Unix
  socket on the controller
- Without this, Ansible would be ~5x slower than it already is

### Pipelining

Set `pipelining = True` in `ansible.cfg`. Flow becomes:
```
1. SSH to target with: python3
2. Pipe the wrapper.py via stdin
3. Read JSON output
```

No file copy, no tmp dir, no cleanup. Typically 2-4x faster. Not default
because it requires `requiretty` off in sudoers, which many enterprise
configs have on.

### What never goes away even with pipelining

1. **Python interpreter starts fresh per task** (~100-200 ms cold start)
2. **Module + utils serialized per task** (same module sent 50 times to the
   same host = 50 serializations)
3. **Facts gathered separately** (the `setup` module ships a Python script
   that returns ~200 KB JSON)
4. **No batch execution** (each task is its own SSH round-trip and Python
   process)

### Where the real overhead is

50-task play, single host, pipelining on:
- 50 Python startups: ~5-10 seconds pure startup
- 50 SSH commands: ~5 seconds round-trip overhead (LAN; WAN much worse)
- 50 module-code transfers: cumulative bandwidth + parse cost

Without pipelining: add ~10-20 seconds of file-copy + cleanup ceremony.

## The OS-version / Python-dep drift problem (the real recurring pain)

This is the most concrete reason to do this work. The problem is getting
**worse, not better,** as the Python ecosystem and Linux distros evolve.

### Why it keeps getting worse

**1. PEP 668 / externally-managed environments.**
Ubuntu 23.04+, Debian 12+, Fedora 38+, RHEL 9+ refuse `pip install` to
system Python. Workarounds: `--break-system-packages` (gross), pipx
(doesn't fit Ansible's module model), per-host venvs (operational
complexity), wait for distro packages (often years out of date).

Ansible modules that need `boto3`, `kubernetes`, `lxml`, `cryptography`,
`pywinrm`, `jmespath`, `netaddr`, `dnspython`, etc., hit this wall on modern
distros.

**2. Python version drift across heterogeneous fleets.**
Real fleet today might include:
- Ubuntu 20.04 (Python 3.8), 22.04 (3.10), 24.04 (3.12)
- RHEL 8 (3.6 default), RHEL 9 (3.9)
- Debian 11 (3.9), Debian 12 (3.11)

Each Ansible collection has its own Python version floor, routinely now
>= 3.9, breaking RHEL 8. Plus distros adopting/dropping packaged versions
on their own cadence.

**3. module_utils version conflicts.**
Two collections requiring incompatible versions of `botocore` or `requests`.
No native per-task environment isolation in Ansible.

**4. Compiled extensions.**
`cryptography`, `lxml`, `psycopg2`, `pyodbc` require system packages
(which lag) or build toolchains at install time (which may not be present
on minimal targets). Each OS upgrade can break the build path.

**5. Interpreter discovery edge cases.**
`ansible_python_interpreter` + auto-discovery is a perpetual source of
"why did this module run against `/usr/bin/python3` instead of my venv?"
debugging.

### How Forge addresses this

1. **Native Rust modules have zero Python deps.** Once ported, the module is
   immune to OS upgrades and Python drift forever.
2. **Native ports of the painful modules.** Cloud/k8s/network modules with
   heavy Python deps get Rust equivalents bundled into the controller
   binary: `aws-sdk-rust`, `kube-rs`, `ring`, `rustls`, `quick-xml`, etc.
3. **Pre-flight dependency analysis.** Before running, walk every task,
   resolve runtimes, probe each host for Python version and installed
   packages, fail fast with a clear report.
4. **Embedded/vendored Python runtime option.** For the Python modules that
   remain, ship a vendored Python interpreter + deps as part of the
   AnsiBallZ payload. PyOxidizer-style.
5. **Per-module dependency manifest.** Each Python module declares its deps;
   the bundle ships them; no pip-install on the target.

## Mid-migration benefit (the steady state)

Most users will live in mid-migration state for years. The gains there are
strong, which is what makes the design viable.

### The non-obvious truth

**Most of the gain comes from the controller, not from the modules.** Going
from "Ansible controller + 100% Python modules" to "Rust controller + 100%
Python modules" gives ~70-80% of the maximum possible speed gain. Native
modules are incremental on top.

Day 1 of adoption, with zero native modules ported, you already get most
of the benefit.

### Concrete math: 30-task play, 200 hosts

| Mode | Per-host time | Wall clock (200 hosts) |
|---|---|---|
| Vanilla Ansible (forks=5) | ~6 sec | ~4 min |
| Vanilla Ansible (forks=50) | ~6 sec | ~50 sec |
| Forge agentless, 0% native (day 1) | ~6 sec | ~10 sec |
| Forge agentless, 60% native | ~3.5 sec | ~6 sec |
| Forge agentless, 100% native | ~1.5 sec | ~3 sec |
| **Forge agent mode, 100% native** | **~0.3 sec** | **~1 sec** |

The first jump (Ansible -> Forge controller) is the big one. Native modules
and agent mode are progressive gravy.

### Other mid-migration wins

1. **Risk-free incremental adoption.** Drop in binary; existing playbooks
   work; `runtime: python` escape hatch pins problem tasks back to vanilla.
2. **Release-train improvement.** Every quarterly release ships more native
   modules; users get faster runs automatically.
3. **Measurable migration progress.** `--runtime-report` tells you exactly
   where you are.
4. **Native modules don't fight OS upgrades.** Permanent surface area
   reduction.
5. **Pre-flight catches Python dep issues before tasks run.**
6. **Long tail keeps working.** No urgency to port rare modules.

### Honest costs

1. Behavior drift risk (native vs Python). Mitigated by test harness +
   `runtime: python` escape hatch.
2. Two code paths to debug. The `[native]` / `[python:fqcn]` annotation
   tells users which ran.
3. Engineering burden internally. Not user-facing.
4. Docs cover both runtimes; test suite is the source of truth.

## Hybrid agent + agentless rationale

The world is hybrid. A real fleet contains:
- **Linux/Windows servers** that can and should run a persistent agent.
- **Network devices, appliances, storage controllers, firewalls, load
  balancers** that are closed: SSH or API only.
- **Embedded / IoT** that may or may not accept an agent.

Existing tools force a choice:
- **Ansible**: agentless only. Cannot give the manageable servers the agent
  benefits.
- **Salt/Chef/Puppet**: agent only. Cannot manage the closed appliances.

Forge does both, configured per host in inventory. Same playbook drives
all three transport modes.

### Why agent mode for managed servers is enormous

Persistent agent eliminates everything the agentless model still pays for:
- No SSH handshake per anything (persistent mTLS)
- No per-task process spawn on the target (agent is one long-running process)
- No module transfer (native modules compiled into the agent)
- Persistent state (fact cache, package state, file checksums all warm)
- Local reconciliation (agent holds policy and reconciles without controller)
- Event streaming (file/service/log changes surfaced in real time)
- Better security at scale (mTLS cert per host, no SSH key sprawl)
- NAT/firewall friendly (outbound long-poll mode)

### Why agentless must remain first-class

Network devices and appliances cannot run agents:
- Cisco IOS, Juniper Junos, Arista EOS, Aruba: closed
- Storage controllers (NetApp, Pure, Dell EMC): API or SSH-CLI
- Firewalls (Palo Alto, Fortinet, Check Point): API or SSH-CLI
- Load balancers (F5, Citrix, A10): API or SSH-CLI
- Hypervisors (ESXi, Proxmox): API or limited shell

Forge handles these via the SSH-with-native-modules path, falling back to
Python-bridge for unported modules.

### Strategic positioning

No tool today supports both transport models as first-class:

| Tool | Agent? | Runs your existing Ansible playbooks? |
|---|---|---|
| Ansible | No | Yes |
| Salt | Yes | No (Salt DSL) |
| Chef | Yes | No (Ruby DSL) |
| Puppet | Yes | No (Puppet DSL) |
| **Forge** | **Yes (optional, per host)** | **Yes (unchanged)** |

That's the wedge. No competitor offers it.

## AWX / AAP integration

Forge integrates with AWX as a backend swap, not a product migration. AWX's
Django app, web UI, RBAC, scheduling, notifications, REST API, workflows:
all unchanged. The engine underneath changes.

### How AWX works today

AWX is Django + PostgreSQL + Redis + Celery. When a job runs:
1. AWX pulls the project (git repo)
2. Spawns a container called an **Execution Environment** with Ansible
   + collections
3. Inside the EE, runs `ansible-playbook` via the `ansible-runner` library
4. ansible-runner captures stdout/stderr, parses event JSON from a callback
   plugin, writes events to `artifacts/<job_id>/job_events/*.json`
5. AWX reads those event files, stores them in PostgreSQL, streams to web UI

### Integration pattern

Ship a custom Execution Environment image with:
- The Forge binary aliased to `ansible-playbook`
- Vanilla Ansible as a fallback option
- Same collections users have today

AWX users add this EE to their instance and select it on job templates.
Forge emits the ansible-runner event format and writes the same artifact
layout, so AWX doesn't know anything changed.

### Effort

~2-3 person-months on top of Tier 1:
- ansible-runner event format compatibility (3-4 weeks)
- Artifact directory layout (1 week)
- Custom Python callback plugin compatibility via bridge (2-3 weeks)
- Fact cache compatibility (1 week)
- Execution Environment image (1-2 weeks)

### Same integration path for AAP (paid Red Hat product)

AAP uses the same EE model. The integration is identical. Free and paid
customers get the same migration story.

## Why JetPorch died (and how this is different)

JetPorch was Michael DeHaan's (Ansible's creator) attempt at a Rust
automation tool. He killed it himself. Stated reason: "lack of outward
excitement" and personal disinterest. No technical postmortem.

Likely structural reasons:
- Solo maintainer couldn't port the module ecosystem
- Invented a new playbook dialect (no migration story)
- Had no funding model

How Forge differs:
- **Drop-in compat with existing Ansible playbooks** removes the migration
  cliff JetPorch couldn't get past.
- **Python bridge** means the module ecosystem works on day one.
- **Hybrid agent + agentless** gives a genuine wedge no tool currently fills.
- **AWX/AAP integration** preserves enterprise users' existing investment.

The risk is still real: Tier 1 is 20-25 person-months. Solo work past 18
months without funding or community is where JetPorch failed. Forge needs
either (a) a small funded team or (b) a focused two-person commitment to
ship Tier 1 in ~10-12 months.

## Naming and trademark detail

### What is NOT allowed

1. Naming the product with "Ansible" in it (`rust-ansible`, `ansible-rs`,
   `Ansible++`). Brand confusion.
2. Using the Ansible logo or trade dress.
3. Implying Red Hat endorses or sponsors the project.
4. Claiming the product IS Ansible.

### What IS allowed (and routine)

1. "Compatible with Ansible playbooks"
2. "Drop-in replacement for ansible-playbook"
3. "Runs your existing Ansible playbooks and roles"
4. "Works with Ansible collections"

Industry precedent for nominative fair use:
- CockroachDB / Yugabyte: "PostgreSQL-compatible"
- MariaDB: "MySQL-compatible"
- DocumentDB: "MongoDB-compatible"
- Many: "Kubernetes-compatible", "S3-compatible API"

### Safety phrase (one line in README footer)

> *Not affiliated with Red Hat. Ansible is a trademark of Red Hat, Inc.*

### Product naming shortlist

| Name | Pros | Cons |
|---|---|---|
| **Forge** | Fits Chef/Puppet/Salt naming culture; short, memorable | Puppet Forge marketplace collision (different category, survivable) |
| Conductor | Orchestration metaphor, semantically right | Netflix Conductor exists |
| Marshal | Coordinates operations | Ruby's Marshal module; generic |
| Rax | Short, made-up, no collision | Meaningless until explained |
| Anvil | Workshop metaphor | Some existing products |
| Wagon | "Carries the load" | Less obvious metaphor |

**Working choice: Forge.**

## Open questions

- Funding source. Solo work is the JetPorch trap.
- License: GPLv3 (Ansible-compatible) vs Apache 2.0 (commercial-friendly).
  Trade off contributor pool vs commercial monetization options.
- First public release scope: alpha with N modules, or beta with full
  Tier 1? Smaller surface ships sooner and gathers feedback earlier.
- Module priority ranking: which 20 modules get ported first determines
  the addressable user base on day one. Suggested: filesystem + package +
  service + user/group + template + git + systemd.
- Network-device support strategy: extend the SSH path with vendor-specific
  CLI parsing, or wrap existing Ansible network collections via the Python
  bridge?
