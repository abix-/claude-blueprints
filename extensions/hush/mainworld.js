// Hush main-world hooks (Stage 3: hybrid with Rust-installed wrappers).
//
// Runs in the page's own JS context at document_start. The approved
// plan (docs/roadmap.md Stage 3) calls for Rust to re-patch prototype
// methods via Reflect::set + Closure; that needs a JS-side `this`-
// capturing factory because wasm-bindgen closures can't forward
// implicit `this`. `makeWrapper` below is that factory (one line of
// logic). Rust calls it for each hook and handles everything else.
//
// Why two install phases:
// - Synchronous stubs at document_start keep events off the floor
//   during the async WASM load (inline <head> scripts would otherwise
//   fire before any hook exists).
// - Once WASM is ready, Rust's `hush_install_from_js` swaps the
//   "simple" prototype methods (canvas FP, WebGL FP, font FP) for
//   Rust-backed wrappers and drains the stub queue through the typed
//   SignalPayload validator.
// - The "complex" hooks (fetch, XHR, sendBeacon, WebSocket.send,
//   OfflineAudioContext constructor, addEventListener density filter,
//   canvas-draw visibility sampler, replay-global poll) stay JS-
//   installed because each needs custom argument extraction that's
//   cheaper in JS than bouncing across the wasm boundary.

