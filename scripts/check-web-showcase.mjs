#!/usr/bin/env node

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";

const url = process.argv[2];
if (!url) {
  console.error("usage: check-web-showcase.mjs <url>");
  process.exit(2);
}

if (typeof WebSocket !== "function") {
  console.error("Node.js with a global WebSocket implementation is required.");
  process.exit(2);
}

const timeoutMs = numberFromEnv("OPERAD_WEB_SMOKE_TIMEOUT_MS", 15_000);
const settleMs = numberFromEnv("OPERAD_WEB_SMOKE_SETTLE_MS", 3_000);
const chromePath = findChrome();
if (!chromePath) {
  console.error(
    "Could not find a Chrome binary. Set CHROME_BIN or install google-chrome/chromium."
  );
  process.exit(2);
}

const profile = fs.mkdtempSync(path.join(os.tmpdir(), "operad-web-smoke-"));
const chrome = spawn(
  chromePath,
  [
    "--headless=new",
    "--remote-debugging-port=0",
    "--enable-unsafe-webgpu",
    "--ignore-gpu-blocklist",
    "--no-first-run",
    "--no-default-browser-check",
    "--no-sandbox",
    `--user-data-dir=${profile}`,
    "about:blank",
  ],
  { stdio: ["ignore", "pipe", "pipe"] }
);

let stdout = "";
let stderr = "";
chrome.stdout.on("data", (chunk) => {
  stdout += chunk;
});
chrome.stderr.on("data", (chunk) => {
  stderr += chunk;
});

try {
  const browserWsUrl = await waitForDevtoolsEndpoint();
  const events = await runSmoke(browserWsUrl);
  const failures = smokeFailures(events);
  if (failures.length > 0) {
    console.error("Web showcase browser smoke failed:");
    for (const failure of failures) {
      console.error(`- ${failure}`);
    }
    process.exitCode = 1;
  } else {
    console.log(`Web showcase browser smoke passed for ${url}`);
  }
} catch (error) {
  console.error(`Web showcase browser smoke failed: ${error.message ?? error}`);
  process.exitCode = 1;
} finally {
  await terminateChrome(chrome);
  await removeProfile(profile);
}

function numberFromEnv(name, fallback) {
  const raw = process.env[name];
  if (!raw) return fallback;
  const parsed = Number(raw);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

function findChrome() {
  const candidates = [
    process.env.CHROME_BIN,
    "/usr/bin/google-chrome",
    "/usr/bin/google-chrome-stable",
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
  ].filter(Boolean);
  return candidates.find((candidate) => fs.existsSync(candidate));
}

function waitForDevtoolsEndpoint() {
  return new Promise((resolve, reject) => {
    const deadline = Date.now() + 10_000;
    const timer = setInterval(() => {
      const text = `${stderr}\n${stdout}`;
      const match = text.match(/DevTools listening on (ws:\/\/[^\s]+)/);
      if (match) {
        clearInterval(timer);
        resolve(match[1]);
      } else if (Date.now() > deadline) {
        clearInterval(timer);
        reject(
          new Error(
            `Chrome did not publish a DevTools endpoint. Recent output:\n${text.slice(-2000)}`
          )
        );
      }
    }, 50);
  });
}

async function runSmoke(browserWsUrl) {
  const ws = new WebSocket(browserWsUrl);
  let id = 0;
  const pending = new Map();
  const events = [];
  const requestUrls = new Map();

  function send(method, params = {}, sessionId = undefined) {
    const message = { id: ++id, method, params };
    if (sessionId) message.sessionId = sessionId;
    ws.send(JSON.stringify(message));
    return new Promise((resolve, reject) => {
      pending.set(message.id, { method, resolve, reject });
    });
  }

  ws.onmessage = (messageEvent) => {
    const message = JSON.parse(messageEvent.data);
    if (message.id && pending.has(message.id)) {
      const pendingMessage = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) {
        pendingMessage.reject(
          new Error(`${pendingMessage.method}: ${JSON.stringify(message.error)}`)
        );
      } else {
        pendingMessage.resolve(message.result);
      }
      return;
    }

    if (message.method === "Network.requestWillBeSent") {
      requestUrls.set(message.params.requestId, message.params.request.url);
    }
    if (
      message.method === "Runtime.consoleAPICalled" ||
      message.method === "Runtime.exceptionThrown" ||
      message.method === "Log.entryAdded" ||
      message.method === "Network.loadingFailed"
    ) {
      events.push({ ...message, url: requestUrls.get(message.params?.requestId) });
    }
  };

  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = reject;
  });

  try {
    const { targetId } = await send("Target.createTarget", { url: "about:blank" });
    const { sessionId } = await send("Target.attachToTarget", {
      targetId,
      flatten: true,
    });
    await send("Runtime.enable", {}, sessionId);
    await send("Log.enable", {}, sessionId);
    await send("Network.enable", {}, sessionId);
    await send("Page.enable", {}, sessionId);
    await send("Page.navigate", { url }, sessionId);
    await waitForShowcaseReady(send, sessionId);
    await delay(settleMs);
    return events;
  } finally {
    ws.close();
  }
}

