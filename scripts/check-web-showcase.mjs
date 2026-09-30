#!/usr/bin/env node

import fs from "node:fs";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";
import { connectCdp } from "./cdp-client.mjs";
import { runRuntimeEditorProbe } from "./runtime-editor-probe.mjs";
import { runRuntimeImeProbe } from "./runtime-ime-probe.mjs";
import { runRuntimeFailureProbe } from "./runtime-failure-probe.mjs";
import { runRuntimeClockProbe, runtimeClockSetup } from "./runtime-clock-probe.mjs";

const url = process.argv[2];
if (!url) {
  console.error("usage: check-web-showcase.mjs <url>");
  process.exit(2);
}

if (typeof WebSocket !== "function") {
  console.error("Node.js with a global WebSocket implementation is required.");
  process.exit(2);
}

if (process.platform === "linux" && !process.env.DISPLAY) {
  console.error(
    "Linux WebGPU checks require an X display for canvas presentation. " +
    "Run with xvfb-run -a node scripts/check-web-showcase.mjs <url>; Chrome remains headless."
  );
  process.exit(2);
}

const timeoutMs = numberFromEnv("OPERAD_WEB_SMOKE_TIMEOUT_MS", 15_000);
const settleMs = numberFromEnv("OPERAD_WEB_SMOKE_SETTLE_MS", 3_000);
const runUat = boolFromEnv("OPERAD_WEB_SHOWCASE_UAT");
// Run just the checkbox/wheel workflow when investigating pointer regressions.
const runCheckbox = boolFromEnv("OPERAD_WEB_CHECKBOX_PROBE");
const runScheduling = boolFromEnv("OPERAD_WEB_SCHEDULING_PROBE");
const runClock = boolFromEnv("OPERAD_WEB_CLOCK_PROBE");
const runEditor = boolFromEnv("OPERAD_WEB_EDITOR_PROBE");
const runIme = boolFromEnv("OPERAD_WEB_IME_PROBE");
const runDeviceLoss = boolFromEnv("OPERAD_WEB_DEVICE_LOSS_PROBE");
const runRenderFailure = boolFromEnv("OPERAD_WEB_RENDER_FAILURE_PROBE");
const viewportWidth = numberFromEnv("OPERAD_WEB_SMOKE_WIDTH", 1440);
const viewportHeight = numberFromEnv("OPERAD_WEB_SMOKE_HEIGHT", 1000);
const showcaseUrl = runUat || runCheckbox ? withQueryParam(url, "operad_uat", "1") : url;
const debuggingPort = await reservePort();
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
    `--remote-debugging-port=${debuggingPort}`,
    "--remote-allow-origins=*",
    "--enable-unsafe-webgpu",
    "--ignore-gpu-blocklist",
    // Keep WebGPU and the compositor on the same software Vulkan backend.
    // Otherwise Chrome can accept input while the WebGPU canvas stays blank.
    // https://github.com/visgl/luma.gl/issues/2874
    ...(process.platform === "linux" ? [
      "--enable-gpu",
      "--enable-features=Vulkan",
      "--use-angle=swiftshader",
      "--use-vulkan=swiftshader",
      "--use-webgpu-adapter=swiftshader",
      "--enable-unsafe-swiftshader",
    ] : []),
    "--disable-dev-shm-usage",
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
let chromeFailure = null;
chrome.on("error", error => { chromeFailure = error; });
chrome.on("exit", (code, signal) => {
  chromeFailure ??= new Error(`Chrome exited (${signal ?? code}). Recent output:\n${stderr.slice(-2000)}`);
});
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

function reservePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      server.close(() => {
        if (address && typeof address === "object") {
          resolve(address.port);
        } else {
          reject(new Error("Could not reserve a Chrome debugging port."));
        }
      });
    });
    server.on("error", reject);
  });
}

async function waitForDevtoolsEndpoint() {
  const endpoint = `http://127.0.0.1:${debuggingPort}/json/version`;
  const deadline = performance.now() + Math.min(timeoutMs, 10_000);
  while (performance.now() < deadline) {
    if (chromeFailure) throw chromeFailure;
    const match = `${stderr}\n${stdout}`.match(/DevTools listening on (ws:\/\/[^\s]+)/);
    if (match) return match[1];

    const controller = new AbortController();
    // Bound headers and body consumption; don't overlap startup requests.
    const timer = setTimeout(() => controller.abort(), Math.min(250, deadline - performance.now()));
    try {
      const response = await fetch(endpoint, { signal: controller.signal });
      if (response.ok) {
        const metadata = await response.json();
        if (typeof metadata.webSocketDebuggerUrl === "string") return metadata.webSocketDebuggerUrl;
      }
    } catch {
      // Retry a starting server or incomplete response until the startup deadline.
    } finally {
      clearTimeout(timer);
      controller.abort();
    }
    await delay(50);
  }
  if (chromeFailure) throw chromeFailure;
  throw new Error(`Chrome did not publish a DevTools endpoint. Recent output:\n${`${stderr}\n${stdout}`.slice(-2000)}`);
}

