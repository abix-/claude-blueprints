---
name: ansible
description: Ansible development standards. Use when writing Ansible playbooks or roles.
metadata:
  version: "1.0"
  updated: "2026-02-09"
---
# Ansible

- Roles contain all logic; playbooks only call roles
- All variables in `vars/main.yml` (not `defaults/`)
- Always FQCN (`ansible.builtin.copy`, not `copy`)
- Always start YAML files with `---`
- Use `validate_certs: false` not `validate_certs: no`
- Prefer modules over shell/command when a module exists
- When shell is required: `ansible.builtin.shell` (Linux) or `ansible.windows.win_shell` (Windows)
- Inline PowerShell formatting for troubleshooting:
  - Pipelines → single line (easy to copy/paste)
  - foreach/for loops → multiline
  - PSCustomObject → multiline OK
  - Avoid dense semicolon chains
