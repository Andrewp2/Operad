// One connection owns its opening deadline, requests, and transport failure.
// A lost response is terminal: later commands must not continue a partial probe.
export function connectCdp(ws, { timeoutMs, onEvent = () => {} }) {
  return new Promise((resolveOpen, rejectOpen) => {
    let nextId = 0;
    let failure = null;
    const pending = new Map();
    const openTimer = setTimeout(() => {
      fail(new Error(`CDP connection timed out after ${timeoutMs} ms`));
    }, timeoutMs);

    function fail(error) {
      if (failure) return;
      failure = error;
      clearTimeout(openTimer);
      ws.onopen = ws.onmessage = ws.onerror = ws.onclose = null;
      for (const request of pending.values()) {
        clearTimeout(request.timer);
        request.reject(new Error(`${request.label}: ${error.message}`, { cause: error }));
      }
      pending.clear();
      rejectOpen(error);
      try { ws.close(); } catch { /* Preserve the original transport failure. */ }
    }

    function send(method, params = {}, sessionId = undefined) {
      const label = sessionId ? `${method} (session ${sessionId})` : method;
      return new Promise((resolve, reject) => {
        if (failure) {
          reject(new Error(`${label}: ${failure.message}`, { cause: failure }));
          return;
        }
        const message = { id: ++nextId, method, params };
        if (sessionId) message.sessionId = sessionId;
        // Serialization errors reject just this unsent request.
        const data = JSON.stringify(message);
        const timer = setTimeout(() => {
          fail(new Error(`CDP request timed out after ${timeoutMs} ms (${label})`));
        }, timeoutMs);
        pending.set(message.id, { label, resolve, reject, timer });
        try {
          ws.send(data);
        } catch (error) {
          fail(new Error(`CDP send failed: ${error.message}`, { cause: error }));
        }
      });
    }

    ws.onopen = () => {
      clearTimeout(openTimer);
      resolveOpen({
        send,
        assertOpen() { if (failure) throw failure; },
        close() { fail(new Error("CDP connection closed by caller")); },
      });
    };
    ws.onerror = event => {
      fail(new Error(`CDP transport error: ${event.message ?? event.error?.message ?? "WebSocket failed"}`));
    };
    ws.onclose = event => {
      fail(new Error(`CDP connection closed (${event.code ?? "unknown"})${event.reason ? `: ${event.reason}` : ""}`));
    };
    ws.onmessage = event => {
      let message;
      try {
        message = JSON.parse(event.data);
        if (!message || typeof message !== "object" || Array.isArray(message)) {
          throw new Error("expected a CDP message object");
        }
      } catch (error) {
        fail(new Error(`Invalid CDP message: ${error.message}`, { cause: error }));
        return;
      }
      if (Object.hasOwn(message, "id")) {
        const request = pending.get(message.id);
        if (!request) return;
        pending.delete(message.id);
        clearTimeout(request.timer);
        if (message.error) {
          request.reject(new Error(`${request.label}: ${JSON.stringify(message.error)}`));
        } else {
          request.resolve(message.result);
        }
      } else {
        try {
          onEvent(message);
        } catch (error) {
          fail(new Error(`CDP event handler failed: ${error.message}`, { cause: error }));
        }
      }
    };
  });
}
