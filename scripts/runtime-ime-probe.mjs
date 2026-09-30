import fs from "node:fs";

// Exercise actual Chrome IME editing, not synthetic composition notifications.
export async function runRuntimeImeProbe(send, sessionId, evaluate, timeoutMs) {
  const inspect = expression => evaluate(send, sessionId, expression);
  const state = () => inspect("window.__IME_STATE__ ?? null");
  const check = (ok, message) => { if (!ok) throw new Error(message); };
  const wait = async (predicate, description) => {
    const deadline = Date.now() + timeoutMs;
    while (Date.now() < deadline) {
      const current = await state();
      if (current && await predicate(current)) return current;
      await new Promise(resolve => setTimeout(resolve, 25));
    }
    throw new Error(`IME: ${description}; state=${JSON.stringify(await state())}; DOM=${JSON.stringify(await inspect("({value: document.activeElement.value, id: document.activeElement.id, focused: window.__OPERAD_UAT__.snapshot().focus.focused})"))}`);
  };
  const command = async name => {
    const commands = (await state()).commands;
    await inspect(`window.__IME_COMMAND__(${JSON.stringify(name)})`);
    return wait(current => current.commands > commands, name);
  };
  const focus = async name => {
    const frames = (await state()).frames;
    const snapshot = await inspect("window.__OPERAD_UAT__.snapshot()");
    const node = snapshot.nodes.find(node => node.name === name);
    for (const type of ["mousePressed", "mouseReleased"]) await send("Input.dispatchMouseEvent", {
      type, x: node.rect.x + 12, y: node.rect.y + 12, button: "left", buttons: type === "mousePressed" ? 1 : 0, clickCount: 1,
    }, sessionId);
    await wait(async current => current.frames > frames && !!current.input && await inspect(
      `document.activeElement.id === 'operad-canvas-ime' && window.__OPERAD_UAT__.snapshot().focus.focused === ${JSON.stringify(name)} && window.__OPERAD_UAT__.snapshot().focus.pressed === null`
    ), `focus ${name} and its native editable element`);
  };
  const compose = (text, start = text.length, end = start) => send("Input.imeSetComposition", {
    text, selectionStart: start, selectionEnd: end,
  }, sessionId);
  const commit = text => send("Input.insertText", { text }, sessionId);
  const key = async (key, modifiers = 0) => {
    const code = key.length === 1 ? `Key${key.toUpperCase()}` : key;
    const windowsVirtualKeyCode = key.length === 1 ? key.toUpperCase().charCodeAt(0) : {Backspace: 8, Enter: 13, Escape: 27, Home: 36, End: 35, ArrowLeft: 37, ArrowRight: 39, Delete: 46}[key];
    for (const type of ["keyDown", "keyUp"]) await send("Input.dispatchKeyEvent", { type, key, code, modifiers, windowsVirtualKeyCode }, sessionId);
  };
  const reset = async () => {
    await command("reset");
    await focus("first");
    const selected = await command("select");
    check(selected.fields[0].anchor === 1 && selected.fields[0].caret === 8,
      "Reset did not establish the application's replacement selection");
    check(await inspect("document.activeElement.selectionStart === 1 && document.activeElement.selectionEnd === 4"),
      "Reset did not publish the replacement selection to the native input");
  };

  await wait(() => true, "first frame");
  for (const [direction, edge] of [["ArrowLeft", 1], ["ArrowRight", 8]]) {
    await reset();
    await key(direction);
    const navigation = await wait(s => s.fields[0].caret === edge, `${direction} collapses the selection`);
    check(navigation.fields[0].text === "a😀旧z" && !navigation.fields[0].undo,
      "Selection navigation changed text/history");
  }
  for (const cluster of ["e\u0301", "👩‍🚀", "🇺🇸", "👍🏽", "क्षि", "각"]) {
    await reset();
    await key("a", 2);
    await wait(s => s.fields[0].anchor === 0 && s.fields[0].caret === 9, "select all for grapheme editing");
    const text = `A${cluster}Z`;
    await commit(text);
    await wait(s => s.fields[0].text === text && s.fields[0].draft === null, "grapheme text insertion");
    await key("Home");
    await wait(s => s.fields[0].caret === 0, "grapheme navigation start");
    await key("ArrowRight");
    await wait(s => s.fields[0].caret === 1, "before grapheme");
    await key("ArrowRight", 8);
    const edge = 1 + Buffer.byteLength(cluster);
    const selected = await wait(s => s.fields[0].caret === edge, `select whole grapheme ${cluster}`);
    check(selected.fields[0].anchor === 1 && selected.fields[0].text === text,
      "Shift-arrow split or edited a grapheme");
    check(await inspect(`document.activeElement.selectionStart === 1 && document.activeElement.selectionEnd === ${1 + cluster.length}`),
      "Grapheme selection was not published as the equivalent UTF-16 range");
    await key("ArrowLeft");
    await wait(s => s.fields[0].caret === 1 && s.fields[0].anchor === null, "collapse grapheme selection");
    await key("Delete");
    await wait(s => s.fields[0].text === "AZ", "delete whole grapheme");
    await key("z", 2);
    await wait(s => s.fields[0].text === text, "undo grapheme deletion");
  }
  await reset();
  await compose("候😀補", 1, 3);
  let current = await wait(s => s.fields[0].draft === "候😀補", "Unicode preedit");
  check(current.fields[0].text === "a😀旧z" && current.fields[0].display === "a候😀補z", "Preedit changed committed text or replaced the wrong range");
  check(!current.fields[0].undo, "Preedit polluted undo history");
  if (process.env.OPERAD_IME_SCREENSHOT) {
    await new Promise(resolve => setTimeout(resolve, 250));
    await send("Page.bringToFront", {}, sessionId);
    await send("Emulation.setDeviceMetricsOverride", {width: 800, height: 600, deviceScaleFactor: 1, mobile: false}, sessionId);
    await new Promise(resolve => setTimeout(resolve, 250));
    const capture = await send("Page.captureScreenshot", {format: "png", fromSurface: false}, sessionId);
    fs.writeFileSync(process.env.OPERAD_IME_SCREENSHOT, Buffer.from(capture.data, "base64"));
  }
  const firstInput = current.input;
  await command("reorder");
  check((await state()).input === firstInput, "Reordering replaced the active input session");
  for (const deviceScaleFactor of [2, 1.25, 1]) {
    const frames = (await state()).frames;
    await send("Emulation.setDeviceMetricsOverride", {width: 1000 + Math.round(deviceScaleFactor * 10), height: 700, deviceScaleFactor, mobile: false}, sessionId);
    current = await wait(s => s.frames > frames, "DPI/viewport update");
    check(current.input === firstInput && current.fields[0].draft === "候😀補", "DPI/viewport change canceled composition");
    const origin = await inspect("(() => {const r=document.activeElement.getBoundingClientRect();return [r.x,r.y]})()");
    check(Math.abs(origin[0] - current.cursor[0]) < 1 && Math.abs(origin[1] - current.cursor[1]) < 1, "DPI change misplaced candidate surface");
  }
  await compose("日本", 1, 2);
  await wait(s => s.fields[0].draft === "日本", "updated candidate");
  await commit("日本");
  current = await wait(s => s.fields[0].text === "a日本z" && s.fields[0].draft === null, "commit");
  check(current.commits === 1, "Composition committed more than once");
  await key("z", 2);
  await wait(s => s.fields[0].text === "a😀旧z" && !s.fields[0].undo, "single undo restores replacement");
  await key("z", 10);
  await wait(s => s.fields[0].text === "a日本z", "redo composition");

  await reset();
  await compose("discard");
  const beforeCancel = await wait(s => s.fields[0].draft === "discard", "cancel setup");
  await compose("");
  current = await wait(s => s.fields[0].draft === null, "native cancellation");
  check(current.fields[0].text === "a😀旧z" && current.commits === 0 && !current.fields[0].undo, "Cancel changed text/history");
  check(current.input && current.input !== beforeCancel.input, "Native cancellation retained an obsolete input session");

  for (const operation of ["select", "reset"]) {
    await reset();
    await compose("discard");
    const beforeReset = await wait(s => s.fields[0].draft === "discard", `${operation} during composition`);
    await command(operation);
    current = await wait(s => s.fields[0].draft === null && s.input && s.input !== beforeReset.input,
      `${operation} cancels the native session`);
    check(current.fields[0].text === "a😀旧z" && current.commits === 0 && !current.fields[0].undo,
      `${operation} changed committed text/history`);
    await compose("新");
    await wait(s => s.fields[0].draft === "新", `composition after ${operation}`);
    await commit("新");
    current = await wait(s => s.fields[0].text === "a新z" && s.fields[0].draft === null,
      `commit after ${operation}`);
    check(current.commits === 1, `${operation} left a duplicate or obsolete commit`);
    await key("z", 2);
    await wait(s => s.fields[0].text === "a😀旧z" && !s.fields[0].undo,
      `one undo restores the replacement after ${operation}`);
  }

  for (const operation of ["disable", "remove"]) {
    await reset();
    await compose("discard");
    await wait(s => s.fields[0].draft === "discard", operation + " setup");
    await command(operation);
    current = await wait(s => s.fields[0].draft === null && !s.input, operation + " cleanup");
    check(current.fields[0].text === "a😀旧z" && current.canceled === 1,
      `${operation} failed semantic cleanup: ${JSON.stringify(current)}`);
  }

  await reset();
  await compose("old draft");
  await wait(s => !!s.fields[0].draft, "focus change setup");
  await focus("second");
  await wait(s => s.fields[0].draft === null && s.input !== firstInput, "focus change cancels original field");
  const firstAfterBlur = (await state()).fields[0].text;
  check(["a😀旧z", "aold draftz"].includes(firstAfterBlur), "Blur altered text outside the original composition range");
  await compose("新");
  await wait(s => s.fields[1].draft === "新", "second field preedit");
  await commit("新");
  current = await wait(s => s.fields[1].text.includes("新") && s.fields[1].draft === null, "second field commit");
  check(current.fields[0].text === firstAfterBlur, "Composition leaked across fields");

  await reset();
  await compose("held key");
  await wait(s => s.fields[0].draft === "held key", "composition owns navigation key");
  await send("Input.dispatchKeyEvent", {
    type: "keyDown", key: "ArrowLeft", code: "ArrowLeft", windowsVirtualKeyCode: 37,
  }, sessionId);
  await inspect(`(() => {
    const input = document.createElement('input');
    input.id = 'ime-external-focus';
    input.style.cssText = 'position:fixed;left:0;top:0';
    document.body.appendChild(input);
    input.focus();
    window.addEventListener('keyup', event => {
      window.__IME_EXTERNAL_RELEASE__ = {
        target: event.target.id, prevented: event.defaultPrevented,
      };
    }, {once: true});
  })()`);
  await send("Input.dispatchKeyEvent", {
    type: "keyUp", key: "ArrowLeft", code: "ArrowLeft", windowsVirtualKeyCode: 37,
  }, sessionId);
  const externalRelease = await inspect("window.__IME_EXTERNAL_RELEASE__");
  check(externalRelease?.target === "ime-external-focus" && !externalRelease.prevented,
    "Operad intercepted the external input's key release");
  await wait(s => s.fields[0].draft === null, "composition finishes before resetting the model");
  await inspect("document.getElementById('ime-external-focus').remove(); delete window.__IME_EXTERNAL_RELEASE__");
  await reset();
  check((await state()).fields[0].caret === 8, "navigation selection setup failed");
  await key("ArrowLeft", 8);
  current = await wait(s => s.fields[0].caret === 5, "released IME key works after focus returns");
  check(current.fields[0].text === "a😀旧z", "navigation changed committed text");

  for (const startingPassword of [false, true]) {
    await reset();
    if (startingPassword) await command("password");
    await compose("discard");
    const beforeModeChange = await wait(s => s.fields[0].draft === "discard", "composition before input mode change");
    await command("password");
    current = await wait(s => s.fields[0].draft === null, "input mode change cancels composition");
    check(current.fields[0].text === "a😀旧z" && current.canceled === beforeModeChange.canceled + 1
      && current.commits === 0 && !current.fields[0].undo,
      `Input mode change lost cancellation or altered committed text/history: ${JSON.stringify(current)}`);
    await compose("新");
    await wait(s => s.fields[0].draft === "新", "replacement element starts composition");
    await commit("新");
    current = await wait(s => s.fields[0].text === "a新z" && !s.fields[0].draft, "replacement element commits composition");
    check(current.commits === 1, "Replacement composition committed more than once");
    await key("z", 2);
    await wait(s => s.fields[0].text === "a😀旧z" && !s.fields[0].undo, "replacement composition has one undo step");
  }

  await reset();
  await key("q");
  current = await wait(s => s.fields[0].text === "aqz", "ordinary key replaces selection once");
  check(current.commits === 1, "DOM and key handlers both inserted text");
  await command("password");
  check(await inspect("document.activeElement.type === 'password'"), "Sensitive input was exposed as a textarea");
  await compose("秘😀");
  await wait(s => s.fields[0].draft === "秘😀", "password composition");
  await commit("秘😀");
  await wait(s => s.fields[0].text === "aq秘😀z", "password commit");
  await command("password");
  await compose("一\n二");
  await wait(s => s.fields[0].draft === "一\n二", "multiline composition");
  await commit("一\n二");
  await wait(s => s.fields[0].text.includes("一\n二"), "multiline commit");

  const positioned = await state();
  const rect = await inspect("(() => {const r=document.activeElement.getBoundingClientRect();return [r.x,r.y,r.height]})()");
  check(Math.abs(rect[0] - positioned.cursor[0]) < 1 && Math.abs(rect[1] - positioned.cursor[1]) < 1 && rect[2] > 0, "Native candidate surface does not follow measured caret");
  await new Promise(resolve => setTimeout(resolve, 150));
  const idle = (await state()).frames;
  await new Promise(resolve => setTimeout(resolve, 200));
  check((await state()).frames === idle, "Composition bridge keeps redrawing while idle");
  console.log("IME probe passed: selection navigation, Unicode, replacement, reorder, commit/undo, cancellation and model-reset session renewal, removal/disable, focus, key release outside the canvas, input mode changes during composition, ordinary keys, password, multiline, candidate geometry, idle.");
}
