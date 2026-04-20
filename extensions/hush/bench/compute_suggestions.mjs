// JS baseline for compute_suggestions, measured against the Rust port
// (see benches/compute_suggestions.rs for the criterion side).
//
// The function body is copied verbatim from commit 3b5f85a (hush 0.8.0,
// the last release before the Rust port deleted it) and trimmed to
// only the helpers it actually needs: findConfigEntry, isSubdomainOf,
// canonicalizeUrl, scriptOriginFromStack, hostOf, median. `log` is
// stubbed to a no-op so the bench doesn't get charged for console I/O.
//
// The synthetic fixture mirrors benches/compute_suggestions.rs exactly:
// same resource counts, same js-call mix, same hostnames, same config
// + allowlist. Node 25's V8 JIT is the same engine chrome extensions
// run, so these numbers are a fair proxy for the in-browser JS cost.
//
// Run with: `node bench/compute_suggestions.mjs`
// Outputs: median time per call for light_tab, heavy_tab, and
// 50-tab sequential, directly comparable with the criterion report.

import { performance } from "node:perf_hooks";

// -------- Helpers pulled from commit 3b5f85a extensions/hush/background.js --

function log() {} // stub; original goes through a ring buffer
function hostOf(url) {
  try { return new URL(url).host; } catch (e) { return ""; }
}
function scriptOriginFromStack(stack) {
  if (!Array.isArray(stack)) return "";
  for (const frame of stack) {
    if (typeof frame !== "string") continue;
    if (frame.includes("mainworld.js")) continue;
    const m = frame.match(/https?:\/\/[^\s:)]+/);
    if (!m) continue;
    try {
      return new URL(m[0]).host;
    } catch (e) { /* not parseable */ }
  }
  return "";
}
function findConfigEntry(config, host) {
  if (config[host]) return { key: host, cfg: config[host] };
  for (const key of Object.keys(config)) {
    if (host === key || host.endsWith("." + key)) {
      return { key, cfg: config[key] };
    }
  }
  return null;
}
function isSubdomainOf(candidate, parent) {
  return candidate !== parent && candidate.endsWith("." + parent);
}
function median(arr) {
  if (!arr.length) return 0;
  const s = arr.slice().sort((a, b) => a - b);
  return s[Math.floor(s.length / 2)];
}
function canonicalizeUrl(url) {
  const noiseParams = ["t", "ts", "_", "nonce", "cb", "callback", "v", "_t", "rand"];
  try {
    const u = new URL(url);
    for (const key of [...u.searchParams.keys()]) {
      if (noiseParams.includes(key.toLowerCase())) u.searchParams.delete(key);
    }
    return u.origin + u.pathname + (u.searchParams.toString() ? "?" + u.searchParams.toString() : "");
  } catch (e) {
    return url;
  }
}

// Minimal subset of LEARN_TEXT (used only for the `learn` string
// attached to each suggestion; doesn't affect measured compute cost).
const LEARN_TEXT = {
  beacon: "beacon", pixel: "pixel", firstPartyTelemetry: "telemetry",
  polling: "polling", hiddenIframe: "iframe", stickyOverlay: "overlay",
  canvasFp: "canvas", webglFpHot: "webgl-hot", webglFp: "webgl",
  audioFp: "audio", fontFp: "font", replayVendor: "replay-v",
  replayListener: "replay-l", rafWaste: "raf",
};

// -------- computeSuggestions: verbatim port from 3b5f85a ------------------
// (This is the function we deleted in commit c91e800 when the Rust
// port shipped. Kept here unchanged so the bench compares apples to
// apples.)

