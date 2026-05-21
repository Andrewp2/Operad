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
const runUat = boolFromEnv("OPERAD_WEB_SHOWCASE_UAT");
const viewportWidth = numberFromEnv("OPERAD_WEB_SMOKE_WIDTH", 1440);
const viewportHeight = numberFromEnv("OPERAD_WEB_SMOKE_HEIGHT", 1000);
const showcaseUrl = runUat ? withQueryParam(url, "operad_uat", "1") : url;
const showcaseWindowIds = [
  "labels",
  "buttons",
  "checkbox",
  "toggles",
  "slider",
  "numeric",
  "text_input",
  "selection",
  "menus",
  "command_palette",
  "date_picker",
  "color_picker",
  "progress",
  "animation",
  "easing",
  "lists_tables",
  "property_inspector",
  "diagnostics",
  "trees",
  "layout_widgets",
  "containers",
  "panels",
  "forms",
  "overlays",
  "drag_drop",
  "media",
  "shaders",
  "shader_lab",
  "timeline",
  "canvas",
  "theme",
  "styling",
];
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
    `--window-size=${viewportWidth},${viewportHeight}`,
    "--force-device-scale-factor=1",
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
    console.log(
      `Web showcase browser ${runUat ? "smoke and UAT" : "smoke"} passed for ${showcaseUrl}`
    );
  }
} catch (error) {
  console.error(`Web showcase browser smoke failed: ${error.stack ?? error.message ?? error}`);
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

function boolFromEnv(name) {
  const raw = process.env[name];
  return raw === "1" || raw === "true" || raw === "yes";
}

function withQueryParam(rawUrl, name, value) {
  const parsed = new URL(rawUrl);
  parsed.searchParams.set(name, value);
  return parsed.toString();
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
    await send("Page.navigate", { url: showcaseUrl }, sessionId);
    await waitForShowcaseReady(send, sessionId);
    if (runUat) {
      await runShowcaseUat(send, sessionId);
    }
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
  return evaluate(send, sessionId, `({
        title: document.title,
        canvas: !!document.getElementById("operad-showcase-canvas"),
        status: document.getElementById("operad-showcase-status")?.textContent ?? null,
        hasGpu: !!navigator.gpu
      })`);
}

async function runShowcaseUat(send, sessionId) {
  await waitForUatHook(send, sessionId);
  let snapshot = await uatSnapshot(send, sessionId);
  assertSnapshotOk(snapshot);
  requireNode(snapshot, "showcase.organize_windows");
  requireNode(snapshot, "controls.add_all");
  requireNode(snapshot, "controls.clear_all");
  requireNode(snapshot, "controls.widget_list.viewport");

  await clickNode(send, sessionId, snapshot, "controls.add_all");
  snapshot = await waitForSnapshotCondition(
    send,
    sessionId,
    (next) => rootWindows(next).length === showcaseWindowIds.length,
    "all showcase windows to open"
  );
  assertSnapshotOk(snapshot);

  await clickNode(send, sessionId, snapshot, "showcase.organize_windows");
  snapshot = await waitForSnapshotCondition(
    send,
    sessionId,
    (next) => rootWindows(next).length === showcaseWindowIds.length,
    "organized showcase windows to settle"
  );
  assertSnapshotOk(snapshot);
  assertRootWindowsContained(snapshot);
  assertRootWindowsDoNotOverlap(snapshot);

  await clickNode(send, sessionId, snapshot, "controls.clear_all");
  snapshot = await waitForSnapshotCondition(
    send,
    sessionId,
    (next) => rootWindows(next).length === 0,
    "showcase windows to clear"
  );
  assertSnapshotOk(snapshot);

  await runTextInputUat(send, sessionId, snapshot);
  snapshot = await uatSnapshot(send, sessionId);

  await runSliderPointerUat(send, sessionId, snapshot);
  snapshot = await uatSnapshot(send, sessionId);

  await runDragDropUat(send, sessionId, snapshot);
  snapshot = await uatSnapshot(send, sessionId);

  await runAnimationScrollUat(send, sessionId, snapshot);
  snapshot = await uatSnapshot(send, sessionId);

  snapshot = await scrollWidgetListToEnd(send, sessionId, snapshot);
  assertScrollAtEnd(snapshot, "controls.widget_list.viewport");
}

async function runTextInputUat(send, sessionId, snapshot) {
  await clickNode(send, sessionId, snapshot, "controls.text_input");
  snapshot = await waitForSnapshotCondition(
    send,
    sessionId,
    (next) => rootWindows(next).some((node) => node.name.endsWith(".text_input")),
    "text input window to open"
  );
  requireNode(snapshot, "showcase.windows.window.text_input");
  snapshot = await ensureWindowExpanded(send, sessionId, snapshot, "text_input", [
    "text.input",
    "text.area",
  ]);

  await replaceTextInputValue(
    send,
    sessionId,
    snapshot,
    "text.input",
    "Browser UAT"
  );
  snapshot = await waitForTextValue(
    send,
    sessionId,
    "text.input",
    "Browser UAT"
  );
  requireFocusedNode(snapshot, "text.input");

  await replaceTextInputValue(
    send,
    sessionId,
    snapshot,
    "text.area",
    "Line one\nLine two"
  );
  snapshot = await waitForTextValue(
    send,
    sessionId,
    "text.area",
    "Line one\nLine two"
  );
  requireFocusedNode(snapshot, "text.area");

  await clickNode(send, sessionId, snapshot, "controls.clear_all");
  await waitForSnapshotCondition(
    send,
    sessionId,
    (next) => rootWindows(next).length === 0,
    "text input window to clear"
  );
}

async function runSliderPointerUat(send, sessionId, snapshot) {
  await clickNode(send, sessionId, snapshot, "controls.slider");
  snapshot = await waitForSnapshotCondition(
    send,
    sessionId,
    (next) => rootWindows(next).some((node) => node.name.endsWith(".slider")),
    "slider window to open"
  );
  requireNode(snapshot, "showcase.windows.window.slider");
  snapshot = await ensureWindowExpanded(send, sessionId, snapshot, "slider", [
    "slider.value",
    "slider.value_text",
  ]);

  const before = requireNode(snapshot, "slider.value").accessibility?.value;
  await dragNodeToFraction(send, sessionId, snapshot, "slider.value", 0.9, 0.5);
  snapshot = await waitForSnapshotCondition(
    send,
    sessionId,
    (next) => {
      const value = requireNode(next, "slider.value").accessibility?.value;
      return value !== before && sliderPercent(value) >= 75;
    },
    "slider drag to update value"
  );
  const edited = requireNode(snapshot, "slider.value").accessibility?.value;
  if (sliderPercent(edited) < 75) {
    throw new Error(`slider value did not move far enough after drag: ${edited}`);
  }

  await clickNode(send, sessionId, snapshot, "controls.clear_all");
  await waitForSnapshotCondition(
    send,
    sessionId,
    (next) => rootWindows(next).length === 0,
    "slider window to clear"
  );
}

async function runDragDropUat(send, sessionId, snapshot) {
  snapshot = await scrollNodeIntoView(send, sessionId, snapshot, "controls.drag_drop");
  await clickNode(send, sessionId, snapshot, "controls.drag_drop");
  snapshot = await waitForSnapshotCondition(
    send,
    sessionId,
    (next) => rootWindows(next).some((node) => node.name.endsWith(".drag_drop")),
    "drag and drop window to open"
  );
  requireNode(snapshot, "showcase.windows.window.drag_drop");
  snapshot = await ensureWindowExpanded(send, sessionId, snapshot, "drag_drop", [
    "drag_drop.text_source",
    "drag_drop.accept_text",
    "drag_drop.status",
  ]);

  await dragNodeToNode(
    send,
    sessionId,
    snapshot,
    "drag_drop.text_source",
    "drag_drop.accept_text"
  );
  snapshot = await waitForNodeLabel(
    send,
    sessionId,
    "drag_drop.status",
    "Text payload accepted"
  );

  await clickNode(send, sessionId, snapshot, "drag_drop.disabled");
  await delay(120);
  snapshot = await uatSnapshot(send, sessionId);
  requireNodeLabel(snapshot, "drag_drop.status", "Text payload accepted");

  await clickNode(send, sessionId, snapshot, "controls.clear_all");
  await waitForSnapshotCondition(
    send,
    sessionId,
    (next) => rootWindows(next).length === 0,
    "drag and drop window to clear"
  );
}

async function runAnimationScrollUat(send, sessionId, snapshot) {
  snapshot = await scrollNodeIntoView(send, sessionId, snapshot, "controls.animation");
  await clickNode(send, sessionId, snapshot, "controls.animation");
  snapshot = await waitForSnapshotCondition(
    send,
    sessionId,
    (next) => rootWindows(next).some((node) => node.name.endsWith(".animation")),
    "animation window to open"
  );
  requireNode(snapshot, "showcase.windows.window.animation");
  snapshot = await ensureWindowExpanded(send, sessionId, snapshot, "animation", [
    "animation.section_scroll",
  ]);

  const viewport = requireNode(snapshot, "animation.section_scroll");
  if (!viewport.scroll || viewport.scroll.maxOffset.y <= 0) {
    throw new Error("animation section did not expose a vertical scroll range");
  }
  const point = nodeCenter(viewport);
  for (let i = 0; i < 12; i += 1) {
    await wheelAt(send, sessionId, point, 360);
    await delay(16);
  }
  snapshot = await waitForSnapshotCondition(
    send,
    sessionId,
    (next) => {
      const scroll = requireNode(next, "animation.section_scroll").scroll;
      return scroll && scroll.offset.y >= scroll.maxOffset.y - 1;
    },
    "animation section scroll to reach the bottom"
  );
  assertScrollAtEnd(snapshot, "animation.section_scroll");

  await clickNode(send, sessionId, snapshot, "controls.clear_all");
  await waitForSnapshotCondition(
    send,
    sessionId,
    (next) => rootWindows(next).length === 0,
    "animation window to clear"
  );
}

async function ensureWindowExpanded(send, sessionId, snapshot, id, requiredNodeNames) {
  if (requiredNodeNames.every((name) => findVisibleNode(snapshot, name))) {
    return snapshot;
  }
  await clickNode(send, sessionId, snapshot, `showcase.windows.window.${id}.collapse`);
  return waitForSnapshotCondition(
    send,
    sessionId,
    (next) => requiredNodeNames.every((name) => findVisibleNode(next, name)),
    `${id} window to expand`
  );
}

async function waitForUatHook(send, sessionId) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() <= deadline) {
    const enabled = await evaluate(
      send,
      sessionId,
      `typeof window.__OPERAD_UAT__?.snapshot === "function"`
    );
    if (enabled) return;
    await delay(100);
  }
  throw new Error("showcase UAT hook was not installed");
}

