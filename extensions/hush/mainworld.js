// Hush main-world hooks (Stage 3 hybrid bootstrap).
//
// Runs in the page's own JS context (manifest content_scripts with
// world=MAIN + run_at=document_start) so we can monkey-patch the real
// window.fetch, XMLHttpRequest, navigator.sendBeacon, WebSocket,
// HTMLCanvasElement, WebGL contexts, OfflineAudioContext, and
// EventTarget.addEventListener before any page script runs.
//
// The installation MUST be synchronous at document_start. WASM
// instantiation is async (fetch + compile), so we can't have the Rust
// engine installed before inline <head> scripts run. The hybrid split
// handles that:
//
//   1. At document_start we install tiny JS stubs on every target
//      prototype method. Each stub captures args + stack and pushes a
//      typed SignalPayload-compatible object onto an in-page queue.
//   2. In parallel we dynamically import dist/pkg/hush.js from the
//      extension's web_accessible_resources. When WASM finishes init,
//      the bootstrap flips a flag and calls drainStubQueue to play
//      everything back through the Rust validator + dispatcher.
//   3. After that flip, new hook invocations skip the queue and go
//      straight through dispatchHook. Every payload is validated by
//      serde against the SignalPayload enum; invalid shapes (the
//      0.5.0 bug class) log and drop instead of silently reaching the
//      detectors.
//
// Everything post-capture is Rust: payload typing, CustomEvent
// construction, dispatch. The JS surface is kept to the physically-
// required minimum (prototype assignments that need the implicit JS
// `this` binding that wasm-bindgen closures can't forward).

(() => {
  // ---- Queue + dispatch bridge -------------------------------------------
  const MAX_QUEUE = 2000; // drop oldest on overflow; bounds pre-WASM memory
  let wasmMod = null;
  let wasmReady = false;
  // Exposed on `window` so the JS emit-contract test harness (which
  // never loads WASM) can read the queue to verify hook behavior.
  const q = (window.__hush_stub_q__ = []);

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
    if (wasmReady && wasmMod) {
      try { wasmMod.dispatchHook(detail); }
      catch (e) { console.error("[Hush] dispatchHook failed", e, detail); }
      return;
    }
    q.push(detail);
    if (q.length > MAX_QUEUE) q.splice(0, q.length - MAX_QUEUE);
  }

  // ---- Prototype patches --------------------------------------------------

  // fetch
  try {
    const _fetch = window.fetch;
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
    const _open = XMLHttpRequest.prototype.open;
    const _send = XMLHttpRequest.prototype.send;
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
    const _wsSend = WebSocket.prototype.send;
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

  // Canvas fingerprinting: toDataURL, toBlob, getImageData, measureText
  try {
    const _toDataURL = HTMLCanvasElement.prototype.toDataURL;
    HTMLCanvasElement.prototype.toDataURL = function hushToDataURL() {
      try { emit({ kind: "canvas-fp", method: "toDataURL", stack: cap() }); } catch (e) {}
      return _toDataURL.apply(this, arguments);
    };
  } catch (e) {}
  try {
    if (HTMLCanvasElement.prototype.toBlob) {
      const _toBlob = HTMLCanvasElement.prototype.toBlob;
      HTMLCanvasElement.prototype.toBlob = function hushToBlob() {
        try { emit({ kind: "canvas-fp", method: "toBlob", stack: cap() }); } catch (e) {}
        return _toBlob.apply(this, arguments);
      };
    }
  } catch (e) {}
  try {
    if (typeof CanvasRenderingContext2D !== "undefined") {
      const _getImageData = CanvasRenderingContext2D.prototype.getImageData;
      CanvasRenderingContext2D.prototype.getImageData = function hushGetImageData() {
        try { emit({ kind: "canvas-fp", method: "getImageData", stack: cap() }); } catch (e) {}
        return _getImageData.apply(this, arguments);
      };
      const _measureText = CanvasRenderingContext2D.prototype.measureText;
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

  // WebGL / WebGL2 getParameter
  const wrapGLGetParameter = (proto) => {
    if (!proto || !proto.getParameter) return;
    const orig = proto.getParameter;
    proto.getParameter = function hushGLGetParameter(param) {
      try {
        const hotParam = param === 37445 || param === 37446;
        emit({
          kind: "webgl-fp",
          param: String(param),
          hotParam: hotParam,
          stack: cap()
        });
      } catch (e) {}
      return orig.apply(this, arguments);
    };
  };
  try { if (typeof WebGLRenderingContext !== "undefined") wrapGLGetParameter(WebGLRenderingContext.prototype); } catch (e) {}
  try { if (typeof WebGL2RenderingContext !== "undefined") wrapGLGetParameter(WebGL2RenderingContext.prototype); } catch (e) {}

  // OfflineAudioContext
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

  // EventTarget.addEventListener density (replay detection)
  try {
    const _addEventListener = EventTarget.prototype.addEventListener;
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

  // Canvas 2D draw ops: visibility-sampled invisible-animation-loop detector
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
        const orig = CanvasRenderingContext2D.prototype[op];
        if (typeof orig !== "function") continue;
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
          return orig.apply(this, arguments);
        };
      }
    }
  } catch (e) {}

  // Replay-global poll: check for known vendor sentinels.
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

  // ---- Helpers ------------------------------------------------------------

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

  // ---- Async WASM load + queue drain -------------------------------------

  try {
    const url = chrome.runtime.getURL("dist/pkg/hush.js");
    import(url).then(async (m) => {
      await m.default();
      if (typeof m.initEngine === "function") m.initEngine();
      wasmMod = m;
      wasmReady = true;
      if (q.length) {
        try { m.drainStubQueue(q); }
        catch (e) { console.error("[Hush] drainStubQueue failed", e); }
        q.length = 0;
      }
    }).catch((e) => {
      // Strict-CSP sites may block the import or WASM execution. Stubs
      // continue capturing into the queue (and will drop on overflow).
      // No re-dispatch happens, which is the same behavior as today on
      // those sites - not a regression.
      console.error("[Hush] wasm load failed", e);
    });
  } catch (e) {
    console.error("[Hush] dynamic import unavailable", e);
  }
})();
