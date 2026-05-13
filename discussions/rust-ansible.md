# Rust Ansible: scoping the build

**Goal: build this in Rust.** This doc is not "should we?" but "what's the
right scope, architecture, and wedge given Rust is the chosen language."

The Python-on-target dependency is the root pain point. The build must
eliminate it. Static-linked Rust binary on the controller, no runtime on the
target beyond what SSH + a shell provides.

## Context (from the thread)

Ansible is mature. Any competitor has to overcome the existing investment:
hundreds of modules, Jinja2 templating, inventory plugins, vault, become,
collections on Galaxy, CI integrations, muscle memory in thousands of ops teams.

A Rust rewrite that's "just faster SSH" loses. The SSH layer is ~5% of why
Ansible exists. Picking a sharp wedge matters more than language choice.

## What "the existing investment" actually is

1. **Module ecosystem**. File, copy, template, systemd, apt, yum, user, lineinfile,
   plus ~hundreds of cloud/network/db modules. Each one is idempotent, has
   check-mode, returns structured results.
2. **Jinja2 + filters**. Templating language, custom filters, lookups.
3. **Inventory**. Static INI/YAML, dynamic plugins (AWS, GCP, k8s, vSphere).
4. **Connection plugins**. Ssh, paramiko, winrm, local, docker, k8s exec.
5. **Vault**. Encrypted secrets at rest in the repo.
6. **Become / privilege escalation**. Sudo, su, doas, runas, with prompts.
7. **Collections / Galaxy**. Distribution mechanism, namespacing.
8. **AWX/Tower**. Web UI, RBAC, scheduling, audit, credential store.

A serious competitor needs answers for at least 1-6.

## What a Rust tool could actually win on

- **No Python on the target.** Static binary copied over SSH, runs, exits.
  Huge for embedded, OpenWRT, minimal containers, anything where you don't
  want a Python runtime.
- **Speed.** Real speed comes from not re-bootstrapping Python per task,
  not from "Rust is fast." Persistent SSH connections + a single agent binary
  on the target could cut a 10-min play to under a minute.
- **Type safety in module interfaces.** Ansible modules silently accept bad args.
- **Better error messages.** Python tracebacks from Ansible are infamous.
- **Single binary distribution.** No `pip install`, no virtualenvs.

## What it cannot easily win on

