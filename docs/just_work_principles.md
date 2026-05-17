# Making Operad Just Work

This document captures the engineering work that would make Operad feel
reliable by default. The goal is not to hide complexity from advanced users. The
goal is to make the obvious path correct, observable, and portable enough that a
new app can get useful UI on screen without building a custom host, layout
debugger, event router, or performance profiler first.

## Product Definition

Operad "just works" when:

- The blessed app entry point opens a working native or web UI with sensible
  defaults.
- Widgets publish enough sizing, input, accessibility, animation, and rendering
  metadata for generic systems to compose them correctly.
- Layout, scroll, focus, overlays, clipboard, IME, cursor behavior, animation
  ticking, and rendering are handled by the runtime unless the app explicitly
  takes lower-level control.
- Failure modes explain what happened, where it happened, and what to do next.
- Debug tooling points to the responsible node, parent constraint, host
  capability, renderer resource, or backend limit.
- CI can catch clipping, unreachable scroll ranges, overlay placement mistakes,
  broken input routing, and performance regressions before release.

## Non-Goals

- Do not make every feature automatic at the cost of removing explicit control.
  Operad should provide escape hatches for custom hosts, custom renderers, and
  advanced canvas input.
- Do not solve showcase bugs with showcase-only sizing constants. Showcase
  should demonstrate the library behavior; it should not compensate for missing
  library contracts.
- Do not make diagnostics depend on screenshots alone. Visual testing is useful,
  but the primary diagnostics should come from the retained document model and
  computed layout, paint, input, scroll, accessibility, and host state.

## Core Principles

### One obvious path

There should be one primary way to start an app, one primary way to run the
showcase, and one primary way to mount debug tools. Advanced APIs can exist, but
the default path should be the best-tested path.

### Generic contracts before widget fixes

Every widget should compute and publish its own intrinsic minimum, preferred,
and flexible size from real content: text metrics, icons, padding, margins,
stroke, shadows, aspect ratios, scroll content, and child composition. Containers
should combine child sizes through their layout algorithm instead of depending
on guessed window sizes.

### Measure, then position, then draw

The runtime should treat UI construction as three separate concerns:

1. Compute sizes and constraints.
2. Compute positions, clips, scroll ranges, and overlay placement.
3. Build paint and render output from the computed layout.

Drawing should not be where layout is discovered. If a widget only learns its
true size during paint, generic window minimums, scrollbars, overlays, and tests
will always be unreliable.

### Explain failures at the source

If text clips, a scroll range is unreachable, a popover is clamped, a cursor grab
is denied, or a frame takes too long, Operad should report the node and the rule
that caused it. Users should not need to infer the cause from visual artifacts.

### Host capabilities are data

Native, web, test, and custom hosts should publish capability data. Widgets and
apps should be able to query support for key release, IME, clipboard, raw mouse
motion, cursor grab, pointer lock, WebGPU, accessibility, drag and drop, and
platform overlays before depending on them.

### Tests are part of the API

The no-clipping, scroll, overlay, input-routing, accessibility, and performance
contracts should be testable through public or crate-internal diagnostics. A
downstream app should be able to run the same class of audits that Operad uses
for showcase.

## Success Criteria

- A new user can create a minimal native app from one small example and run it
  without host glue.
- A new user can run the showcase in a browser through the supported web target.
- A widget window cannot be resized below its computed content minimum unless
  the widget explicitly opts into clipping, overflow, or scrolling.
- Scrollbars appear only when there is a scroll range, consume input when
  visually on top, and can reach the full content extent.
- Popovers, menus, tooltips, color pickers, and context menus are positioned
  relative to their triggering control and stay reachable through the overlay
  system.
- Canvas and animation widgets publish aspect and minimum-size constraints
  generically, not with per-demo patches.
- Debug mode can identify the node responsible for layout, scroll, overlay,
  input, animation, host-capability, and renderer failures.
- With all showcase widgets open, performance diagnostics identify the dominant
  frame costs and CI prevents known regressions.

## Blessed App Runtime

### User promise

Most apps should start from a single runtime entry point, such as
`operad::run(...)`, and get a complete default stack.

### Library contract

The default runtime should own:

- Native window creation.
- WebGPU renderer setup.
- Default theme and text system.
- Pointer, keyboard, wheel, clipboard, IME, cursor, drag, drop, and resize
  routing.
- Focus and accessibility publication.
- Overlay and portal dispatch.
- Scroll state persistence.
- Animation ticking and idle redraw.
- Recoverable debug error overlay.
- Performance counters and development diagnostics.

