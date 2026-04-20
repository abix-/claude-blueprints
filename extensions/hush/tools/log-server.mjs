// Tiny HTTP sink for Hush debug logs. The extension POSTs lines to
// http://localhost:8765/log and the server appends them to
// /tmp/hush.log. Run with `node tools/log-server.mjs`.

import http from "node:http";
import fs from "node:fs";

const LOG_PATH = "C:/tmp/hush.log";

// Truncate on start so each debug session is fresh.
try { fs.writeFileSync(LOG_PATH, ""); } catch {}

const server = http.createServer((req, res) => {
  if (req.method !== "POST" || !req.url.startsWith("/log")) {
    res.writeHead(404);
    res.end();
    return;
  }
  let body = "";
  req.on("data", (chunk) => { body += chunk; });
  req.on("end", () => {
    const line = `[${new Date().toISOString()}] ${body}\n`;
    try { fs.appendFileSync(LOG_PATH, line); } catch (e) { /* ignore */ }
    res.writeHead(204, { "Access-Control-Allow-Origin": "*" });
    res.end();
  });
});

server.listen(8765, "127.0.0.1", () => {
  console.log(`hush log sink listening on http://127.0.0.1:8765, writing to ${LOG_PATH}`);
});