async function waitForShowcaseReady(send, sessionId) {
  const deadline = Date.now() + timeoutMs;
  let lastState = null;
  while (Date.now() <= deadline) {
    lastState = await evaluateShowcaseState(send, sessionId);
    if (lastState.status === null && lastState.canvas && lastState.hasGpu) {
      return;
    }
    if (typeof lastState.status === "string" && lastState.status.startsWith("Failed")) {
      throw new Error(`showcase reported startup failure: ${lastState.status}`);
    }
    await delay(250);
  }
  throw new Error(
    `showcase did not finish startup within ${timeoutMs}ms; last state: ${JSON.stringify(
      lastState
    )}`
  );
}

async function evaluateShowcaseState(send, sessionId) {
  const result = await send(
    "Runtime.evaluate",
    {
      expression: `({
        title: document.title,
        canvas: !!document.getElementById("operad-showcase-canvas"),
        status: document.getElementById("operad-showcase-status")?.textContent ?? null,
        hasGpu: !!navigator.gpu
      })`,
      returnByValue: true,
    },
    sessionId
  );
  return result.result.value;
}

function smokeFailures(events) {
  const failures = [];
  for (const event of events) {
    if (event.method === "Runtime.exceptionThrown") {
      failures.push(
        `uncaught exception: ${
          event.params.exceptionDetails?.exception?.description ??
          event.params.exceptionDetails?.text ??
          "unknown exception"
        }`
      );
    } else if (
      event.method === "Runtime.consoleAPICalled" &&
      event.params.type === "error"
    ) {
      failures.push(`console.error: ${consoleArgs(event.params.args)}`);
    } else if (event.method === "Log.entryAdded") {
      const entry = event.params.entry;
      if (entry.level === "error") {
        failures.push(`browser log error: ${entry.text}`);
      } else if (
        entry.level === "warning" &&
        /\b(WGSL|WebGPU|GPU|ShaderModule)\b/i.test(entry.text)
      ) {
        failures.push(`browser GPU warning: ${entry.text}`);
      }
    } else if (
      event.method === "Network.loadingFailed" &&
      !String(event.url ?? "").endsWith("/favicon.ico")
    ) {
      failures.push(
        `network failure for ${event.url ?? event.params.requestId}: ${event.params.errorText}`
      );
    }
  }
  return failures;
}

function consoleArgs(args) {
  return args
    .map((arg) => arg.value ?? arg.description ?? arg.unserializableValue ?? arg.type)
    .join(" ");
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function terminateChrome(process) {
  if (process.exitCode !== null || process.signalCode !== null) {
    return Promise.resolve();
  }
  return new Promise((resolve) => {
    let resolved = false;
    const finish = () => {
      if (resolved) return;
      resolved = true;
      clearTimeout(termTimeout);
      clearTimeout(killTimeout);
      resolve();
    };
    const termTimeout = setTimeout(() => {
      if (process.exitCode === null && process.signalCode === null) {
        process.kill("SIGKILL");
      }
    }, 1_000);
    const killTimeout = setTimeout(finish, 3_000);
    process.once("exit", () => {
      finish();
    });
    process.kill("SIGTERM");
  });
}

async function removeProfile(profile) {
  let lastError = null;
  for (let attempt = 0; attempt < 10; attempt += 1) {
    try {
      await fs.promises.rm(profile, { recursive: true, force: true });
      return;
    } catch (error) {
      lastError = error;
      await delay(100 * (attempt + 1));
    }
  }
  throw lastError;
}
