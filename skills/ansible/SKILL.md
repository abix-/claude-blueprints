---
name: ansible
description: Ansible standards for playbooks, roles, and inventories. Canonical best-practices (the user has no original Ansible repos to mine; awx, awx-operator, ascender-install are forks with no authored content).
user-invocable: false
version: "2.0"
updated: "2026-05-11"
---
# Ansible

**Provenance note:** Unlike other coding skills here, this one is
built from canonical Ansible best-practices, not from the user's own
authored code. The user's `awx`, `awx-operator`, and `ascender-install`
repos are forks with only trivial commits (gitignore tweaks). If the
user starts authoring real Ansible content, mine the new repos and
rewrite this skill against actual style.

For Jinja templating inside Ansible, read the `jinja` skill.
For YAML formatting, read the `yaml` skill.

## File layout

```
<project>/
  ansible.cfg
  inventory/
    production         # INI or YAML inventory
    staging
    group_vars/
      all.yml
      web.yml
    host_vars/
      host01.yml
  playbooks/
    site.yml           # top-level orchestration
    deploy.yml
  roles/
    common/
      tasks/main.yml
      handlers/main.yml
      defaults/main.yml
      vars/main.yml
      templates/...
      files/...
      meta/main.yml
  collections/         # requirements.yml installs go here
```

- **Roles hold all logic. Playbooks just call roles.** A playbook is a
  one-page glue file; complexity lives in the role.
- **Inventory by environment**, not by purpose. `production` and
  `staging` are separate inventories with their own `group_vars`.
- `requirements.yml` (collections + roles) at repo root; CI runs
  `ansible-galaxy install -r requirements.yml`.

## Variables

- `defaults/main.yml` -- user-overridable defaults. Anything the
  caller might want to change goes here.
- `vars/main.yml` -- internal constants the caller should not
  change. Higher precedence than `defaults`.
- `group_vars/<group>.yml` -- environment-specific overrides.
- `host_vars/<host>.yml` -- per-host overrides. Last resort.
- Never set vars at the playbook level for production. Inventory or
  role; nothing in between.
- Var names: lowercase snake_case. Role-scoped vars prefixed with the
  role name (`common_user`, `webapp_port`) to avoid collisions in
  the global var namespace.

## Module usage

- **Always FQCN.** `ansible.builtin.copy`, never bare `copy`.
- **Prefer modules over `shell` / `command`.** If a module exists
  (`ansible.builtin.user`, `ansible.builtin.file`,
  `ansible.posix.mount`), use it. It's idempotent for free.
- `shell` only when you actually need shell features (pipes,
  redirection). Otherwise `command` (no shell).
- For Windows: `ansible.windows.win_shell`, `ansible.windows.win_copy`,
  etc. Never `ansible.builtin.*` on Windows targets.
- `changed_when: false` on read-only checks. Otherwise the task
  reports "changed" on every run.

## Task shape

```yaml
- name: Install nginx
  ansible.builtin.apt:
    name: nginx
    state: present
    update_cache: true
  become: true
  notify: restart nginx

- name: Ensure config is current
  ansible.builtin.template:
    src: nginx.conf.j2
    dest: /etc/nginx/nginx.conf
    owner: root
    group: root
    mode: "0644"
  become: true
  notify: restart nginx
```

- Every task has a `name`. Imperative, capitalized, describes the
  desired state ("Install X", "Ensure Y is present").
- `become: true` per task, not at the play level, unless every task
  needs it. Explicit > implicit.
- `notify:` triggers handlers; one handler per logical action.
- Modes as **quoted strings** (`"0644"`), not octal numbers. YAML's
  number parsing strips the leading zero.
- Booleans: `true` / `false`. Never `yes` / `no` / `True` / `False` --
  YAML 1.1 ambiguity. ansible-lint will flag the others.

## Idempotency

The cardinal rule. Every task must produce the same end state when
run multiple times.

- Use modules; they handle idempotency for you.
- For `shell` / `command`, gate with `creates:`, `removes:`, or
  `when:` conditions that check current state.
- `changed_when:` and `failed_when:` to define the success criteria.
- Don't use timestamps in file names that re-trigger on every run.

## Handlers

```yaml
# roles/web/handlers/main.yml
- name: restart nginx
  ansible.builtin.service:
    name: nginx
    state: restarted
```