(() => {
  // ---- Shared state ------------------------------------------------------
  const MAX_QUEUE = 2000;
  const q = (window.__hush_stub_q__ = []);
  let wasmMod = null;
  let wasmReady = false;

  // Emitted-spoof-kinds dedup. Per-page: mainworld.js runs fresh on
  // every navigation, so the set resets naturally. One FirewallEvent
  // per (kind, page) keeps a busy fingerprinter from flooding the log
  // — the popup cares that spoof FIRED, not that it fired 40x/second.
  const hushSpoofEmitted = new Set();
  function emitSpoofHit(kind) {
    if (hushSpoofEmitted.has(kind)) return;
    hushSpoofEmitted.add(kind);
    try {
      document.dispatchEvent(new CustomEvent("__hush_spoof_hit__", {
        detail: { kind: String(kind), t: new Date().toISOString() }
      }));
    } catch (e) { /* ignore — detached document */ }
  }

  // Check at call time whether the site has opted into spoofing a
  // given kind. Content script writes dataset.hushSpoof = cfg.spoof
  // .join(",") at document_start; reading at call time means the
  // install-time race between content.js and mainworld.js is moot
  // by the time any page script invokes the spoofed API.
  function hasSpoofTag(tag) {
    try {
      const el = document.documentElement;
      const v = el && el.dataset && el.dataset.hushSpoof;
      return !!(v && v.indexOf(tag) >= 0);
    } catch (e) { return false; }
  }

  // Constant 1x1 transparent PNG used as the bland return value from
  // canvas `toDataURL` / `toBlob` when the `canvas` spoof is active.
  // Fingerprinters rely on subpixel rendering differences across
  // GPUs / drivers to hash a unique value; returning a fixed byte
  // sequence takes that entropy to zero.
  const BLAND_PNG_DATAURL =
    "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAA" +
    "DUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
  const BLAND_PNG_BYTES = (() => {
    try {
      const b64 =
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
      const bin = atob(b64);
      const out = new Uint8Array(bin.length);
      for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
      return out;
    } catch (e) { return new Uint8Array(0); }
  })();

  // Same dedup shape for neuter + silence — one event per
  // (type, origin, page). Each set is keyed by the matched origin
  // host so different replay vendors on the same page each get a
  // firewall-log row.
  const hushNeuterEmitted = new Set();
  const hushSilenceEmitted = new Set();
  function emitHit(type, origin, match) {
    const set = type === "neuter" ? hushNeuterEmitted : hushSilenceEmitted;
    if (set.has(origin)) return;
    set.add(origin);
    try {
      document.dispatchEvent(new CustomEvent("__hush_" + type + "_hit__", {
        detail: {
          t: new Date().toISOString(),
          origin: String(origin),
          match: String(match || "")
        }
      }));
    } catch (e) { /* ignore */ }
  }

  // Extract the initiating-script host from a V8 stack. Mirrors
  // src/stack.rs::script_origin_from_stack — can't call wasm from
  // mainworld (CSP), so reimplement the ~10 lines here. Skips Hush's
  // own `mainworld.js` frames so we attribute to the real caller.
  function stackOriginHost(stack) {
    if (!Array.isArray(stack)) return "";
    for (let i = 0; i < stack.length; i++) {
      const frame = stack[i];
      if (typeof frame !== "string") continue;
      if (frame.indexOf("mainworld.js") >= 0) continue;
      const http = frame.indexOf("http://");
      const https = frame.indexOf("https://");
      let start = -1;
      if (http >= 0 && https >= 0) start = Math.min(http, https);
      else if (http >= 0) start = http;
      else if (https >= 0) start = https;
      else continue;
      const rest = frame.slice(start);
      let end = rest.length;
      for (let j = 0; j < rest.length; j++) {
        const c = rest.charCodeAt(j);
        if (c === 41 /* ) */ || c === 32 /* space */ || c === 9 /* tab */) {
          end = j;
          break;
        }
      }
      try {
        const u = new URL(rest.slice(0, end));
        return u.host;
      } catch (e) { /* not a URL, keep scanning */ }
    }
    return "";
  }

  // uBlock-style URL filter match against a host. Strips `||`
  // anchor prefix and `^` boundary suffix. Returns the matched
  // filter string on hit, "" on miss.
  function matchesUrlFilter(host, rawFilter) {
    if (!host || !rawFilter) return "";
    let f = rawFilter;
    if (f.startsWith("||")) f = f.slice(2);
    if (f.endsWith("^")) f = f.slice(0, -1);
    if (!f) return "";
    // Anchored (||) = host is exactly `f` OR host ends with `.f`.
    if (rawFilter.startsWith("||")) {
      if (host === f || host.endsWith("." + f)) return rawFilter;
      return "";
    }
    // Bare pattern = substring match on host.
    return host.indexOf(f) >= 0 ? rawFilter : "";
  }

  // Parse the comma-separated dataset attribute into a filter list.
  // Caller filters empty entries so the list is always usable.
  function datasetFilters(name) {
    try {
      const raw = document.documentElement
        && document.documentElement.dataset
        && document.documentElement.dataset[name];
      if (!raw) return [];
      return String(raw).split(",").map(s => s.trim()).filter(Boolean);
    } catch (e) { return []; }
  }

  // Find the first filter in dataset.<attr> that matches the host.
  // Returns "" on no match.
  function findFilterMatch(host, attr) {
    if (!host) return "";
    const filters = datasetFilters(attr);
    for (let i = 0; i < filters.length; i++) {
      const m = matchesUrlFilter(host, filters[i]);
      if (m) return m;
    }
    return "";
  }

  // Interaction event types the neuter rule denies. Keep aligned
  // with the replay-listener detector's own set so "we detected
  // this signal, we neutralize it" is a 1:1 story.
  const NEUTER_EVENT_TYPES = new Set([
    "mousemove", "mousedown", "mouseup", "click",
    "keydown", "keyup", "keypress", "input",
    "scroll", "wheel",
    "touchmove", "touchstart", "touchend"
  ]);

  // Attention / page-lifecycle event types. Hooked by session-
  // replay vendors, A/B-test frameworks, and "engagement
  // analytics" to measure how long you keep the tab visible,
  // whether you tabbed away, and when you were about to leave.
  // Tracked for detection and neuterable at the same layer as
  // interaction listeners since the enforcement mechanism (deny
  // addEventListener) is identical.
  const ATTENTION_EVENT_TYPES = new Set([
    "visibilitychange", "focus", "blur",
    "pagehide", "pageshow", "beforeunload"
  ]);

  function cap() {
    try {
      const s = new Error().stack || "";
      return s.split("\n").slice(2, 8).map(l => l.trim()).filter(Boolean);
    } catch (e) {
      return [];
    }
  }

  function emit(detail) {
    detail.t = new Date().toISOString();
    // Push to the window-exposed queue (capture buffer for the jsdom
    // emit-contract tests, and a debug aid in live pages). Capped so
    // a long-running tab doesn't grow it unboundedly.
    q.push(detail);
    if (q.length > MAX_QUEUE) q.splice(0, q.length - MAX_QUEUE);
    // Dispatch a CustomEvent to the isolated-world content script,
    // which runs WASM and validates every payload against the typed
    // `SignalPayload` union on receipt. The main world cannot itself
    // load the WASM bundle (no `chrome.runtime.getURL` here), so
    // validation happens entirely on the other side of this event.
    try {
      document.dispatchEvent(new CustomEvent("__hush_call__", { detail }));
    } catch (e) { /* CustomEvent may fail in deeply-strict contexts */ }
  }

  // Originals the stubs replace - kept so Rust's install_from_js can
  // rewire prototypes with Rust-backed wrappers that still forward to
  // the real implementations. Key format: "ProtoName.method".
  const orig = {};
  function captureOrig(proto, name) {
    const key = proto.constructor && proto.constructor.name
      ? `${proto.constructor.name}.${name}`
      : `?${name}`;
    orig[key] = proto[name];
    return orig[key];
  }

  // ---- makeWrapper JS factory (see module doc) ---------------------------
  //
  // Given a Rust dispatch function + the original method + a kind tag,
  // return a `this`-capturing wrapper. The wrapper:
  //   - captures `this`, `arguments`, and stack
  //   - calls rustDispatch(this, { args, stack, ...kindExtras })
  //     where rustDispatch builds + validates + dispatches the
  //     SignalPayload
  //   - forwards to the original with the original `this` + args
  //
  // ONE line of JS that wasm-bindgen cannot provide on its own
  // because Rust closures don't see JS `this`.
  function makeWrapper(rustDispatch, origFn, kindTag) {
    return function hushRustBacked() {
      const args = Array.prototype.slice.call(arguments);
      const call = { args, stack: cap() };
      if (kindTag === "font-fp") {
        try { call.font = this.font || ""; } catch (e) { call.font = ""; }
      }
      try { rustDispatch.call(null, this, call); } catch (e) { /* never break the page */ }
      return origFn.apply(this, args);
    };
  }

  // ---- Synchronous stubs at document_start -------------------------------
  //
  // These cover both the methods Rust will later re-patch (canvas FP,
  // WebGL FP, font FP) and the complex ones Rust won't (fetch/XHR/etc).
  // Every stub emits through emit() which queues pre-WASM and calls
  // dispatchHook post-WASM.

  // fetch
  try {
    const _fetch = captureOrig(window, "fetch");
    window.fetch = function hushFetch(input, init) {
      const stack = cap();
      try {
        emit({
          kind: "fetch",
          url: urlOf(input),
          method: (init && init.method) || (input && input.method) || "GET",
          bodyPreview: previewBody(init && init.body),
          stack
        });
      } catch (e) {}
      // Silence: if the initiating-script origin matches a
      // silence rule, skip the real fetch and fake a 204.
      try {
        const origin = stackOriginHost(stack);
        const match = findFilterMatch(origin, "hushSilence");
        if (match) {
          emitHit("silence", origin, match);
          return Promise.resolve(new Response(null, { status: 204 }));
        }
      } catch (e) {}
      const p = _fetch.apply(this, arguments);
      if (p && typeof p.catch === "function") p.catch(() => {});
      return p;
    };
  } catch (e) {}

  // XMLHttpRequest
  try {
    const _open = captureOrig(XMLHttpRequest.prototype, "open");
    const _send = captureOrig(XMLHttpRequest.prototype, "send");
    XMLHttpRequest.prototype.open = function hushXhrOpen(method, url) {
      try { this.__hush_method = method; this.__hush_url = url; } catch (e) {}
      return _open.apply(this, arguments);
    };
    XMLHttpRequest.prototype.send = function hushXhrSend(body) {
      const stack = cap();
      try {
        emit({
          kind: "xhr",
          url: this.__hush_url || "",
          method: this.__hush_method || "",
          bodyPreview: previewBody(body),
          stack
        });
      } catch (e) {}
      // Silence: fake a 204 completion without touching the wire.
      // The readystatechange dispatch mirrors what an HTTP 204
      // response would look like from the page's perspective so
      // callers that poll readyState don't hang.
      try {
        const origin = stackOriginHost(stack);
        const match = findFilterMatch(origin, "hushSilence");
        if (match) {
          emitHit("silence", origin, match);
          const xhr = this;
          setTimeout(() => {
            try {
              Object.defineProperty(xhr, "readyState", { value: 4, configurable: true });
              Object.defineProperty(xhr, "status", { value: 204, configurable: true });
              Object.defineProperty(xhr, "statusText", { value: "No Content", configurable: true });
              Object.defineProperty(xhr, "responseText", { value: "", configurable: true });
              xhr.dispatchEvent(new Event("readystatechange"));
              xhr.dispatchEvent(new Event("load"));
              xhr.dispatchEvent(new Event("loadend"));
            } catch (e) { /* XHR already finalized */ }
          }, 0);
          return;
        }
      } catch (e) {}
      return _send.apply(this, arguments);
    };
  } catch (e) {}

  // navigator.sendBeacon
  try {
    if (navigator.sendBeacon) {
      const _sendBeacon = navigator.sendBeacon.bind(navigator);
      orig["Navigator.sendBeacon"] = _sendBeacon;
      navigator.sendBeacon = function hushSendBeacon(url, body) {
        const stack = cap();
        try {
          emit({
            kind: "beacon",
            url: typeof url === "string" ? url : String(url),
            bodyPreview: previewBody(body),
            stack
          });
        } catch (e) {}
        // Silence: per MDN, sendBeacon returns boolean indicating
        // whether the UA queued the transfer. `true` is the "all
        // good" answer the replay lib expects — we never send,
        // they never know.
        try {
          const origin = stackOriginHost(stack);
          const match = findFilterMatch(origin, "hushSilence");
          if (match) {
            emitHit("silence", origin, match);
            return true;
          }
        } catch (e) {}
        return _sendBeacon(url, body);
      };
    }
  } catch (e) {}

  // WebSocket.send
  try {
    const _wsSend = captureOrig(WebSocket.prototype, "send");
    WebSocket.prototype.send = function hushWsSend(data) {
      try {
        emit({
          kind: "ws-send",
          url: this.url || "",
          bodyPreview: previewBody(data),
          stack: cap()
        });
      } catch (e) {}
      return _wsSend.apply(this, arguments);
    };
  } catch (e) {}

  // Canvas FP: toDataURL, toBlob.
  //
  // Detection emits a `canvas-fp` observation. Spoof (kind tag
  // `canvas`): when opted in, both return constant bland bytes — a
  // 1x1 transparent PNG — so the fingerprinter's hash is invariant
  // across users. Opt-in per site since legitimate uses (image
  // resize, drawing tools, thumbnail export) would break.
  try {
    const _toDataURL = captureOrig(HTMLCanvasElement.prototype, "toDataURL");
    HTMLCanvasElement.prototype.toDataURL = function hushToDataURL() {
      try { emit({ kind: "canvas-fp", method: "toDataURL", stack: cap() }); } catch (e) {}
      if (hasSpoofTag("canvas")) {
        emitSpoofHit("canvas");
        return BLAND_PNG_DATAURL;
      }
      return _toDataURL.apply(this, arguments);
    };
  } catch (e) {}
  try {
    if (HTMLCanvasElement.prototype.toBlob) {
      const _toBlob = captureOrig(HTMLCanvasElement.prototype, "toBlob");
      HTMLCanvasElement.prototype.toBlob = function hushToBlob(cb) {
        try { emit({ kind: "canvas-fp", method: "toBlob", stack: cap() }); } catch (e) {}
        if (hasSpoofTag("canvas")) {
          emitSpoofHit("canvas");
          try {
            const blob = new Blob([BLAND_PNG_BYTES], { type: "image/png" });
            // Spec: callback fires asynchronously.
            Promise.resolve().then(() => {
              try { cb && cb(blob); } catch (e) {}
            });
            return;
          } catch (e) { /* fall through to real toBlob */ }
        }
        return _toBlob.apply(this, arguments);
      };
    }
  } catch (e) {}

  // Canvas 2D getImageData + measureText.
  //
  // getImageData spoof (kind tag `canvas`): return a fresh ImageData
  // of the requested dimensions with all pixels transparent-black.
  // Kills the pixel-hash fingerprint while keeping the API surface
  // intact (instanceof, .data length, .width, .height all correct).
  //
  // measureText spoof (kind tag `font-enum`): return a synthetic
  // TextMetrics-shaped plain object whose fields depend only on
  // text length, not on font. Fingerprinters who compare
  // measureText widths across many font-family strings see the
  // same numbers every time → zero entropy. Note: returned object
  // is not an instanceof TextMetrics; that trade-off is acceptable
  // for opt-in spoof.
  try {
    if (typeof CanvasRenderingContext2D !== "undefined") {
      const _getImageData = captureOrig(CanvasRenderingContext2D.prototype, "getImageData");
      CanvasRenderingContext2D.prototype.getImageData = function hushGetImageData(x, y, w, h) {
        try { emit({ kind: "canvas-fp", method: "getImageData", stack: cap() }); } catch (e) {}
        if (hasSpoofTag("canvas")) {
          emitSpoofHit("canvas");
          try {
            const ww = (w | 0) > 0 ? (w | 0) : 1;
            const hh = (h | 0) > 0 ? (h | 0) : 1;
            return new ImageData(ww, hh);
          } catch (e) { /* fall through to real getImageData */ }
        }
        return _getImageData.apply(this, arguments);
      };
      const _measureText = captureOrig(CanvasRenderingContext2D.prototype, "measureText");
      CanvasRenderingContext2D.prototype.measureText = function hushMeasureText(text) {
        try {
          emit({
            kind: "font-fp",
            font: this.font || "",
            text: text ? String(text).slice(0, 20) : "",
            stack: cap()
          });
        } catch (e) {}
        if (hasSpoofTag("font-enum")) {
          emitSpoofHit("font-enum");
          const chars = text == null ? 0 : String(text).length;
          const width = chars * 8;
          return {
            width: width,
            actualBoundingBoxLeft: 0,
            actualBoundingBoxRight: width,
            actualBoundingBoxAscent: 10,
            actualBoundingBoxDescent: 2,
            fontBoundingBoxAscent: 12,
            fontBoundingBoxDescent: 3,
            emHeightAscent: 12,
            emHeightDescent: 3,
            hangingBaseline: 9,
            alphabeticBaseline: 0,
            ideographicBaseline: -3
          };
        }
        return _measureText.apply(this, arguments);
      };
    }
  } catch (e) {}

  // WebGL / WebGL2 getParameter (stub now; Rust re-patches)
  //
  // The wrapper does two things:
  //   1. emit a webgl-fp observation for the detector
  //   2. if the site's `spoof` config includes "webgl-unmasked", return
  //      bland identical-across-users strings for UNMASKED_VENDOR_WEBGL
  //      (37445) and UNMASKED_RENDERER_WEBGL (37446) instead of the real
  //      GPU identity. Every other param passes through unchanged, so
  //      legitimate rendering code (size limits, extension queries, etc.)
  //      keeps working.
  //
  // Spoof opt-in is communicated from the isolated-world content script
  // via `document.documentElement.dataset.hushSpoof` (comma-separated
  // kind tags). Read at call time, not install time, so the inherent
  // race between content.js and mainworld.js at document_start is moot
  // by the time any page script invokes WebGL.
  const wrapGLStub = (proto) => {
    if (!proto || !proto.getParameter) return;
    const origFn = captureOrig(proto, "getParameter");
    proto.getParameter = function hushGLGetParameter(param) {
      try {
        emit({
          kind: "webgl-fp",
          param: String(param),
          hotParam: param === 37445 || param === 37446,
          stack: cap()
        });
      } catch (e) {}
      if ((param === 37445 || param === 37446) && hasSpoofTag("webgl-unmasked")) {
        emitSpoofHit("webgl-unmasked");
        return param === 37445 ? "Google Inc." : "ANGLE (Generic)";
      }
      return origFn.apply(this, arguments);
    };
  };
  try { if (typeof WebGLRenderingContext !== "undefined") wrapGLStub(WebGLRenderingContext.prototype); } catch (e) {}
  try { if (typeof WebGL2RenderingContext !== "undefined") wrapGLStub(WebGL2RenderingContext.prototype); } catch (e) {}

  // OfflineAudioContext constructor (stays JS-only - constructor
  // replacement pattern).
  //
  // Detection: emit `audio-fp` observation on construction. Spoof
  // (kind tag `audio`): when opted in, `startRendering` resolves to
  // a silent AudioBuffer of the requested dimensions instead of the
  // real rendered waveform. Audio fingerprinters rely on tiny
  // floating-point divergences across platforms when rendering a
  // known graph (oscillator → compressor → destination); returning
  // silence zeroes that entropy.
  try {
    if (typeof OfflineAudioContext !== "undefined") {
      const OrigOAC = OfflineAudioContext;
      const HushOAC = function hushOAC() {
        try { emit({ kind: "audio-fp", method: "OfflineAudioContext", stack: cap() }); } catch (e) {}
        return Reflect.construct(OrigOAC, arguments, HushOAC);
      };
      HushOAC.prototype = OrigOAC.prototype;
      HushOAC.prototype.constructor = HushOAC;
      window.OfflineAudioContext = HushOAC;

      const _startRendering = captureOrig(OrigOAC.prototype, "startRendering");
      if (_startRendering) {
        OrigOAC.prototype.startRendering = function hushStartRendering() {
          if (hasSpoofTag("audio")) {
            emitSpoofHit("audio");
            try {
              const silent = this.createBuffer(
                this.numberOfChannels,
                this.length,
                this.sampleRate
              );
              return Promise.resolve(silent);
            } catch (e) { /* fall through to real render */ }
          }
          return _startRendering.apply(this, arguments);
        };
      }
    }
  } catch (e) {}

  // Clipboard API (navigator.clipboard.readText / writeText).
  //
  // readText() is gesture-gated by Chrome but sites probe for it
  // and wrap paste events to sniff clipboard content for coupon
  // codes, competitor URLs, and tracking parameters. Any script
  // that calls readText() is high-signal — legit uses are rare
  // and almost always initiated by a very explicit user action
  // (paste button on a password manager, clipboard inspector in
  // a dev tool). Emit a `clipboard-fp` observation so the
  // detector can surface a block suggestion for the script
  // origin.
  //
  // writeText() is NOT hooked here. Sites use it for "copy link"
  // buttons routinely; flagging those would drown the user in
  // false positives. If a concrete writeText abuse pattern turns
  // up (e.g. copy-hijack injecting tracking params), add a
  // separate detector with a more specific heuristic.
  try {
    if (typeof Clipboard !== "undefined" && Clipboard.prototype
        && typeof Clipboard.prototype.readText === "function") {
      const _readText = captureOrig(Clipboard.prototype, "readText");
      Clipboard.prototype.readText = function hushReadText() {
        try {
          emit({
            kind: "clipboard-fp",
            method: "readText",
            stack: cap()
          });
        } catch (e) {}
        return _readText.apply(this, arguments);
      };
    }
  } catch (e) {}

  // New-Web-API hardware device probes
  // (Bluetooth / USB / HID / Serial).
  //
  // These APIs are user-gesture-gated and surface a native
  // permission prompt, so merely calling them doesn't automatically
  // give the site anything. But the call itself is a fingerprint
  // vector: the site learns whether the browser implements the
  // API, which OS / Chrome channel the user is on, and in the case
  // of successful requests, the distinct device list (another
  // entropy signal).
  //
  // Legitimate uses are rare and tend to be obvious dev-tool /
  // industrial / maker-space contexts where the user explicitly
  // clicks a "connect" button. On a random web page a
  // `requestDevice` call is high-signal suspicious. Brave doesn't
  // hook any of these APIs; a Hush block suggestion catches the
  // gap.
  //
  // Implementation: one wrapper factory, applied to each
  // prototype's entry point. Emits `new-api-probe` with the
  // constructor-qualified method name so the detector can tell
  // Bluetooth from USB in the firewall log.
  function wrapDeviceApi(proto, methodName, tag) {
    try {
      if (proto && typeof proto[methodName] === "function") {
        const _orig = captureOrig(proto, methodName);
        proto[methodName] = function hushDeviceApi() {
          try {
            emit({
              kind: "new-api-probe",
              method: tag,
              stack: cap()
            });
          } catch (e) {}
          return _orig.apply(this, arguments);
        };
      }
    } catch (e) {}
  }
  try {
    if (typeof Bluetooth !== "undefined") {
      wrapDeviceApi(Bluetooth.prototype, "requestDevice", "Bluetooth.requestDevice");
    }
  } catch (e) {}
  try {
    if (typeof USB !== "undefined") {
      wrapDeviceApi(USB.prototype, "requestDevice", "USB.requestDevice");
    }
  } catch (e) {}
  try {
    if (typeof HID !== "undefined") {
      wrapDeviceApi(HID.prototype, "requestDevice", "HID.requestDevice");
    }
  } catch (e) {}
  try {
    if (typeof Serial !== "undefined") {
      wrapDeviceApi(Serial.prototype, "requestPort", "Serial.requestPort");
    }
  } catch (e) {}

  // addEventListener density (stays JS-only - needs `this === document` filter)
  try {
    const _addEventListener = captureOrig(EventTarget.prototype, "addEventListener");
    const REPLAY_EVENT_TYPES = new Set([
      "mousemove", "mousedown", "mouseup", "click",
      "keydown", "keyup", "keypress", "input",
      "scroll", "touchmove", "touchstart", "touchend"
    ]);
    EventTarget.prototype.addEventListener = function hushAddEventListener(type) {
      const typeIsString = typeof type === "string";
      const typeIsInteraction = typeIsString
        && (REPLAY_EVENT_TYPES.has(type) || NEUTER_EVENT_TYPES.has(type));
      const typeIsAttention = typeIsString && ATTENTION_EVENT_TYPES.has(type);
      const typeIsTracked = typeIsInteraction || typeIsAttention;
      let stack = null;
      try {
        if (typeIsTracked) {
          const onDocLike = (this === document || this === window ||
                             (typeof document !== "undefined" && this === document.body));
          if (onDocLike) {
            stack = cap();
            emit({ kind: "listener-added", eventType: type, stack });
          }
        }
      } catch (e) {}
      // Neuter: deny listener registrations from matching script
      // origins. Runs before the real addEventListener so the
      // listener never binds — no CPU burn, no capture, no exfil.
      // Applies to both interaction events (session-replay capture
      // surface) and attention events (engagement analytics /
      // session-replay dwell-time signals).
      try {
        const typeIsNeuterable = typeIsString
          && (NEUTER_EVENT_TYPES.has(type) || ATTENTION_EVENT_TYPES.has(type));
        if (typeIsNeuterable) {
          if (!stack) stack = cap();
          const origin = stackOriginHost(stack);
          const match = findFilterMatch(origin, "hushNeuter");
          if (match) {
            emitHit("neuter", origin, match);
            return undefined;
          }
        }
      } catch (e) {}
      return _addEventListener.apply(this, arguments);
    };
  } catch (e) {}

  // Canvas 2D draw ops with visibility sampling (stays JS-only - per-canvas throttle)
  try {
    if (typeof CanvasRenderingContext2D !== "undefined") {
      const _perfNow = (typeof performance !== "undefined" && performance.now)
        ? performance.now.bind(performance)
        : Date.now.bind(Date);
      const lastVisCheck = new WeakMap();
      function canvasVisible(canvas) {
        if (!canvas || !canvas.getBoundingClientRect) return true;
        try {
          const rect = canvas.getBoundingClientRect();
          if (rect.width < 2 || rect.height < 2) return false;
          const vw = window.innerWidth || 1, vh = window.innerHeight || 1;
          if (rect.right < 0 || rect.bottom < 0 || rect.left > vw || rect.top > vh) return false;
          const cs = window.getComputedStyle ? window.getComputedStyle(canvas) : null;
          if (cs) {
            if (cs.display === "none") return false;
            if (cs.visibility === "hidden") return false;
            if (parseFloat(cs.opacity) === 0) return false;
          }
          return true;
        } catch (e) { return true; }
      }
      function canvasDescriptor(canvas) {
        if (!canvas) return "";
        try {
          const id = canvas.id ? "#" + canvas.id : "";
          let cls = "";
          if (typeof canvas.className === "string") {
            cls = canvas.className.trim().split(/\s+/).filter(Boolean).slice(0, 2).join(".");
          }
          return "canvas" + id + (cls ? "." + cls : "");
        } catch (e) { return "canvas"; }
      }
      const DRAW_OPS = ["fillRect", "strokeRect", "clearRect", "drawImage", "fill", "stroke", "putImageData"];
      for (const op of DRAW_OPS) {
        const origFn = CanvasRenderingContext2D.prototype[op];
        if (typeof origFn !== "function") continue;
        CanvasRenderingContext2D.prototype[op] = function hushCanvasDraw() {
          try {
            const canvas = this && this.canvas;
            if (canvas) {
              const now = _perfNow();
              const last = lastVisCheck.get(canvas) || 0;
              if (now - last >= 100) {
                lastVisCheck.set(canvas, now);
                emit({
                  kind: "canvas-draw",
                  op,
                  visible: canvasVisible(canvas),
                  canvasSel: canvasDescriptor(canvas),
                  stack: cap()
                });
              }
            }
          } catch (e) {}
          return origFn.apply(this, arguments);
        };
      }
    }
  } catch (e) {}

  // Replay-global poll (stays JS - no prototype hook, just a poll).
  const REPLAY_GLOBALS = [
    ["_hjSettings", "Hotjar"], ["_hjid", "Hotjar"], ["hj", "Hotjar"],
    ["FS", "FullStory"], ["_fs_debug", "FullStory"],
    ["clarity", "Microsoft Clarity"],
    ["LogRocket", "LogRocket"], ["_lr_loaded", "LogRocket"],
    ["smartlook", "Smartlook"],
    ["mouseflow", "Mouseflow"],
    ["__posthog", "PostHog"]
  ];
  function pollReplayGlobals() {
    const found = [];
    for (const [key, vendor] of REPLAY_GLOBALS) {
      try {
        const v = window[key];
        if (typeof v !== "undefined" && v !== null) found.push({ key, vendor });
      } catch (e) {}
    }
    if (found.length) {
      try { emit({ kind: "replay-global", vendors: found }); } catch (e) {}
    }
  }
  try {
    const scheduleReplayCheck = () => {
      setTimeout(pollReplayGlobals, 2000);
      setTimeout(pollReplayGlobals, 8000);
    };
    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", scheduleReplayCheck, { once: true });
    } else {
      scheduleReplayCheck();
    }
  } catch (e) {}

  // ---- Helpers -----------------------------------------------------------

  function urlOf(input) {
    if (typeof input === "string") return input;
    if (input && typeof input.url === "string") return input.url;
    try { return String(input); } catch (e) { return ""; }
  }

  function previewBody(body) {
    if (body == null) return null;
    try {
      if (typeof body === "string") return body.slice(0, 500);
      if (typeof FormData !== "undefined" && body instanceof FormData) {
        const fields = [];
        for (const [k] of body) fields.push(k);
        return "FormData(fields=[" + fields.slice(0, 20).join(",") + "])";
      }
      if (typeof URLSearchParams !== "undefined" && body instanceof URLSearchParams) return body.toString().slice(0, 500);
      if (typeof Blob !== "undefined" && body instanceof Blob) return "Blob(type=" + (body.type || "?") + ", size=" + body.size + ")";
      if (typeof ArrayBuffer !== "undefined" && body instanceof ArrayBuffer) return "ArrayBuffer(bytes=" + body.byteLength + ")";
      if (ArrayBuffer.isView && ArrayBuffer.isView(body)) return body.constructor.name + "(bytes=" + body.byteLength + ")";
      return String(body).slice(0, 500);
    } catch (e) { return "(unreadable body)"; }
  }

  // Main-world WASM load is intentionally absent: content_scripts with
  // world: "MAIN" do not have access to chrome.runtime.getURL, so there's
  // no way to resolve the WASM URL from here. The JS stubs above
  // dispatch `__hush_call__` CustomEvents via `emit()` directly to the
  // isolated-world content script, which runs WASM and validates every
  // payload against the typed `SignalPayload` union on receipt.
})();
