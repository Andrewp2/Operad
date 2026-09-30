#!/usr/bin/env node

// A deliberately incomplete Chrome substitute for runner failure/cleanup tests.
// It accepts the runner's launch arguments but never opens a window or uses a GPU.
import { createHash } from "node:crypto";
import fs from "node:fs";
import http from "node:http";

fs.writeFileSync(process.env.OPERAD_FAKE_CHROME_PID, String(process.pid));
const mode = process.env.OPERAD_FAKE_CHROME_MODE;
if (mode === "early-exit") {
  console.error("fake Chrome startup failure");
  process.exit(23);
}
const port = Number(process.argv.find(arg => arg.startsWith("--remote-debugging-port=")).split("=")[1]);
const endpoint = `ws://127.0.0.1:${port}/devtools/browser/fake`;
const server = http.createServer((request, response) => {
  // A successful header does not imply that response.json() will ever resolve.
  response.writeHead(200, { "Content-Type": "application/json" });
  if (mode === "metadata-close") {
    response.end(JSON.stringify({ webSocketDebuggerUrl: endpoint }));
    return;
  }
  response.write('{"webSocketDebuggerUrl":');
});
server.on("upgrade", (request, socket) => {
  socket.on("error", () => {});
  if (mode === "stall-open") return;
  const accept = createHash("sha1")
    .update(request.headers["sec-websocket-key"] + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11")
    .digest("base64");
  socket.write("HTTP/1.1 101 Switching Protocols\r\n" +
    "Upgrade: websocket\r\nConnection: Upgrade\r\n" +
    `Sec-WebSocket-Accept: ${accept}\r\n\r\n`);
  socket.once("data", () => {
    if (mode === "close-command" || mode === "metadata-close") socket.destroy();
    if (mode === "invalid-message") {
      socket.write(Buffer.from([0x81, 1, 0x7b])); // Text frame containing incomplete JSON.
    }
    // stall-command deliberately never replies.
  });
});
server.listen(port, "127.0.0.1", () => {
  if (mode !== "startup-body" && mode !== "metadata-close") {
    console.error(`DevTools listening on ${endpoint}`);
  }
});
