---
name: typescript
description: TypeScript and JavaScript standards. Sourced from abix-/chromium-extensions (Hush + filter-anything-everywhere). Use when writing TS/JS, including browser extension bootstrap shims, MV3 service workers, and small web frontends.
user-invocable: false
version: "2.0"
updated: "2026-05-11"
---
# TypeScript / JavaScript

Source repos: `abix-/chromium-extensions/filter-anything-everywhere`
(TS, jQuery, jest), `abix-/chromium-extensions/hush` (plain JS shims
fronting a Rust/WASM core).

The decision between TS and plain JS in this codebase is **not**
"always TS." Tiny bootstrap shims and content scripts are written in
plain JS because adding a build step costs more than it saves.

## When to pick TS vs JS

- **TypeScript:** anything with logic, types, multiple files, or that
  imports other modules. Use it for: options/popup pages, business
  logic, shared utility code, tests.
- **Plain JS:** MV3 service worker bootstrap (`hush/background.js`),
  content scripts that just bridge events, main-world page hooks
  (`hush/mainworld.js`). These need to load synchronously at
  `document_start`, can't afford a build step, and call WASM for the
  real work.
- Never JS in a 500+ line file unless there's a hard reason (e.g.
  document_start install ordering). At that point comment the reason
  at the top of the file.

## TypeScript compiler config

The `filter-anything-everywhere/tsconfig.json` baseline:

```jsonc
{
  "compilerOptions": {
    "target": "es2020",
    "module": "ESNext",
    "strict": true,
    "esModuleInterop": true,
    "forceConsistentCasingInFileNames": true,
    "skipLibCheck": true,
    "rootDir": "./extension"
  },
  "exclude": ["node_modules", "**/*.spec.ts"]
}
```

- `strict: true` always. Non-negotiable.
- `target: es2020` for browser; bump to `es2022+` only when targeting
  evergreen Node.
- `skipLibCheck: true` to avoid third-party `.d.ts` noise blocking
  your build.
- `forceConsistentCasingInFileNames: true` keeps macOS/Linux/Windows
  builds identical.

## Types

- Function param syntax mirrors `filter-anything-everywhere`:
  ```ts
  export function getCanonicalHostname(name:string) {
    if (name.startsWith('www.')) return name.substring(4);
    return name;
  }
  ```
  Tight, no space before `:`. Match the existing style.
- Discriminated unions over enums:
  ```ts
  type Action =
    | { kind: 'block' }
    | { kind: 'allow'; pattern: string };
  ```
- `unknown` over `any`. Narrow with type guards (`x instanceof X`,
  `typeof x === 'string'`).
- `interface` for object shapes that can be extended; `type` for
  unions and aliases. Pick one per shape and stay consistent.
- `as const` for literal narrowness on config objects.
- `as Foo` only at trust boundaries (JSON parse, DOM lookup); inside
  the codebase, write a guard.

## Narrowing patterns from real code

```ts
function getInputElement(): HTMLInputElement {
  const e = document.getElementById('input');
  if (!(e instanceof HTMLInputElement)) {
    console.log('expected input to be an HTMLInputElement', e);
    throw new Error('Expected input to be an HTMLInputElement');
  }
  return e;
}
```

- Use `instanceof` for DOM elements. The type system already knows.
- Throw early on impossible state. Don't propagate `null` further than
  the boundary.

## Async / await

```ts
async function addWord(word: string) {
  const items = await GetOptions();
  items.blacklist[word] = true;
  await chrome.storage.local.set({blacklist: items.blacklist});
  await rerender();
}
```

- `async`/`await` everywhere. No `.then` chains in new code.
- `Promise.all` for independent parallel work.
- No floating promises. If you don't await, `void cmd();` to make
  intent explicit.
- For event handlers that need async work, wrap in an arrow:
  `$('#btn').click(async () => { ... })`.

## Browser extensions (MV3)

The Hush patterns are the reference:

**Background service worker bootstrap** (`hush/background.js`):

