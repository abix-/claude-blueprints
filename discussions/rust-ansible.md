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

## Notes / scratch