function buildSuggestion({ key, layer, value, reason, confidence, count, evidence, fromFrame, learn, tabHostname, matchedKey, configHasSite, existingBlock, existingRemove, existingHide }) {
  const isFromIframe = !!(fromFrame && fromFrame !== tabHostname);
  const existingForLayer =
    layer === "block" ? existingBlock :
    layer === "remove" ? existingRemove :
    existingHide;
  const dedupResult = existingForLayer.includes(value)
    ? "MATCH (should have been filtered)"
    : "no match";
  return {
    key, layer, value, reason, confidence, count,
    evidence: evidence || [],
    fromIframe: isFromIframe,
    frameHostname: fromFrame || null,
    diag: {
      value, layer,
      tabHostname,
      frameHostname: fromFrame || tabHostname,
      isFromIframe,
      matchedKey: matchedKey || null,
      configHasSite,
      existingBlockCount: existingBlock.length,
      existingBlockSample: existingBlock.slice(0, 10),
      dedupResult,
    },
    learn: learn || "",
  };
}

function computeSuggestions(state, config) {
  const hostname = state.pageHost || "";
  if (!hostname) return [];
  const match = findConfigEntry(config, hostname);
  const cfg = match ? match.cfg : {};
  const existingBlock =
    (Array.isArray(cfg.block) ? cfg.block : []).map(p =>
      typeof p === "string" && p.endsWith("^") ? p.slice(0, -1) : p
    );
  const existingRemove = Array.isArray(cfg.remove) ? cfg.remove : [];
  const existingHide = Array.isArray(cfg.hide) ? cfg.hide : [];
  const ctxFields = {
    tabHostname: hostname,
    matchedKey: match && match.key,
    configHasSite: !!match,
    existingBlock,
    existingRemove,
    existingHide,
  };
  const resources = state.seenResources;
  const out = [];

  const firstNonTopFrame = (records) => {
    for (const r of records) {
      if (r && r.reporterFrame && r.reporterFrame !== hostname) return r.reporterFrame;
    }
    return null;
  };

  // 1. sendBeacon
  const beaconByHost = new Map();
  for (const r of resources) {
    if (r.initiatorType !== "beacon") continue;
    if (!r.host || r.host === hostname) continue;
    (beaconByHost.get(r.host) || beaconByHost.set(r.host, []).get(r.host)).push(r);
  }
  for (const [host, hits] of beaconByHost) {
    const value = "||" + host;
    if (existingBlock.includes(value)) continue;
    out.push(buildSuggestion({
      key: "block::" + value, layer: "block", value,
      reason: "sendBeacon target (" + hits.length + " beacon" + (hits.length > 1 ? "s" : "") + " sent)",
      confidence: 95, count: hits.length,
      evidence: hits.slice(0, 5).map(h => h.url),
      fromFrame: firstNonTopFrame(hits),
      learn: LEARN_TEXT.beacon,
      ...ctxFields,
    }));
  }

  // 2. pixels
  const pixelByHost = new Map();
  for (const r of resources) {
    if (r.initiatorType !== "img") continue;
    if (!r.host || r.host === hostname) continue;
    if (r.transferSize <= 0 || r.transferSize >= 200) continue;
    (pixelByHost.get(r.host) || pixelByHost.set(r.host, []).get(r.host)).push(r);
  }
  for (const [host, hits] of pixelByHost) {
    const value = "||" + host;
    if (existingBlock.includes(value)) continue;
    const med = median(hits.map(h => h.transferSize));
    out.push(buildSuggestion({
      key: "block::" + value, layer: "block", value,
      reason: "tracking pixels: " + hits.length + " tiny image" + (hits.length > 1 ? "s" : "") + " (median " + med + "b)",
      confidence: 85, count: hits.length,
      evidence: hits.slice(0, 5).map(h => h.url + " (" + h.transferSize + "b)"),
      fromFrame: firstNonTopFrame(hits),
      learn: LEARN_TEXT.pixel,
      ...ctxFields,
    }));
  }

  // 3. first-party telemetry
  const subByHost = new Map();
  for (const r of resources) {
    if (!r.host || r.host === hostname) continue;
    if (!isSubdomainOf(r.host, hostname)) continue;
    (subByHost.get(r.host) || subByHost.set(r.host, []).get(r.host)).push(r);
  }
  for (const [host, requests] of subByHost) {
    const sizes = requests.filter(r => r.transferSize > 0).map(r => r.transferSize);
    if (!sizes.length) continue;
    const med = median(sizes);
    const max = Math.max(...sizes);
    if (med >= 1024 || max >= 5120) continue;
    const value = "||" + host;
    if (existingBlock.includes(value)) continue;
    out.push(buildSuggestion({
      key: "block::" + value, layer: "block", value,
      reason: "first-party subdomain with " + requests.length + " tiny response" + (requests.length > 1 ? "s" : "") + " (median " + med + "b)",
      confidence: 70, count: requests.length,
      evidence: requests.slice(0, 5).map(r => r.url + " (" + r.transferSize + "b, " + r.initiatorType + ")"),
      fromFrame: firstNonTopFrame(requests),
      learn: LEARN_TEXT.firstPartyTelemetry,
      ...ctxFields,
    }));
  }

  // 4. polling
  const byCanon = new Map();
  for (const r of resources) {
    if (!r.host || r.host === hostname) continue;
    const canon = canonicalizeUrl(r.url);
    const entry = byCanon.get(canon) || { count: 0, sizes: [], firstSeen: r.startTime, lastSeen: r.startTime, host: r.host, sample: r.url };
    entry.count++;
    entry.sizes.push(r.transferSize);
    if (r.startTime < entry.firstSeen) entry.firstSeen = r.startTime;
    if (r.startTime > entry.lastSeen) entry.lastSeen = r.startTime;
    byCanon.set(canon, entry);
  }
  for (const info of byCanon.values()) {
    if (info.count < 4) continue;
    const span = info.lastSeen - info.firstSeen;
    if (span < 5000 || span > 600000) continue;
    const med = median(info.sizes);
    if (med >= 2048) continue;
    const value = "||" + info.host + "^";
    const key = "block::" + value;
    if (existingBlock.includes(value)) continue;
    if (out.find(s => s.key === key)) continue;
    out.push(buildSuggestion({
      key, layer: "block", value,
      reason: "polled " + info.count + "x over " + Math.round(span / 1000) + "s (median " + med + "b)",
      confidence: 75, count: info.count,
      evidence: [info.sample],
      learn: LEARN_TEXT.polling,
      ...ctxFields,
    }));
  }

  // 5. hidden iframes
  const iframeByHost = new Map();
  for (const f of state.latestIframes) {
    if (!f.src || !f.host) continue;
    const entry = iframeByHost.get(f.host) || { reasons: new Set(), samples: [] };
    for (const r of f.reasons || []) entry.reasons.add(r);
    entry.samples.push(f);
    iframeByHost.set(f.host, entry);
  }
  for (const [host, info] of iframeByHost) {
    const selector = 'iframe[src*="' + host + '"]';
    if (existingRemove.includes(selector)) continue;
    out.push(buildSuggestion({
      key: "remove::" + selector, layer: "remove", value: selector,
      reason: "hidden iframe: " + Array.from(info.reasons).join(", "),
      confidence: 80, count: info.samples.length,
      evidence: info.samples.slice(0, 3).map(s => s.outerHTMLPreview),
      fromFrame: firstNonTopFrame(info.samples),
      learn: LEARN_TEXT.hiddenIframe,
      ...ctxFields,
    }));
  }

  // 6. main-world jsCalls summary (canvas/webgl/audio/font/replay/listener/raf)
  const jsCalls = Array.isArray(state.jsCalls) ? state.jsCalls : [];
  const nowTs = Date.now();
  const firstTs = jsCalls.length ? Date.parse(jsCalls[0].t) || nowTs : nowTs;
  const secondsSinceFirst = Math.max(1, Math.round((nowTs - firstTs) / 1000));
  const kindCounts = {};
  const originsByKind = {};
  const hotParamsByOrigin = {};
  const distinctFontsByOrigin = {};
  const listenerTypesByOrigin = {};
  const replayVendors = new Map();
  const rafWasteByKey = new Map();
  for (const c of jsCalls) {
    const k = c.kind || "?";
    kindCounts[k] = (kindCounts[k] || 0) + 1;
    const origin = scriptOriginFromStack(c.stack) || "(unknown script)";
    if (!originsByKind[k]) originsByKind[k] = new Map();
    originsByKind[k].set(origin, (originsByKind[k].get(origin) || 0) + 1);
    if (k === "webgl-fp" && c.hotParam) {
      hotParamsByOrigin[origin] = (hotParamsByOrigin[origin] || 0) + 1;
    }
    if (k === "font-fp" && c.font) {
      if (!distinctFontsByOrigin[origin]) distinctFontsByOrigin[origin] = new Set();
      distinctFontsByOrigin[origin].add(c.font);
    }
    if (k === "listener-added" && c.eventType) {
      if (!listenerTypesByOrigin[origin]) listenerTypesByOrigin[origin] = { count: 0, types: new Set() };
      listenerTypesByOrigin[origin].count++;
      listenerTypesByOrigin[origin].types.add(c.eventType);
    }
    if (k === "replay-global" && Array.isArray(c.vendors)) {
      for (const v of c.vendors) {
        if (v && v.vendor) replayVendors.set(v.vendor, (replayVendors.get(v.vendor) || 0) + 1);
      }
    }
    if (k === "canvas-draw") {
      const sel = c.canvasSel || "canvas";
      const key = origin + "|" + sel;
      const entry = rafWasteByKey.get(key) || { origin, canvasSel: sel, total: 0, invisible: 0, firstT: c.t, lastT: c.t };
      entry.total++;
      if (c.visible === false) entry.invisible++;
      entry.lastT = c.t;
      rafWasteByKey.set(key, entry);
    }
  }
  const emitOriginBlock = (origin, reason, confidence, kind, learn) => {
    if (!origin || origin === "(unknown script)") return;
    const value = "||" + origin;
    if (existingBlock.includes(value)) return;
    out.push(buildSuggestion({
      key: "block::" + value + "::" + kind, layer: "block", value,
      reason, confidence, count: 1, evidence: [],
      learn,
      ...ctxFields,
    }));
  };
  if (originsByKind["canvas-fp"]) for (const [origin, cnt] of originsByKind["canvas-fp"]) if (cnt >= 3) emitOriginBlock(origin, `canvas fingerprinting (${cnt} reads)`, 90, "canvas-fp", LEARN_TEXT.canvasFp);
  for (const [origin, hotCount] of Object.entries(hotParamsByOrigin)) if (hotCount >= 1) emitOriginBlock(origin, "WebGL fingerprinting (UNMASKED)", 95, "webgl-fp-hot", LEARN_TEXT.webglFpHot);
  if (originsByKind["webgl-fp"]) for (const [origin, cnt] of originsByKind["webgl-fp"]) if (cnt >= 8 && !(hotParamsByOrigin[origin] >= 1)) emitOriginBlock(origin, `WebGL fingerprinting (${cnt} getParameter reads)`, 75, "webgl-fp", LEARN_TEXT.webglFp);
  if (originsByKind["audio-fp"]) for (const [origin, cnt] of originsByKind["audio-fp"]) emitOriginBlock(origin, `audio fingerprinting (${cnt}x)`, 90, "audio-fp", LEARN_TEXT.audioFp);
  for (const [origin, fontSet] of Object.entries(distinctFontsByOrigin)) if (fontSet.size >= 20) emitOriginBlock(origin, `font enumeration (${fontSet.size} fonts)`, 85, "font-fp", LEARN_TEXT.fontFp);
  for (const [vendor, cnt] of replayVendors) {
    const vendorHost = { Hotjar: "hotjar.com", FullStory: "fullstory.com", "Microsoft Clarity": "clarity.ms", LogRocket: "logrocket.com", Smartlook: "smartlook.com", Mouseflow: "mouseflow.com", PostHog: "posthog.com" }[vendor];
    if (!vendorHost) continue;
    const value = "||" + vendorHost;
    if (existingBlock.includes(value)) continue;
    out.push(buildSuggestion({
      key: "block::" + value + "::replay-" + vendor, layer: "block", value,
      reason: vendor + " session replay detected", confidence: 95, count: cnt,
      evidence: ["sentinel"],
      learn: LEARN_TEXT.replayVendor,
      ...ctxFields,
    }));
  }
  for (const [origin, info] of Object.entries(listenerTypesByOrigin)) {
    if (info.count >= 12 && info.types.size >= 3 && secondsSinceFirst < 60) {
      emitOriginBlock(origin, `session replay pattern (${info.count} listeners)`, 80, "listener-density", LEARN_TEXT.replayListener);
    }
  }
  for (const entry of rafWasteByKey.values()) {
    if (entry.total < 20) continue;
    const span = (Date.parse(entry.lastT) || 0) - (Date.parse(entry.firstT) || 0);
    if (span < 3000) continue;
    const ratio = entry.invisible / entry.total;
    if (ratio < 0.8) continue;
    if (!entry.origin || entry.origin === "(unknown script)") continue;
    const value = "||" + entry.origin;
    if (existingBlock.includes(value)) continue;
    const seconds = Math.max(1, Math.round(span / 1000));
    out.push(buildSuggestion({
      key: "block::" + value + "::raf-waste::" + entry.canvasSel, layer: "block", value,
      reason: `invisible animation loop (${entry.invisible} draws in ${seconds}s)`,
      confidence: 70, count: entry.invisible,
      evidence: [entry.canvasSel, entry.origin, `${seconds}s`],
      learn: LEARN_TEXT.rafWaste,
      ...ctxFields,
    }));
  }

  // 7. sticky overlays
  const stickySeen = new Set();
  for (const s of state.latestStickies) {
    if (!s.selector || stickySeen.has(s.selector)) continue;
    stickySeen.add(s.selector);
    if (existingHide.includes(s.selector)) continue;
    out.push(buildSuggestion({
      key: "hide::" + s.selector, layer: "hide", value: s.selector,
      reason: `fixed overlay covering ${s.coverage}% (z-index ${s.zIndex})`,
      confidence: 55, count: 1,
      evidence: [`${s.rect.w}x${s.rect.h} at z-index ${s.zIndex}`],
      fromFrame: s.reporterFrame && s.reporterFrame !== hostname ? s.reporterFrame : null,
      learn: LEARN_TEXT.stickyOverlay,
      ...ctxFields,
    }));
  }

  const dismissed = new Set(state.dismissed);
  return out
    .filter(s => !dismissed.has(s.key))
    .sort((a, b) => (b.confidence - a.confidence) || (b.count - a.count));
}