async function uatSnapshot(send, sessionId) {
  const snapshot = await evaluate(
    send,
    sessionId,
    `window.__OPERAD_UAT__.snapshot()`
  );
  assertSnapshotOk(snapshot);
  return snapshot;
}

async function evaluate(send, sessionId, expression) {
  const result = await send(
    "Runtime.evaluate",
    {
      expression,
      returnByValue: true,
      awaitPromise: true,
    },
    sessionId
  );
  if (result.exceptionDetails) {
    throw new Error(
      `browser evaluation failed: ${
        result.exceptionDetails.exception?.description ??
        result.exceptionDetails.text ??
        "unknown exception"
      }`
    );
  }
  return result.result.value;
}

async function waitForSnapshotCondition(send, sessionId, predicate, description) {
  const deadline = Date.now() + timeoutMs;
  let lastSnapshot = null;
  while (Date.now() <= deadline) {
    lastSnapshot = await uatSnapshot(send, sessionId);
    if (predicate(lastSnapshot)) {
      return lastSnapshot;
    }
    await delay(100);
  }
  throw new Error(
    `timed out waiting for ${description}; last snapshot: ${snapshotSummary(lastSnapshot)}`
  );
}

async function clickNode(send, sessionId, snapshot, name) {
  const node = requireNode(snapshot, name);
  const point = nodeCenter(node);
  assertPointInsideClip(node, point);
  await send(
    "Input.dispatchMouseEvent",
    {
      type: "mousePressed",
      x: point.x,
      y: point.y,
      button: "left",
      buttons: 1,
      clickCount: 1,
    },
    sessionId
  );
  await send(
    "Input.dispatchMouseEvent",
    {
      type: "mouseReleased",
      x: point.x,
      y: point.y,
      button: "left",
      buttons: 0,
      clickCount: 1,
    },
    sessionId
  );
  await delay(120);
}

