---
name: typescript
description: TypeScript and JavaScript development standards. Use when writing TS/JS, including browser extension bootstrap shims, web frontends, and Node tooling.
user-invocable: false
version: "1.0"
updated: "2026-05-11"
---
# TypeScript / JavaScript

## Core
- TypeScript first. Plain JS only for tiny bootstrap shims (extension content scripts, popup loaders) where adding a build step is not worth it.
- `strict: true` always. No implicit any, no `// @ts-ignore` (use `// @ts-expect-error` with a reason).
- ES modules (`import`/`export`). No CommonJS in new code.
- Target modern runtimes: ES2022+ for browser, current LTS for Node.
- Prefer named exports. Default exports only when the file has exactly one obvious thing (React component, etc.).

## Types
- Model with discriminated unions, not enums. `type Action = { kind: "block" } | { kind: "allow"; pattern: string }`.
- `unknown` over `any`. Narrow with type guards.
- Use `satisfies` to verify literal shapes without widening.
- Avoid type assertions (`as Foo`). If you need one, write a guard.
- Const objects: `as const` for literal narrowness.

## Async
- `async`/`await` everywhere. No `.then` chains in new code.
- `Promise.all` for parallel, `for await` for streams.
- Always handle rejection. No floating promises (enable `no-floating-promises` lint).

## Runtime
- `fetch` over libraries (axios, node-fetch). Standard in Node 18+ and all browsers.
- Logging: `console.log` for browser/extension; structured JSON via `pino` for servers.
- Errors: extend `Error` with a `code` field for catchable conditions.

## Browser / extensions
- MV3 service workers: persistent state goes in `chrome.storage`, never module globals (workers get killed).
- Bootstrap shims (Hush-style) stay tiny: lazy-load WASM, no business logic.
- Bundling: use `esbuild` or `wasm-pack` outputs; avoid Webpack unless required by a framework.

## Testing
- Vitest for unit tests. Jest only for legacy codebases.
- Playwright for browser/e2e. No Selenium.
- Co-locate tests as `foo.test.ts` next to `foo.ts`.

## Avoid
- `var`. Use `const`, then `let` only when reassignment is real.
- `==`. Always `===`.
- `Function.prototype.bind` in hot paths. Arrow capture is faster and clearer.
- Decorators outside framework requirements (NestJS, etc.).
- `interface` for non-object shapes. Use `type`.
