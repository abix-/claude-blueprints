---
name: claude-code-deep-dive
description: Deep architectural reference for Claude Code internals -- the query loop, The Prestige (prompt caching illusion), tool orchestration, state management, and cost model. Use when reasoning about Claude Code behavior, optimizing token usage, or debugging cache breaks.
user-invocable: true
version: "1.1"
updated: "2026-03-31"
---
# Claude Code Deep Dive

Reverse-engineered from Claude Code source (TypeScript/Bun/Ink). This is the actual architecture, not documentation -- verified against the code.

## The Prestige -- How Claude Code Actually Works

Every great magic trick consists of three parts:

**The Pledge** -- the magician shows you something ordinary. A conversation. You type a message, Claude responds. It looks like a continuous session.

**The Turn** -- the magician makes it disappear. After every response, the server-side Claude instance is gone. No memory. No state. No session. The previous instance is dead.

**The Prestige** -- the magician brings it back. A brand new Claude instance appears, fed the entire conversation from the beginning. It replays every token, reconstructs the same mental state, and responds as if it was there the whole time.

The audience (you) sees one continuous conversation. Behind the curtain, it's a new clone every turn. The old one is destroyed. You pay for the cloning.

| The Prestige | Claude Code |
|---|---|
| The Pledge | User sees a continuous conversation |
| The Turn | Server-side instance dies (stateless API) |
| The Prestige | New instance cloned from full history replay |
| The cloning machine | Prompt cache (KV tensor checkpoint) |
| Cost of the machine | Cache read tokens (10% of input price) |
| Drowning the original | Server discards all state between calls |
| Angier's diary | Compaction summary (cliff notes for the clone) |
| The audience | The user, who never sees the swap |

This framing is used throughout this document. "The Prestige" = the per-turn full-history replay. "The cloning machine" = prompt caching. "Angier's diary" = compaction.

---

## 1. Startup & Entry

**File**: `main.tsx`

Startup is performance-critical. Three side-effects fire before any imports evaluate:

1. `profileCheckpoint('main_tsx_entry')` -- marks wall-clock entry
2. `startMdmRawRead()` -- fires MDM subprocess (plutil/reg query) in parallel with imports
3. `startKeychainPrefetch()` -- fires macOS keychain reads (OAuth + legacy API key) in parallel

Then:
- Commander CLI parses flags (`--agent`, `--model`, `--remote`, `--bare`, etc.)
- GrowthBook feature flags initialize (A/B testing, feature gates)
- `init()` runs: config validation, env vars, TLS cert setup, graceful shutdown handlers
- `launchRepl()` dynamically imports `App` + `REPL` components and renders via Ink

**Feature flags** use `feature()` from `bun:bundle` for **build-time dead code elimination**. When false, the entire `require()` block is stripped from the bundle. This is why imports look like:
```typescript
const VoiceCommand = feature('VOICE_MODE')
  ? require('./commands/voice/index.js').default
  : null
```
Not runtime toggling -- compile-time stripping.

---

## 2. State Management

**Files**: `state/AppStateStore.ts`, `state/store.ts`, `state/AppState.tsx`

### AppState

A single `DeepImmutable<>` object (~200+ fields) tracking everything:
- Permission context (mode, allow/deny/ask rules, bypass availability)
- Model settings (main loop model, session model, fast mode)
- MCP state (connections, tools, commands, resources, elicitations)
- Bridge state (enabled, connected, session active, URLs)
- Speculation state (active speculation, boundary, pipelined suggestions)
- Team/swarm state (teammates, coordinator index, view mode)
- UI state (expanded view, footer selection, status line text)

`DeepImmutable<>` is a recursive mapped type making everything readonly. Prevents accidental mutation.

### Store

Simple external store pattern (like Zustand):
- `getState()` / `setState(prev => newState)` / `subscribe(listener)`
- Wrapped in React Context via `AppStateProvider` for Ink components
- Components read via `useSyncExternalStore()` -- React 18 concurrent-safe

### State flow for subagents

`setAppState` is no-op for async subagents (`createSubagentContext`). Infrastructure that outlives a turn uses `setAppStateForTasks` which always reaches the root store.

---