async function scrollNodeIntoView(send, sessionId, snapshot, name) {
  for (let attempt = 0; attempt < 24; attempt += 1) {
    const node = findVisibleNode(snapshot, name);
    if (node && pointInsideRect(nodeCenter(node), node.clipRect)) {
      return snapshot;
    }
    const viewport = requireNode(snapshot, "controls.widget_list.viewport");
    const target = findNode(snapshot, name);
    const targetCenterY = target ? nodeCenter(target).y : Number.POSITIVE_INFINITY;
    const down = !target || targetCenterY > viewport.clipRect.bottom;
    await wheelAt(send, sessionId, nodeCenter(viewport), down ? 420 : -420);
    await delay(16);
    snapshot = await uatSnapshot(send, sessionId);
  }
  throw new Error(`could not scroll ${name} into view`);
}

async function dragNodeToFraction(send, sessionId, snapshot, name, xFraction, yFraction) {
  const node = requireNode(snapshot, name);
  const from = nodeCenter(node);
  const to = nodePointAtFraction(node, xFraction, yFraction);
  await dragPointer(send, sessionId, from, to);
}

async function dragNodeToNode(send, sessionId, snapshot, sourceName, targetName) {
  const source = requireNode(snapshot, sourceName);
  const target = requireNode(snapshot, targetName);
  const from = nodeCenter(source);
  const to = nodeCenter(target);
  assertPointInsideClip(source, from);
  assertPointInsideClip(target, to);
  await dragPointer(send, sessionId, from, to);
}