Custom runners should still be possible, but the blessed runtime should be the
reference implementation used by examples, tests, docs, and CI.

### Acceptance gates

- A minimal native app is under 50 lines and does not construct a host manually.
- A simple form, canvas app, command palette app, and docked workspace app can
  all use the same runner.
- The default runner exposes the same diagnostics as custom hosts through stable
  frame output types.

## First-Class Web Target

### User promise

The showcase and ordinary apps should run as WASM/WebGPU applications without
copying example-specific host code.

### Library contract

The web runtime should provide:

- A reusable `operad::web::run(...)` style entry point.
- Automatic canvas lookup or creation.
- WebGPU surface creation and device-pixel-ratio handling.
- Resize handling through browser APIs.
- Pointer, wheel, keyboard, text, clipboard, focus, and open-url integration.
- A clear WebGPU-unavailable fallback.
- GitHub Pages friendly build output.
- CI coverage for `wasm32-unknown-unknown`.

### Acceptance gates

- `examples/showcase_web.rs` is thin and mostly app setup.
- CI builds the web showcase artifact.
- The fallback page explains browser and GPU requirements instead of failing
  silently.
- Input parity gaps between native and web are documented through the host
  capability matrix.

## Layout And Sizing Contract

### User promise

Users should not need to manually widen windows because a label, button, header,
input, canvas, scrollbar, or child row forgot how large it is.

### Library contract

Every widget should publish:

- Minimum size.
- Preferred size.
- Flexible growth behavior.
- Baseline or text alignment data where relevant.
- Aspect-ratio constraints where relevant.
- Scroll content size and viewport policy where relevant.
- Explicit overflow policy when clipping is intended.

Every container should combine child constraints through its layout rule:

- Rows sum child widths and take the maximum child height.
- Columns sum child heights and take the maximum child width.
- Wrapping containers compute line breaks from real child minimums.
- Scroll containers distinguish viewport minimum from content extent.
- Windows include chrome, title text, resize handle, content padding, and child
  minimums in their own minimum size.
- Collapsible containers include header content and expanded child content when
  computing the expanded minimum.

### Acceptance gates

- Resizing a window to its minimum never clips text or controls unless the
  clipped node opted into clipping or scrolling.
- Text measurement uses the real font and shaping mode selected by the app.
- Buttons size from text or icon, padding, border, focus ring, and margins.
- Inputs with bounded numeric ranges can reserve enough width for their largest
  formatted value when configured to stay single-line.
- Canvas-like widgets publish aspect constraints so they scale from available
  width and height instead of stretching on one axis.

## Debug Mode That Explains Problems

### User promise

When something looks wrong, the user should be able to ask Operad why.

### Library contract

The debug surface should expose:

- Stable node path and widget kind.
- Computed minimum, preferred, final, and content sizes.
- Parent constraints and layout rule.
- Margins, padding, border, stroke, shadow, gaps, and alignment.
- Text measurement source, shaping mode, line breaks, and overflow policy.
- Position, visible rect, clip chain, scroll viewport, content extent, and
  scroll offset.
- Paint order, z-order, hit-test eligibility, focus state, and pointer target.
- Accessibility role, label, value, actions, focus order, and relations.
- Overlay origin, placement, clamping, dismissal, and reachability.
- Input event routing, capture, consumption, bubbling, and fallback behavior.
- Host capability checks and denied platform requests.

This should build on the existing v8 debug snapshot work and become the shared
foundation for in-app inspectors, tests, and runtime error reports.

### Acceptance gates

- A developer can select a clipped text node and see which constraint caused the
  clipping.
- A developer can inspect a missed scroll event and see which node consumed it.
- A developer can inspect a menu or color picker and see the trigger rect,
  portal layer, final placement, and clamp decision.
- A test can consume the same diagnostics without relying on a screenshot.

## Golden No-Clipping Audit

### User promise

The class of bugs where text, controls, scroll ranges, and overlays fall off the
edge should be caught generically.

### Library contract

Operad should provide a reusable audit pass over a computed frame. It should
flag:

- Text clipped without an explicit clipping or overflow policy.
- Interactive controls smaller than their computed minimum.
- Buttons whose text or icon cannot fit inside the button.
- Windows or panels resizable below their content minimum.
- Scrollable content with hidden or unreachable scroll range.
- Scrollbars visible when there is no scroll range.
- Scrollbars that cannot reach the full scroll extent.
- Focusable nodes outside the visible viewport.
- Overlays outside reachable bounds.
- Hit targets below the minimum interactive size unless explicitly compact.
- Input routed to a visually lower window while a higher window should consume
  the event.

