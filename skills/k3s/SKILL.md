---
name: k3s
description: k3s Kubernetes cluster running in WSL2 Ubuntu 24.04. Use when troubleshooting the cluster itself, nodes, networking, storage, or WSL2 integration.
version: "3.0"
---

k3s is a lightweight Kubernetes distribution running inside WSL2 Ubuntu 24.04 on this Windows 10 machine.

## Access

kubectl is NOT on Windows PATH. Always use the WSL wrapper:
```bash
wsl -d Ubuntu-24.04 -- bash -c "sudo k3s kubectl ..."
```

`nerdctl` (container builds) also requires the WSL wrapper:
```bash
wsl -d Ubuntu-24.04 -- bash -c "sudo nerdctl --address /run/k3s/containerd/containerd.sock --namespace k8s.io ..."
```

## WSL2 NAT caveat

The Windows host talks to k3s via the WSL2 NAT IP. If WSL has no active session, the NAT goes stale and k8s API calls return empty/timeout. Fix: wake WSL first:
```bash
wsl -d Ubuntu-24.04 -- bash -c "echo ok"
```

## Namespace

All agent workloads run in `claude-agents` namespace.

## Storage

| PVC | Mount | Purpose |
|-----|-------|---------|
| cargo-target | /cargo-target | shared Rust build artifacts (ext4) |
| cargo-home | /cargo-home | crate registry cache |
| workspaces | /workspaces | persistent git clones per agent slot |

Skills/commands/CLAUDE.md mounted read-only from `~/.claude` via hostPath.

## Image builds

Images are built with nerdctl directly into the k3s containerd namespace (no registry):
```bash
wsl -d Ubuntu-24.04 -- bash -c "cd /mnt/c/code/k3sc && sudo nerdctl --address /run/k3s/containerd/containerd.sock --namespace k8s.io build -t claude-agent:latest image/"
```

Pods use `imagePullPolicy: Never` -- if you see `ErrImageNeverPull`, rebuild with `--namespace k8s.io`.

## Troubleshooting

- **K8s data empty**: WSL2 NAT stale. Wake WSL.
- **ErrImageNeverPull**: rebuild image with `--namespace k8s.io`
- **Node not ready**: `wsl -d Ubuntu-24.04 -- bash -c "sudo k3s kubectl get nodes"`
- **Pod stuck**: `wsl -d Ubuntu-24.04 -- bash -c "sudo k3s kubectl describe pod <name> -n claude-agents"`
