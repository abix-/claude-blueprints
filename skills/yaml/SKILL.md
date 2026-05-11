---
name: yaml
description: YAML standards for config files, Ansible playbooks, k8s manifests, GitHub Actions, docker-compose, and any project config. Built from the YAML 1.2 spec, yamllint defaults, and the practical pitfalls (Norway problem, type coercion, anchor gotchas).
user-invocable: false
version: "2.0"
updated: "2026-05-11"
---
# YAML

YAML 1.2 is the current spec (2009, last clarified 2021). Most tooling
implements YAML 1.1 by default for backwards compatibility, which means
old type-coercion rules (the "Norway problem" with `NO`, `on`/`off`)
still bite. Write for the lowest common denominator: be explicit.

For Jinja templating inside YAML, read the `jinja` skill.

## File structure

- **2-space indent.** Never tabs (YAML rejects them outright).
- **LF line endings**, UTF-8, no BOM, final newline at EOF.
- **One document per file** by default. Multi-doc (`---` separator)
  only when the tool expects it (k8s manifests, helm).
- **Optional top `---`**. Some tools require it, most don't. Pick
  one style per repo. Trailing `...` end-marker is almost never needed.
- **No trailing whitespace.** yamllint flags it; editors should auto-strip.

## Keys

Naming follows the host system's convention. Don't reinvent.

| Context | Convention | Example |
|---------|------------|---------|
| Ansible | snake_case | `become_user: root` |
| Kubernetes | camelCase | `metadata.creationTimestamp` |
| GitHub Actions | kebab-case (jobs/steps), snake_case (inputs) | `runs-on: ubuntu-latest` |
| docker-compose | snake_case | `depends_on: [db]` |
| Helm values | usually camelCase | `image.pullPolicy` |
| OpenAPI / JSON Schema | camelCase | `additionalProperties` |

When in doubt, **read three nearby keys and match**.

## Strings and quoting

YAML aggressively coerces unquoted strings. The safe rule: quote
anything that could plausibly be misinterpreted.

```yaml
# unquoted: usually fine
name: production
version: 1.0       # parsed as float!
count: 42

# MUST quote
ratio: "1.0"               # if you want string "1.0", not 1.0
empty: ""                  # not null
country_code: "NO"         # the Norway problem -- unquoted = bool false
country_code: "no"         # same
on_event: "on"             # YAML 1.1 booleans
phone: "+44 1234 5678"     # starts with `+` is fine, but quote for clarity
date_str: "2026-05-11"     # if you want string, not Date
ip: "10.0.0.1"             # IPv4 literals are floats in some parsers
key_path: "/etc/nginx"     # leading / is fine; leading {/[/&/*/etc. is not
template: "{{ var }}"      # Jinja must be quoted
percent: "50%"             # safe as-is but quote anyway

# Forbidden unquoted starts (will fail parse or coerce):
# { [ & * ! | > ' " % @ `
```

**Prefer double quotes** when you need escapes (`\n`, `\t`, `\"`).
Single quotes are pure literal: only `''` escapes a single quote, no
other escapes.

**The Norway problem:** YAML 1.1 parsers see `NO`, `yes`, `no`,
`on`, `off`, `Y`, `N`, `True`, `False` as booleans. YAML 1.2 drops
this but most libraries default to 1.1 behavior. **Always use
`true` / `false`.** Quote anything that looks like a YAML 1.1 boolean
if you mean a string.

## Numbers

- `42` integer, `1.5` float, `1e6` scientific, `0xff` hex, `0o17` octal
  (1.2) / `017` octal (1.1).
- Leading zero is octal in 1.1: `mode: 0644` -> `420 decimal`. **File
  modes must be quoted strings**: `mode: "0644"`.
- `.NaN`, `.inf`, `-.inf` are special floats.
- `_` digit separators (`1_000_000`) are NOT standard in YAML 1.2;
  some parsers accept them.

## Booleans and nulls

- **`true` / `false`**. Use these. Lowercase.
- **`null`**. Explicit. `~` is the legacy alternative; avoid.
- Empty value `key:` parses as null in most parsers. **Don't rely on
  it.** Write `key: null` if you mean null, `key: ""` for empty
  string.
- Don't mix conventions inside one file.

## Multi-line strings

Five styles. Pick the one that matches the consumer's needs:

```yaml
# `|` literal: preserve newlines, strip final
script: |
  set -euo pipefail
  echo "hello"

# `|-` literal strip: no final newline
script: |-
  one
  two

# `|+` literal keep: preserve all trailing newlines
script: |+
  one


# `>` folded: newlines become spaces, blank lines preserved
desc: >
  This is a long
  description that
  collapses to one line.

# `>-` folded strip: no final newline
desc: >-
  paragraph one.

# Plain multi-line (with continuation)
title: This is
  one logical
  line.
```

- `|` keeps the structure verbatim. Use for scripts, regex, exact
  whitespace.
- `>` collapses to a paragraph. Use for prose / long descriptions.
- The chomping indicator (`|`, `|-`, `|+`) controls trailing newlines.
- Indent the content relative to the key. Yamllint enforces 2 spaces.

## Lists and maps

```yaml
# block style (preferred)
servers:
  - name: a
    ip: 10.0.0.1
  - name: b
    ip: 10.0.0.2

# flow style (for short inline)
tags: [prod, web, us-east]
labels: {env: prod, tier: frontend}
```

- **Block style for anything non-trivial.** Diffs are clean,
  comments work, nesting is obvious.
- **Flow style only for short, atomic values.** A list of 3 strings,
  a 2-key map.
- **No trailing commas in flow style.** YAML rejects them.
- **Don't mix styles in the same list.** Pick one per file.
- Lists of one item: still use `- item` block style for diff
  friendliness.