// -------- Fixture builder (mirror of benches/compute_suggestions.rs) ------

function sampleState(shape) {
  const state = {
    pageHost: "site.test",
    seenResources: [],
    latestIframes: [],
    latestStickies: [],
    jsCalls: [],
    dismissed: [],
    suggestions: [],
  };
  const scale = shape.resources;
  for (let i = 0; i < scale; i++) {
    if (i % 5 === 0) state.seenResources.push({ url: `https://tracker.test/beacon?i=${i}`, host: "tracker.test", initiatorType: "beacon", transferSize: 0, duration: 5, startTime: i });
    else if (i % 5 === 1) state.seenResources.push({ url: `https://ads.test/p${i}.gif`, host: "ads.test", initiatorType: "img", transferSize: 43, duration: 2, startTime: i });
    else if (i % 5 === 2) state.seenResources.push({ url: `https://log.site.test/h${i}`, host: "log.site.test", initiatorType: "fetch", transferSize: 150, duration: 10, startTime: i });
    else if (i % 5 === 3) state.seenResources.push({ url: "https://api.test/poll", host: "api.test", initiatorType: "fetch", transferSize: 50, duration: 5, startTime: i * 1000 });
    else state.seenResources.push({ url: `https://site.test/asset${i}.js`, host: "site.test", initiatorType: "script", transferSize: 8192, duration: 15, startTime: i });
  }
  for (let i = 0; i < shape.iframes; i++) {
    state.latestIframes.push({ src: `https://${i % 2 === 0 ? "a" : "b"}.ads.test/frame${i}`, host: `${i % 2 === 0 ? "a" : "b"}.ads.test`, reasons: ["display:none", "1x1 size"], width: 1, height: 1, outerHTMLPreview: "<iframe ...>" });
  }
  for (let i = 0; i < shape.stickies; i++) {
    state.latestStickies.push({ selector: `div.popup-${i}`, coverage: 45, zIndex: 9999, rect: { w: 400, h: 300 } });
  }
  for (let i = 0; i < shape.js_calls; i++) {
    const stack = [`at x (https://fp.test/fp.js:${i}:1)`];
    const kinds = ["canvas-fp", "webgl-fp", "font-fp", "audio-fp", "listener-added", "canvas-draw"];
    const kind = kinds[i % 6];
    const call = { kind, t: "2026-04-19T12:00:00.000Z", stack };
    if (kind === "webgl-fp") call.hotParam = i % 2 === 0;
    if (kind === "font-fp") { call.font = `12px font-${i}`; call.text = "probe"; }
    if (kind === "listener-added") call.eventType = "mousemove";
    if (kind === "canvas-draw") { call.op = "fillRect"; call.visible = (i % 3) !== 0; call.canvasSel = `canvas#c${i % 3}`; }
    state.jsCalls.push(call);
  }
  state.jsCalls.push({ kind: "replay-global", t: "2026-04-19T12:00:00.000Z", vendors: [{ key: "_hjSettings", vendor: "Hotjar" }] });
  return state;
}

