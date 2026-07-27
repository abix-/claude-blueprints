---
name: "source-command-ctop"
description: "Codex Top. Dashboard of k3s agent pods, GitHub issue status, and cluster health"
---

# source-command-ctop

Use this skill when the user asks to run the migrated source command `ctop`.

## Command Template

Run the ctop dashboard:

```bash
k3sc.exe top --once 2>&1
```

If k8s data is empty (WSL2 NAT stale), wake WSL first:
```bash
wsl -d Ubuntu-24.04 -- bash -c "sudo k3s kubectl get nodes 2>&1"
```
Then retry.

Display the output to the user as-is. Do not reformat or summarize.
