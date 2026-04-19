// Hush main-world hooks
//
// Runs in the page's own JavaScript context (not the isolated content-script
// world) so we can monkey-patch the real window.fetch, XMLHttpRequest,
// navigator.sendBeacon, and WebSocket. Without this we only see resources
// that the browser's network service records - we have no idea which
// script fired each call, what's in the body, or any context around it.
//
// For each intercepted call we:
//   1. capture URL, method, body preview, and a short JS stack trace
//   2. dispatch a CustomEvent("__hush_call__") on the document
//   3. pass through to the original API
//
// The isolated-world content.js listens for that event and forwards the
// details to the background service worker. From the page's perspective
// the hooks are transparent - same arguments in, same behavior out.

(() => {
  // Capture originals ONCE so subsequent page-world re-assignments
  // (some sites defensively reset these) don't break us.
  const _fetch = window.fetch;
  const _xhrOpen = XMLHttpRequest.prototype.open;
  const _xhrSend = XMLHttpRequest.prototype.send;
  const _sendBeacon = navigator.sendBeacon ? navigator.sendBeacon.bind(navigator) : null;
  const _wsSend = WebSocket.prototype.send;

  function captureStack() {
    try {
      const s = new Error().stack || "";
      // Drop the first couple of lines (this function + the hook itself)
      // and cap at 6 frames - we want "who called us," not the browser internals.
      return s.split("\n").slice(2, 8).map(l => l.trim()).filter(Boolean);
    } catch (e) {
      return [];
    }
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
      if (typeof URLSearchParams !== "undefined" && body instanceof URLSearchParams) {
        return body.toString().slice(0, 500);
      }
      if (typeof Blob !== "undefined" && body instanceof Blob) {
        return "Blob(type=" + (body.type || "?") + ", size=" + body.size + ")";
      }
      if (typeof ArrayBuffer !== "undefined" && body instanceof ArrayBuffer) {
        return "ArrayBuffer(bytes=" + body.byteLength + ")";
      }
      if (ArrayBuffer.isView && ArrayBuffer.isView(body)) {
        return body.constructor.name + "(bytes=" + body.byteLength + ")";
      }
      return String(body).slice(0, 500);
    } catch (e) {
      return "(unreadable body)";
    }
  }

  function urlOf(input) {
    if (typeof input === "string") return input;
    if (input && typeof input.url === "string") return input.url;
    try { return String(input); } catch (e) { return ""; }
  }

  function emit(kind, data) {
    try {
      document.dispatchEvent(new CustomEvent("__hush_call__", {
        detail: {
          kind,
          url: data.url || "",
          method: data.method || "",
          bodyPreview: data.bodyPreview || null,
          stack: data.stack || [],
          t: new Date().toISOString()
        }
      }));
    } catch (e) {
      // Some sites or Chrome itself may restrict CustomEvent dispatch.
      // Silent fail is acceptable - hook still passes through.
    }
  }

  // ===== fetch =====
  window.fetch = function hushFetch(input, init) {
    try {
      emit("fetch", {
        url: urlOf(input),
        method: (init && init.method) || (input && input.method) || "GET",
        bodyPreview: previewBody(init && init.body),
        stack: captureStack()
      });
    } catch (e) { /* hook must never break the site */ }
    const p = _fetch.apply(this, arguments);
    // Attach a silent rejection handler to the original promise so
    // Chrome's unhandled-rejection tracking doesn't attribute site-level
    // fetch failures (bad URLs, CORS errors, network drops) to THIS frame.
    // The site still receives the same rejecting promise from `return p`;
    // if the site doesn't attach its own .catch(), the unhandled-rejection
    // warning will fire at the site's own `.then()` or `await` site -
    // accurate to their code, not noise attributed to Hush.
    if (p && typeof p.catch === "function") p.catch(() => {});
    return p;
  };

  // ===== XMLHttpRequest =====
  XMLHttpRequest.prototype.open = function hushXhrOpen(method, url) {
    try {
      this.__hush_method = method;
      this.__hush_url = url;
    } catch (e) {}
    return _xhrOpen.apply(this, arguments);
  };

  XMLHttpRequest.prototype.send = function hushXhrSend(body) {
    try {
      emit("xhr", {
        url: this.__hush_url || "",
        method: this.__hush_method || "",
        bodyPreview: previewBody(body),
        stack: captureStack()
      });
    } catch (e) {}
    return _xhrSend.apply(this, arguments);
  };

  // ===== navigator.sendBeacon =====
  if (_sendBeacon) {
    navigator.sendBeacon = function hushSendBeacon(url, body) {
      try {
        emit("beacon", {
          url: typeof url === "string" ? url : String(url),
          method: "POST",
          bodyPreview: previewBody(body),
          stack: captureStack()
        });
      } catch (e) {}
      return _sendBeacon(url, body);
    };
  }

  // ===== WebSocket.send =====
  WebSocket.prototype.send = function hushWsSend(data) {
    try {
      emit("ws-send", {
        url: this.url || "",
        method: "WS",
        bodyPreview: previewBody(data),
        stack: captureStack()
      });
    } catch (e) {}
    return _wsSend.apply(this, arguments);
  };
})();