- Named in lowercase, no period. The handler name is what `notify:`
  references.
- Run **once** at the end of the play, even if notified many times.
- Force-run with `meta: flush_handlers` if you need them to fire
  mid-play (e.g., before a subsequent task that depends on the
  restart).

## Loops

```yaml
- name: Create users
  ansible.builtin.user:
    name: "{{ item.name }}"
    groups: "{{ item.groups | default([]) }}"
    state: present
  loop:
    - { name: alice, groups: [admin] }
    - { name: bob }
  loop_control:
    label: "{{ item.name }}"
```

- `loop:` over `with_items:` (legacy).
- `loop_control.label` to cut log noise when iterating over big
  dicts.
- `loop_control.pause:` for rate-limiting (e.g. API calls).
- Use `dict2items` filter when you have a dict you want to iterate:
  `loop: "{{ my_dict | dict2items }}"`.

## Conditionals

- `when:` is Jinja already. Don't double-template:
  ```yaml
  when: env == "prod"        # correct
  when: "{{ env }} == 'prod'"  # WRONG
  ```
- Group conditions in a list (implicit AND):
  ```yaml
  when:
    - env == "prod"
    - ansible_facts.os_family == "RedHat"
  ```
- `or` requires a string: `when: env == "prod" or env == "stage"`.

## Templates

- File extension `.j2`. Lives under `roles/<r>/templates/`.
- Strip whitespace in YAML/INI templates: `{%- ... -%}` blocks.
  Default Jinja preserves newlines, which usually breaks structured
  output.
- Render with `ansible.builtin.template`, not `copy`. Template
  always rewrites if content differs.
- Set `mode: "0644"` even for templates; the module respects it.

## Secrets

- `ansible-vault` for secrets in inventory or vars files. Encrypt
  the file, not just one value, unless you need partial encryption.
- `vault_password_file` in `ansible.cfg` so CI can decrypt
  automatically.
- Never commit unencrypted secrets, even temporarily. `git filter-repo`
  is a bad afternoon.

## Testing

- **Molecule** is the standard. One scenario per role:
  `roles/common/molecule/default/{molecule.yml,converge.yml,verify.yml}`.
- Drivers: Docker for fast local runs, `delegated` for VMs / real infra.
- `verify.yml` uses `ansible.builtin.assert` for assertions or invokes
  Inspec / Testinfra for richer checks.
- `molecule test` runs the full lifecycle (create, converge, idempotence,
  verify, destroy).
- CI runs `molecule test` per role on every PR.

## Linting

- `ansible-lint` on every change. Wire into pre-commit.
- `yamllint` for YAML structure.
- Skip lint rules in a `.ansible-lint` file with a comment explaining
  why; never `# noqa` inline without a reason.

## Performance

- `gather_facts: false` at the play level when no fact is used.
  Fact gathering is ~5s per host.
- `strategy: free` lets hosts run independently instead of in
  lockstep. Big speedup on big inventories.
- `serial: 25%` for rolling updates.
- `async: 600` + `poll: 0` for fire-and-forget tasks (e.g. long-running
  installs).
- Use `delegate_to: localhost` for tasks that just talk to APIs;
  no point SSH-ing to a target to call AWS.

## ansible.cfg

Minimal config:

```ini
[defaults]
inventory       = inventory/production
roles_path      = roles
collections_path = collections
host_key_checking = false
stdout_callback = yaml
retry_files_enabled = false
forks = 25

[ssh_connection]
pipelining = true
control_path = ~/.ssh/cm-%%r@%%h:%%p
```

- `stdout_callback = yaml` makes output readable.
- `pipelining = true` for 2x speedup on small tasks.
- `host_key_checking = false` only inside CI or known-good networks.

## Avoid

- Logic in playbooks. Roles hold logic; playbooks orchestrate.
- `with_*` loops (legacy). Use `loop`.
- Bare `shell:` without `creates:` or `changed_when:`.
- Re-templating values that are already Jinja in `when:`.
- Vars set on the command line (`-e`) except for one-shot operations.
- Free-form `name:` strings that don't describe state.
- Roles that import other roles. Use `dependencies:` in `meta/main.yml`
  for that, and only when truly required.
- Mixing `ansible.builtin.*` and `ansible.windows.*` on the same target.