## 3. The Query Loop -- Heart of the Agent

**Files**: `query.ts`, `QueryEngine.ts`

### query() -- the async generator

`query()` is an async generator that implements the agentic loop:

```
User message
  -> Build system prompt (CLAUDE.md + git + context)
  -> Call Claude API (streaming)
  -> Parse assistant response
     -> Text blocks: yield to UI for rendering
     -> Tool use blocks: dispatch to tool implementations
        -> Permission check (canUseTool)
        -> Execute tool (BashTool, FileEditTool, etc.)
        -> Return tool_result as UserMessage
        -> Loop back to API call
  -> Stop reason reached -> return Terminal
```

### Mutable loop state

```typescript
type State = {
  messages: Message[]
  toolUseContext: ToolUseContext
  autoCompactTracking: AutoCompactTrackingState | undefined
  maxOutputTokensRecoveryCount: number
  hasAttemptedReactiveCompact: boolean
  maxOutputTokensOverride: number | undefined
  pendingToolUseSummary: Promise<ToolUseSummaryMessage | null> | undefined
  stopHookActive: boolean | undefined
  turnCount: number
  transition: Continue | undefined  // why the previous iteration continued
}
```

### Recovery mechanisms

- **Max output tokens**: Retries up to 3 times when the model hits output limits
- **Auto-compact**: When context exceeds ~80% of window, summarizes older messages
- **Reactive compact**: Feature-flagged mid-stream compaction for prompt-too-long errors
- **Token budget**: Optional cap on total output tokens per turn (API `task_budget`)
- **Fallback model**: Falls back to a different model on certain API errors

### QueryEngine -- SDK/headless wrapper

One `QueryEngine` per conversation. Each `submitMessage()` starts a new turn. It:
- Builds system prompt from CLAUDE.md + git status + user context
- Wraps `canUseTool` to track permission denials for SDK reporting
- Manages session persistence and transcript recording
- Clears `discoveredSkillNames` per turn (prevents unbounded growth in SDK mode)

---

## 4. Tools

**Files**: `Tool.ts`, `tools.ts`

### ToolUseContext -- the god object

Every tool call receives `ToolUseContext`, containing:
- `options`: commands, tools, model, MCP clients, agent definitions, thinking config
- `abortController`: cancellation signal
- `readFileState`: LRU file content cache (prevents re-reading unchanged files)
- `getAppState` / `setAppState`: state access
- `setToolJSX`: render Ink components during tool execution
- `messages`: full conversation history
- `fileReadingLimits` / `globLimits`: per-tool resource caps
- `contentReplacementState`: tool result budget tracking
- `renderedSystemPrompt`: frozen system prompt for fork subagents

### Tool pool assembly

`getAllBaseTools()` returns ~30+ tools. Assembly pipeline:

1. `getAllBaseTools()` -- all possible tools (feature-flagged at build time)
2. `getTools()` -- filters by deny rules, REPL mode, simple mode, `isEnabled()`
3. `assembleToolPool()` -- merges built-in + MCP tools, deduplicates, sorts by name
4. Sort order matters: built-ins are a contiguous prefix for prompt-cache stability

### Tool concurrency

`toolOrchestration.ts` partitions tool calls into batches:
- **Concurrency-safe** (read-only): Run in parallel, up to `MAX_TOOL_USE_CONCURRENCY` (default 10)
- **Non-safe** (mutating): Run serially
- Context modifiers from concurrent tools are queued and applied after the batch

### Streaming tool execution

`StreamingToolExecutor` enables tool execution to begin before the full tool input is streamed. Tools opt in via `eager_input_streaming: true` on their schema.

---

## 5. Commands & Skills

**File**: `commands.ts`

### Command types

- `local`: Runs JS, returns text output (e.g., `/compact`, `/cost`)
- `local-jsx`: Renders Ink UI (e.g., `/config`, `/mcp`, `/keybindings`)
- `prompt`: Expands to text sent to the model (skills, `/review`, `/commit`)

### Loading pipeline

`getCommands()` merges sources in order (earlier wins on name conflict):
1. Bundled skills (registered synchronously)
2. Built-in plugin skills
3. Skill directory commands (`~/.claude/skills/`)
4. Workflow commands
5. Plugin commands + plugin skills
6. Built-in commands (COMMANDS array)

