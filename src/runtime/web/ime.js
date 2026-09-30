// The browser owns composition in a real editable element. Operad owns the
// committed model and paints the editor; this element supplies native IME,
// surrounding text, and candidate positioning for the canvas.
export function createImeBridge(canvas, emit) {
  const listeners = [];
  let element = null;
  let session = null;
  let composing = false;
  let suppress = false;
  let base = "";
  let replacement = [0, 0];
  let draft = "";
  let lastValue = "";
  const suppressedKeys = new Set();
  const measure = document.createElement("canvas").getContext("2d");

  const listen = (target, name, callback) => {
    target.addEventListener(name, callback);
    listeners.push([target, name, callback]);
  };
  const removeElement = () => {
    for (let i = listeners.length - 1; i >= 0; i--) {
      const [target, name, callback] = listeners[i];
      if (target === element) {
        target.removeEventListener(name, callback);
        listeners.splice(i, 1);
      }
    }
    element?.remove();
  };
  const selection = () => {
    const range = [element.selectionStart ?? 0, element.selectionEnd ?? 0];
    return element.selectionDirection === "backward" ? range.reverse() : range;
  };
  const select = range => element.setSelectionRange(
    Math.min(...range), Math.max(...range), range[0] > range[1] ? "backward" : "forward"
  );
  const send = (kind, text = "", source = base, range = replacement, selected = null) => {
    if (session && !suppress) emit({ kind, input: session.input, text, source, range, selection: selected });
  };
  const change = (before, after) => {
    // Work in code points so a shared high surrogate cannot split two emoji.
    const a = [...before], b = [...after];
    let start = 0, end = 0;
    while (start < a.length && start < b.length && a[start] === b[start]) start++;
    while (end < a.length - start && end < b.length - start && a[a.length - end - 1] === b[b.length - end - 1]) end++;
    return {
      range: [a.slice(0, start).join("").length, a.slice(0, a.length - end).join("").length],
      text: b.slice(start, b.length - end).join(""),
    };
  };
  const updateDraft = () => {
    const prefix = base.slice(0, replacement[0]), suffix = base.slice(replacement[1]);
    if (element.value.startsWith(prefix) && element.value.endsWith(suffix)
        && element.value.length >= prefix.length + suffix.length) {
      draft = element.value.slice(prefix.length, element.value.length - suffix.length);
    } else {
      // Some input methods begin reconversion with their own replacement range.
      const delta = change(base, element.value);
      replacement = delta.range;
      draft = delta.text;
    }
    const selected = selection().map(offset => Math.max(0, Math.min(draft.length, offset - replacement[0])));
    send("preedit", draft, base, replacement, selected);
  };
  const cancel = () => {
    if (!composing) return;
    composing = false;
    send("cancel");
    suppress = true;
    element.value = base;
    select(replacement);
    lastValue = base;
    suppress = false;
  };

  const makeElement = sensitive => {
    const input = document.createElement(sensitive ? "input" : "textarea");
    if (sensitive) input.type = "password";
    else input.wrap = "off";
    input.id = `${canvas.id}-ime`;
    input.tabIndex = -1;
    input.setAttribute("aria-label", "Text input");
    input.setAttribute("autocomplete", "off");
    input.setAttribute("autocorrect", "off");
    input.setAttribute("autocapitalize", "off");
    input.spellcheck = false;
    input.style.cssText = "position:fixed;opacity:0;pointer-events:none;width:1px;padding:0;border:0;margin:0;resize:none;outline:none;white-space:pre;font:16px monospace;overflow:hidden;z-index:-1;";
    document.body.appendChild(input);
    listen(input, "compositionstart", () => {
      if (suppress || !session) return;
      composing = true;
      base = input.value;
      replacement = [input.selectionStart ?? 0, input.selectionEnd ?? 0];
      draft = "";
    });
    listen(input, "compositionupdate", event => {
      if (suppress || !session || !composing) return;
      draft = event.data ?? "";
      // The following input event supplies the updated DOM selection. Updating
      // here also covers input methods that only emit compositionupdate.
      send("preedit", draft, base, replacement, [draft.length, draft.length]);
    });
    listen(input, "input", event => {
      if (suppress || !session) return;
      if (composing || event.isComposing) {
        updateDraft();
      } else if (input.value !== lastValue) {
        const delta = change(lastValue, input.value);
        send("commit", delta.text, lastValue, delta.range);
        lastValue = input.value;
      }
    });
    listen(input, "compositionend", event => {
      if (suppress || !session || !composing) return;
      if (!event.data) { cancel(); return; }
      composing = false;
      send("commit", event.data, base, replacement);
      lastValue = base.slice(0, replacement[0]) + event.data + base.slice(replacement[1]);
      // A browser may send its final input before or after compositionend.
      // A following input with this same value must not commit a second time.
    });
    listen(input, "beforeinput", event => {
      if (suppress || !session || composing || event.isComposing) return;
      const kind = event.inputType;
      if (kind === "historyUndo" || kind === "historyRedo") {
        event.preventDefault();
        send(kind === "historyUndo" ? "undo" : "redo");
      } else if (!session.multiline && (kind === "insertLineBreak" || kind === "insertParagraph")) {
        event.preventDefault();
        send("enter");
      }
    });
    listen(input, "blur", cancel);
    return input;
  };
  listen(document, "selectionchange", () => {
    if (!suppress && composing && document.activeElement === element) updateDraft();
  });
  listen(window, "blur", () => {
    suppressedKeys.clear();
    cancel();
  });

  return {
    sync(next, left, top, width, height) {
      const changed = session?.input !== next.input;
      const replaceElement = !element || (element.tagName === "INPUT") !== next.sensitive;
      // compositionstart can precede the first nonempty draft by a frame.
      // Only an acknowledged draft transitioning to absent is an app cancel.
      const ended = composing && session?.composing && !next.composing;
      if (replaceElement && !changed && !ended && composing) {
        // Replacing the editable element ends its native composition. Notify
        // the model and seed the new element with committed text while the
        // cancellation waits for the next application frame.
        cancel();
        next = { ...next, text: base, selectionStart: replacement[0],
          selectionEnd: replacement[1], composing: false };
      }
      suppress = true;
      if (changed || ended || replaceElement) {
        composing = false;
        element?.blur();
      }
      if (replaceElement) {
        removeElement();
        element = makeElement(next.sensitive);
      }
      session = next;
      element.style.left = `${left}px`;
      element.style.top = `${top}px`;
      element.style.height = `${Math.max(1, height)}px`;
      element.style.lineHeight = `${Math.max(1, height)}px`;
      element.style.display = "block";
      element.setAttribute("enterkeyhint", next.multiline ? "enter" : "done");
      if (!composing) {
        if (element.value !== next.text) element.value = next.text;
        select([next.selectionStart, next.selectionEnd]);
        lastValue = next.text;
      }
      suppress = false;
      this.focus();
      // Keep the native caret inside the one-pixel input surface positioned at
      // Operad's measured caret, including selections earlier in multiline text.
      const prefix = element.value.slice(0, element.selectionEnd ?? 0).split("\n");
      if (measure) {
        measure.font = "16px monospace";
        element.scrollLeft = Math.max(0, measure.measureText(prefix.at(-1)).width - 1);
      }
      element.scrollTop = Math.max(0, (prefix.length - 1) * height);
    },
    deactivate(input) {
      if (session?.input !== input) return;
      const ownedFocus = document.activeElement === element;
      suppress = true;
      composing = false;
      session = null;
      element.blur();
      element.value = "";
      element.style.display = "none";
      lastValue = "";
      if (ownedFocus) canvas.focus({ preventScroll: true });
      suppress = false;
    },
    focus() {
      if (!session) return;
      const focused = document.activeElement;
      if (focused === canvas || focused === document.body || focused === element) {
        element.focus({ preventScroll: true });
      }
    },
    isFocused() { return !!session && document.activeElement === element; },
    isComposing() { return composing; },
    ownsKey(code, pressed, nativeComposing) {
      if (!pressed) {
        const owned = suppressedKeys.delete(code);
        return owned || composing || nativeComposing;
      }
      if (composing || nativeComposing) {
        suppressedKeys.add(code);
        return true;
      }
      return suppressedKeys.has(code);
    },
    destroy() {
      suppress = true;
      for (const [target, name, callback] of listeners) target.removeEventListener(name, callback);
      element?.remove();
      session = null;
    },
  };
}
