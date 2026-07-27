---
name: codex-deep-dive
description: Use when investigating Codex execution behavior, instruction loading, configuration precedence, tools, context handling, sandboxing, approvals, skills, plugins, hooks, subagents, or product-surface differences.
---

# Codex Deep Dive

Investigate Codex from current sources. Do not transpose Claude Code behavior,
file names, commands, environment variables, pricing, or implementation details
onto Codex.

## Source order

1. Read the current Codex manual through the OpenAI documentation skill's
   `fetch-codex-manual.mjs` helper.
2. Read the relevant manual section and its linked source page.
3. Inspect current-session tool schemas and behavior when the question concerns
   a capability available in the session.
4. Inspect local Codex configuration or logs only when the user authorizes that
   scope. Never read authentication or credential files.
5. Use the public Codex repository for implementation details not covered by
   the manual.
6. State bounded uncertainty when none of those sources establishes the claim.

Current-session behavior takes precedence when it conflicts with published
documentation for the installed build. State the conflict explicitly.

## Keep these systems separate

- Prompt and thread context: one task or conversation
- `AGENTS.md`: durable user or repository guidance
- `config.toml`: runtime defaults and project overrides
- Skills: reusable workflows loaded by description or explicit `$skill-name`
- Plugins: installable bundles of skills and optional tools or connections
- MCP servers and app connectors: external data and actions
- Hooks: lifecycle enforcement around Codex events
- Automations: scheduled or recurring work
- Subagents: delegated tasks when current instructions permit them

Do not describe one system using another system's behavior.

## Configuration and instruction discovery

- User configuration: `~/.codex/config.toml`
- Project configuration: `.codex/config.toml` in trusted projects
- User guidance: `~/.codex/AGENTS.md`
- Repository guidance: `AGENTS.md` from the project root toward the current
  directory, with closer files taking precedence
- User skills: `~/.agents/skills`
- Repository skills: `.agents/skills`

Validate configuration with `codex --strict-config --version`.

## Windows execution

The Codex app runs natively on Windows through PowerShell and the Windows
sandbox. WSL is optional and must not be assumed. When diagnosing command
execution, distinguish:

- PowerShell syntax or executable lookup failures
- Windows sandbox policy denials
- approval requirements
- process-launch failures before the command starts
- failures returned by the launched command

Quote the exact layer and error. Do not rewrite a process-launch failure as a
shell syntax failure.

## Answer contract

For every behavioral claim:

- cite the current manual section, verified configuration line, tool schema, log
  line, or source line
- label inference as inference
- distinguish documented behavior from observed behavior
- do not claim undocumented internals from analogy with another agent product