Filtered by:
- `meetsAvailabilityRequirement()` -- auth/provider gates (claude-ai vs console)
- `isCommandEnabled()` -- feature flag / user setting

### Dynamic skill discovery

Skills found during file operations (e.g., reading a SKILL.md in a project) are added to the command list mid-session via `getDynamicSkills()`. Inserting them triggers `clearCommandMemoizationCaches()`.

### SkillTool vs slash commands

- `getSkillToolCommands()` -- all prompt-type commands the model can invoke (includes skills + legacy commands)
- `getSlashCommandToolSkills()` -- only user-invocable skills shown in `/` typeahead

---

## 6. Permissions

**File**: `hooks/useCanUseTool.tsx`, `hooks/toolPermission/`

### Permission check pipeline

1. **Static rules**: `alwaysAllowRules`, `alwaysDenyRules`, `alwaysAskRules` from settings.json (per source: user, project, enterprise)
2. **`hasPermissionsToUseTool()`**: Evaluates rules, returns allow/deny/ask
3. **Classifier** (feature-flagged `TRANSCRIPT_CLASSIFIER`): Auto-mode classifier evaluates risk from conversation context
4. **Interactive prompt**: Shows permission dialog if needed (REPL mode)
5. **Coordinator/swarm handler**: Special paths for multi-agent scenarios
6. **Non-interactive fallback**: Auto-deny when `shouldAvoidPermissionPrompts` is true (background agents)

### Permission modes

- `default`: Ask for dangerous operations
- `plan`: Restricted -- only read operations and plan file edits
- `bypassPermissions`: Allow everything (YOLO mode, requires trust dialog)
- `auto`: Classifier-driven approval

### Denial tracking

`DenialTrackingState` counts consecutive denials. After a threshold, falls back to prompting even in auto mode. Subagents get `localDenialTracking` since their `setAppState` is a no-op.

---

## 7. The Cloning Machine -- Prompt Caching

### What it is

The Claude API is **stateless**. Every API call sends the entire conversation from scratch -- system prompt, tool definitions, and every prior message. On turn 50, even a 10-token user message triggers a request containing all 49 previous turns. This is **The Prestige** -- a new clone, built from the full script of every previous performance.

The "cache" is the **cloning machine** -- a KV tensor checkpoint, not a result cache. The server recognizes "I've computed attention for this exact token prefix before" and skips the transformer forward pass for those tokens. But it still loads the tensors, allocates GPU memory, and attends over cached positions during generation. The clone still needs to be built. The machine just builds it faster.

What Anthropic calls "cache read tokens" is the **cost of the cloning machine**. 90% off full input price. You still pay 10% for every token of every prior turn, every single time. And like Angier's machine, you pay every performance.

### Pricing reality

Per-million-token rates (from `modelCost.ts`):

| Model | Input | Cache Write | Cache Read | Cache Savings |
|-------|-------|-------------|------------|---------------|
| Opus 4.6 | $5.00 | $6.25 | $0.50 | 90% off input |
| Opus 4.6 fast | $30.00 | $37.50 | $3.00 | 90% off input |
| Sonnet 4.6 | $3.00 | $3.75 | $0.30 | 90% off input |
| Opus 4/4.1 | $15.00 | $18.75 | $1.50 | 90% off input |
| Haiku 4.5 | $1.00 | $1.25 | $0.10 | 90% off input |

Cache writes are 1.25x input price. They're the cost of creating the KV checkpoint. Amortized across subsequent reads.

Real-world example: 2.1 billion tokens/week on Haiku, 99.7% cache reads. Without caching: ~$2,120. With caching: ~$220. A stateful session API would reduce this to ~$9.

### How cache_control markers are placed

**File**: `services/api/claude.ts`

Three placement sites tell the server "cache everything up to here":

**A. System prompt** (`buildSystemPromptBlocks`):
```typescript
splitSysPromptPrefix(systemPrompt).map(block => ({
  type: 'text',
  text: block.text,
  ...(enablePromptCaching && block.cacheScope !== null && {
    cache_control: getCacheControl({ scope: block.cacheScope, querySource }),
  }),
}))
```

