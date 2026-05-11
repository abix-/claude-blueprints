---
name: jinja
description: Jinja2 templating standards. Use when writing Jinja templates, especially in Ansible playbooks/roles, AWX operator/awx repos, and any YAML-with-templating context.
user-invocable: false
version: "1.0"
updated: "2026-05-11"
---
# Jinja2

## Core
- Whitespace control matters. Use `{%- ... -%}` to strip surrounding whitespace in YAML output. Default Jinja preserves newlines.
- In Ansible, prefer filters over Python-style expressions. `value | default('x')` not `value or 'x'`.
- Native types: enable `jinja2_native = True` (Ansible) when templating non-string values, or wrap with `| int` / `| bool`.
- Quoting in YAML: a value starting with `{{` MUST be quoted. `key: "{{ var }}"`. Unquoted breaks the YAML parser.

## Filters
- `default(value, true)` to coerce empty strings to the default.
- `to_nice_yaml` / `to_nice_json` for embedding structured data.
- `mandatory` to fail loudly when a var is missing: `{{ var | mandatory }}`.
- Custom filters in `filter_plugins/` (Ansible) or registered via `env.filters` (Python).

## Control flow
- `{% if x %}` blocks: end with `{% endif %}` on its own line for readability.
- Loops: `{% for x in xs %}{{ x }}{% if not loop.last %}, {% endif %}{% endfor %}`. Use `loop.last`, `loop.first`, `loop.index` instead of manual counters.
- `{% set %}` for local variables. In Ansible, prefer `set_fact` outside templates when possible.

## Ansible-specific
- Use `{{ }}` only where Ansible expects it. `when:` clauses already eval as Jinja, so `when: x == 'y'` not `when: "{{ x }}" == 'y'`.
- Lookup plugins (`lookup('file', ...)`, `lookup('env', ...)`) run on the control node, not the target.
- `hostvars[inventory_hostname]` for per-host facts.

## Avoid
- Complex logic in templates. Push it into Python filter plugins or `set_fact`.
- Embedding multi-line shell scripts via templating. Use `script:` module with a real file.
- String concatenation with `+` for paths. Use the `path_join` filter or `os.path.join` in a plugin.