const LIGHT_TAB = { resources: 100, js_calls: 50, iframes: 5, stickies: 5 };
const HEAVY_TAB = { resources: 500, js_calls: 500, iframes: 20, stickies: 20 };

const config = { "site.test": { block: ["||already-blocked.test"] } };

// -------- Timing harness --------------------------------------------------

function bench(label, iters, thunk) {
  // Warm-up.
  for (let i = 0; i < Math.min(100, iters); i++) thunk();
  const samples = new Float64Array(Math.min(iters, 10_000));
  // Take `samples.length` individual timings. Median is the primary
  // number reported; p99 highlights tail latency.
  for (let i = 0; i < samples.length; i++) {
    const t0 = performance.now();
    thunk();
    samples[i] = performance.now() - t0;
  }
  samples.sort();
  const median = samples[Math.floor(samples.length * 0.5)];
  const p99 = samples[Math.floor(samples.length * 0.99)];
  const mean = samples.reduce((a, b) => a + b, 0) / samples.length;
  console.log(
    `${label.padEnd(26)} median ${formatMs(median)}  mean ${formatMs(mean)}  p99 ${formatMs(p99)}   (${samples.length} iters)`
  );
}

function formatMs(ms) {
  if (ms < 1) return `${(ms * 1000).toFixed(1).padStart(6)} us`;
  return `${ms.toFixed(3).padStart(7)} ms`;
}

// -------- Run -------------------------------------------------------------

const lightState = sampleState(LIGHT_TAB);
const heavyState = sampleState(HEAVY_TAB);

// Sanity: we're actually producing suggestions.
const sanity = computeSuggestions(heavyState, config);
console.log(`heavy_tab fixture -> ${sanity.length} suggestions`);
console.log("");

bench("light_tab", 20_000, () => computeSuggestions(lightState, config));
bench("heavy_tab", 5_000, () => computeSuggestions(heavyState, config));
bench("50_tabs_of_heavy", 200, () => {
  for (let i = 0; i < 50; i++) computeSuggestions(heavyState, config);
});
