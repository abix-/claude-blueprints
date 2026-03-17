---
name: wsl
description: Manage WSL2 Ubuntu 24.04 on Windows 10. Use when checking WSL status, running commands inside WSL2, or troubleshooting the WSL environment.
version: "2.0"
---

WSL2 runs Ubuntu 24.04 on this Windows 10 Home N machine. k3s and BuildKit run inside it as systemd services.

## Environment

- Distro: Ubuntu-24.04 (WSL2)
- Networking: NAT mode (Windows 10 does not support mirrored networking)
- Resource limits: `C:\Users\Abix\.wslconfig` (8GB RAM, 4 cores, 4GB swap)
- Passwordless sudo: configured for user `abix`
- systemd: enabled (`/etc/wsl.conf` has `[boot] systemd=true`)
- Services: k3s, buildkit (both auto-start via systemd)

## Running commands inside WSL2

Passwordless sudo is configured. Run commands directly:
```bash
wsl -d Ubuntu-24.04 -- bash -c "sudo k3s kubectl get pods -A 2>&1"
```

IMPORTANT: Wrap commands in `bash -c "..."` to avoid Windows Git bash mangling paths (e.g. `/run/` becomes `C:/Program Files/Git/run/`).

Commands that need a real TTY (interactive prompts, pagers) still require the user to paste in their WSL terminal. Use `clip.exe` to send to clipboard.

## Common operations

### Check WSL2 status
```bash
wsl -l -v
```

### Start WSL2
```
wsl -d Ubuntu-24.04
```

### Shutdown WSL2
```bash
wsl --shutdown
```
Stops ALL WSL2 distros, k3s, and buildkit. Everything auto-starts on next launch.

### Check resource usage
```bash
wsl -d Ubuntu-24.04 -- bash -c "free -h && echo '---' && df -h / 2>&1"
```

### Fix kubeconfig after WSL restart
WSL2 NAT IP changes on every restart:
```bash
# get new IP
wsl -d Ubuntu-24.04 -- bash -c "hostname -I | awk '{print \$1}'"
```
Then update `C:\Users\Abix\.kube\config` server field with the new IP.

### Update .wslconfig resource limits
Edit `C:\Users\Abix\.wslconfig`. Requires `wsl --shutdown` and relaunch.

## Installed tools (inside WSL2)

- k3s (kubernetes) -- `sudo k3s kubectl ...`
- nerdctl 2.0.4 -- `sudo nerdctl --address /run/k3s/containerd/containerd.sock ...`
- BuildKit -- systemd service, uses k3s containerd with namespace k8s.io
- kubectl (Windows): `C:\Users\Abix\AppData\Local\Microsoft\WinGet\Packages\Kubernetes.kubectl_Microsoft.Winget.Source_8wekyb3d8bbwe\kubectl.exe`

## Troubleshooting

- **k3s not starting**: `wsl -d Ubuntu-24.04 -- bash -c "sudo journalctl -u k3s --no-pager -n 30 2>&1"`
- **kubectl timeout from Windows**: WSL2 IP changed. Fix kubeconfig (see above).
- **Out of memory**: bump `memory` in `.wslconfig` and restart WSL.
- **systemd not running**: verify `/etc/wsl.conf` has `[boot] systemd=true`. Restart WSL.
- **Path mangling**: always use `bash -c "..."` wrapper for WSL commands from Git bash.