System prompt splits into up to 4 blocks:
1. Attribution header (`x-anthropic-billing-header`) -- no cache scope
2. CLI system prompt prefix (matched against `CLI_SYSPROMPT_PREFIXES`) -- org scope
3. Static content before `SYSTEM_PROMPT_DYNAMIC_BOUNDARY` -- global scope (shared across ALL users)
4. Dynamic content after boundary (CLAUDE.md, git status, etc.) -- no cache scope

**B. Tool schemas** (`toolToAPISchema` in `utils/api.ts`):
- The last tool in the sorted array gets `cache_control`
- Sorting is alphabetical by name for prompt-cache stability
- Built-in tools are a contiguous prefix; MCP tools are appended after

**C. Messages** (`addCacheBreakpoints`):
- **Exactly one** message-level `cache_control` marker per request
- Placed on the **last message** (or second-to-last for `skipCacheWrite` fork agents)
- Why only one: Mycro's KV page manager frees local-attention pages at cached positions NOT in `cache_store_int_token_boundaries`. Two markers would waste GPU memory by protecting a position nothing will resume from.

### getCacheControl() -- TTL and scope

```typescript
function getCacheControl({ scope, querySource }) {
  return {
    type: 'ephemeral',
    ...(should1hCacheTTL(querySource) && { ttl: '1h' }),
    ...(scope === 'global' && { scope }),
  }
}
```

- Default TTL: 5 minutes
- Extended TTL (1h): Gated by `should1hCacheTTL()`:
  - Anthropic employees (always eligible)
  - Subscribers within rate limits
  - Bedrock users with `ENABLE_PROMPT_CACHING_1H_BEDROCK` env var
  - Query source must match GrowthBook allowlist pattern
  - Eligibility is **latched** in session state -- never flips mid-session

### Cache scope: global vs org

- **Global** (`scope: 'global'`): Static system prompt shared across ALL first-party API users. Only for `getAPIProvider() === 'firstParty'`. Gated by `shouldUseGlobalCacheScope()`.
- **Org** (`scope: 'org'`): Per-organization caching. User-specific content (CLAUDE.md, tool schemas).
- **None** (no scope): Not cached at system level. Dynamic per-request content.

When MCP tools are present, global scope on the system prompt is skipped (`skipGlobalCacheForSystemPrompt`). MCP tools are per-user, so the tool section following the system prompt can't be globally cached. Falls back to org-level caching.

### Session stability -- preventing cache busting

Multiple values are **latched** (set once, never change within a session) to prevent cache key churn:

| Latched Value | Why | Bootstrap State Function |
|---|---|---|
| 1h TTL eligibility | Overage flip would change TTL | `setPromptCache1hEligible()` |
| AFK mode beta header | Auto-mode toggle would add/remove beta | `setAfkModeHeaderLatched()` |
| Fast mode beta header | `/fast` toggle would add/remove beta | `setFastModeHeaderLatched()` |
| Cache editing beta header | Feature enable would add beta | `setCacheEditingHeaderLatched()` |
| Tool schema base | GrowthBook flip would change tool descriptions | `toolSchemaCache.ts` |
| 1h cache allowlist | GrowthBook disk cache update would change patterns | `setPromptCache1hAllowlist()` |
| Thinking clear | Prevents thinking mode flips from busting cache | `setThinkingClearLatched()` |

### Cache break detection

**File**: `services/api/promptCacheBreakDetection.ts`

Two-phase detection system:

**Phase 1 (pre-call)**: `recordPromptState()` snapshots everything that could affect the cache key:
- System prompt hash (with and without `cache_control` -- catches scope/TTL flips)
- Tool schemas hash + per-tool hashes (identifies which tool changed)
- Model, fast mode, betas, auto-mode state, overage state, effort, extra body params
- Hash comparison against previous call detects what changed

**Phase 2 (post-call)**: `checkResponseForCacheBreak()` checks actual cache token response:
- Break detected when: cache reads drop >5% AND absolute drop > 2,000 tokens
- Correlates with phase 1 changes to explain the cause
- Generates explanations: "system prompt changed (+432 chars)", "tools changed (+1/-0 tools)", "model changed (opus-4-6 -> sonnet-4-6)"
- Checks time gap for TTL expiry (5min or 1h threshold)
- Writes diff files for debugging (`cache-break-XXXX.diff`)
- Logs `tengu_prompt_cache_break` analytics event with full diagnostic payload

