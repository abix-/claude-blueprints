---
name: k3s
description: Manage k3s Claude agent pods for the Endless project. Use when checking agent status, viewing logs, deploying, rebuilding images, or troubleshooting the k3s-based agent system.
version: "2.0"
---

k3s runs Claude Code agents as Kubernetes pods inside WSL2. Each pod works one GitHub issue using the `/issue` workflow. All tooling is one Go binary: `k3s-claude`.

## Architecture

- **Dispatcher CronJob**: runs every 3 minutes, checks GitHub for `ready`/`needs-review` issues, creates Jobs
- **Agent Jobs**: each Job runs one pod with Claude Code working one issue
- **Slot mapping**: k8s slots 1-26 map to letter-based agent IDs claude-a through claude-z (Windows agents use numbered IDs claude-1+)
- **Max concurrency**: controlled by `MAX_SLOTS` env var in dispatcher (default 3)
- **Repo**: `C:\code\k3s-claude`
- **Namespace**: `claude-agents`

## Commands

`k3s-claude` is on PATH. All subcommands:

```bash
k3s-claude top              # live TUI dashboard (q/n/p/d/l/r/+/-)
k3s-claude top --once       # one-shot text output (used by /ctop)
k3s-claude dispatch         # find issues, create k8s Jobs
k3s-claude logs             # summary of all pods
k3s-claude logs 120         # full log for issue 120
k3s-claude logs -f 120      # follow live
k3s-claude deploy           # build image + apply manifests
k3s-claude cargo-lock       # serialize cargo builds (replaces cargo-lock.py)
```

## TUI hotkeys

| Key | Action |
|-----|--------|
| `q` | quit |
| `n` | dispatch now |
| `p` | pause/resume dispatcher |
| `d` | toggle dispatcher section |
| `l` | toggle live output |
| `r` | manual refresh |
| `+`/`-` | adjust max agents (1-5) |

## WSL2 NAT caveat

The Go binary talks to k3s via the WSL2 NAT IP. If WSL has no active session, the NAT goes stale and k8s data is empty. Fix: wake WSL first:
```bash
wsl -d Ubuntu-24.04 -- bash -c "sudo k3s kubectl get nodes 2>&1"
```

## Deploy / rebuild

After changing Go code:
```bash
cd /c/code/k3s-claude && go build -o k3s-claude.exe .
```

After changing image or manifests:
```bash
k3s-claude deploy
```

Cross-compile Linux binary for container:
```bash
cd /c/code/k3s-claude && GOOS=linux GOARCH=amd64 go build -o image/k3s-claude .
```

Update configmap only (after editing manifests/job-template.yaml):
```bash
wsl -d Ubuntu-24.04 -- bash -c "cd /mnt/c/code/k3s-claude && sudo k3s kubectl create configmap dispatcher-scripts -n claude-agents --from-file=job-template.yaml=manifests/job-template.yaml --dry-run=client -o yaml | sudo k3s kubectl apply -f - 2>&1"
```

## Killing pods safely

NEVER delete agent jobs without cleaning up their GitHub issue claims:

```bash
# find orphaned claims
gh issue list --repo abix-/endless --state open --label claimed --json number,labels --jq '.[] | "\(.number) \(.labels[] | select(.name | startswith("claude-")) | .name)"'

# reset one issue
gh issue edit <N> --repo abix-/endless --remove-label claimed --remove-label <owner> --add-label needs-review
```

## Shared volumes (PVCs)

| PVC | Mount | Purpose |
|-----|-------|---------|
| cargo-target | /cargo-target | shared build artifacts (ext4, fast) |
| cargo-home | /cargo-home | crate registry cache |
| workspaces | /workspaces | persistent git clones per slot |

Skills/commands/CLAUDE.md mounted read-only from `~/.claude` via hostPath.

## Auth

- **Claude Code**: `CLAUDE_CODE_OAUTH_TOKEN` in k8s secret (from `claude setup-token`, valid 1 year)
- **GitHub**: `~/.gh-token` file mounted read-only into pods via hostPath

## Troubleshooting

- **K8s data empty in TUI**: WSL2 NAT stale. Run `wsl -d Ubuntu-24.04 -- bash -c "echo ok"` to wake it.
- **Pods not starting**: `ErrImageNeverPull` -- rebuild image with `--namespace k8s.io`
- **Claude auth fails**: regenerate with `claude setup-token`, update k8s secret
- **Orphaned claims**: pods killed mid-work leave issues `claimed`. Clean up manually.
- **Client throttling**: QPS set to 50, should not throttle. If it does, check concurrent pod count.
