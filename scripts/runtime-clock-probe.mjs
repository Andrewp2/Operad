// A page can exist long before an embedded runtime starts. Keep its monotonic
// clock rate unchanged, but use a large origin offset to expose mixed clocks
// without making the test wait for an old page in real time.
export const runtimeClockSetup = `
  window.__OPERAD_CLOCK_PROBE__ = true;
  const originalNow = Performance.prototype.now;
  Performance.prototype.now = function() { return originalNow.call(this) + 60000; };
`;

export async function runRuntimeClockProbe(send, sessionId, evaluate, timeoutMs) {
  const inspect = expression => evaluate(send, sessionId, expression);
  const check = (ok, message) => { if (!ok) throw new Error(`runtime clock: ${message}`); };
  const wait = async (predicate, description, timeout = timeoutMs) => {
    const deadline = Date.now() + timeout;
    let state;
    while (Date.now() < deadline) {
      state = await inspect("window.__CLOCK_STATE__ ?? null");
      if (state && predicate(state)) return state;
      await new Promise(resolve => setTimeout(resolve, 25));
    }
    throw new Error(`runtime clock: ${description}; state=${JSON.stringify(state)}`);
  };
  await wait(state => state.ticks >= 2, "initial application ticks");
  check(await inspect("performance.now() - window.__CLOCK_STATE__.elapsed >= 60000"),
    "page and runtime clock origins were not separated");
  const point = await inspect(`(() => {
    const node = window.__OPERAD_UAT__.snapshot().nodes.find(node => node.name === 'text');
    return { x: node.rect.x + 12, y: node.rect.y + 12 };
  })()`);
  for (const type of ["mousePressed", "mouseReleased"]) {
    await send("Input.dispatchMouseEvent", {type, ...point, button: "left",
      buttons: type === "mousePressed" ? 1 : 0, clickCount: 1}, sessionId);
  }
  const focusDeadline = Date.now() + timeoutMs;
  while (!await inspect("document.activeElement.id === 'operad-canvas-ime'")) {
    check(Date.now() < focusDeadline, "text input did not acquire focus");
    await new Promise(resolve => setTimeout(resolve, 25));
  }
  let expectedText = "";
  for (const [character, nativeKey] of [["a", "Process"], ["b", "Dead"]]) {
    const observed = await inspect(`(() => {
      const input = document.activeElement;
      const before = window.__CLOCK_STATE__;
      const key = ${JSON.stringify(character)};
      const code = 'Key' + key.toUpperCase();
      // Keep the ordinary key queued until the native composition key flushes
      // it synchronously, before another animation frame can run.
      for (const type of ['keydown', 'keyup']) input.dispatchEvent(new KeyboardEvent(type,
        {key, code, bubbles: true, cancelable: true}));
      const native = {key: ${JSON.stringify(nativeKey)}, code: 'KeyZ',
        bubbles: true, cancelable: true};
      input.dispatchEvent(new KeyboardEvent('keydown', native));
      const flushed = window.__CLOCK_STATE__;
      input.dispatchEvent(new KeyboardEvent('keyup', native));
      return {before, flushed, surrounding: input.value};
    })()`);
    expectedText += character;
    check(observed.flushed.frames > observed.before.frames, "composition did not synchronously flush input");
    check(observed.flushed.text === expectedText && observed.surrounding === expectedText,
      `composition received stale surrounding text: ${JSON.stringify(observed)}`);
    const elapsed = observed.flushed.elapsed - observed.before.elapsed;
    const ticks = observed.flushed.ticks - observed.before.ticks;
    const resumed = await wait(state => state.ticks >= observed.flushed.ticks + 2,
      `application ticks stalled after composition: ${JSON.stringify(observed)}`,
      Math.min(timeoutMs, 2000));
    check(elapsed >= 0 && ticks <= Math.floor(elapsed / 50) + 1,
      `composition advanced application time: ${JSON.stringify(observed)}`);
    check(resumed.text === expectedText, "flushed input was replayed by a scheduled frame");
  }
  console.log("Clock probe passed: distinct page/runtime origins, synchronous surrounding text, bounded ticks, scheduled recovery.");
}
