// Exercise terminal frame errors and real WebGPU device loss. The scheduling
// fixture exposes frame observations and a controllable background job.
export async function runRuntimeFailureProbe(send, evaluate, events, url, timeoutMs, failure) {
  const renderFailure = failure === "render";
  const errorPrefix = renderFailure ? 'render failed: canvas "invalid-canvas"'
    : "WebGPU device lost (Destroyed):";
  const expectedErrors = new Set();
  const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
  const check = (ok, message) => { if (!ok) throw new Error(`runtime ${failure} failure: ${message}`); };
  const recordFailure = (sessionId, eventStart) => {
    const losses = events.slice(eventStart).filter(event => event.sessionId === sessionId
      && event.method === "Runtime.consoleAPICalled" && event.params.type === "error"
      && event.params.args.length === 1
      && event.params.args[0].value?.startsWith(errorPrefix));
    check(losses.length === 1, `expected one terminal report, got ${losses.length}`);
    expectedErrors.add(losses[0]);
  };
  const modes = renderFailure ? ["idle", "continuous", "delayed", "composition"]
    : ["startup", "startup-without-status", "idle", "continuous", "delayed"];
  for (const mode of modes) {
    const startup = mode.startsWith("startup");
    const showStatus = mode !== "delayed" && mode !== "startup-without-status";
    const { targetId } = await send("Target.createTarget", { url: "about:blank" });
    const { sessionId } = await send("Target.attachToTarget", { targetId, flatten: true });
    const inspect = expression => evaluate(send, sessionId, expression);
    const wait = async (expression, description) => {
      const deadline = Date.now() + timeoutMs;
      while (Date.now() < deadline) {
        if (await inspect(expression)) return;
        await delay(25);
      }
      throw new Error(`runtime ${failure} failure (${mode}): timed out waiting for ${description}`);
    };
    try {
      for (const domain of ["Runtime", "Log", "Network", "Page"]) {
        await send(`${domain}.enable`, {}, sessionId);
      }
      await send("Page.addScriptToEvaluateOnNewDocument", { source: `
        window.__OPERAD_DEVICE_LOSS_STATUS__ = ${showStatus};
        if (${startup}) {
          const requestDevice = GPUAdapter.prototype.requestDevice;
          GPUAdapter.prototype.requestDevice = async function(descriptor) {
            const device = await requestDevice.call(this, descriptor);
            device.destroy();
            await device.lost;
            window.__startupDeviceLost = true;
            return device;
          };
        }
        window.__frameCallbacks = 0;
        window.__pendingFrames = new Set();
        const requestFrame = window.requestAnimationFrame.bind(window);
        const cancelFrame = window.cancelAnimationFrame.bind(window);
        window.requestAnimationFrame = callback => {
          const id = requestFrame(time => {
            window.__pendingFrames.delete(id);
            window.__frameCallbacks++;
            callback(time);
          });
          window.__pendingFrames.add(id);
          return id;
        };
        window.cancelAnimationFrame = id => {
          window.__pendingFrames.delete(id);
          cancelFrame(id);
        };
        window.__pendingTimers = new Set();
        const setTimer = window.setTimeout.bind(window);
        const clearTimer = window.clearTimeout.bind(window);
        window.setTimeout = (callback, delay, ...args) => {
          const id = setTimer(() => {
            window.__pendingTimers.delete(id);
            callback(...args);
          }, delay);
          window.__pendingTimers.add(id);
          return id;
        };
        window.clearTimeout = id => {
          window.__pendingTimers.delete(id);
          clearTimer(id);
        };
      ` }, sessionId);
      const navigationStart = events.length;
      await send("Page.navigate", { url }, sessionId);
      if (startup) {
        await wait("window.__startupDeviceLost === true", "device loss during acquisition");
        await wait("!!window.__OPERAD_UAT__ && !document.getElementById('operad-canvas-ime')",
          "startup input cleanup");
        if (showStatus) {
          await wait("document.getElementById('device-loss-status')?.textContent.includes('graphics device was lost')",
            "startup error report");
        }
        const propagation = await inspect(`(() => {
          const canvas = document.getElementById('operad-canvas');
          canvas.focus();
          const defaults = [
            new PointerEvent('pointerdown', { bubbles: true, cancelable: true }),
            new WheelEvent('wheel', { bubbles: true, cancelable: true, deltaY: 20 }),
            new KeyboardEvent('keydown', { bubbles: true, cancelable: true, key: 'Tab', code: 'Tab' }),
          ].map(event => canvas.dispatchEvent(event));
          const close = new Event('beforeunload', { cancelable: true });
          window.dispatchEvent(close);
          window.dispatchEvent(new Event('resize'));
          return { defaults, closePrevented: close.defaultPrevented };
        })()`);
        check(propagation.defaults.every(Boolean), "failed startup still prevented browser input");
        check(propagation.closePrevented, "failed startup lost close protection");
        await delay(150);
        const after = await inspect(`({ frames: window.__frameCallbacks,
          pending: window.__pendingFrames.size + window.__pendingTimers.size,
          status: document.getElementById('probe-status').textContent })`);
        check(after.frames === 0 && after.pending === 0 && !after.status.startsWith('Frames:'),
          `startup rendered or retained work after loss: ${JSON.stringify(after)}`);
        if (!showStatus) {
          check(await inspect("!document.getElementById('operad-status') && !document.getElementById('device-loss-status')"),
            "startup without_status created an error element");
        }
        recordFailure(sessionId, navigationStart);
        console.log(`Device loss probe passed: ${mode}, no first frame, input cleanup, close protection`);
        continue;
      }
      await wait(`document.getElementById('probe-status')?.textContent.startsWith('Frames:')
        && !!window.__OPERAD_UAT__`, "first frame");
      // Observe actual DOM capture, including browsers assigning a different id.
      await inspect(`document.getElementById('operad-canvas').addEventListener('pointerdown',
        event => { window.__captureId = event.pointerId; })`);
      const point = async name => inspect(`(() => {
        const node = window.__OPERAD_UAT__.snapshot().nodes.find(n => n.name === ${JSON.stringify(name)});
        return { x: node.rect.x + 12, y: node.rect.y + 12 };
      })()`);
      const mouse = (type, position) => send("Input.dispatchMouseEvent", {
        type, ...position, button: "left", buttons: type === "mousePressed" ? 1 : 0, clickCount: 1,
      }, sessionId);
      const click = async name => {
        const position = await point(name);
        await mouse("mousePressed", position);
        await mouse("mouseReleased", position);
      };
      await click("task");
      await wait("typeof window.__completeTask === 'function'", "pending background job");
      await click("text");
      await wait("document.activeElement.id === 'operad-canvas-ime'", "native text input focus");
      // Keep a real composition alive when the idle device is lost.
      if (mode === "idle") {
        await mouse("mousePressed", await point("text"));
        await wait("document.activeElement.id === 'operad-canvas-ime'", "captured text input focus");
        await send("Input.imeSetComposition", { text: "draft", selectionStart: 5, selectionEnd: 5 }, sessionId);
      } else if (mode !== "composition") {
        await click(mode);
        await wait(`document.getElementById('probe-status').textContent.includes(${JSON.stringify(
          mode === "continuous" ? "Continuous: true" : "Delayed: pending"
        )})`, `${mode} work scheduled`);
      }
      if (mode === "continuous") {
        // Cover an existing host-owned status element as well as a removed one.
        await inspect(`(() => {
          const status = document.createElement('aside');
          status.id = 'device-loss-status'; status.dataset.hostOwned = 'true';
          status.style.cssText = 'position:fixed;top:16px;left:16px;background:#201c1c;color:white;padding:16px';
          document.body.appendChild(status);
        })()`);
      }
      if (mode === "idle") {
        await delay(150);
        const before = await inspect("window.__frameCallbacks");
        await delay(150);
        check(await inspect("window.__frameCallbacks") === before, "fixture was not idle before loss");
      }
      if (mode === "continuous") {
        await click("lock");
        await wait("document.pointerLockElement?.id === 'operad-canvas'", "pointer lock");
      }
      if (mode === "idle") {
        check(await inspect("document.getElementById('operad-canvas').hasPointerCapture(window.__captureId)"),
          "test did not acquire pointer capture");
      }
      const eventStart = events.length;
      const pendingWork = await inspect(`(async () => {
        const device = document.getElementById('operad-canvas').getContext('webgpu').getConfiguration().device;
        const bounded = async promise => {
          let timeout;
          try {
            return await Promise.race([promise, new Promise((_, reject) => {
              timeout = setTimeout(() => reject(new Error('WebGPU operation did not complete')), ${timeoutMs});
            })]);
          } finally { clearTimeout(timeout); }
        };
        await bounded(device.queue.onSubmittedWorkDone());
        const pending = { frames: window.__pendingFrames.size, timers: window.__pendingTimers.size };
        if (${renderFailure}) {
          window.__failureFrameCallbacks = window.__frameCallbacks;
          window.__OPERAD_RENDER_FAILURE__ = true;
          if (${mode === "composition"}) {
            const input = document.activeElement;
            input.dispatchEvent(new KeyboardEvent('keydown', {
              key: 'q', code: 'KeyQ', bubbles: true, cancelable: true,
            }));
            input.dispatchEvent(new KeyboardEvent('keydown', {
              key: 'Process', code: 'KeyA', keyCode: 229, bubbles: true, cancelable: true,
            }));
            window.__synchronousFailureCleanedUp = !document.getElementById('operad-canvas-ime');
          } else {
            window.dispatchEvent(new Event('resize'));
          }
        } else {
          device.destroy();
          await bounded(device.lost);
        }
        return pending;
      })()`);
      check(mode !== "delayed" || pendingWork.timers > 0, "no delayed callback was pending at failure");
      check(mode !== "continuous" || pendingWork.frames > 0, "no animation callback was pending at failure");
      if (renderFailure) {
        // Fail quickly with the actual retry count on the old implementation.
        if (mode === "composition") {
          check(await inspect("window.__synchronousFailureCleanedUp"),
            "synchronous text-input rendering did not stop on failure");
        } else {
          await wait("window.__frameCallbacks > window.__failureFrameCallbacks", "render failure");
        }
        await delay(150);
        const reports = events.slice(eventStart).filter(event => event.sessionId === sessionId
          && event.method === "Runtime.consoleAPICalled" && event.params.type === "error");
        check(reports.length === 1, `render failure kept retrying (${reports.length} error reports)`);
      }
      await wait("!document.getElementById('operad-canvas-ime')", "text input cleanup");
      await wait("!document.pointerLockElement", "pointer lock cleanup");
      if (mode !== "delayed") {
        await wait(`document.getElementById('device-loss-status')?.textContent.includes(${JSON.stringify(
          renderFailure ? "Rendering stopped" : "graphics device was lost"
        )})`, "visible terminal error");
        check(await inspect(`(() => {
          const status = document.getElementById('device-loss-status');
          const rect = status.getBoundingClientRect();
          return rect.width > 0 && rect.height > 0 && rect.bottom > 0 && rect.top < innerHeight
            && getComputedStyle(status).visibility !== 'hidden';
        })()`), "terminal error was not visible");
      }
      const stopped = await inspect(`({ frames: window.__frameCallbacks,
        status: document.getElementById('probe-status').textContent })`);
      const released = await inspect(`(() => {
        const canvas = document.getElementById('operad-canvas');
        return !document.pointerLockElement && (!window.__captureId || !canvas.hasPointerCapture(window.__captureId));
      })()`);
      check(released, "browser capture survived shutdown");
      if (mode === "idle") await mouse("mouseReleased", await point("text"));
      await inspect("window.__completeTask(42); true");
      await wait("window.__taskReceiverClosed === true", "task receiver shutdown");
      await click("increment");
      const propagation = await inspect(`(() => {
        const canvas = document.getElementById('operad-canvas');
        const pointer = new PointerEvent('pointerdown', { bubbles: true, cancelable: true });
        const wheel = new WheelEvent('wheel', { bubbles: true, cancelable: true, deltaY: 20 });
        const key = new KeyboardEvent('keydown', { bubbles: true, cancelable: true, key: 'Tab', code: 'Tab' });
        canvas.focus();
        const defaults = [pointer, wheel, key].map(event => canvas.dispatchEvent(event));
        const close = new Event('beforeunload', { cancelable: true });
        window.dispatchEvent(close);
        document.getElementById('canvas-host').style.width = '600px';
        window.dispatchEvent(new Event('resize'));
        return { defaults, closePrevented: close.defaultPrevented };
      })()`);
      check(propagation.defaults.every(Boolean), "stopped app still prevented browser input");
      check(propagation.closePrevented, "unsaved-change close handler was disabled");
      // Cross the delayed repaint deadline and allow job/resize/input wakeups.
      await delay(650);
      const after = await inspect(`({ frames: window.__frameCallbacks,
        pending: window.__pendingFrames.size + window.__pendingTimers.size,
        status: document.getElementById('probe-status').textContent })`);
      check(after.frames === stopped.frames && after.pending === 0 && after.status === stopped.status,
        `failed runtime continued scheduling or updating (${mode}): ${JSON.stringify({ stopped, after })}`);
      if (mode === "delayed") {
        check(await inspect("!document.getElementById('operad-status') && !document.getElementById('device-loss-status')"),
          "without_status created an error element");
      } else if (mode === "continuous") {
        check(await inspect("document.getElementById('device-loss-status').dataset.hostOwned === 'true'"),
          "replaced the host's existing status element");
      }
      if (renderFailure) {
        // A later device-loss notification must not report or clean up twice.
        await inspect(`(async () => {
          const device = document.getElementById('operad-canvas').getContext('webgpu').getConfiguration().device;
          device.destroy(); await device.lost; return true;
        })()`);
        await delay(50);
      }
      recordFailure(sessionId, eventStart);
      console.log(`Runtime ${failure} failure probe passed: ${mode}, input cleanup, background completion, resize, close protection`);
    } finally {
      await send("Target.closeTarget", { targetId });
    }
  }
  return expectedErrors;
}
