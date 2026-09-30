// Uses the existing isolated Chrome/CDP smoke runner.
export async function runRuntimeEditorProbe(send, sessionId, evaluate, timeoutMs) {
  const inspect = (expression) => evaluate(send, sessionId, expression);
  const state = () => inspect("window.__OPERAD_EDITOR__ ?? null");
  const snapshot = () => inspect("window.__OPERAD_UAT__.snapshot()");
  const check = (condition, message) => { if (!condition) throw new Error(message); };
  const waitFor = async (predicate, description) => {
    const deadline = Date.now() + timeoutMs;
    while (Date.now() < deadline) {
      const current = await state();
      if (current && predicate(current)) return current;
      await new Promise((resolve) => setTimeout(resolve, 25));
    }
    throw new Error(`Editor probe timed out waiting for ${description}: ${JSON.stringify(await state())}; events=${JSON.stringify(await inspect("window.__EDITOR_EVENTS__"))}`);
  };
  const mouse = (type, x, y, buttons = 0, button = type === "mouseMoved" && buttons === 0 ? "none" : "left") => send("Input.dispatchMouseEvent", {
    type, x, y, buttons, button, clickCount: 1,
  }, sessionId);
  const key = async (value) => {
    const params = {key: value, code: `Key${value.toUpperCase()}`, windowsVirtualKeyCode: value.toUpperCase().charCodeAt(0)};
    await send("Input.dispatchKeyEvent", {type: "keyDown", ...params}, sessionId);
    await send("Input.dispatchKeyEvent", {type: "keyUp", ...params}, sessionId);
  };
  const node = (current, name) => {
    const found = current.nodes.find((candidate) => candidate.name === name);
    check(found, `Missing editor node: ${name}`);
    return found;
  };
  const beginDrag = async () => {
    const before = await state();
    const timeline = node(await snapshot(), "timeline");
    await mouse("mousePressed", timeline.rect.x + 40.375, timeline.rect.y + 30.625, 1);
    const pressed = await waitFor((current) => current.downs === before.downs + 1 && current.dragging, "captured editor press");
    check(Math.abs(pressed.localX - 40.375) < 0.001 && Math.abs(pressed.localY - 30.625) < 0.001,
      `Browser pointer coordinates lost fractional precision: ${pressed.localX}, ${pressed.localY}`);
    return pressed;
  };

  await waitFor(() => true, "first observed frame");
  await inspect(`window.__EDITOR_EVENTS__ = [];
    for (const kind of ['pointerdown', 'pointermove', 'pointerup', 'pointercancel', 'gotpointercapture', 'lostpointercapture', 'blur']) {
      window.addEventListener(kind, event => window.__EDITOR_EVENTS__.push({
        type: event.type, button: event.button, buttons: event.buttons, id: event.pointerId,
        target: event.target.id, x: event.clientX, y: event.clientY
      }), {capture: true});
    }
    document.getElementById('operad-canvas').addEventListener('pointerdown', event => {
    window.__EDITOR_POINTER_ID__ = event.pointerId;
  }, {capture: true})`);
  const baseline = await snapshot();
  const initialCanvas = node(baseline, "timeline").index;
  const status = node(baseline, "status");
  check(status.visible && status.rect.width > 0, "Observation did not expose laid out content");
  const label = node(baseline, "arrangement.name");
  check(label.accessibility?.label?.includes("many more details"), "Fitted label lost accessible text");
  const builds = await inspect(`(() => {
    const before = window.__OPERAD_EDITOR_BUILD_COUNT__();
    for (let i = 0; i < 20; i++) window.__OPERAD_UAT__.snapshot();
    return [before, window.__OPERAD_EDITOR_BUILD_COUNT__()];
  })()`);
  check(builds[0] === builds[1], "Inspecting a rendered frame rebuilt the application view");

  const disabled = node(baseline, "disabled.control");
  await mouse("mousePressed", disabled.rect.x + 10, disabled.rect.y + 10, 1);
  await mouse("mouseReleased", disabled.rect.x + 10, disabled.rect.y + 10);
  await new Promise((resolve) => setTimeout(resolve, 100));
  check((await state()).downs === 0 && (await state()).blockedActivations === 0,
    "Disabled overlay passed a click into the underlying editor");

  const pressed = await beginDrag();
  check(pressed.sectionsReused === 1 && pressed.sectionsRebuilt === 0,
    "Dragging rebuilt the unchanged toolbar section");
  check(node(await snapshot(), "timeline").index !== initialCanvas,
    "Editor fixture did not reorder its document during capture");
  check(await inspect("document.getElementById('operad-canvas').hasPointerCapture(window.__EDITOR_POINTER_ID__)"),
    "Browser did not capture the real DOM pointer ID");
  await mouse("mouseMoved", 820, 470, 1);
  await waitFor((current) => current.moves > pressed.moves, "drag outside the DOM canvas");
  await mouse("mouseReleased", 860, 490);
  await waitFor((current) => current.releases === 1 && !current.dragging, "release outside the DOM canvas");

  for (const reason of ["lostcapture", "blur", "disabled", "removed"]) {
    const before = await beginDrag();
    if (reason === "lostcapture") {
      // Activate the pending capture before revoking it. Revoking a pending
      // override before another pointer event need not emit lostpointercapture.
      const timeline = node(await snapshot(), "timeline");
      await mouse("mouseMoved", timeline.rect.x + 40.375, timeline.rect.y + 30.625, 1);
      await waitFor((current) => current.moves > before.moves, "active DOM capture before revocation");
      await inspect("document.getElementById('operad-canvas').releasePointerCapture(window.__EDITOR_POINTER_ID__)");
      // Capture changes become observable when the browser processes the next
      // pointer event; releasing the pending override alone need not dispatch.
      await mouse("mouseMoved", 820, 470, 1);
    } else if (reason === "blur") {
      await inspect("window.dispatchEvent(new Event('blur'))");
    } else {
      await key(reason === "disabled" ? "d" : "r");
    }
    await waitFor((current) => current.cancellations === before.cancellations + 1 && !current.dragging,
      `one cancellation after ${reason}`);
    await mouse("mouseReleased", 80, 130);
    await new Promise((resolve) => setTimeout(resolve, 75));
    const after = await state();
    check(after.cancellations === before.cancellations + 1 && after.releases === before.releases,
      `${reason} produced a duplicate cancellation or unrelated commit`);
    if (reason === "disabled" || reason === "removed") {
      await key(reason === "disabled" ? "d" : "r");
      await waitFor((current) => !current.disabled && !current.removed, "editor restored");
    }
  }
  // Pointer Events reports intermediate chord changes as pointermove. Releasing
  // the button that owns the edit must commit it before the final button lifts.
  const chord = await beginDrag();
  await mouse("mousePressed", 80, 130, 3, "right");
  await mouse("mouseReleased", 820, 470, 2, "left");
  await waitFor((current) => current.releases === chord.releases + 1 && !current.dragging,
    "primary release while secondary remains pressed");
  await mouse("mouseReleased", 820, 470, 0, "right");
  const afterChord = await beginDrag();
  await mouse("mouseReleased", 820, 470);
  await waitFor((current) => current.releases === afterChord.releases + 1 && !current.dragging,
    "fresh drag after a button chord");

  const lock = await state();
  await key("l");
  await waitFor((current) => current.lockResponses > lock.lockResponses,
    "asynchronous pointer lock response");
  check(await inspect("document.pointerLockElement === document.getElementById('operad-canvas')"),
    "Browser did not apply the platform pointer-lock request");
  const motion = await state();
  await mouse("mouseMoved", 180, 160);
  await waitFor((current) => current.rawMotion > motion.rawMotion, "normalized raw mouse motion");
  await key("u");
  await waitFor((current) => current.lockResponses > motion.lockResponses,
    "pointer unlock response");
  check(await inspect("document.pointerLockElement === null"), "Browser did not release pointer lock");

  const idle = await state();
  await key("i");
  await waitFor((current) => current.idle && current.builds > idle.builds + 5,
    "idle hook rebuilding the view");
  await key("i");
  await waitFor((current) => !current.idle, "idle redraw stopped");
  await new Promise((resolve) => setTimeout(resolve, 100));
  const stoppedBuilds = await inspect("window.__OPERAD_EDITOR_BUILD_COUNT__()");
  await new Promise((resolve) => setTimeout(resolve, 150));
  check(await inspect("window.__OPERAD_EDITOR_BUILD_COUNT__()") === stoppedBuilds,
    "Disabling the idle hook continued rebuilding the view");

  console.log("Editor browser probe passed: shared hooks, rebuilds, outside capture, cancellation, button chords, pointer lock/raw motion, idle redraw, disabled overlay and observation reuse");
}