```js
(async () => {
  try {
    const result = await migrateConfigSchema(chrome.storage.local);
    if (!result.skipped) console.log(`[Hush bg] migrated v${result.fromVersion} -> v${result.toVersion}`);
    await initWasm({ module_or_path: "./dist/pkg/hush_bg.wasm" });
    try { initEngine(); } catch (e) { console.error("[Hush bg] initEngine threw", e); }
    hushBackgroundMain();
  } catch (e) {
    console.error("[Hush bg] bootstrap failed", e);
  }
})();
```

- Top-level IIFE because module top-level await is workable but the
  IIFE pattern reads more clearly and isolates scope.
- Always log a tagged prefix (`[Hush bg]`) so multi-component
  extensions are debuggable.
- Catch broadly at the bootstrap boundary. One thrown error
  shouldn't kill the whole worker silently.

**Main-world content scripts** (`hush/mainworld.js`):

- IIFE wrapper (`(() => { ... })()`) to avoid leaking globals into
  the page.
- Use `window.__app_state__` only when you actually need page-script
  visibility; otherwise keep state inside the closure.
- Sync stubs first, then patch with the real implementation once
  WASM is ready. Comment why install order matters.
- `try { ... } catch (e) { /* ignore - detached document */ }` for
  ops that fail benignly after navigation.

**Storage**:

- `chrome.storage.local` is async, returns a Promise in MV3.
- Persistent state goes in storage; module globals get wiped when the
  worker idles.

**Manifest:**

- One service worker, one content script per surface. Add a
  `"web_accessible_resources"` entry for any file the page loads
  (WASM, main-world JS).

## Runtime APIs

- `fetch` over libraries (axios, node-fetch). Standard in all
  browsers and Node 18+.
- `URL` / `URLSearchParams` over manual parsing.
- `structuredClone(x)` over `JSON.parse(JSON.stringify(x))`.
- `crypto.randomUUID()` over npm uuid packages.
- `Intl.NumberFormat` / `Intl.DateTimeFormat` over moment/date-fns
  for display.

## Defensive coercion at boundaries

Pattern seen in `mainworld.js`:

```js
const v = el && el.dataset && el.dataset.hushSpoof;
if (!v) return false;
const parts = String(v).split(",");
```

- Page-script DOM access can return anything. `String(x)`,
  `Number(x)`, `Boolean(x)` to normalize before use.
- For data that crosses the WASM boundary, validate shape on the
  TS side. Don't trust untyped JSON from a page.

## Testing

- Jest + babel-jest is what `filter-anything-everywhere` uses. Tests
  named `foo.spec.ts`, co-located with sources.
- Test boundaries: pure functions, type-narrowing helpers, regex
  builders. Don't unit-test the DOM; use Playwright/Puppeteer for
  that.
- Vitest is the modern default for new projects. Jest is fine for
  legacy.

## Linting and formatting

- ESLint + Prettier. `eslint-config-google` + `eslint-config-prettier`
  is the chosen baseline (`filter-anything-everywhere/package.json`).
- `prettier --write .` before commit. Same config across the project.
- `husky` + `lint-staged` for pre-commit if the project has many
  contributors. Skip for solo projects.

## Comments

- Comment WHY, especially for browser quirks, race conditions, and
  install ordering. The `mainworld.js` header is the model: it
  explains why two install phases exist before any code starts.
- Comment exactly once. Don't restate types or method names.
- `// TODO(handle):` with a name; `// FIXME:` for known-broken.

## Performance

- `Set` / `Map` over plain objects for keyed lookups.
- Avoid creating closures inside hot loops; hoist the function.
- `for (let i = 0; i < arr.length; i++)` is fine for hot paths;
  `for...of` is fine everywhere else. Don't write `forEach` if you
  need `break`.
- `requestAnimationFrame` for DOM-dependent timing, never `setTimeout(fn, 16)`.
- For WASM-backed apps, push hot loops into Rust. JS is the bridge.

### V8 / engine specifics

V8 powers Chrome, Edge, Node.js, Deno (and most extension contexts).
Knowing its rules pays off for hot paths.

- **Hidden classes**: V8 builds a hidden class for every object
  shape. Adding properties in the same order across all instances
  keeps them sharing one class -- fast property access. Adding in
  different orders, or deleting properties, creates a new hidden
  class and slows lookups.