### Acceptance gates

- Showcase state-matrix tests run the audit for every widget in normal,
  collapsed, expanded, scrolled, focused, hovered, and narrow states.
- Event replay can apply a long synthetic scroll and assert that the scrollbar
  reaches the bottom.
- Overlay tests assert relative placement for controls at different window
  positions.
- Audit failures include node path, reason, measured values, and a short
  remediation hint.

## Unified Scroll Containers

### User promise

Scrolling should work the same way everywhere without each demo or widget
manually wiring scrollbars.

### Library contract

The scroll container should own:

- Content extent calculation.
- Viewport size calculation.
- Scrollbar visibility.
- Thumb size and position.
- Wheel, drag, keyboard, and programmatic scroll input.
- Hit testing and event consumption while visually on top.
- Nested scroll behavior.
- Accessibility actions and values.

### Acceptance gates

- Scrollbars disappear and become uninteractable when no scrolling is possible.
- A long synthetic scroll reaches the exact bottom of the content.
- Wheel input over a topmost window with no scroll range is consumed when that
  window visually owns the pointer region, preventing accidental scrolling of a
  lower window.
- All existing scroll surfaces use the shared scroll container contract.

## Overlay And Portal System

### User promise

Menus, popovers, color pickers, tooltips, and context menus should appear next
to the control that opened them, remain reachable, and route input correctly.

### Library contract

The overlay system should own:

- Trigger rect capture.
- Layer ordering.
- Viewport-aware placement.
- Collision and clamp decisions.
- Focus transfer and restoration.
- Dismissal on outside click, escape, command, or trigger toggle.
- Hit testing above ordinary content.
- Diagnostics for placement and dismissal.

### Acceptance gates

- Dropdowns and color pickers are positioned relative to their triggering
  controls, not the root origin.
- Overlays remain reachable at screen edges.
- Overlays consume pointer and wheel input before lower windows.
- Placement decisions are visible in debug mode.

## Host Capability Matrix

### User promise

Apps should know whether a host can support the interaction they are asking for.
Missing platform features should degrade clearly.

### Library contract

Each host should publish support for:

- Key press and release.
- Text input and IME.
- Clipboard read and write.
- Open URL.
- Drag and drop.
- Cursor icon, cursor visibility, and cursor grab.
- Raw mouse motion and pointer lock.
- Wheel phases and high-resolution scroll.
- Gamepad or non-pointer input.
- Accessibility publication.
- WebGPU surface rendering.
- Native child windows or platform overlays.

Widgets should choose one of four outcomes:

- Use the feature.
- Use a fallback behavior.
- Disable the dependent control.
- Emit a diagnostic explaining why the feature is unavailable.

### Acceptance gates

- Canvas flycam-style input can request key release, raw mouse motion, pointer
  lock, cursor visibility, and cursor grab through host capabilities.
- Command and hotkey routing can depend on key press and release consistently.
- Unsupported requests appear in diagnostics instead of failing silently.

## Runtime Errors And Recovery

### User promise

When a runtime failure happens, the error should be actionable.

### Library contract

Errors should include:

- Backend and target.
- Enabled features when relevant.
- Operation that failed.
- Node, resource, or host subsystem when relevant.
- User-visible consequence.
- Practical next step.

Important error families:

- Missing or unavailable WebGPU.
- Surface creation failure.
- Shader compile failure.
- Broken native event loop or pipe failure.
- Unsupported platform request.
- Missing clipboard provider.
- Invalid render target.
- Resource upload mismatch.
- Fatal layout cycle or invalid constraint.

Recoverable failures in examples and debug builds should be shown in an overlay
instead of only printing a raw panic or backend error.

### Acceptance gates

- Showcase startup failures explain the failing subsystem and next step.
- Recoverable renderer and host failures can be surfaced in a debug overlay.
- Panic messages avoid raw index-only failures when stable node paths or
  subsystem names are available.

## Performance Guardrails

### User promise

If an app is slow, Operad should identify the expensive subsystem and the
largest contributing nodes.

### Library contract

The runtime should publish:

- FPS and frame time.
- Layout time.
- Intrinsic measurement time.
- Text shaping time.
- Paint build time.
- Render time.
- Node count.
- Paint item count.
- Rebuild count per frame.
- Widget action count.
- Animation update cost.
- Scroll and overlay cost.
- GPU timing when available.
- Worst offending node path or subsystem.

