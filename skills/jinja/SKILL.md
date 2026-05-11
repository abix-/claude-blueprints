---
name: jinja
description: Jinja2 templating standards. Use when writing Jinja templates, especially in Ansible playbooks/roles, AWX, and any YAML-with-templating context. Built from canonical Jinja docs + Ansible recommended practices.
user-invocable: false
version: "2.0"
updated: "2026-05-11"
---
# Jinja2

Templating engine used by Ansible, Salt, AWX, Flask, FastAPI,
MkDocs, and others. Same syntax everywhere, but the surrounding
context (Ansible vs. Python web) changes what you reach for.

For YAML formatting, read the `yaml` skill.
For Ansible-specific layout, read the `ansible` skill.

## Delimiters

- `{{ expr }}` -- output expression.
- `{% stmt %}` -- statement (for, if, set, include, etc).
- `{# comment #}` -- comment, never rendered.
- `{%- ... -%}` -- strip surrounding whitespace.
- `{%+ ... %}` -- force keep leading whitespace (rare).
- Inside double-curly: spaces around the expression always:
  `{{ name }}`, not `{{name}}`. Readable diffs.

## Whitespace control

The single most important Jinja skill. Default is **preserve all
whitespace and newlines**, which breaks YAML / JSON / INI output.

```jinja2
servers:
{%- for s in servers %}
  - name: {{ s.name }}
    ip: {{ s.ip }}
{%- endfor %}
```

- `{%-` strips whitespace BEFORE the tag (including the preceding newline).
- `-%}` strips whitespace AFTER the tag.
- `{%-` on `for` and `endfor` is the standard YAML pattern.
- Set environment-level: `trim_blocks=True, lstrip_blocks=True` in
  Python Jinja, or `trim_blocks: true` in Ansible config. Then bare
  `{%` already strips after the tag. Less code in the template,
  same output.
- Test the rendered output. Whitespace bugs are silent in templates
  and loud in the parser that consumes them.

## Quoting in YAML

```yaml
key: "{{ var }}"                    # always quote
host: {{ var }}                     # WRONG -- YAML parser sees a flow mapping
port: "{{ port | int }}"            # quote and coerce when type-sensitive
when: var == "prod"                 # in `when:`, no double-curly needed
```

- Any value starting with `{{` MUST be quoted.
- Any value starting with `{%` MUST be quoted.
- Values with `:` followed by space need quoting too (YAML rule, not
  Jinja).
- `when:` clauses already eval as Jinja. Don't double-template.

## Filters (most-used)

```jinja2
{{ name | default('anon') }}                # default if undefined or empty-string-as-falsy
{{ name | default('anon', true) }}          # default also when "" or 0
{{ var | mandatory }}                       # raise UndefinedError if missing
{{ items | length }}                        # count
{{ items | join(', ') }}                    # to string
{{ items | unique | sort }}                 # chained transforms
{{ d | dict2items }}                        # dict -> list of {key, value}
{{ d | combine(other, recursive=true) }}    # dict merge
{{ list | map(attribute='name') | list }}   # pluck
{{ list | selectattr('active', 'equalto', true) | list }}  # filter
{{ obj | to_nice_yaml(indent=2) }}          # pretty-print
{{ obj | to_json }}                         # compact JSON
{{ s | regex_replace('^prod-', '') }}       # regex sub (Ansible)
{{ path | basename }}                       # path ops (Ansible)
{{ ip | ipaddr('network') }}                # netaddr ops (Ansible)
```

- `default` filter is the most common safety net. `default('x', true)`
  triggers on falsy too (empty string, 0). Default behavior triggers
  only on undefined.
- Chain filters with `|`. Each filter is one transform.
- Filters are pure: same input, same output. Side effects belong
  in plugins.

## Tests vs filters

Tests follow `is`/`is not`. They return booleans.

```jinja2
{% if name is defined %}{{ name }}{% endif %}
{% if items is sequence %}...{% endif %}
{% if val is divisibleby 3 %}...{% endif %}
{% if items is iterable and items is not string %}...{% endif %}
```

- Common tests: `defined`, `undefined`, `none`, `string`, `number`,
  `sequence`, `mapping`, `iterable`, `eq`, `ne`, `gt`, `lt`,
  `in`, `divisibleby`.
- `is not` is the negation form: `{% if x is not defined %}`.
- Use `is defined` over `default()` when you need a clear branch,
  not a fallback value.

## Control flow

```jinja2
{% for s in servers %}
  {%- if loop.first %}- list start{% endif %}
  - {{ loop.index }}: {{ s.name }}
  {%- if not loop.last %},{% endif %}
{% else %}
  - empty list
{% endfor %}
```

- `loop.first`, `loop.last`, `loop.index` (1-based), `loop.index0`
  (0-based), `loop.length`, `loop.cycle('a','b')` for alternation.
- `{% for ... %}{% else %}` -- the `else` runs only if the iterable
  was empty. Cleaner than separate length checks.
