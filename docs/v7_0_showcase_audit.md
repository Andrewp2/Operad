# Operad 7.0 Showcase Audit

The showcase is a learning example. It should demonstrate the public API and
normal widget behavior, not carry tests, stress probes, screenshot paths, or
hidden diagnostics.

## Public API Use

`examples/showcase.rs` imports from `operad` and `operad::widgets`. Because
Cargo builds examples as external crates, this verifies that the showcase is
using public crate APIs instead of private module paths.

Checked patterns:

```bash
rg -n "crate::|super::" examples/showcase.rs
```

Result: no private crate/module imports.

## No Hidden Test Harness

The showcase no longer contains the old headless, screenshot, stress, capture,
or environment-variable paths. The only image-like match in the audit is the
literal `logo.png` label used inside the tree widget sample.

Checked patterns:

```bash
rg -n "cfg\\(test\\)|test\\]|stress|screenshot|snapshot|headless|std::env|OPERAD_|perf|diagnostic|capture|png|zlib|crc" examples/showcase.rs
```

Result: no harness code, no screenshot path, no performance path, no diagnostic
path, and no environment-variable mode switch.

## Widget State Coverage

The showcase issue tracker records the current state coverage pass in
`docs/showcase_widget_issue_tracker.md`. It includes:

- normal, hovered, pressed, pressed-hovered, disabled, focused, selected, and
  toggle states for controls where those states apply
- min-size and resized floating-window behavior
- popup-open and submenu-open states
- overlapping floating-window z-order and hit routing
- text input caret, selection, clipboard, placeholder, deletion, keyboard, and
  IME-facing paths
- scrollbar alignment, click/drag behavior, and scroll position updates
- combo box, submenu, tooltip, popup, toast, command palette, and modal overlay
  behavior without inline layout shifts

Remaining visual regressions should be added to the issue tracker first, then
fixed in the widget primitive or runtime surface rather than patched only in the
example.
