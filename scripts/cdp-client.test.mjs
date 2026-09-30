import assert from "node:assert/strict";
import test from "node:test";
import { connectCdp } from "./cdp-client.mjs";

class Socket {
  sent = [];
  closes = 0;
  send(data) {
    this.sent.push(JSON.parse(data));
    this.onSend?.(this.sent.at(-1));
  }
  close() { this.closes += 1; }
  open() { this.onopen?.({}); }
  message(message) { this.onmessage?.({ data: JSON.stringify(message) }); }
}

function observe(promise) {
  const state = { kind: "pending" };
  promise.then(value => Object.assign(state, { kind: "resolved", value }),
    error => Object.assign(state, { kind: "rejected", error }));
  return state;
}

async function open(t, onEvent = () => {}) {
  const ws = new Socket();
  const ready = connectCdp(ws, { timeoutMs: 100, onEvent });
  ws.open();
  const connection = await ready;
  t.after(() => connection.close());
  return { ws, connection };
}

test("CDP correlates out-of-order replies and isolates protocol errors", async t => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  const events = [];
  const { ws, connection } = await open(t, event => events.push(event));
  const first = connection.send("Runtime.evaluate", { expression: "first" }, "session-a");
  const bad = connection.send("Runtime.evaluate", { expression: "bad" }, "session-b");
  const last = connection.send("Target.getTargets");
  const rejected = assert.rejects(bad, /Runtime.evaluate.*session-b.*method unavailable/);
  assert.equal(new Set(ws.sent.map(message => message.id)).size, 3);
  assert.equal(ws.sent[0].sessionId, "session-a");
  assert.deepEqual(ws.sent[0].params, { expression: "first" });
  assert.equal(ws.sent[2].sessionId, undefined);
  const event = { method: "Network.loadingFailed", params: { requestId: "r" }, sessionId: "session-a" };
  ws.message(event);
  ws.message({ id: ws.sent[2].id, result: { targetInfos: ["target"] } });
  ws.message({ id: ws.sent[1].id, error: { code: -32601, message: "method unavailable" } });
  ws.message({ id: ws.sent[0].id, result: { value: 41 } });
  assert.deepEqual(await first, { value: 41 });
  assert.deepEqual(await last, { targetInfos: ["target"] });
  await rejected;
  assert.deepEqual(events, [event]);

  // Completed success/error timers cannot later poison a healthy connection.
  t.mock.timers.tick(1000);
  connection.assertOpen();
  assert.equal(ws.closes, 0);
  // Also cover a transport that replies during send: registration comes first.
  ws.onSend = message => ws.message({ id: message.id, result: { value: 42 } });
  assert.deepEqual(await connection.send("Runtime.evaluate"), { value: 42 });
  ws.message({ id: ws.sent[0].id, result: { value: "late duplicate" } });
  assert.deepEqual(events, [event], "replies must not leak into the event consumer");
});

test("CDP opening ends on a deadline, close, or error", async t => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  for (const fault of ["deadline", "close", "error"]) {
    const ws = new Socket();
    const state = observe(connectCdp(ws, { timeoutMs: 100 }));
    t.mock.timers.tick(99);
    await Promise.resolve();
    assert.equal(state.kind, "pending");
    if (fault === "deadline") t.mock.timers.tick(1);
    if (fault === "close") ws.onclose({ code: 1006, reason: "startup closed" });
    if (fault === "error") ws.onerror({ message: "startup error" });
    await Promise.resolve();
    assert.equal(state.kind, "rejected", fault);
    assert.match(state.error.message, fault === "deadline" ? /timed out/ : /startup/);
    ws.open();
    t.mock.timers.tick(1000);
    assert.equal(ws.closes, 1);
    assert.equal(state.kind, "rejected");
  }
});

test("CDP transport failure rejects every request and prevents later commands", async t => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  for (const fault of ["deadline", "close", "error", "send", "json", "null", "handler", "caller"]) {
    const { ws, connection } = await open(t, () => { throw new Error("observer failed"); });
    const first = observe(connection.send("Runtime.evaluate", {}, "a"));
    t.mock.timers.tick(40);
    const second = observe(connection.send("Page.captureScreenshot", {}, "b"));
    const extra = [];
    if (fault === "deadline") t.mock.timers.tick(60);
    if (fault === "close") ws.onclose({ code: 1006, reason: "lost" });
    if (fault === "error") ws.onerror({ message: "lost" });
    if (fault === "send") {
      ws.onSend = () => { throw new Error("socket write failed"); };
      extra.push(observe(connection.send("Target.getTargets")));
    }
    if (fault === "json") ws.onmessage({ data: "{" });
    if (fault === "null") ws.message(null);
    if (fault === "handler") ws.message({ method: "Runtime.consoleAPICalled" });
    if (fault === "caller") connection.close();
    await Promise.resolve();
    for (const state of [first, second, ...extra]) assert.equal(state.kind, "rejected", fault);
    assert.match(first.error.message, /Runtime.evaluate.*session a/);
    assert.match(second.error.message, /Page.captureScreenshot.*session b/);
    const sentCount = ws.sent.length;
    await assert.rejects(connection.send("Runtime.enable"), /Runtime.enable/);
    assert.equal(ws.sent.length, sentCount, "a failed connection must not send new work");
    assert.throws(() => connection.assertOpen());
    ws.message({ id: ws.sent[0].id, result: { stale: true } });
    t.mock.timers.tick(1000);
    connection.close();
    assert.equal(ws.closes, 1, "terminal cleanup must be idempotent");
  }
});

test("CDP rejects an unsent serialization error without failing other requests", async t => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  const { ws, connection } = await open(t);
  const pending = connection.send("Runtime.evaluate");
  const circular = {};
  circular.self = circular;
  await assert.rejects(connection.send("Runtime.evaluate", circular), /circular/i);
  assert.equal(ws.sent.length, 1);
  ws.message({ id: ws.sent[0].id, result: { ok: true } });
  assert.deepEqual(await pending, { ok: true });
  t.mock.timers.tick(1000);
  connection.assertOpen();
});

test("CDP records an idle disconnect so the probe cannot report success", async t => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  const { ws, connection } = await open(t);
  t.mock.timers.tick(1000);
  connection.assertOpen();
  ws.onclose({ code: 1006, reason: "renderer disappeared" });
  assert.throws(() => connection.assertOpen(), /renderer disappeared/);
});