- Module breadth. This is the killer. Years of contributor-hours.
- Jinja2 compatibility. Re-implementing Jinja2 semantics in Rust is a project
  on its own (minijinja exists but isn't 100%).
- Trust. Ops teams don't switch config-management tools on a whim.

## Why JetPorch died (best guess)

Michael DeHaan (Ansible's creator) started JetPorch in Rust, then abandoned it.
Public reasons were vague (burnout, market fit). Likely real reasons:
- Solo maintainer can't port hundreds of modules.
- "Faster Ansible" isn't a strong enough wedge to get contributors.
- No clear funding model.

## Possible wedges that aren't "rewrite Ansible"

1. **Agent-on-host model.** Lightweight Rust daemon on each target, control
   plane pushes plans. Closer to Salt's architecture but without ZeroMQ pain.
2. **Embedded/router focus.** OpenWRT, IoT, anything Ansible can't reach.
   No Python = market that literally cannot use Ansible today.
3. **Ansible-compatible module runner.** Run existing Ansible modules from a
   Rust controller. Cheat: keep the ecosystem, replace only the slow Python
   controller. Probably the highest-leverage play.
4. **Declarative + drift detection.** A Rust controller that continuously
   reconciles, not just push-once. Crosses into Chef/Puppet territory.

## Open questions to talk through

- Is there enough pain in the Ansible status quo to drive adoption, or is it
  "good enough" forever?
- Is the Python-on-target requirement actually a blocker for anyone with
  budget, or just an aesthetic complaint?
- Would an "Ansible-compatible Rust controller" (option 3 above) be
  legally/technically viable? Modules are GPL.
- What's the smallest viable scope? "Rust binary that runs YAML playbooks with
  the 20 most-used modules" might be 6 months of work for one person. Worth it?
- Who pays for this? Red Hat owns Ansible and won't fund a competitor.
  Cloud vendors don't care. Hobby project = JetPorch's fate.

## Deep dive: what "Ansible without Python on targets" means

The Python dependency on targets shows up in three places. A replacement has
to answer all three:

1. **Module execution.** Ansible ships Python source to `/tmp/.ansible/` then
   runs `python3 module.py args.json`. Every task does this.
2. **Fact gathering.** The `setup` module is a Python script that reads
   `/proc`, `/sys`, `dmidecode`, etc., and returns a giant JSON blob.
3. **Connection bootstrap.** Before any module runs, Ansible probes the target
   with `python -c "..."` to find the interpreter and check the environment.

### Three architectures

**A. Static agent binary, copied per run (Ansible-shaped)**
- Controller scp's one `agent` binary to `/tmp/agent-<hash>`
- Runs it with a JSON plan: `./agent < plan.json`
- Agent contains every module as a subcommand (file, copy, systemd, apt, ...)
- Returns JSON, controller deletes binary, done
- Pros: no install on target, no daemon, works on anything with SSH + sh
- Cons: must cross-compile for every target arch (x86_64, aarch64, armv7,
  mips for routers, ppc64le, s390x for the unlucky)
- Binary size matters; static musl build of "all modules" could be 20-50 MB.
  Solvable with per-module binaries or compression.

**B. Persistent agent daemon (Salt-shaped)**
- Install once via SSH bootstrap, agent stays running as a service
- Controller talks to it over mTLS or SSH-tunneled RPC
- Pros: fast (no per-task copy), drift detection, event streaming, can run
  scheduled local reconciliation
- Cons: now you're Salt/Chef, not Ansible. Ops teams resist installing agents.
  Trust/security review for the daemon. Update mechanism becomes a problem.

**C. Pure-shell transpiler (no binary at all)**
- Compile the playbook to a portable shell script
- ssh-pipe it: `ssh target sh -s < generated.sh`
- Pros: works literally everywhere, including OpenWRT and embedded boxes
  that can't run a Rust binary at all
- Cons: shell is a terrible execution environment. Idempotency requires
  careful test-then-act patterns. Fact gathering is awful (parse `uname`,
  `/etc/os-release`, `ip`, `ss`, ... in shell). Error handling is brittle.
- Realistically only viable for a small core of modules.

### The likely right answer

**A for 90% of cases, C as fallback, B as opt-in.**

- Default: copy the static binary, run it, delete it. Solves Linux servers,
  most BSDs, macOS, and anything with a real CPU.
- Fallback: for targets where the binary can't run (weird arch, no writable
  /tmp, super-locked-down embedded), transpile a subset of modules to shell.
- Opt-in: for environments doing config management at high frequency
  (continuous reconciliation, drift detection), install the persistent agent.

### What this buys you that Ansible can't

- **Embedded / routers.** OpenWRT, MikroTik (RouterOS scripting is its own
  hell), home-lab gear. Ansible cannot reach these today without ugly hacks.
- **Minimal containers.** Distroless, scratch, alpine without Python. You
  can config-manage a running container without baking Python into the image.
- **Speed.** Static binary executes a 50-task play in seconds, not minutes,
  because there's no Python interpreter startup per task.
- **Single artifact to audit.** Security teams review one signed binary,
  not "Python + 200 module scripts copied to /tmp".

### What this doesn't solve

- Module breadth. You still need to write the modules. Static-binary delivery
  doesn't write `apt`, `systemd`, `file`, `template` for you.
- Jinja2. Templating runs on the controller, not the target, so the Python
  dep there is your problem (controller side) not the target's. A Rust
  controller using minijinja covers most cases.
- Network device modules (Cisco, Juniper). Those mostly use API/SSH-CLI, not
  Python-on-device, so they port fine.

## Prior art (research)

### pyinfra (the elephant in the room)
- https://pyinfra.com / https://github.com/pyinfra-dev/pyinfra
- **Already solves "no Python on target."** Compiles operations to one-off
  shell commands, ships them over SSH, parses output. Target needs only
  POSIX shell.
- Controller is Python. Configuration is Python (not YAML), which is both
  its biggest feature and biggest blocker for Ansible refugees.
- Claimed ~10x faster than Ansible. Mature, actively maintained.
- ~hundreds of operations (their equivalent of modules). Mature ecosystem
  for Linux server config. Lighter on weird platforms.
- **Architectural lesson**: their pure-shell transpiler approach (option C
  in earlier analysis) is more viable than I gave it credit for. They make
  it work by being disciplined: each operation is a small, well-tested
  shell snippet, not arbitrary translated logic.

### cdist
- Zero deps on target, only SSH. Controller is Python 3.
- Operations ("types") are written as shell scripts. Very Unix-y.
- Niche but loved by a small audience. Slow community growth.

### JetPorch
- DeHaan (Ansible's creator) tried, killed it himself.
- Stated reason: "lack of outward excitement" and personal disinterest.
  No technical postmortem.
- Code is GPLv3 on GitHub, mirrored from SourceHut. Read-only mirror.
- Used Rust modules built-in, with planned external modules speaking JSON
  over stdin/stdout (same architecture I sketched independently).
- **Lesson**: technically viable, socially failed. Solo founder + no funding
  + Ansible "good enough" for most users = no contributor flywheel.

### Other Rust-adjacent things
- **NixOS deploy tools** (colmena, deploy-rs, nixos-anywhere): stateless,
  parallel, fast. Only relevant if you're already on NixOS.
- No mature general-purpose Rust Ansible replacement exists in 2026.

## Total effort estimate

Given pyinfra already proved the architecture, the realistic scopes are:

**Tier 1: pyinfra-shaped, but in Rust.** Static-binary controller, shell-
transpiler model, YAML playbooks for Ansible familiarity, ~20 core modules
(file, copy, template, systemd, apt/dnf, user, group, lineinfile, command,
shell, service, cron, mount, sysctl, hostname, timezone, package, git, get_url,
unarchive).
- Effort: **~6-9 person-months** to alpha. Modules are repetitive once you
  have a good operation framework. Jinja2 via minijinja covers ~95%.
- Risks: edge cases in idempotency, fact gathering on heterogeneous distros,
  vault format compatibility.
- Outcome: usable for personal infra, hard to displace Ansible at work.

**Tier 2: production-credible (1.0 release).** Above plus: become/sudo,
vault, inventory plugins (static + a couple dynamic), ssh multiplexing,
check mode, diff mode, handlers/notify, role/include semantics, ~50 modules,
docs, test harness, Windows target support.
- Effort: **~2-3 person-years** with 1-2 people. JetPorch territory.
- Risks: scope creep, contributor recruitment, Jinja2 corner cases.

**Tier 3: Ansible parity.** Hundreds of modules, collections, AWX-like UI,
network device support, all connection plugins, every dynamic inventory.
- Effort: **~5-10 person-years minimum**. Requires a real org and funding.
- Risks: this is what nobody has done because Red Hat already did it.

### What you'd actually ship as a solo or small-team project

**Don't build Tier 3.** That's the trap JetPorch fell into (implicitly aimed
too broad). Pick a sharp wedge:

1. **"pyinfra but Rust"**. Single static binary, no Python anywhere, shell
   transpiler. Wedge: people who already accept pyinfra's model but want
   one-binary distribution and faster execution. Tier 1 effort.
2. **"Ansible-compatible runner for embedded/routers"**. Focus on OpenWRT,
   minimal containers, BSD jails. Modules tuned for busybox/ash. Wedge:
   markets Ansible literally cannot reach. Tier 1 effort.
3. **"Ansible playbook executor that's faster"**. Accept existing YAML
   playbooks, reimplement the controller + a few core modules natively.
   Fallback to real Ansible for unsupported modules. Wedge: drop-in speed
   boost without ecosystem rewrite. Tier 1-2 effort but legally fraught
   (modules are GPL).

## Rust-specific build notes

Since Rust is the goal, calling out what the language actually buys and what
it doesn't:

**What Rust buys you here:**
- Single static binary distribution (musl target). One artifact, no runtime,
  no dependency chain. Huge for sysadmin trust and audit.
- Cross-compilation matrix: x86_64-unknown-linux-musl, aarch64-unknown-linux-musl,
  armv7-unknown-linux-musleabihf, mips-unknown-linux-musl (OpenWRT), plus
  Windows/macOS controllers. Cargo handles this cleanly.
- Async parallelism via tokio without GIL or fork-per-host shenanigans.
  Thousands of concurrent SSH sessions on one box, realistically.
- Strong types on module input/output schemas. Each module is a Rust struct
  with serde derive. No more "Ansible swallowed my bad arg silently."
- Compile-time playbook validation (optional): parse YAML at startup, type-
  check module args against schemas, fail fast.

**What Rust does NOT solve:**
- Module breadth. You still write every module.
- Jinja2 semantics. `minijinja` covers ~95%, the last 5% is a long tail.
- Target heterogeneity. Distro differences, busybox vs GNU coreutils, init
  system variants. Pure Rust controller doesn't help; the modules must.
- Adoption gravity. pyinfra is technically excellent and still niche.

**Recommended crate stack:**
- `tokio` + `russh` for async SSH (pure Rust, no libssh2 C dep)
- `serde` + `serde_yaml` for playbooks
- `minijinja` for templating (Jinja2-compatible, pure Rust)
- `clap` for CLI
- `tracing` for structured logging
- `age` for vault (modern, pure Rust, simpler than Ansible Vault format)
- `indicatif` for progress UI

**Key architectural choice for Rust: shell-transpile vs binary-shipping.**

Option A (ship a static agent binary per run): plays to Rust's strengths.
One musl binary, scp-and-execute, parse JSON. But: target arch matrix is
your problem, and 50 MB binaries get awkward.

Option C (pyinfra-style shell transpiler): the binary stays on the
controller; targets only see shell commands. No arch matrix problem at all.
Loses some Rust advantage on the target side but keeps it on the controller.

**The pragmatic Rust answer is hybrid:**
- Default to **shell transpilation** (Option C). Most operations are
  `mkdir`, `chmod`, `systemctl`, `apt-get`, etc. Rust generates idempotent
  shell, ships it, parses output. No target-side binary, no arch matrix.
- For complex operations that are hard to express in shell (template
  rendering with Jinja2, JSON manipulation, fact gathering), **ship a small
  static helper binary on demand** (Option A, scoped). One musl binary per
  target arch, ~5 MB, contains just the operations that need it.
- Persistent agent (Option B) explicitly out of scope. Keep it agentless.

This hybrid matches pyinfra's proven model while exploiting Rust's strengths
for the controller and for the helper binaries.

## Verdict on total effort

To build something *useful* (Tier 1): **6-9 months solo, or 3-4 months for
two committed people.** Achievable.

To build something *that displaces Ansible in production* (Tier 2): **2-3
person-years.** Hard but possible with funding.

To build *Ansible parity* (Tier 3): **don't.** That's how you become
JetPorch. The market doesn't reward it enough to fund it.

The actual constraint isn't engineering hours, it's **distribution and
contributor flywheel**. pyinfra exists, is excellent, has been around for
~10 years, and is still niche. A Rust rewrite of pyinfra is unlikely to
overcome the same adoption gravity unless it nails a specific underserved
wedge.

## Impact on the existing Ansible Python module ecosystem

You lose most of the ecosystem on day one. That's the central pain. Here is
the module-by-module reality.

### How Ansible modules work today

Each module is a Python file that:
1. Receives JSON args via stdin or argv
2. Uses `AnsibleModule` helper (arg validation, fact merging, changed/failed reporting)
3. Does its thing, prints JSON to stdout
4. Gets shipped to target, executed by Python on target

The "module" is a contract: JSON in, JSON out, exit code, with semantics for
`changed`, `failed`, `diff`, `check_mode`. The contract is reusable. The
Python implementation is not, if the goal is no Python on target.

### What survives, what dies

**Survives trivially (port in days):**
- `command`, `shell`, `raw`, `script`. Already just run things on target.
- `file`, `stat`, `copy`, `fetch`. Filesystem ops, easy in Rust.
- `service`, `systemd`, `cron`. Shell out to systemctl/cron.
- `lineinfile`, `blockinfile`, `replace`, `template`. Text munging + minijinja.
- `apt`, `dnf`, `yum`, `pacman`, `apk`. Shell out to package managers.
- `user`, `group`, `authorized_key`, `hostname`, `timezone`, `mount`, `sysctl`.
- `git`, `get_url`, `unarchive`, `uri` (HTTP requests).

Maybe 40-60 modules. Covers ~80% of real-world playbooks for plain server config.

**Hard but doable (port in weeks-months each):**
- Cloud modules: `amazon.aws.*` (~200 modules), `google.cloud.*`,
  `azure.azcollection.*`. Each wraps a cloud SDK call. Rust has
  `aws-sdk-rust`, `google-cloud-rust`, `azure-sdk-for-rust`. Mechanical but
  voluminous. This is the bulk of modern Ansible value.
- Kubernetes: `kubernetes.core.*`. Rust has `kube-rs`. Few dozen modules.
- VMware: `community.vmware.*`. ~150 modules. Rust SDK story is weaker.
- Network devices: `cisco.ios.*`, `arista.eos.*`, `juniper.junos.*`. These
  talk API/SSH-CLI to devices, not Python-on-device. Port the API/CLI logic.

**Effectively lost (won't get rewritten):**
- Long tail of community modules in Galaxy: ~20,000 modules across all
  collections. 99% are niche. Most don't matter to most users, but the one
  you need will always be the missing one.

### The three compatibility strategies

**1. API-compatible rewrite (pyinfra model).**
Match the YAML interface. Playbook drops in. Modules are Rust-native. Drop
coverage of obscure modules.
- Pro: clean, fast, no Python anywhere.
- Con: rebuild from zero. Don't claim Ansible compatibility (Red Hat
  trademark) and watch which modules you mimic too closely.

**2. Hybrid bridge mode.**
Rust controller is primary. For unsupported modules, spawn a Python sidecar
on the controller that runs the module against the target via the Ansible
connection plugin. Still requires Python on target for Python-based modules,
but only for modules the user opts into.
- Pro: full ecosystem fallback. Users can adopt incrementally.
- Con: defeats the "no Python anywhere" promise. Complex codebase. Two
  execution paths.

**3. Module API translation layer.**
Define a stable Rust-side module contract (e.g.
`fn run(args: T) -> Result<ModuleResult>`). Provide a Python module shim
that translates between Ansible's module protocol and yours, so existing
Python modules can run against the runtime if the user accepts Python on the
target.
- Pro: future-proof, separates contract from implementation.
- Con: most users won't run Python modules anyway, so this is engineering
  surface for a small audience.

### Licensing reality

Ansible core and most modules are GPLv3. Implications:
- Reimplementing module behavior from spec/observation is fine. APIs aren't
  copyrightable in most jurisdictions.
- Copying module source into a Rust repo taints the whole repo with GPLv3.
- Running unmodified Ansible modules from a Rust controller via a separate
  process is the "system library" / "mere aggregation" case. Probably fine,
  but get a lawyer if commercializing.
- Forking Ansible modules and embedding the translated logic is the danger zone.

Safe path: clean-room rewrite of the module contract, look at module
*documentation* for behavior (not source), implement in Rust.

### Bottom line

The Ansible Python module ecosystem is the single biggest reason you can't
just "replace Ansible." A Rust tool can:
- **Match** the playbook YAML format (no IP risk, huge UX win).
- **Port** the core 50-100 modules covering real-world use.
- **Cede** the long tail (cloud SDK breadth, niche community modules).
- **Optionally** bridge to Python for users who need specific modules.

The strategic question becomes **which 50 modules to port first.** That list
defines the addressable user base on day one. Suggested ranking: filesystem
+ package + service + user/group + template + git + systemd. That's the
"configure a Linux box" wedge and it's achievable in months, not years.

## The drop-in compatibility vision (chosen direction)

Reframe: the right architecture is **drop-in replacement for `ansible-playbook`
that uses native Rust modules where available, falls back to running existing
Python modules where not.** No migration project. The user installs the new
binary, points it at their existing playbooks/roles/inventory/vault, and
everything keeps working. Native Rust takes over silently for tasks where a
Rust implementation exists.

This is strategy 2 from earlier (hybrid bridge), elevated from fallback to
primary design constraint. Zero switching cost is the wedge.

### What the user actually sees

```
$ rust-ansible-playbook site.yml -i inventory
PLAY [webservers] *********************************************

TASK [Install nginx]                              [native]
ok: [web-01]
ok: [web-02]

TASK [Configure firewall]                         [python:ansible.posix.firewalld]
changed: [web-01]
changed: [web-02]
```

Every task line tells the user which runtime ran it. `--runtime-report`
produces a migration-progress summary across the whole play. As more native
modules ship, the `[python:...]` lines disappear and Python on the target
becomes unnecessary for those hosts.

### What "uses all of Ansible" requires technically

1. **Module registry.** Internal table mapping FQCN to a Rust implementation
   when one exists. Lookup at task dispatch. Unknown? Use Python.
2. **Ansible module wire protocol (AnsiBallZ).** For Python modules, ship the
   module + `module_utils` dependencies as a zip-embedded Python script,
   `scp` to target, run `python3 wrapper.py`, parse JSON. ~1-2 months to
   re-implement in Rust correctly.
3. **Playbook + role parser with 100% YAML compatibility.** All of: tasks,
   handlers, vars, vars_files, defaults, meta, role search paths,
   include_tasks/import_tasks/include_role/import_role, block/rescue/always,
   when, loop/with_*, register, notify, delegate_to, run_once, tags,
   strategies (linear/free/host_pinned), serial, become*, vars_prompt.
4. **Jinja2 evaluator.** `minijinja` covers the language. Ansible-specific
   filters (`to_nice_yaml`, `hash`, `password_hash`, ...) get re-implemented
   as Rust functions. Lookup plugins are Python today; build a Rust registry
   mirroring built-in lookup names, with Python fallback for unknown ones.
5. **Inventory: static + dynamic.** INI/YAML trivial. Dynamic inventory and
   inventory plugins are Python. Easiest path: shell out to `ansible-inventory`
   and parse its JSON output.
6. **Vault.** Read/write Ansible Vault file format (AES-256-CTR + PBKDF2).
   Documented format, ~500 lines of Rust. Mandatory.
7. **Connection plugins.** SSH via `russh` (pure Rust). WinRM, docker, k8s,
   local each need an implementation, mostly bounded.
8. **Become.** Sudo, su, doas, runas, pbrun. Prefix-the-command logic plus
   password prompt handling over SSH.
9. **Callback plugins.** Native Rust equivalents for default, json, yaml,
   minimal. External Python callbacks get dropped or run via Python bridge.
10. **Strategy plugins.** linear/free/host_pinned native. This is where Rust
    wins biggest on speed.

### Module runtime: control and identification

**Requirement: the user must always be able to see which runtime ran a task,
and must be able to override the choice.** Two layers: defaults that Just
Work, plus escape hatches for everything.

**Default behavior (zero config):**
- For every task, the controller checks the module registry. If a native Rust
  implementation exists, use it. Otherwise use the Python module.
- Every task line in the output is annotated with the runtime that ran it:
  `[native]`, `[python:fqcn]`, or `[shell]` (for transpiled-to-shell ops).
- Summary at end of play: `Tasks: 47 native, 12 python, 0 unknown`.

**CLI-level control:**
```
--prefer rust          # default. use native if available, fall back to python
--prefer python        # use python everywhere available (vanilla Ansible mode)
--require rust         # fail the play if any task would fall back to python
--require python       # force python execution; useful for parity testing
--runtime-report       # dry-run plan: show planned runtime per task without running
--list-native          # show all FQCNs with native Rust implementations
```

**Per-play default in YAML:**
```yaml
- hosts: webservers
  runtime: rust            # rust | python | auto (default: auto)
  tasks: ...
```

**Per-task override:**
```yaml
- name: install nginx
  ansible.builtin.apt:
    name: nginx
  runtime: python          # force python for this task only
```

**Per-module pinning (config file `rust-ansible.cfg`):**
```yaml
runtime_overrides:
  ansible.builtin.template: python   # always use python for templates
  community.aws.*: python            # any AWS module uses python
  ansible.builtin.file: rust         # but file always uses rust
```

This gives four levels of precedence (most specific wins):
1. Per-task `runtime:` keyword
2. Per-play `runtime:` keyword
3. Config file `runtime_overrides`
4. CLI `--prefer` / `--require`

**Why explicit control matters:**
- Native and Python implementations can have subtle behavior differences
  (rounding, default values, edge cases). Users need to pin Python when they
  hit a bug in a native module.
- Compliance/audit environments may require "approved" implementations only.
  Config file pins everything to known-good runtimes.
- Migration verification: `--require rust` on a host group proves you can
  remove Python from those targets.
- Performance debugging: forcing python lets you A/B compare speed and behavior.

### Honest costs

- Engineering surface is 5-10x larger than a clean-room rewrite. You build
  all of Ansible's runtime *plus* the Python bridge.
- Python bridge is permanent. Long-tail modules keep Python in the loop forever.
- Performance ceiling is capped for Python-heavy playbooks. Worst case:
  Ansible-speed. Best case (all native): 10x faster.
- Maintenance burden: track Ansible's module API changes, vault format
  changes, playbook syntax additions.

### Honest wins

- Zero migration friction. 1000 servers don't get touched.
- Gradual migration story with measurable per-host, per-playbook progress.
- You don't have to win on breadth, only on the hot path. Python bridge
  handles the long tail.
- Adoption story is concrete: "install this binary, see what runs native, port
  the modules you use most." No big-bang rewrite.

### Effort recalibration (drop-in compat version)

- **Tier 1**: drop-in compat for basic playbooks + 20 native modules.
  ~9-12 person-months. Ansible-runtime emulation alone is ~6 months.
- **Tier 2**: vault, dynamic inventory, become, callbacks, 50 native modules.
  ~2-3 person-years.
- **Tier 3**: network modules, all strategy/connection plugins, AWX-like UI.
  ~5+ person-years.

Bigger than clean-room, but with a much higher chance of adoption because
nobody has to throw anything away.

## Total effort: bottom-up breakdown (drop-in compat approach)

Honest component-by-component estimate, not top-down guesses.

### Tier 1: alpha that runs real playbooks

**Core runtime emulation (the hard part):**

| Component | Effort |
|---|---|
| Playbook YAML parser (tasks/plays/blocks/loops/when/register/notify) | 4-6 weeks |
| Variable precedence engine (22 levels in Ansible) | 2-3 weeks |
| Jinja2 + ~80 Ansible filters + ~30 tests + lookup shim | 3-4 weeks |
| Role resolution + loader + dependencies | 2 weeks |
| Handler dispatch (notify/listen/flush) | 1 week |
| Strategy engine (linear/free/host_pinned, tokio parallel) | 3 weeks |
| SSH connection layer (russh + multiplexing + become) | 3 weeks |
| **AnsiBallZ wrapper (Python module shipping protocol)** | **6-8 weeks** |
| Inventory (static + dynamic shell-out) | 2-3 weeks |
| Vault (AES-256-CTR + PBKDF2) | 1-2 weeks |
| Callback plugins (default/json/yaml/minimal) | 2 weeks |
| CLI compatibility (ansible-playbook/ansible/ansible-inventory flags) | 2 weeks |

**Core subtotal: 30-40 weeks = 7-9 person-months solo.**

**Plus:**
- 20 native module implementations (file/copy/template/apt/systemd/...),
  ~4 days each: **~4 person-months**
- Runtime routing system (registry, precedence, `--runtime-report`,
  output annotation): **~1 person-month**
- Python bridge edge cases (check_mode, diff_mode, async actions,
  interpreter discovery): **~1-2 person-months**
- Test harness (diff against vanilla Ansible on real playbooks):
  **~2-3 person-months**
- Docs + onboarding: **~1-2 person-months**

**Tier 1 grand total: 16-21 person-months.**
- Solo: ~14-18 calendar months
- Two committed people: ~8-11 calendar months
- Funded team of 4: ~5-7 calendar months

That's "runs real playbooks against real fleets, has rough edges, early
adopters can use it."

### Tier 2: production-credible

Adds:
- 30 more native modules: **6 person-months**
- WinRM, docker, k8s connection plugins: **2 person-months**
- Native dynamic inventory plugins (AWS, GCP, k8s): **3 person-months**
- All become methods + edge cases: **1 person-month**
- Network device support architecture (framework, not modules):
  **2 person-months**
- Performance work, scale to 10k hosts: **2 person-months**
- Bug fixes, polish, packaging: **3 person-months**

**Tier 2 add-on: ~19 person-months on top of Tier 1.**

**Tier 1 + Tier 2: ~35-40 person-months.**
- ~3 person-years solo
- ~18 calendar months for two people
- ~9-12 months for a funded team of 4

### Tier 3: Ansible parity + AWX-equivalent

Adds:
- 200+ more native modules: **30+ person-months**
- Network modules (Cisco/Juniper/Arista/etc): **12 person-months**
- AWX/Tower-equivalent web UI: **18-24 person-months**
- Compliance/audit, RBAC, enterprise auth: **6+ person-months**

**Tier 3 add-on: 60-80 person-months.**

**Full Tier 3: 8-10 person-years for a real team.** "Red Hat Ansible Tower
with a Rust core." Not a solo project.

### What dominates the cost

AnsiBallZ wrapper + variable precedence + Jinja2 filters are ~40% of the
core runtime work. Unglamorous, mostly invisible to users, no shortcut.
Skipping any of them breaks compatibility with real playbooks.

Native modules are the second-biggest line item but they are *parallelizable*
and *contributable*. Modules are where outside contributors plug in once the
core exists.

### Realistic landing zone

A solo or two-person effort lands at **Tier 1 in 8-18 months**. Enough to
publish, get feedback, attract contributors. The trap is trying Tier 2 alone
without funding or contributors. Tier 3 requires a company or foundation.

### Comparison to JetPorch's situation

DeHaan was solo on JetPorch and didn't try drop-in compat. He invented a new
playbook dialect. He still couldn't sustain it past ~18 months. The
"use existing playbooks" angle changes the user-acquisition math, but the
engineering cost goes up, not down. Solo founders should expect the same
outcome unless they have a year of runway and high motivation.

## What this work actually buys (gains analysis)

Honest assessment: the gains are real but concentrated in specific use cases.

### Real, measurable gains

**1. Controller-side parallelism at scale.**
Ansible forks Python processes for parallel hosts. Default fork count is 5;
bumping to 50+ eats memory and hits OS limits. Rust + tokio runs thousands
of concurrent SSH connections in one process with no fork overhead.
- 1000-host play: Ansible ~30-60 min, Rust ~5-10 min (realistic).
- 10,000-host play: Ansible borderline impossible without sharding,
  Rust achievable.
- Biggest single gain. Hardest to dismiss.

**2. Per-task Python startup elimination (native modules only).**
Python interpreter startup is ~100-200ms. A 50-task / 100-host play =
~5000-10000 seconds of pure interpreter startup. Native Rust modules skip
this entirely. Only applies to modules ported to native; bridged Python
tasks see no improvement.

**3. Single static binary distribution.**
No `pip install ansible`, no virtualenv, no requirements.txt drift. One
signed file in `/usr/local/bin`. Faster CI, simpler security review,
simpler air-gapped install.

**4. Targets Ansible cannot reach today.**
With native modules and no-Python-on-target paths:
- Distroless containers (no Python image)
- OpenWRT / embedded Linux on routers
- Hardened servers where installing Python is disallowed
- Tiny VMs where Python footprint is meaningful (~50 MB)
- Strongest wedge: no incumbent here.

**5. Memory + type safety in the controller.**
Ansible crashes from bad plugin code, weird YAML, missing imports. Rust
controller doesn't. Real but boring.

### Gains smaller than they sound

- "Better error messages": maybe. Ansible's errors are tolerable.
- "Faster Ansible" for typical playbooks: if your playbook is mostly cloud
  API modules (AWS, GCP), bridged Python = ~no speedup.
- "Rust is faster than Python": true but mostly irrelevant; SSH round-trip
  dominates.

### Who actually benefits, ranked

| Audience | Magnitude of gain |
|---|---|
| Operators managing 1000+ host fleets | **Large.** Parallelism unlocks plays that don't fit Ansible. |
| Embedded / OpenWRT / IoT operators | **Large.** Enables config management where there is none today. |
| Distroless container shops | **Large.** No more Python-in-the-image debate. |
| Mixed-OS fleets fighting Python version drift | **Large.** See next section. |
| DevSecOps with strict supply-chain rules | **Medium.** Static binary + signed = simpler audit. |
| 100-host standard Linux fleets | **Small.** Ansible is fine. |
| Cloud-heavy infra (lots of API modules) | **Small.** Bridged Python, no speedup. |
| Homelab / personal (10 servers) | **Marginal.** |
| AWX/Tower shops | **Negative initially.** Switching cost > immediate gain. |

### The honest summary

Worth doing if your problem is:
1. Ansible's parallelism ceiling.
2. Targets Ansible can't reach (embedded, routers, minimal containers).
3. Supply-chain / single-binary requirements.
4. Ongoing Python-version / Python-dep drift across heterogeneous fleets
   (see next section).

Not worth doing for:
1. "Ansible is annoying but works on my 50 servers."
2. "I want a more elegant playbook language." (Drop-in compat doesn't
   change syntax.)
3. "Python is bloat." (Aesthetic, not a real cost on a server.)

The best honest pitch is not "better Ansible." It's **"Ansible for the
places Ansible can't go, with backward compat so existing fleets still
work."**

## The OS-version / Python-dep drift problem (real ongoing pain)

This is the single most concrete reason to do this work. The problem is
**getting worse, not better,** as the Python ecosystem and Linux distros
evolve.

### Why it keeps getting worse

**1. PEP 668 / externally-managed environments.**
Ubuntu 23.04+, Debian 12+, Fedora 38+, RHEL 9+ now refuse `pip install` to
system Python. You either:
- Use `--break-system-packages` (gross, dangerous)
- Use `pipx` (per-app venvs, doesn't fit Ansible module model)
- Build per-host venvs (operational complexity, drift)
- Wait for distro packages (often years out of date)

Ansible modules that need `boto3`, `kubernetes`, `lxml`, `cryptography`,
`pywinrm`, `jmespath`, `netaddr`, `dnspython`, etc., now hit this wall on
modern distros.

**2. Python version drift across heterogeneous fleets.**
Real fleet today might include:
- Ubuntu 20.04 (Python 3.8)
- Ubuntu 22.04 (Python 3.10)
- Ubuntu 24.04 (Python 3.12)
- RHEL 8 (Python 3.6 default, 3.9/3.11 optional)
- RHEL 9 (Python 3.9 default)
- Debian 11 (Python 3.9)
- Debian 12 (Python 3.11)

Each Ansible collection has its own Python version floor. Collections now
routinely require >= 3.9, breaking RHEL 8. Soon some will require >= 3.10.
Plus distros adopting/dropping packaged versions on their own cadence.

**3. Module_utils version conflicts.**
Two collections might require incompatible versions of `botocore` or
`requests`. Resolving means picking one or running collections in isolated
environments, which Ansible doesn't natively support per-task.

**4. Compiled extensions.**
`cryptography`, `lxml`, `psycopg2`, `pyodbc` require either system packages
(which lag) or build toolchains at install time (which may not be present
on minimal targets). Each OS upgrade can break the build path.

**5. Interpreter discovery edge cases.**
`ansible_python_interpreter` + auto-discovery is a constant source of
"why did this module run against `/usr/bin/python3` instead of
`/opt/ansible/venv/bin/python`?" debugging.

### How the Rust drop-in approach addresses this

**1. Native Rust modules have zero Python dependencies.**
Once a module is ported to native Rust, OS upgrades and Python drift stop
affecting it. Period.
- `apt`, `dnf`, `systemd`, `file`, `user`, `cron`, etc., as native modules
  don't care if the host has Python 3.6, 3.12, or no Python at all.
- The hot-path modules used in 80% of plays become immune to OS drift.

**2. Native ports of the painful modules.**
The modules causing the most pain today are cloud/k8s/network modules with
heavy Python deps (`boto3`, `kubernetes`, `lxml`). Rust has mature
equivalents bundled into the controller binary:
- `boto3` -> `aws-sdk-rust` (statically linked, zero target dep)
- `kubernetes-client` -> `kube-rs`
- `cryptography` -> `ring` / `rustls`
- `lxml` -> `quick-xml` / `roxmltree`
- `pywinrm` -> native Rust WinRM client

Port these to native Rust modules and the dependency hell disappears for
those tasks.

**3. Pre-flight dependency analysis.**
Before running a play, the Rust controller can:
- Walk every task, resolve to a runtime (rust/python).
- For Python-bridged tasks, list every required Python dep.
- Probe each host: Python version, installed packages, version constraints.
- **Fail fast with a clear report** instead of cryptic mid-task errors:
  ```
  Pre-flight check FAILED:
    host web-01: ansible.builtin.dnf requires Python >= 3.6 (has 3.6) OK
    host web-01: amazon.aws.s3_object requires boto3 >= 1.34 (has 1.20) FAIL
    host web-02: kubernetes.core.k8s requires Python >= 3.9 (has 3.6) FAIL

  Suggestion: run with --prefer rust to use native implementations
  where available (would skip 2 of 3 failures).
  ```

**4. Embedded/vendored Python runtime option.**
For the Python modules that remain, the Rust controller could ship a
**vendored Python interpreter + bundled deps** as part of the AnsiBallZ
payload. Like AppImage / Snap / PyInstaller / PyOxidizer.
- Python module and its deps run against a controller-provided Python,
  not the OS Python.
- OS Python version becomes irrelevant for module execution.
- Caveats: cross-distro static CPython is fiddly (glibc versions, musl
  pain historically), bundles are large (50-100 MB), but doable.
- Real precedent: pyoxidizer, conda-pack, pex.

**5. Per-module dependency manifest.**
Each Python module declares its deps. The bundle ships them. No "pip
install on the target" step required for bridged modules.

### What this looks like end-to-end

Today (Ansible):
```
1. Update fleet to Ubuntu 24.04.
2. Half your AWS modules break because boto3 wheels need newer Python.
3. Debug per-host. Build venvs. Maintain venv config in inventory.
4. Repeat next OS upgrade.
```

With Rust drop-in (mature):
```
1. Update fleet to Ubuntu 24.04.
2. All AWS modules use native Rust runtime with aws-sdk-rust bundled in
   the controller binary. Zero target deps. No change needed.
3. Lifecycle decoupled from OS Python entirely.
```

With Rust drop-in (interim, before all modules ported):
```
1. Update fleet to Ubuntu 24.04.
2. Pre-flight check identifies which Python modules will break.
3. --prefer rust uses native where available, immune to the breakage.
4. For the remaining Python modules, vendored Python bundle ships with
   the run. Zero dependency on OS Python.
```

### Why this is the strongest concrete gain

Of all the gains listed earlier, **this one compounds over time and gets
more valuable the longer it exists.** Every OS upgrade saved, every dep
hell avoided, every fleet that doesn't fragment along Python-version lines
is a permanent cost reduction.

The other gains (parallelism, single binary, embedded targets) are
one-time wins. This one is an *ongoing* relief from a problem that
otherwise compounds annually.

## How Ansible's module shipping actually works

This is the key context for understanding both the inefficiency and the
opportunity. Per task, per host, every single time:

### The default flow (no pipelining)

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
- A base64-encoded zip with the module's source code (`apt.py`, `file.py`, etc.)
- Plus all `module_utils` the module imports (the import graph gets walked)
- Plus bootstrap code that extracts the zip to a temp dir, sets up `sys.path`,
  and `exec()`s the actual module
- Arguments embedded inline (so wrapper is regenerated per task because args
  change)

Size: ~100-500 KB for simple modules, several MB for complex ones (AWS
modules pull in a lot of `module_utils`).

### SSH multiplexing (ControlPersist) softens the pain

Ansible enables SSH's `ControlMaster auto` + `ControlPersist 60s` by default:
- First SSH does full handshake (expensive)
- Subsequent SSH commands within 60s reuse the same TCP connection via a
  Unix socket on the controller
- Without this, Ansible would be ~5x slower than it already is

### Pipelining: the optimization most people don't enable

Set `pipelining = True` in `ansible.cfg`. The flow becomes:
```
1. SSH to target with: python3
2. Pipe the wrapper.py via stdin
3. Read JSON output
```

No file copy, no tmp dir, no cleanup. Typically 2-4x faster.

Not default because: requires `requiretty` off in sudoers, which many
enterprise configs have on.

### What never goes away even with pipelining

1. **Python interpreter starts fresh per task.** ~100-200 ms cold start,
   every time. No state preserved between tasks on the same host.
2. **Module + utils serialized per task.** Same `apt` module sent 50 times
   to the same host = 50 serializations. Target receives the same code 50
   times.
3. **Facts gathered separately.** The `setup` module ships a giant Python
   script that reads `/proc`, `/sys`, `dmidecode`, etc., returning ~200 KB JSON.
4. **No batch execution.** Each task is its own SSH round-trip and its own
   Python process. Ansible cannot say "run these 5 tasks in one shot."

### Where the real overhead is

For a 50-task play, single host, pipelining on:
- 50 Python startups: ~5-10 seconds pure startup
- 50 SSH commands: ~5 seconds round-trip overhead (LAN; WAN much worse)
- 50 module-code transfers (even if cached): cumulative bandwidth + parse cost
- Actual work: variable

Without pipelining: add ~10-20 seconds of file-copy + cleanup ceremony.

### How the Rust drop-in eliminates this

The biggest gain from the Rust approach is **eliminating the per-task
Python ceremony entirely.** Specifically:

**1. No Python startup per task (for native modules).**
Native Rust modules are function calls in the same controller process. Zero
interpreter startup, zero process fork, zero serialization.

**2. No re-transmission of the same module 50 times.**
Where Ansible reships `AnsiballZ_apt.py` to the same host for every `apt`
task, the Rust controller has the module logic compiled into its own binary
and never sends it across the wire at all.

**3. Persistent SSH session running multiple commands.**
russh keeps the SSH connection alive in-process (no ControlPersist
60-second window timing out). Sequential tasks reuse the same channel.

**4. Batched command execution.**
Native module path can ship multiple operations in one SSH exec:
`mkdir foo && chmod 755 foo && touch foo/bar` as a single round-trip,
where Ansible would need three separate AnsiballZ wrapper executions.

**5. No tmp-dir ceremony.**
No `~/.ansible/tmp/ansible-tmp-*` directories created and torn down per
task. Native execution writes nothing to the target filesystem unless the
task itself does.

**6. Fact gathering: streamed or replaced.**
A native fact-gathering implementation can issue a few targeted commands
(`uname -a`, `cat /etc/os-release`, `ip -j addr`) instead of shipping a
~200 KB Python script that parses everything itself. Or skip facts entirely
when the play doesn't reference them.

### The math on a real play

50-task play, 100 hosts, all native modules, Rust controller:
- 100 SSH connections opened in parallel: ~1-2 seconds via tokio
- 50 tasks ran sequentially per host (each ~50 ms work + 1 round trip):
  ~5-10 seconds per host wall clock
- Total: ~10-15 seconds

Same play under Ansible (pipelining on):
- 5 forks at a time across 100 hosts: 20 batches
- Per batch: 50 tasks * 100-200 ms Python startup * 5 hosts = ~50 seconds
- Total: ~15-20 minutes

This is the order-of-magnitude gain. **Not "Rust is faster than Python"
microbenchmarks.** It's eliminating the per-task ceremony: Python startup,
module transfer, tmp dir setup, separate process per task, all of it.

## Mid-migration benefit (the most important section)

Most users will live in mid-migration state for years. If the gains there
are weak, the whole approach fails. If the gains there are strong, there's
an immediate adoption story.

### The non-obvious truth

**Most of the gain comes from the controller, not from the modules.**
Going from "Ansible controller + 100% Python modules" to "Rust controller +
100% Python modules" gives ~70-80% of the maximum possible speed gain.
Native modules are incremental on top.

This means: **day 1 of adoption, with zero native modules ported, you
already get most of the benefit.** Native modules are long-tail bonus.

### What you get immediately, day 1 (zero native modules ported)

**1. Controller-side parallelism.**
Rust + tokio scales to thousands of concurrent SSH connections in one
process. Default Ansible 5-fork model becomes 500-way real parallelism
without memory pressure. Applies to every task regardless of module language.
- Real impact: 10k-host plays go from "shard or die" to "runs in one shot."

**2. SSH connection persistence beyond ControlPersist's 60-second window.**
russh keeps the SSH session alive in the controller process for the entire
play. No reconnect overhead. Same connection serves native and Python tasks.

**3. Pre-flight dependency analysis across the whole play.**
Walk every task before running. Build dep map (Python versions, packages,
target arch). Fail fast with complete report. Works even if every task is
Python-bridged.

**4. Better error reporting and debugging.**
Structured task execution log, machine-readable JSON callback output, faster
surfacing of failures. Independent of module language.

**5. No new playbook syntax to learn.**
Existing playbooks, roles, vaults, inventories work unchanged. Zero
operator-side migration cost.

### What incrementally accrues as native modules ship

For tasks using a ported module:
1. Zero Python startup (~100-200 ms saved per task per host)
2. Zero module transfer (logic is in the controller binary)
3. Zero tmp-dir ceremony (no mkdir/scp/cleanup dance)
4. Immunity to OS Python drift for that module
5. Batch potential (multiple native tasks share a single SSH exec)

Multiplicative with parallelism, not additive. If 1000 hosts run in parallel
and 60% of tasks become native, wall-clock improves on the parallel critical
path, not just the serial sum.

### Concrete math: a mid-migration play

Scenario: 30-task play, 200 hosts, linear strategy. 18 native (60%),
12 still Python.

| Mode | Per-host time | Wall clock (200 hosts) |
|---|---|---|
| Vanilla Ansible (forks=5, pipelining on) | ~6 sec | ~4 min |
| Vanilla Ansible (forks=50, pipelining on) | ~6 sec | ~50 sec |
| Rust controller, 0% native (day 1) | ~6 sec | ~10 sec |
| Rust controller, 60% native (mid-migration) | ~3.5 sec | ~6 sec |
| Rust controller, 100% native | ~1.5 sec | ~3 sec |

- Ansible to Rust controller (zero native modules): **5-25x faster**
- 0% native to 60% native: **~1.7x further faster**
- 60% native to 100% native: **~2x further faster**

**The first jump is the big one. Native modules are gravy.**

### Other mid-migration wins (not just speed)

**1. Risk-free incremental adoption.**
Drop in the binary. Run playbooks. Nothing breaks. If something does,
`runtime: python` pins the offending task back to vanilla behavior.

**2. Native modules become a release-train improvement.**
Every release ships more native modules. Users automatically get faster
runs without changing anything. Like a free CPU upgrade.

**3. Measurable migration progress.**
`--runtime-report` tells you "30 tasks total, 18 native, 12 python." Track
across teams, set targets, prioritize porting the modules used most.

**4. Native modules don't fight OS upgrades.**
The 60% of tasks that are native become immune to "Python 3.10 deprecated
something boto3 needs." Each port is a permanent reduction in OS-drift
surface area.

**5. Pre-flight catches Python dep issues before tasks run.**
Mid-migration you still have Python dep risk for the 12 Python tasks. The
controller can tell you upfront: "host web-01 missing boto3, will fail at
task 8." Today, Ansible discovers this at task 8 with cryptic errors.

**6. Hybrid lets you keep rare modules.**
Don't have to wait for someone to port `community.zabbix.zabbix_host_update`
before adopting. The 0.1% of obscure modules stay Python forever. Still work.

### The honest costs of mid-migration

**1. Behavior drift between native and Python implementations.**
Native `apt` and Python `apt` could differ on an edge case. Mitigated by:
extensive test harness diffing against vanilla Ansible, and the
`runtime: python` escape hatch.

**2. More surface area to debug.**
"Is this a native module bug or a Python module bug?" The `[native]` /
`[python:fqcn]` tag on every task line tells users which path ran.

**3. Two code paths in the controller.**
Engineering burden, not user-facing.

**4. Documentation has to cover both runtimes.**
Native modules need their own docs. They claim Ansible-compatible behavior
but the only true source of truth is the test suite.

### Strategic point

**Mid-migration is not a transitional phase to grit through. It is the
steady state for most users for the lifetime of the product.** Design must
make mid-migration feel like the normal mode, not a degraded mode.

The analysis above says it already is the normal mode in user experience:
controller-side gains arrive immediately, native module count gradually
improves performance and dependency posture over time. No awkward valley.

This is the **adoption story**: install binary, get 5-25x speedup on day 1
with zero changes, then accrue further gains automatically as native
modules ship.

## Notes / scratch