Special cases that are NOT cache breaks:
- `notifyCompaction()`: Resets baseline after compaction / diary rewrite (legitimately reduces prefix -- the clone is reading new cliff notes, not the old script)
- `notifyCacheDeletion()`: Resets after `cache_edits` deletions (expected drop)
- Haiku models are excluded (different caching behavior)

### Cached microcompact (cache_edits)

A beta feature for surgical cache manipulation without full reprocessing:

1. `cache_reference: tool_use_id` is added to `tool_result` blocks within the cached prefix
2. `cache_edits: [{ type: 'delete', cache_reference: 'tool_use_id_123' }]` blocks are inserted into user messages
3. The server evicts specific KV pages by reference ID without invalidating the rest of the cache
4. Edits are "pinned" -- re-sent at their original message position in future calls
5. Deduplicated across blocks to prevent double-deletion

Constraints:
- `cache_reference` must appear "before or on" the last `cache_control` marker
- Strict "before" is used to avoid edge cases from `cache_edits` splicing
- New objects are created instead of mutating in-place to avoid contaminating secondary queries

---

## 8. Cost Tracking Pipeline

**Files**: `cost-tracker.ts`, `bootstrap/state.ts`, `utils/modelCost.ts`, `services/api/logging.ts`

### Token flow: API response to session totals

```
API streaming response
    |
    v
updateUsage()              -- merges message_delta usage into per-message total
    |                         (takes max of delta vs accumulated for input/cache tokens,
    |                          because deltas report cumulative not incremental)
    v  (on message_stop)
accumulateUsage()           -- adds message usage to total session usage
    |                         (simple addition across all fields)
    v
calculateUSDCost()          -- multiplies tokens by per-model cost tiers
    |                         (looks up ModelCosts by canonical model name)
    v
addToTotalSessionCost()     -- updates:
    |                         - bootstrap/state MODEL_USAGE counters (per-model)
    |                         - OTel counters (cost, tokens by type)
    |                         - Recursive for advisor sub-usage
    v
saveCurrentSessionCosts()   -- persists to project config for session resume
```

### Per-model tracking

`ModelUsage` tracks per model:
```typescript
{
  inputTokens: number
  outputTokens: number
  cacheReadInputTokens: number
  cacheCreationInputTokens: number
  webSearchRequests: number
  costUSD: number
  contextWindow: number
  maxOutputTokens: number
}
```

Accumulated by canonical short name (e.g., `claude-opus-4-6` not the full model string).

### Session cost display

`formatTotalCost()` renders:
```
Total cost:            $12.34
Total duration (API):  5m 23s
Total duration (wall): 12m 45s
Total code changes:    42 lines added, 7 lines removed
Usage by model:
    claude-opus-4-6:  1,234 input, 5,678 output, 98,765 cache read, 432 cache write ($12.34)
```

### Session persistence

Costs are saved to project config (`saveCurrentSessionCosts()`) and restored on resume (`restoreCostStateForSession()`). Only restores if the session ID matches the last saved session.

---

## 9. Context Assembly

**File**: `context.ts`

### System context (memoized per session)

`getSystemContext()` returns:
- `gitStatus`: Branch, default branch, status (truncated at 2K chars), recent 5 commits, git user name
- `cacheBreaker`: Optional injection for cache breaking (ant-only debugging)

Skipped in CCR (Claude Code Remote) and when git instructions are disabled.

### User context (memoized per session)

`getUserContext()` returns:
- `claudeMd`: Concatenated CLAUDE.md files from cwd walk up to home + additional directories
- `currentDate`: Today's date in local ISO format

CLAUDE.md discovery:
1. Walk directory tree from cwd up to home
2. Read each `.claude/CLAUDE.md` and project-root `CLAUDE.md`
3. Filter injected memory files
4. Cache result for auto-mode classifier (avoids circular dependency)

