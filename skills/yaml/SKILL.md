---
name: yaml
description: YAML standards for config files, Ansible playbooks, k8s manifests, GitHub Actions, and any project config. Use when writing or editing .yml/.yaml.
user-invocable: false
version: "1.0"
updated: "2026-05-11"
---
# YAML

## Core
- 2-space indent. Never tabs (YAML rejects them).
- LF line endings, UTF-8, no BOM. Final newline at EOF.
- Lowercase keys, kebab-case for multi-word: `start-time`, not `startTime` or `start_time`. Exception: respect the host system's convention (k8s = camelCase, Ansible = snake_case).
- One document per file unless multi-doc (`---` separator) is the intent (k8s manifests, helm).

## Quoting
- Don't quote strings unless required. Quote when:
  - The value starts with `{`, `[`, `&`, `*`, `!`, `|`, `>`, `'`, `"`, `%`, `@`, ``` ` ```.
  - The value is `yes`, `no`, `true`, `false`, `on`, `off`, `null`, `~`, or a number, but you want a string.
  - The value contains `:` followed by space, or `#`.
  - It is a Jinja expression: `key: "{{ var }}"`.
- Prefer double quotes when escaping is needed. Single quotes for literals.

## Booleans and nulls
- `true` / `false`. Avoid `yes`/`no`, `on`/`off` (legacy YAML 1.1, causes Norway problem with `NO`, `no` becoming bool).
- Explicit `null` over empty. Don't leave bare keys (`key:`) unless you mean null.

## Strings
- Multi-line literal (preserve newlines): `|` or `|-` (strip trailing newline).
- Multi-line folded (collapse to spaces): `>` or `>-`.
- `|2` to force indent when content starts with whitespace.

## Lists and maps
- Block style (`- item`) for readable lists. Flow style (`[a, b]`) only for short inline lists.
- Maps: block style by default. Flow (`{k: v}`) only for tiny single-line maps.
- Trailing commas: not allowed in flow style. Don't add them.

## Anchors and aliases
- Use sparingly. They confuse readers and most tooling does not follow them when merging.
- Acceptable for repeated CI matrix values or shared k8s sidecars. Pull into a separate file with `!include` (helm, kustomize) once it gets complex.

## Comments
- `#` to end of line. No block comments in YAML.
- Comment WHY, not WHAT. The key name already tells you what.

## Project-specific
- **Ansible:** `when:` is already Jinja. Don't double-template (`when: "{{ x }} == 'y'"` is wrong). See the `jinja` and `ansible` skills.
- **k8s:** camelCase keys, even though it violates the general rule. Required by the API.
- **GitHub Actions:** `on:` keyword needs quoting in some YAML parsers (`"on":`) but Actions accepts both. Pick one style per repo.
- **docker-compose:** keep version pinning explicit. `image: nginx:1.27.0`, not `:latest`.

## Validation
- `yamllint` on every change. Wire into pre-commit.
- For schemas (k8s, Actions), use editor LSP (red-hat YAML extension) so you see drift immediately.

## Avoid
- Tabs anywhere. YAML will reject the file.
- Mixing flow and block styles in the same list.
- Trailing whitespace.
- YAML for code (write code in code). YAML is config.