- **Monomorphic > polymorphic > megamorphic**: a function called
  with one object shape is inlined and specialized. Two shapes is
  polymorphic (slower). 4+ shapes is megamorphic (very slow,
  inline cache flushed). Avoid passing wildly different shapes to
  the same function in hot paths.
- **Avoid `delete obj.prop`** in hot objects. Triggers transition to
  dictionary mode. Set to `undefined` if you must clear.
- **Pre-size arrays** with `new Array(n)` or fill via `Array.from`,
  not `push` in a loop, when length is known. V8 reallocates the
  backing store ~once per doubling but can avoid it.
- **Typed arrays** (`Float32Array`, `Uint8Array`, etc.) for
  numerical data. Tightly packed, predictable, no boxing.
- **String concat with `+`** is fine; V8 uses cons-strings. Avoid
  repeated `s = s + char` in a loop only because the cons tree gets
  deep; reach for an array + `.join('')` past ~1000 chars.
- **`JSON.parse` on a string is faster than building the object
  literally** when the data is more than a few hundred items.
  V8 has a fast-path parser.
- **Avoid `with`, `eval`, `arguments`** in hot functions. Each
  inhibits inlining.

### Async and event loop

- **Microtasks run before macrotasks.** `Promise.resolve().then(...)`
  fires before `setTimeout(..., 0)`. Long microtask chains starve
  rendering.
- **`queueMicrotask(fn)`** for "run after current code but before
  next event" without creating a Promise.
- **`setTimeout(..., 0)`** has a 4ms minimum in most browsers
  (clamped). Use `MessageChannel.port.postMessage(...)` for true
  near-zero delay.
- **`requestIdleCallback`** for background work that should yield
  to user interactions. Not available in all Node versions.
- **Avoid `await` in tight loops** when work is independent --
  `Promise.all([...])` runs in parallel.

### Profiling and benchmarking

- **Chrome DevTools Performance tab** for full timeline: scripting,
  rendering, painting, GC. Indispensable for browser code.
- **`console.time` / `timeEnd`** for ad-hoc measurement. Pair with
  `performance.now()` for sub-ms precision.
- **`performance.mark` + `performance.measure`** to surface in the
  DevTools timeline.
- **Node**: `node --prof` then `node --prof-process` for V8
  profiling. `--inspect` to attach Chrome DevTools to Node.
- **Memory leaks**: DevTools Memory tab, take heap snapshots
  before/after, compare. Look for detached DOM nodes (extension
  bug class) and closures retaining caller frames.
- **Bundle size matters more than runtime perf for most web apps.**
  Use `webpack-bundle-analyzer` / `esbuild --analyze` / source maps
  explorer to see what's shipping. Code-split routes.

### Browser specifics

- **MV3 service workers idle out after ~30s.** Module-scope state
  is wiped. Persist with `chrome.storage`.
- **`chrome.storage.local` is async** and has quota (~10MB).
  `session` is in-memory, cleared on browser close.
- **DOM mutations batch:** reading `offsetWidth` (or any layout
  property) flushes pending mutations and forces sync layout.
  Don't interleave reads and writes; batch reads, then writes.
- **`IntersectionObserver` / `MutationObserver`** for reactive DOM
  watching; far cheaper than `setInterval` polling.

## File and import conventions

- Named exports by default. Default exports only when the file has
  one obvious export (a React component).
- One concept per file. `hostname.ts` exports `getCanonicalHostname`,
  nothing else.
- Imports: external first, then relative. Group with blank lines.
- `.js` suffix in TS import paths when `module: ESNext` (required by
  some bundlers): `import {x} from './hostname.js'` even though the
  source is `.ts`.

## Avoid

- `var`. Use `const`, then `let` only when reassignment is real.
- `==`. Always `===`.
- `Function.prototype.bind` in hot paths. Arrow capture is faster and
  clearer.
- `any` outside trust boundaries. `unknown` then narrow.
- Decorators outside framework requirements.
- `interface` for non-object shapes. Use `type`.
- Adding a build step to a 20-line content script. Plain JS is fine.
- jQuery in new code. `filter-anything-everywhere` uses it because
  it's legacy; new projects should not.
- Top-level `await` in a `.js` script tag. Modules only.
- Reaching for Redux/Zustand/Recoil for a 3-field state. Use
  `chrome.storage` or a `Map`.