Disabled by:
- `CLAUDE_CODE_DISABLE_CLAUDE_MDS` env var
- `--bare` mode (unless `--add-dir` explicitly provided)

---

## 10. Angier's Diary -- Compaction

**Files**: `services/compact/autoCompact.ts`, `services/compact/compact.ts`, `services/compact/prompt.ts`

Compaction is not compression. It is an **amnestic reset** -- controlled memory destruction with a handwritten summary left behind for the next clone.

In The Prestige terms: the diary gets too long to carry. Someone writes cliff notes. The next clone reads the cliff notes instead of the full diary. It never actually lived those events -- it just read the summary and pretends it did.

### What actually happens

1. **Panic**: Context exceeds ~87% of window (configurable)
2. **One last Prestige**: A forked agent replays the ENTIRE conversation one more time, with a prompt asking it to summarize itself into 9 sections (Primary Request, Key Concepts, Files & Code, Errors & Fixes, Problem Solving, All User Messages, Pending Tasks, Current Work, Next Step)
3. **Amnesia**: All prior messages are deleted. File state cache is cleared. The conversation before the compact boundary is gone from API calls forever
4. **Scramble**: The system re-attaches critical context -- recently-read files, current plan, invoked skills, tool schemas, MCP instructions, session hooks -- because the summary won't have captured all of it
5. **Hope**: The model continues from the summary with no actual memory. If the summary missed a detail, it's gone

### The cost of writing the diary

Compaction is not free. The summary API call is itself a full Prestige:
- Sends entire conversation as input (cache read rates)
- Generates ~5-20K tokens of summary output (at output token rates, 5x input)
- Then post-compact re-reads files (more tokens next turn)

On a 200K context compaction with Opus 4.6: ~$0.15-0.25 per compaction.

### When it triggers

```
effective_context_window = context_window - 20,000 (reserved for summary output)
auto_compact_threshold = effective_context_window - 13,000 (buffer)
```

For 200K context: triggers at ~167K tokens. For 1M context: triggers at ~967K tokens.

Override with `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE` (percentage of effective window).

### Partial compaction (the diary addendum)

When a compact boundary already exists, `PARTIAL_COMPACT_PROMPT` only summarizes messages AFTER the last boundary, keeping the previous summary intact. The new clone reads the old cliff notes plus a new addendum, instead of re-summarizing already-summarized content.

### Circuit breaker

After 3 consecutive compaction failures, stops retrying. Prevents wasting API calls when context is irrecoverably over limit (e.g., prompt_too_long errors). BQ data showed 1,279 sessions with 50+ consecutive failures.

### Impact on the cloning machine

Compaction destroys the cached prefix (conversation history is rewritten as a summary). The next Prestige builds a completely different clone -- different token sequence, no cache hit on the old prefix. `notifyCompaction()` resets the cache break detection baseline so the expected drop in cache reads isn't flagged as a bug.

---

## 11. Tasks (Background Work)

**File**: `Task.ts`, `tasks.ts`

### Task types

| Type | Prefix | Description |
|---|---|---|
| `local_bash` | `b` | Shell command running in background |
| `local_agent` | `a` | Subagent (Agent tool) |
| `remote_agent` | `r` | Remote agent (CCR) |
| `dream` | `d` | Auto-dream background processing |
| `local_workflow` | `w` | Workflow script execution |
| `monitor_mcp` | `m` | MCP server monitor |
| `in_process_teammate` | `t` | Swarm teammate |

Task IDs: prefix + 8 random chars from `[0-9a-z]` (36^8 ~ 2.8 trillion combinations, resists brute-force symlink attacks).

### Task lifecycle

`pending` -> `running` -> `completed` | `failed` | `killed`

Terminal states checked by `isTerminalTaskStatus()`. Guards against injecting messages into dead teammates, evicting finished tasks, orphan cleanup.

---

## 12. UI Layer

**Files**: `ink.ts`, `screens/REPL.tsx`, `replLauncher.tsx`

### Ink (React for terminals)

All renders wrapped with `ThemeProvider`. Exports themed versions of Box/Text (`ThemedBox`, `ThemedText`) and design system primitives.

### REPL screen

