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
      try {
        emit({
          kind: "fetch",
          url: urlOf(input),
          method: (init && init.method) || (input && input.method) || "GET",
          bodyPreview: previewBody(init && init.body),
          stack: cap()
        });
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
      try {
        emit({
          kind: "xhr",
          url: this.__hush_url || "",
          method: this.__hush_method || "",
          bodyPreview: previewBody(body),
          stack: cap()
        });
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
        try {
          emit({
            kind: "beacon",
            url: typeof url === "string" ? url : String(url),
            bodyPreview: previewBody(body),
            stack: cap()
          });
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

  // Canvas FP: toDataURL, toBlob (stub now; Rust re-patches post-load)
  try {
    const _toDataURL = captureOrig(HTMLCanvasElement.prototype, "toDataURL");
    HTMLCanvasElement.prototype.toDataURL = function hushToDataURL() {
      try { emit({ kind: "canvas-fp", method: "toDataURL", stack: cap() }); } catch (e) {}
      return _toDataURL.apply(this, arguments);
    };
  } catch (e) {}
  try {
    if (HTMLCanvasElement.prototype.toBlob) {
      const _toBlob = captureOrig(HTMLCanvasElement.prototype, "toBlob");
      HTMLCanvasElement.prototype.toBlob = function hushToBlob() {
        try { emit({ kind: "canvas-fp", method: "toBlob", stack: cap() }); } catch (e) {}
        return _toBlob.apply(this, arguments);
      };
    }
  } catch (e) {}

  // Canvas 2D getImageData + measureText (stub now; Rust re-patches)
  try {
    if (typeof CanvasRenderingContext2D !== "undefined") {
      const _getImageData = captureOrig(CanvasRenderingContext2D.prototype, "getImageData");
      CanvasRenderingContext2D.prototype.getImageData = function hushGetImageData() {
        try { emit({ kind: "canvas-fp", method: "getImageData", stack: cap() }); } catch (e) {}
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
      if (param === 37445 || param === 37446) {
        try {
          const el = document.documentElement;
          const spoof = el && el.dataset && el.dataset.hushSpoof;
          if (spoof && spoof.indexOf("webgl-unmasked") >= 0) {
            return param === 37445 ? "Google Inc." : "ANGLE (Generic)";
          }
        } catch (e) { /* fall through to real value */ }
      }
      return origFn.apply(this, arguments);
    };
  };
  try { if (typeof WebGLRenderingContext !== "undefined") wrapGLStub(WebGLRenderingContext.prototype); } catch (e) {}
  try { if (typeof WebGL2RenderingContext !== "undefined") wrapGLStub(WebGL2RenderingContext.prototype); } catch (e) {}

  // OfflineAudioContext constructor (stays JS-only - constructor replacement pattern)
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
      try {
        if (typeof type === "string" && REPLAY_EVENT_TYPES.has(type)) {
          const onDocLike = (this === document || this === window ||
                             (typeof document !== "undefined" && this === document.body));
          if (onDocLike) {
            emit({ kind: "listener-added", eventType: type, stack: cap() });
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