### Acceptance gates

- Showcase has a visible FPS counter with normal text margins.
- Opening all widgets remains within an agreed interactive budget.
- CI has at least one performance smoke test or trace budget for all-widgets
  showcase state.
- Diagnostics identify whether the bottleneck is layout, text, paint, render,
  animation, or host input.

## Starter Templates And Documentation

### User promise

New users should be able to copy a small example and succeed.

### Library contract

Provide maintained templates for:

- Minimal native app.
- Minimal web app.
- Simple form.
- Canvas/WebGPU app.
- Docked workspace app.
- Command palette and hotkeys app.
- Theme customization app.
- Animation state machine app.

Longer term, this can become a `cargo operad new` workflow. Until then, the
templates should be ordinary checked examples that compile in CI.

### Acceptance gates

- Every template is compiled or checked in CI.
- Examples use the blessed runtime and public APIs.
- Showcase remains a teaching surface, not a hidden test harness.
- Regression tests live outside `examples/showcase.rs`.

## Window And Workspace Management

### User promise

Floating windows should be manageable when many are open. Automatic organization
should preserve content and avoid visual overlap without guessing from collapsed
sizes alone.

### Library contract

Window organization should use the computed expanded minimum and preferred sizes
of each window, then place windows through a generic packing strategy. The
strategy may use a rectangle-packing crate, but it must still respect Operad's
computed sizes, viewport constraints, z-order, and user-resized windows.

### Acceptance gates

- Organize uses expanded content constraints, not only current collapsed
  headers.
- The algorithm avoids overlap when the viewport can fit the windows.
- When everything cannot fit, the result is deterministic and diagnostic rather
  than silently clipping important content.
- Collapsing windows can be an intentional fallback, but not a substitute for
  knowing the expanded size.

## Implementation Order

### Phase 1: turn existing diagnostics into a contract

- Promote the current debug snapshot data into a documented audit surface.
- Add a `JustWorkAudit` style pass over computed document frames.
- Report clipping, scroll, overlay, focus, event-routing, and host-capability
  failures through stable diagnostics.
- Make showcase diagnostics consume the same data used by tests.

### Phase 2: enforce layout and scroll invariants

- Require intrinsic size publication from core widgets.
- Make windows derive resize minimums from content and chrome.
- Ensure scroll containers own visibility, range, hit testing, and event
  consumption.
- Add event replay tests for long scroll, nested scroll, overlay scroll, and
  resize-to-minimum cases.

### Phase 3: promote runtime and web support

- Move reusable web showcase host code into the library.
- Make native and web runners share the same app contract where possible.
- Publish host capability data for native, web, and test hosts.
- Improve runtime error reporting and debug overlays.

### Phase 4: make it easy to start and hard to regress

- Add starter templates.
- Add docs that explain the blessed app path and escape hatches.
- Add CI gates for native, web, no-clipping audits, scroll replay, overlay
  placement, and performance budgets.
- Keep showcase readable by moving test-only logic out of `examples/showcase.rs`.

## Test And Release Gates

Before calling the "just works" baseline complete for a release, run:

- `cargo fmt --all -- --check`
- `cargo check --locked --no-default-features --all-targets`
- `cargo check --locked --all-features --all-targets`
- `cargo test --locked --all-features`
- WASM showcase check or build for `wasm32-unknown-unknown`
- Showcase state-matrix tests outside `examples/showcase.rs`
- No-clipping audit tests for all public widgets
- Long synthetic scroll replay tests
- Overlay placement and dismissal replay tests
- Host capability matrix tests for native, web, and test hosts
- Runtime error smoke tests for missing or denied host capabilities
- Performance smoke test for all showcase widgets open

## Done When

This work is done when:

- The blessed native and web paths are documented, tested, and used by examples.
- Core widgets compute intrinsic sizes from real content and publish them to the
  layout system.
- Containers combine child constraints generically.
- Windows cannot shrink below their computed content minimum unless content is
  explicitly scrollable or clipped.
- Scroll, overlay, focus, and event routing use shared systems instead of
  per-widget wiring.
- Debug mode can explain layout, scroll, overlay, input, host, renderer, and
  performance failures.
- CI catches the recurring edge-falloff bugs that originally motivated this
  work.
- Showcase remains a readable demonstration of public APIs, while deeper
  validation lives in reusable diagnostics and tests.