- `{% set var = expr %}` for local vars. Scope is the template block.
- `{% set var %}block content{% endset %}` for block-valued vars
  (useful for capturing multiline content).

## Macros

```jinja2
{% macro field(name, required=false) -%}
<input name="{{ name }}"{% if required %} required{% endif %}>
{%- endmacro %}

{{ field('email', required=true) }}
```

- Reusable template fragments. Take params, return text.
- `import` to use from another file: `{% import "forms.j2" as forms %}`,
  then `{{ forms.field('name') }}`.
- Use macros over duplicating Jinja blocks. Don't use them for
  business logic; that's what Python filter plugins are for.

## Native types (Ansible)

```ini
# ansible.cfg
[defaults]
jinja2_native = True
```

- Without native: everything Jinja outputs is a string. `{{ 1 + 1 }}`
  becomes `"2"`.
- With native: `{{ 1 + 1 }}` is the integer `2`, `{{ [1, 2] }}` is
  a real list. Necessary when passing structured data between tasks.
- Trade-off: some templates that depended on string coercion break.
  Enable per-project, not globally without testing.

## Custom filters and plugins (Ansible)

Drop a `filter_plugins/<name>.py` next to your role or playbook:

```python
# filter_plugins/path.py
import os

class FilterModule:
    def filters(self):
        return {'path_join': os.path.join}
```

Use: `{{ ['/etc', 'nginx', 'sites'] | path_join }}`.

- Push complex logic into Python plugins. Templates stay readable.
- Filters are stateless, side-effect free. For side effects (querying
  a DB, calling an API), write an Ansible module instead.

## Performance

Template rendering is rarely the bottleneck in Ansible runs, but
some patterns matter:

- **Hoist invariants out of loops.** Compute once in `set_fact` or
  in a `{% set %}` above the loop, then reuse.
- **Avoid `lookup()` inside hot loops.** Each call hits the
  filesystem / API.
- **`map` and `selectattr` are lazy generators.** Add `| list` only
  when you need to enumerate twice. Otherwise the chain composes
  without materializing intermediates.
- **`combine(recursive=true)` is O(n*m).** For deep merges of huge
  dicts, build the structure in a Python plugin instead.
- **Compiled environments** (`jinja2.Environment(cache_size=...)`)
  reuse parsed templates. Default cache is fine; set larger for
  servers rendering thousands of templates.
- **`async_results = false`** in production: don't load Jinja with
  Python's async machinery unless you need it. Sync is faster for
  template work.
- **String operations in Python plugins beat filter chains** in
  Jinja. Three-filter chains are fine; ten-filter chains are a
  rewrite signal.

## Security

- **Untrusted templates are remote code execution.** Jinja's
  `Environment(autoescape=True)` only escapes HTML output; it does
  not sandbox the template language. User-supplied templates need
  `SandboxedEnvironment` or a different tool.
- `autoescape=True` for HTML, XML, or any context where injection
  matters. Default off for non-HTML formats.
- Use `| safe` only when you're sure the content is pre-escaped.
- Never log a fully rendered config that contains secrets; redact
  before output.

## Debugging

- `{{ var | type_debug }}` (Ansible) prints the Python type. Useful
  when type juggling causes surprises.
- `{{ var | to_nice_yaml }}` dumps a value for inspection.
- `ansible-playbook --syntax-check` catches structural errors.
- `ansible-playbook --check --diff` shows would-be changes without
  applying.
- In Python, `Environment(undefined=StrictUndefined)` makes typos
  loud. Default is silent empty string, which is the worst possible
  failure mode for production.

## Ansible-specific

- `hostvars[inventory_hostname]` -- this host's facts and vars.
- `hostvars[other_host]` -- another host's vars (requires that play
  to have run against the other host).
- `groups['groupname']` -- list of hostnames in a group.
- `inventory_hostname`, `ansible_facts.os_family`, `ansible_default_ipv4.address`
  -- the most-used built-ins.
- Lookups run on the control node:
  - `lookup('file', 'path')` -- file content from the control node.
  - `lookup('env', 'NAME')` -- env var from the control node.
  - `lookup('pipe', 'cmd')` -- run command on the control node.
  - `lookup('vars', 'name')` -- indirect var ref.
- For target-node values, use `slurp`, `command`, or a module.

## Avoid

- Complex logic in templates. Push to filter plugins or `set_fact`.
- Embedding multi-line shell scripts via templating. Use the
  `script:` module with a real file.
- String concat with `+` for paths. Use the `path_join` filter.
- Double-templating in `when:` / `failed_when:`. They're already
  Jinja.
- Mutating data in templates. Templates are pure functions of input.
- `{% raw %}` blocks for delimiters in normal content. If you need
  literal `{{`, escape it: `{{ '{{' }}`. Use `{% raw %}` only when
  multiple consecutive delimiters appear (Helm templates, JavaScript
  in HTML).
- Relying on Python truthiness across templates and Ansible. Use
  explicit comparisons (`is defined`, `| length > 0`).