async function dragPointer(send, sessionId, from, to) {
  await send(
    "Input.dispatchMouseEvent",
    {
      type: "mousePressed",
      x: from.x,
      y: from.y,
      button: "left",
      buttons: 1,
      clickCount: 1,
    },
    sessionId
  );
  const steps = 8;
  for (let step = 1; step <= steps; step += 1) {
    const t = step / steps;
    await send(
      "Input.dispatchMouseEvent",
      {
        type: "mouseMoved",
        x: from.x + (to.x - from.x) * t,
        y: from.y + (to.y - from.y) * t,
        button: "left",
        buttons: 1,
      },
      sessionId
    );
    await delay(16);
  }
  await send(
    "Input.dispatchMouseEvent",
    {
      type: "mouseReleased",
      x: to.x,
      y: to.y,
      button: "left",
      buttons: 0,
      clickCount: 1,
    },
    sessionId
  );
  await delay(120);
}

async function waitForNodeLabel(send, sessionId, name, expectedText) {
  return waitForSnapshotCondition(
    send,
    sessionId,
    (next) => nodeLabel(requireNode(next, name)).includes(expectedText),
    `${name} label to include ${JSON.stringify(expectedText)}`
  );
}

async function replaceTextInputValue(send, sessionId, snapshot, name, value) {
  await clickNode(send, sessionId, snapshot, name);
  snapshot = await waitForSnapshotCondition(
    send,
    sessionId,
    (next) => next.focus.focused === name,
    `${name} to receive focus`
  );
  requireFocusedNode(snapshot, name);
  await pressKey(send, sessionId, "a", { modifiers: 2, code: "KeyA", virtualKeyCode: 65 });
  await typeText(send, sessionId, value);
}