async function runSmoke(browserWsUrl) {
  const events = [];
  const requestUrls = new Map();
  const connection = await connectCdp(new WebSocket(browserWsUrl), {
    timeoutMs,
    onEvent(message) {
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
    },
  });
  const { send } = connection;

  try {
    if (runDeviceLoss || runRenderFailure) {
      const expected = await runRuntimeFailureProbe(send, evaluate, events, url, timeoutMs,
        runRenderFailure ? "render" : "device");
      connection.assertOpen();
      return events.filter(event => !expected.has(event));
    }
    const { targetId } = await send("Target.createTarget", { url: "about:blank" });
    const { sessionId } = await send("Target.attachToTarget", {
      targetId,
      flatten: true,
    });
    await send("Runtime.enable", {}, sessionId);
    await send("Log.enable", {}, sessionId);
    await send("Network.enable", {}, sessionId);
    await send("Page.enable", {}, sessionId);
    if (runClock) {
      await send("Page.addScriptToEvaluateOnNewDocument", { source: runtimeClockSetup }, sessionId);
    }
    await send("Page.navigate", { url: showcaseUrl }, sessionId);
    if (runClock) {
      await runRuntimeClockProbe(send, sessionId, evaluate, timeoutMs);
    } else if (runIme) {
      await runRuntimeImeProbe(send, sessionId, evaluate, timeoutMs);
    } else if (runEditor) {
      await runRuntimeEditorProbe(send, sessionId, evaluate, timeoutMs);
    } else if (runScheduling) {
      await runSchedulingProbe(send, sessionId);
    } else {
      await waitForShowcaseReady(send, sessionId);
      await assertShowcasePainted(send, sessionId);
    }
    if (runUat) {
      await runShowcaseUat(send, sessionId);
    } else if (runCheckbox) {
      await waitForUatHook(send, sessionId);
      await runCheckboxWheelUat(send, sessionId, await uatSnapshot(send, sessionId));
    }
    await delay(settleMs);
    connection.assertOpen();
    return events;
  } finally {
    connection.close();
  }
}