## Anchors and aliases

```yaml
defaults: &defaults
  retries: 3
  timeout: 30

prod:
  <<: *defaults
  host: prod.example.com

stage:
  <<: *defaults
  host: stage.example.com
```

- `&name` defines an anchor, `*name` references it.
- `<<: *name` merge key (YAML 1.1 only). Removed in YAML 1.2 but
  widely supported. Helm and kustomize support it; some tooling
  doesn't.
- **Use sparingly.** Anchors confuse readers and many tools don't
  follow them across files.
- Acceptable for CI matrix shared values, k8s probe templates.
- When it gets complex, switch to Helm / kustomize / jsonnet /
  a real templating layer.

## Comments

```yaml
# Single-line comment.
servers:
  - name: prod      # inline comment
    timeout: 30     # in seconds
```

- `#` to end of line. **No block comments.**
- Comment the WHY: limits, rationale, links to issues. Skip what
  the key already explains.
- Inline comments need at least two spaces before `#`.

## Project-specific gotchas

### Ansible

```yaml
- name: Set port
  ansible.builtin.set_fact:
    port: "{{ env_port | int }}"        # string -> int
    enabled: true                        # not "yes"
    mode: "0644"                         # quoted! avoid octal trap
  when: env == "prod"                    # already Jinja; don't double-template
```

- Modes ALWAYS as quoted strings: `"0644"`.
- Booleans as `true` / `false`; ansible-lint flags `yes`/`no`.
- `when:` clauses are already Jinja; no `{{ }}` needed.

### Kubernetes

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: my-config
  labels:
    app: backend
data:
  config.json: |
    {
      "key": "value"
    }
```

- camelCase keys (`apiVersion`, `containerPort`). Required by the
  API server.
- Multi-doc files separated by `---`. Helm renders these.
- Embed JSON / shell scripts with `|` literal block.
- Resource quantities (`memory: "256Mi"`, `cpu: "500m"`) as quoted
  strings to avoid float coercion of plain numbers.

### GitHub Actions

```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run tests
        run: |
          cargo test
```

- `on:` is fine unquoted in Actions despite being a YAML 1.1 boolean
  (the parser is aware).
- Job IDs are kebab-case. Step `name`s are free-form.
- `uses:` references with `@v4` or `@<commit-sha>`. SHA-pin for
  security.
- Long `run:` scripts get `|` literal block. Multi-line bash works
  here.

### docker-compose

```yaml
version: "3.9"               # quote; YAML treats 3.9 as float
services:
  web:
    image: nginx:1.27.0      # pin; no :latest
    ports:
      - "8080:80"            # quote ports (1.1 bool: NO)
    depends_on:
      - db
```

- Version string quoted.
- Ports as quoted strings to avoid the `:` parsing oddities.
- `:latest` is operational debt. Always pin.

## Performance

YAML parse speed rarely matters, but a few things do:

- **Large YAML in tight loops:** prefer JSON. Most YAML parsers do
  a 2-pass parse and are 5-10x slower than JSON.
- **PyYAML is slow.** For Python, install `libyaml` C bindings
  (`pip install pyyaml[c]` or just `pyyaml` if available) so
  `yaml.CSafeLoader` is usable. 10x faster than the pure-Python
  loader.
- **Big anchor graphs blow up memory.** Anchors are expanded at
  parse time; a deeply shared anchor referenced 1000 times
  materializes the data 1000 times.
- **For huge configs (>10k lines), split into multiple files** and
  let the consumer (kustomize, helm, ansible) compose them.
  Single-file 50k-line YAMLs are an antipattern.

## Security

- **`yaml.load()` in Python (PyYAML) executes arbitrary code.**
  Always use `yaml.safe_load()`. Same applies to Ruby's `Psych.load`
  (vs `safe_load`).
- **Don't load untrusted YAML** with a non-safe loader. Custom tags
  (`!!python/object`) can execute code.
- **Schema validation** for any externally-supplied YAML: JSON
  Schema with a YAML-aware validator, or Pydantic / Cue / Dhall.

## Validation

- **`yamllint`** on every change. Standard config catches most
  issues. Wire into pre-commit:
  ```bash
  yamllint -d "{extends: default, rules: {line-length: {max: 120}}}" .
  ```
- **Schema validators** for typed configs:
  - k8s: `kubeval`, `kubeconform`, `kustomize build | kubectl apply --dry-run=server`
  - GitHub Actions: VS Code with the redhat.vscode-yaml extension auto-loads schemas
  - JSON Schema: `ajv-cli` or `check-jsonschema`
- **`yq`** for YAML in scripts (jq syntax). `yq eval '.servers[0]' file.yml`.

## Avoid

- **Tabs.** YAML will reject the file with a useless error.
- **Mixing flow and block** in the same list / map.
- **Trailing whitespace.** Editor should auto-strip on save.
- **Unquoted version-like strings**: `1.0`, `2.10` are floats; quote.
- **Unquoted Norway-problem values**: `no`, `yes`, `on`, `off`,
  `Y`, `N`, `True`, `False`, `NULL`.
- **Octal-looking modes** unquoted: `0644` becomes 420 decimal.
- **Code in YAML.** Multi-line shell scripts in playbooks get long
  fast; extract to a real file and call it.
- **Repeating yourself.** Anchors and aliases are the workaround;
  templating layers (Helm, kustomize, jsonnet, jinja) are the fix.
- **Leading zero numbers** unquoted. `version: 042` is 34 decimal in
  YAML 1.1.
- **Single quotes for keys.** Keys are strings; just leave them
  unquoted unless they start with a forbidden char.
- **Bare `key:`** when you mean empty string. Be explicit:
  `key: ""` or `key: null`.