async function waitForTextValue(send, sessionId, name, expected) {
  return waitForSnapshotCondition(
    send,
    sessionId,
    (next) => requireNode(next, name).accessibility?.value === expected,
    `${name} value to become ${JSON.stringify(expected)}`
  );
}

async function typeText(send, sessionId, text) {
  for (const char of text) {
    if (char === "\n") {
      await pressKey(send, sessionId, "Enter", { code: "Enter", virtualKeyCode: 13 });
    } else {
      await pressKey(send, sessionId, char, keyOptionsForChar(char));
    }
  }
  await delay(120);
}

async function pressKey(send, sessionId, key, options = {}) {
  const params = {
    key,
    code: options.code ?? key,
    windowsVirtualKeyCode: options.virtualKeyCode ?? key.toUpperCase().charCodeAt(0),
    nativeVirtualKeyCode: options.virtualKeyCode ?? key.toUpperCase().charCodeAt(0),
    modifiers: options.modifiers ?? 0,
  };
  await send("Input.dispatchKeyEvent", { type: "keyDown", ...params }, sessionId);
  await send("Input.dispatchKeyEvent", { type: "keyUp", ...params }, sessionId);
}

function keyOptionsForChar(char) {
  if (char === " ") {
    return { code: "Space", virtualKeyCode: 32 };
  }
  if (/^[a-z]$/i.test(char)) {
    return { code: `Key${char.toUpperCase()}`, virtualKeyCode: char.toUpperCase().charCodeAt(0) };
  }
  if (/^[0-9]$/.test(char)) {
    return { code: `Digit${char}`, virtualKeyCode: char.charCodeAt(0) };
  }
  return { code: char, virtualKeyCode: char.charCodeAt(0) };
}

async function wheelAt(send, sessionId, point, deltaY) {
  await send(
    "Input.dispatchMouseEvent",
    {
      type: "mouseWheel",
      x: point.x,
      y: point.y,
      deltaX: 0,
      deltaY,
    },
    sessionId
  );
}

async function scrollWidgetListToEnd(send, sessionId, snapshot) {
  const viewport = requireNode(snapshot, "controls.widget_list.viewport");
  const point = nodeCenter(viewport);
  for (let i = 0; i < 24; i += 1) {
    await wheelAt(send, sessionId, point, 600);
    await delay(16);
  }
  return waitForSnapshotCondition(
    send,
    sessionId,
    (next) => {
      const scroll = requireNode(next, "controls.widget_list.viewport").scroll;
      return scroll && scroll.offset.y >= scroll.maxOffset.y - 1;
    },
    "widget list scroll to reach the end"
  );
}

function assertSnapshotOk(snapshot) {
  if (!snapshot || typeof snapshot !== "object") {
    throw new Error("showcase UAT snapshot was not an object");
  }
  if (snapshot.error) {
    throw new Error(
      `showcase UAT ${snapshot.error} failed: ${snapshot.message ?? "unknown error"}`
    );
  }
  if (!Array.isArray(snapshot.nodes)) {
    throw new Error("showcase UAT snapshot did not include nodes");
  }
}

function requireNode(snapshot, name) {
  const node = findNode(snapshot, name);
  if (!node) {
    throw new Error(`missing UAT node ${name}; ${snapshotSummary(snapshot)}`);
  }
  if (!node.visible) {
    throw new Error(`UAT node ${name} is hidden`);
  }
  const { rect } = node;
  if (
    !rect ||
    !Number.isFinite(rect.width) ||
    !Number.isFinite(rect.height) ||
    rect.width <= 0 ||
    rect.height <= 0
  ) {
    throw new Error(`UAT node ${name} has invalid rect ${JSON.stringify(rect)}`);
  }
  return node;
}

function findNode(snapshot, name) {
  return snapshot.nodes.find((candidate) => candidate.name === name);
}

function findVisibleNode(snapshot, name) {
  const node = findNode(snapshot, name);
  return node?.visible ? node : undefined;
}

function requireFocusedNode(snapshot, name) {
  if (snapshot.focus?.focused !== name) {
    throw new Error(
      `expected ${name} to be focused; focus state was ${JSON.stringify(snapshot.focus)}`
    );
  }
}