// This fixture deliberately has no tick action: input, delayed repaints, and
// continuous rendering must each provide their own wakeups.
async function runSchedulingProbe(send, sessionId) {
  await waitForUatHook(send, sessionId);
  const status = async () => {
    const text = await evaluate(
      send, sessionId, 'document.getElementById("probe-status").textContent'
    );
    // The UAT hook is installed before the first animation callback runs.
    if (text === "Loading…") return null;
    const match = text.match(/Frames: (\d+) \| Count: (\d+) \| Continuous: (true|false) \| Delayed: (\w+) \| Width: ([0-9.]+) \| Async: (true|false) \| Task: (\w+)/);
    if (!match) throw new Error(`invalid scheduling probe status: ${text}`);
    return {
      frames: Number(match[1]),
      count: Number(match[2]),
      continuous: match[3] === "true",
      delayed: match[4],
      width: Number(match[5]),
      async: match[6] === "true",
      task: match[7],
    };
  };
  const waitFor = async (predicate, description) => {
    const deadline = Date.now() + timeoutMs;
    while (Date.now() < deadline) {
      const current = await status();
      if (current && predicate(current)) return current;
      await delay(25);
    }
    const display = await evaluate(send, sessionId, `({
      ratio: devicePixelRatio,
      pixels: document.getElementById("operad-canvas").width
    })`);
    throw new Error(
      `scheduling probe timed out: ${description}; ` +
      JSON.stringify({ status: await status(), display })
    );
  };
  const assertIdle = async () => {
    await delay(150);
    const before = await status();
    await delay(300);
    const after = await status();
    if (!before || !after || after.frames !== before.frames) {
      throw new Error(`idle probe kept rendering: ${JSON.stringify({ before, after })}`);
    }
    return after;
  };
  await waitFor(current => current.frames > 0, "first frame");
  await assertIdle();
  let snapshot = await uatSnapshot(send, sessionId);
  await clickNode(send, sessionId, snapshot, "increment");
  await waitFor(current => current.count === 1, "input after idle");
  await assertIdle();
  snapshot = await uatSnapshot(send, sessionId);
  await clickNode(send, sessionId, snapshot, "async");
  await waitFor(current => current.async, "asynchronous service response without unrelated input");
  await assertIdle();
  for (const [finish, expected] of [["__completeTask(42)", "42"], ["__failTask('read failed')", "error"]]) {
    snapshot = await uatSnapshot(send, sessionId);
    await clickNode(send, sessionId, snapshot, "task");
    await waitFor(current => current.task === "pending", "background job started");
    const pending = await assertIdle();
    await evaluate(send, sessionId, `window.${finish}; delete window.__completeTask; delete window.__failTask; true`);
    await waitFor(current => current.task === expected && current.frames > pending.frames,
      "background completion wakes the idle UI without input");
    await assertIdle();
  }
  snapshot = await uatSnapshot(send, sessionId);
  await clickNode(send, sessionId, snapshot, "delayed", 0);
  const pendingRepaint = await waitFor(current => current.delayed === "pending", "delayed repaint scheduled");
  await waitFor(
    current => current.delayed === "complete" && current.frames > pendingRepaint.frames,
    "delayed repaint on a later frame without input"
  );
  await assertIdle();
  snapshot = await uatSnapshot(send, sessionId);
  await clickNode(send, sessionId, snapshot, "continuous");
  const started = await waitFor(current => current.continuous, "continuous rendering enabled");
  await waitFor(current => current.frames >= started.frames + 3, "continuous frames");
  snapshot = await uatSnapshot(send, sessionId);
  await clickNode(send, sessionId, snapshot, "continuous");
  await waitFor(current => !current.continuous, "continuous rendering disabled");
  const beforeResize = await assertIdle();
  await send("Emulation.setDeviceMetricsOverride", {
    width: 1100, height: 800, deviceScaleFactor: 1, mobile: false,
  }, sessionId);
  await waitFor(current => current.frames > beforeResize.frames, "resize after idle");
  await assertIdle();
  await evaluate(send, sessionId, 'document.getElementById("resize-host").click()');
  await waitFor(current => current.width === 600, "embedded canvas resize without a window event");
  await assertIdle();
  // Chrome's CDP override changes devicePixelRatio without delivering resolution
  // change events when the viewport is unchanged (verified with an independent
  // matchMedia listener). Include a viewport change to model a display switch.
  for (const ratio of [2, 0.75]) {
    const beforeScale = await status();
    await send("Emulation.setDeviceMetricsOverride", {
      width: ratio === 2 ? 1101 : 1102,
      height: 800, deviceScaleFactor: ratio, mobile: false,
    }, sessionId);
    await waitFor(current => current.frames > beforeScale.frames, `DPI ${ratio} change after idle`);
    const pixels = await evaluate(send, sessionId, 'document.getElementById("operad-canvas").width');
    if (pixels !== Math.ceil(600 * ratio)) {
      throw new Error(`wrong canvas backing size at DPI ${ratio}: ${pixels}`);
    }
    await assertIdle();
  }
  const final = await assertIdle();
  console.log(`Scheduling probe passed: idle, input, async response, background success/failure, delayed repaint, continuous start/stop, window and embedded resize, DPI (${final.frames} frames)`);
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

async function assertShowcasePainted(send, sessionId) {
  await evaluate(send, sessionId, `(async () => {
    await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    const canvas = document.getElementById("operad-showcase-canvas");
    const device = canvas.getContext("webgpu").getConfiguration().device;
    let timeout;
    try {
      await Promise.race([
        device.queue.onSubmittedWorkDone(),
        new Promise((_, reject) => {
          timeout = setTimeout(() => reject(new Error("WebGPU frame did not complete")), ${timeoutMs});
        }),
      ]);
    } finally {
      clearTimeout(timeout);
    }
  })()`);
  // Capture the presented frame, not the WebGPU drawing buffer between frames.
  const { data } = await send("Page.captureScreenshot", { format: "png" }, sessionId);
  const painted = await evaluate(send, sessionId, `(async () => {
    const image = new Image();
    image.src = ${JSON.stringify(`data:image/png;base64,${data}`)};
    await image.decode();
    const copy = document.createElement("canvas");
    copy.width = image.width;
    copy.height = image.height;
    const context = copy.getContext("2d", { willReadFrequently: true });
    context.drawImage(image, 0, 0);
    const pixels = context.getImageData(0, 0, copy.width, copy.height).data;
    let firstColor;
    for (let offset = 0; offset < pixels.length; offset += 4) {
      if (pixels[offset + 3] === 0) continue;
      const color = pixels[offset] | (pixels[offset + 1] << 8) | (pixels[offset + 2] << 16);
      if (firstColor === undefined) firstColor = color;
      else if (color !== firstColor) return true;
    }
    return false;
  })()`);
  if (!painted) {
    throw new Error("showcase canvas is blank or uniformly colored despite successful startup");
  }
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

  await runCheckboxWheelUat(send, sessionId, snapshot);
  snapshot = await uatSnapshot(send, sessionId);

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

async function runCheckboxWheelUat(send, sessionId, snapshot) {
  if (rootWindows(snapshot).length > 0) {
    await clickNode(send, sessionId, snapshot, "controls.clear_all");
    snapshot = await waitForSnapshotCondition(
      send, sessionId, (next) => rootWindows(next).length === 0,
      "windows to clear before checkbox wheel checks"
    );
  }
  snapshot = await scrollNodeIntoView(send, sessionId, snapshot, "controls.checkbox");

  const checkWheelAndOtherButtons = async (name) => {
    const before = requireNode(snapshot, name).accessibility?.value;
    if (before !== "checked" && before !== "unchecked") {
      throw new Error(`${name} did not expose its checked state: ${before}`);
    }
    const windows = JSON.stringify(rootWindows(snapshot).map((node) => node.name).sort());
    for (const part of ["box", "label"]) {
      for (const delta of [8, -8]) {
        const target = requireNode(snapshot, `${name}.${part}`);
        const point = nodeCenter(target);
        assertPointInsideClip(target, point);
        const scrollBefore = requireNode(snapshot, "controls.widget_list.viewport").scroll;
        await wheelAt(send, sessionId, point, delta);
        // Observe after the runtime has had an opportunity to process and paint
        // the wheel event, so an immediate unchanged snapshot cannot pass.
        await evaluate(send, sessionId,
          "new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve(true))))"
        );
        snapshot = await uatSnapshot(send, sessionId);
        const after = requireNode(snapshot, name).accessibility?.value;
        if (after !== before) {
          throw new Error(`wheel toggled ${name}.${part}: ${before} -> ${after}, delta=${delta}`);
        }
        const afterWindows = JSON.stringify(rootWindows(snapshot).map((node) => node.name).sort());
        if (afterWindows !== windows) {
          throw new Error(`wheel over ${name}.${part} changed open windows: ${windows} -> ${afterWindows}`);
        }
        if (name === "controls.checkbox" && delta > 0 && scrollBefore.offset.y < scrollBefore.maxOffset.y) {
          const scrollAfter = requireNode(snapshot, "controls.widget_list.viewport").scroll;
          if (scrollAfter.offset.y <= scrollBefore.offset.y) {
            throw new Error("wheel over a sidebar checkbox did not scroll its containing list");
          }
        }
      }
      for (const [button, buttons] of [["middle", 4], ["right", 2]]) {
        const target = requireNode(snapshot, `${name}.${part}`);
        const point = nodeCenter(target);
        assertPointInsideClip(target, point);
        for (const type of ["mousePressed", "mouseReleased"]) {
          await send("Input.dispatchMouseEvent", {
            type, ...point, button, buttons: type === "mousePressed" ? buttons : 0, clickCount: 1,
          }, sessionId);
        }
        await evaluate(send, sessionId,
          "new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve(true))))"
        );
        snapshot = await uatSnapshot(send, sessionId);
        const after = requireNode(snapshot, name).accessibility?.value;
        const afterWindows = JSON.stringify(rootWindows(snapshot).map(node => node.name).sort());
        if (after !== before || afterWindows !== windows) {
          throw new Error(`${button} button activated ${name}.${part}: ${before} -> ${after}; windows ${windows} -> ${afterWindows}`);
        }
      }
    }
  };

  await checkWheelAndOtherButtons("controls.checkbox");
  await clickNode(send, sessionId, snapshot, "controls.checkbox");
  snapshot = await waitForSnapshotCondition(
    send, sessionId,
    (next) => rootWindows(next).some((node) => node.name.endsWith(".checkbox")),
    "checkbox window to open on click"
  );
  snapshot = await ensureWindowExpanded(send, sessionId, snapshot, "checkbox", ["checkbox.enabled"]);
  await checkWheelAndOtherButtons("controls.checkbox");
  await checkWheelAndOtherButtons("checkbox.enabled");
  const beforeClick = requireNode(snapshot, "checkbox.enabled").accessibility.value;
  const toggled = beforeClick === "checked" ? "unchecked" : "checked";
  await clickNode(send, sessionId, snapshot, "checkbox.enabled");
  snapshot = await waitForSnapshotCondition(
    send, sessionId,
    (next) => requireNode(next, "checkbox.enabled").accessibility?.value === toggled,
    "checkbox value to toggle on click"
  );
  await checkWheelAndOtherButtons("checkbox.enabled");
  await clickNode(send, sessionId, snapshot, "controls.checkbox");
  await waitForSnapshotCondition(
    send, sessionId, (next) => rootWindows(next).length === 0,
    "checkbox window to close on click"
  );
  console.log("Checkbox pointer probe passed: checked/unchecked sidebar and widget controls, box/label, wheel directions, middle/right buttons, and primary click activation");
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

async function clickNode(send, sessionId, snapshot, name, settleDelayMs = 120) {
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
  await delay(settleDelayMs);
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
  if (!process.pid || process.exitCode !== null || process.signalCode !== null) {
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
