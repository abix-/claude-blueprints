---
name: codex-config
description: Use when inspecting or changing Codex configuration, AGENTS.md, skills, plugins, hooks, MCP servers, sandbox settings, or user and project instruction discovery.
---

# Codex Configuration

Use the current Codex manual as the authority. Run the OpenAI documentation
skill's `fetch-codex-manual.mjs` helper and read only the relevant sections
before changing Codex configuration.

## Configuration locations

- User settings: `~/.codex/config.toml`
- User hooks: `~/.codex/hooks.json` or inline hooks in user config
- User guidance: `~/.codex/AGENTS.md`
- User skills: `~/.agents/skills/<skill-name>/SKILL.md`
- Project settings: `<repo>/.codex/config.toml`
- Project hooks: `<repo>/.codex/hooks.json` or inline hooks in project config
- Project guidance: `<repo>/AGENTS.md`, with closer nested files taking precedence
- Project skills: `<repo>/.agents/skills/<skill-name>/SKILL.md`

Project `.codex` configuration loads only for trusted projects.

## Skills

Each skill is a directory containing `SKILL.md`:

```text
skill-name/
  SKILL.md
  agents/openai.yaml
  scripts/
  references/
  assets/
```

Only `name` and `description` belong in `SKILL.md` frontmatter:

```yaml
---
name: skill-name
description: Use when the task matches specific conditions.
---
```

Use lowercase letters, digits, and hyphens for the name. Match the directory
name. Put optional Codex invocation policy and UI metadata in
`agents/openai.yaml`. To require explicit invocation:

```yaml
policy:
  allow_implicit_invocation: false
```

Invoke a skill explicitly with `$skill-name`. Codex can invoke skills
implicitly when their descriptions match and policy allows it. Do not use
Claude frontmatter such as `allowed-tools`, `argument-hint`,
`disable-model-invocation`, `user-invocable`, `context`, or `agent`.

Validate every changed skill with:

```powershell
$env:PYTHONUTF8 = "1"
python <skill-creator-dir>\scripts\quick_validate.py <skill-directory>
```

## AGENTS.md

Use `AGENTS.md` for durable behavioral guidance, repository commands,
verification requirements, and review expectations. Keep global guidance
personal and cross-repository. Keep repository guidance specific to that
repository. Codex discovers applicable files from the project root to the
current directory, with closer guidance taking precedence.

## Config and hooks

Use `config.toml` for models, reasoning, approvals, sandboxing, MCP servers,
plugins, skills, feature flags, and other durable runtime settings. Use
`hooks.json` or inline config hooks for lifecycle enforcement.

Validate configuration after editing:

```powershell
codex --strict-config --version
```

Never read or edit authentication files while auditing configuration.

## Windows

Codex runs natively on Windows through PowerShell and the Windows sandbox.
Use PowerShell unless the user explicitly requests WSL or another shell.
Do not carry Claude Bash, `settings.json`, status-line scripts, environment
variables, tool names, or slash-command instructions into Codex configuration.

## Plugins

Plugins distribute reusable skills and optional connections or tools. Installed
plugin settings live in `config.toml`. Plugin cache contents are generated
installation state, not the source to edit. Use the current Codex manual and
the plugin creator skill before creating or changing a plugin.