function requireNodeLabel(snapshot, name, expectedText) {
  const label = nodeLabel(requireNode(snapshot, name));
  if (!label.includes(expectedText)) {
    throw new Error(
      `${name} label did not include ${JSON.stringify(expectedText)}; got ${JSON.stringify(label)}`
    );
  }
}

function rootWindows(snapshot) {
  const windowNames = new Set(
    showcaseWindowIds.map((id) => `showcase.windows.window.${id}`)
  );
  return snapshot.nodes.filter(
    (node) => node.visible && windowNames.has(node.name)
  );
}

function assertRootWindowsContained(snapshot) {
  const desktopWidth = snapshot.viewport.width - 300;
  const bounds = {
    x: 0,
    y: 44,
    right: desktopWidth,
    bottom: snapshot.viewport.height,
  };
  for (const node of rootWindows(snapshot)) {
    const rect = node.rect;
    if (
      rect.x < bounds.x - 1 ||
      rect.y < bounds.y - 1 ||
      rect.right > bounds.right + 1 ||
      rect.bottom > bounds.bottom + 1
    ) {
      throw new Error(
        `${node.name} was organized outside the desktop bounds: ${JSON.stringify(rect)}`
      );
    }
  }
}

function assertRootWindowsDoNotOverlap(snapshot) {
  const windows = rootWindows(snapshot);
  for (let left = 0; left < windows.length; left += 1) {
    for (let right = left + 1; right < windows.length; right += 1) {
      if (rectsOverlap(windows[left].rect, windows[right].rect, 1)) {
        throw new Error(
          `organized windows overlap: ${windows[left].name} ${JSON.stringify(
            windows[left].rect
          )} and ${windows[right].name} ${JSON.stringify(windows[right].rect)}`
        );
      }
    }
  }
}

function assertScrollAtEnd(snapshot, name) {
  const node = requireNode(snapshot, name);
  const scroll = node.scroll;
  if (!scroll) {
    throw new Error(`${name} did not expose scroll state`);
  }
  if (scroll.maxOffset.y <= 0) {
    throw new Error(`${name} did not have vertical scroll range`);
  }
  if (scroll.offset.y < scroll.maxOffset.y - 1) {
    throw new Error(
      `${name} stopped before the bottom: offset=${scroll.offset.y}, max=${scroll.maxOffset.y}`
    );
  }
}

function rectsOverlap(a, b, tolerance = 0) {
  return (
    a.x < b.right - tolerance &&
    a.right > b.x + tolerance &&
    a.y < b.bottom - tolerance &&
    a.bottom > b.y + tolerance
  );
}

function nodeCenter(node) {
  return {
    x: node.rect.x + node.rect.width / 2,
    y: node.rect.y + node.rect.height / 2,
  };
}

function assertPointInsideClip(node, point) {
  if (!pointInsideRect(point, node.clipRect)) {
    throw new Error(
      `${node.name} center ${JSON.stringify(point)} is outside its clip rect ${JSON.stringify(
        node.clipRect
      )}`
    );
  }
}

function pointInsideRect(point, rect) {
  if (!rect) return true;
  return (
    point.x >= rect.x - 0.5 &&
    point.x <= rect.right + 0.5 &&
    point.y >= rect.y - 0.5 &&
    point.y <= rect.bottom + 0.5
  );
}

function nodePointAtFraction(node, xFraction, yFraction) {
  return {
    x: node.rect.x + node.rect.width * xFraction,
    y: node.rect.y + node.rect.height * yFraction,
  };
}

function nodeLabel(node) {
  return node.accessibility?.label ?? node.accessibility?.value ?? "";
}

function sliderPercent(value) {
  const match = String(value ?? "").match(/\(([-+]?\d+(?:\.\d+)?)%\)/);
  return match ? Number(match[1]) : Number.NaN;
}

function snapshotSummary(snapshot) {
  if (!snapshot) return "no snapshot";
  return `${snapshot.nodeCount ?? "?"} nodes, ${
    rootWindows(snapshot).length
  } root windows`;
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