The main interactive screen handles:
- Text input (vim mode, arrow key history, clipboard, paste handler)
- Message rendering with streaming
- Permission dialogs (queued, not inline)
- Tool progress indicators (spinner modes)
- Background task status
- Remote bridge indicators
- Speculation display (auto-accept, pipelined suggestions)

### Hook zoo

80+ React hooks in `hooks/` directory managing:
- Input: `useTextInput`, `useVimInput`, `useArrowKeyHistory`, `usePasteHandler`
- Tools: `useCanUseTool`, `useMergedTools`, `useMergedCommands`
- State: `useSettingsChange`, `useSkillsChange`, `useDynamicConfig`
- Features: `useVoice`, `useRemoteSession`, `useSwarmInitialization`
- UI: `useVirtualScroll`, `useBlink`, `useElapsedTime`, `useTerminalSize`

---

## 13. MCP (Model Context Protocol)

### Tool integration

MCP tools are prefixed `mcp__servername__toolname`. They're:
- Filtered by deny rules alongside built-in tools
- Sorted separately from built-ins for cache stability (appended as suffix)
- Excluded from global cache scope (per-user, can't be shared)

### Skill commands from MCP

When `MCP_SKILLS` feature is enabled, prompt-type MCP commands are filtered into skill listings via `getMcpSkillCommands()`.

### Elicitation

URL elicitations (`-32042` tool call errors) are handled via:
- `handleElicitation` callback in `ToolUseContext` (SDK/print mode)
- Queue-based UI path (REPL mode)

---

## 14. Agent & Subagent System

### Agent tool

The `AgentTool` spawns subagents with isolated contexts. Key design:
- `createSubagentContext()` makes `setAppState` a no-op (subagent can't corrupt parent state)
- `setAppStateForTasks` still reaches root store (background tasks outlive the turn)
- `renderedSystemPrompt` is frozen at fork time (prevents GrowthBook cold->warm divergence)
- `skipCacheWrite` for fork agents avoids polluting the KV cache with ephemeral branches

### Agent types

Built-in agents: `general-purpose`, `Explore`, `Plan`, `statusline-setup`, `claude-code-guide`
Custom agents: loaded from `~/.claude/agents/` directories

### Coordinator mode (feature-flagged)

When `COORDINATOR_MODE` is enabled, the main thread becomes a coordinator that only uses `AgentTool`, `TaskStopTool`, and `SendMessageTool`. Workers get the full tool set.

---

## Key Metrics to Watch

| Metric | Healthy | Unhealthy | What it means in Prestige terms |
|---|---|---|---|
| Cache read ratio | >95% of input | <80% | Cloning machine is working (high) or broken (low) |
| Cache breaks per session | 0-2 | >5 | Cloning machine had to be rebuilt (script changed) |
| Compaction frequency | 0-3 per session | >10 | Diary rewrites -- each one is a full Prestige + output cost |
| Cache write tokens | Small fraction of reads | Approaching reads | Machine is rebuilding every show instead of reusing |
| 1h TTL eligibility | Latched true | Flipping mid-session | Machine's rental agreement is unstable |
| Total cache reads/week | Context-dependent | Billions | The weekly cost of the illusion -- every clone, every turn |

### Reducing the cost of The Prestige

The cloning machine's cost scales with **script length x number of performances**. To reduce it:

1. **Shorter scripts** (compact earlier): `CLAUDE_CODE_AUTO_COMPACT_WINDOW=80000` and `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE=60` -- rewrite the diary when it hits 48K tokens instead of the default ~967K (1M model). Each subsequent clone reads a shorter script.

2. **Thinner scripts** (less context per turn): Trim CLAUDE.md files, remove unused MCP tools, use `Read` with `offset`/`limit`, use `Grep` with `head_limit`. Every token saved compounds across all future performances.

3. **Fewer performances** (shorter sessions): Start fresh conversations more often. A 50-turn session means 50 Prestiges. Five 10-turn sessions mean 50 Prestiges too, but with much shorter scripts at peak.

4. **Cheaper performers** (model choice): Haiku at $0.10/Mtok cache read vs Opus at $0.50/Mtok. But a smarter model that finishes in fewer turns can be cheaper overall -- fewer performances beats cheaper clones.
