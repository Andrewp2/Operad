# Operad 6.0 Module Organization

Operad v5 shipped with a deliberately broad API surface, but the source layout
still mostly reflects incremental growth: most modules sit directly under
`src/`, and `src/lib.rs` owns core document types, text measurement, widget
helpers, tests, module declarations, and public re-exports. That made sense
while stabilizing v5, but it is not a good long-term shape.

V6 should reorganize the crate around ownership boundaries, not around whatever
was added first.

## Goals

- Make source layout communicate the architecture.
- Keep backend-neutral contracts separate from backend adapters.
- Keep reusable widgets separate from core document/runtime contracts.
- Keep diagnostics/testing/devtool utilities separate from production contracts.
- Keep `lib.rs` small enough to act as a crate entry point instead of a second
  implementation module.
- Preserve public paths intentionally, or break them intentionally as part of
  the v6 migration guide. Avoid accidental churn.

## V5 Shape

V5 shipped with these high-friction areas:

- `src/lib.rs` is over 11k lines and contains foundational types, widget helper
  implementations, public re-exports, and tests.
- Most conceptual subsystems are top-level siblings: `actions`, `commands`,
  `input`, `input_devices`, `transactions`, `navigation`, `overlays`,
  `renderer`, `paint`, `compositor`, `wgpu_renderer`, `host`, `runtime`,
  `windows`, and many more all live in the same directory.
- Optional backend integrations (`egui_host`, `wgpu_renderer`,
  `accesskit_winit_adapter`) sit next to backend-neutral contracts.
- `widget_ext` is the only real subtree, but core widgets still live in
  `lib.rs`, so widget implementation ownership is split.
- Tests are split between module-local tests and integration tests, but there is
  no obvious source-level separation between test support and runtime support.

## Proposed Source Tree

The source tree should move toward this shape:

```text
src/
  lib.rs
  prelude.rs

  core/
    mod.rs
    document.rs
    geometry.rs
    layout.rs
    text.rs
    animation.rs
    localization.rs
    versioning.rs

  interaction/
    mod.rs
    input.rs
    devices.rs
    actions.rs
    commands.rs
    drag_drop.rs
    navigation.rs
    overlays.rs
    transactions.rs
    focus.rs

  render/
    mod.rs
    paint.rs
    renderer.rs
    compositor.rs
    display.rs
    effective_geometry.rs
    resources.rs
    assets.rs
    fonts.rs

  runtime/
    mod.rs
    host.rs
    platform.rs
    windows.rs
    scheduler.rs

  adapters/
    mod.rs
    egui.rs
    accesskit_winit.rs
    wgpu.rs

  accessibility/
    mod.rs
    tree.rs
    adapter.rs
    help.rs

  widgets/
    mod.rs
    core.rs
    text_input.rs
    menu.rs
    data.rs
    pickers.rs
    surfaces.rs

  shell/
    mod.rs
    workspace.rs
    bars.rs
    panels.rs

  domain/
    mod.rs
    charts.rs
    editor.rs

  theme/
    mod.rs
    tokens.rs
    stability.rs

  diagnostics/
    mod.rs
    debug.rs
    errors.rs
    limits.rs
    testing.rs
```

This does not mean all of those files must exist immediately. It is the intended
ownership map.

## Initial V6 Restructure

The first v6 pass moved the existing modules into ownership folders while
preserving the flat v5 public module paths:

```text
src/
  accessibility/        # accessibility contracts and tooltip/help policy
  adapters/             # egui, wgpu, and accesskit-winit integrations
  core/                 # layout, localization, state, versioning
  diagnostics/          # debug, diagnostics reports, errors, limits, testing
  domain/               # charts and editor primitives
  interaction/          # input, devices, actions, commands, drag/drop, forms
  render/               # paint, renderer, compositor, assets, fonts, caches
  runtime/              # host, platform, frame runtime, windows
  shell/
  theme/
  widgets/ext/
```

`src/lib.rs` now uses path-based compatibility declarations for common v5
imports such as `operad::layout`, `operad::renderer`, and `operad::input`.
Grouped facades were added for `operad::core`, `operad::interaction`,
`operad::render`, `operad::runtime`, `operad::adapters`,
`operad::diagnostics`, and `operad::domain` so new code can start using the v6
shape without forcing every downstream consumer to migrate at once.

## GPU Canvas Rendering

V6 makes WGPU canvases first-class render targets. A canvas can declare a
texture-backed GPU context with `CanvasContent::gpu_context()` or
`UiNode::gpu_canvas(...)`; native-window apps can register app-owned rendering
with `NativeWgpuCanvasRenderRegistry`.

The registered renderer receives a `WgpuCanvasContext`, so it can create command
encoders, begin render passes, use its own pipelines, and submit any number of
WGPU passes into the canvas texture. The normal UI render pass samples that same
texture when it paints the canvas item.

## Public API Shape

The physical layout and public API do not have to be identical.

V6 should expose a clearer high-level API:

```rust
operad::core
operad::interaction
operad::render
operad::runtime
operad::adapters
operad::accessibility
operad::widgets
operad::theme
operad::diagnostics
operad::prelude
```

For common v5 paths, choose one of two strategies per module:

- Keep stable aliases when churn is not worth it, for example
  `operad::layout` re-exporting `operad::core::layout`.
- Break intentionally where the old path was actively misleading, with a
  migration guide entry and deprecation window if possible.

Do not keep every old flat module forever. That would preserve the current
problem under a layer of aliases.

## What Belongs Where

### Core

Core is the retained document model and data types needed before a host,
renderer, or widget exists:

- `UiDocument`, `UiNode`, `UiNodeId`
- geometry primitives such as `UiPoint`, `UiSize`, `UiRect`
- layout facade and layout snapshots
- text content, text style, measurement traits
- animation metadata
- localization and versioning metadata

Core should not know about WGPU, egui, AccessKit, widget extension controls, or
native windows.

### Interaction

Interaction owns user intent before product semantics:

- raw and normalized input
- pointer, wheel, keyboard, touch, stylus, and gamepad records
- actions, commands, shortcuts, drag/drop
- focus, navigation, overlays, transactions, selection

Interaction should be renderer-neutral and host-neutral. It may emit platform
requests as contracts, but it should not execute platform behavior.

### Render

Render owns backend-neutral drawing intent and resource contracts:

- paint records and display lists
- renderer adapter traits and render frame requests
- compositor feature requirements and fallback plans
- effective geometry used by paint, hit testing, and accessibility
- resources, assets, fonts, cache lifecycle

WGPU does not belong here as a backend-neutral contract; it belongs in
`adapters/wgpu.rs` or `adapters/wgpu/`.

### Runtime

Runtime owns host loop contracts:

- host frame request/output records
- scheduler, repaint, timer, idle, lifecycle contracts
- platform service request/response records
- multi-window/document/surface routing

Runtime should not include a concrete winit event loop implementation except
through an adapter module.

### Adapters

Adapters are optional backend integrations:

- `egui`
- `wgpu`
- `accesskit_winit`
- future native host bridges

Feature gates should live at this boundary whenever possible. Backend-specific
types should not leak back into core modules.

### Accessibility

Accessibility deserves its own subsystem because it cuts across document,
interaction, host, and widgets:

- accessibility tree and metadata records
- adapter request/response contracts
- focus traps, live regions, navigable targets
- help, tooltip, validation help, context menu policy where it affects
  accessible behavior

The AccessKit winit bridge still belongs under `adapters`.

### Widgets

Widgets should own reusable controls only:

- buttons, checkboxes, sliders, scrollbars
- text input and selectable text
- menus, popups, command palette, select/search controls
- data widgets, pickers, surfaces

Widget helpers can depend on core, interaction, render, accessibility, and
theme contracts. Core must not depend on widgets.

### Domain

Charts and editor primitives are reusable, but they are not the same layer as
buttons and menus. They should live under a domain-oriented subtree until they
are either split into a separate crate or promoted into a clearer subsystem.

### Diagnostics

Diagnostics, debug views, limits, errors, and testing helpers are related but
should not be mixed into the runtime path. They should be easy to find and easy
to feature-gate later if package size or compile time becomes a concern.

## Migration Plan

### Phase 1: Create Facades Without Moving Behavior

- Add group modules such as `core`, `interaction`, `render`, and `runtime` that
  re-export existing flat modules.
- Keep existing public paths working.
- Update docs and examples to prefer the grouped paths.

This phase is partly complete. `prelude.rs` is still intentionally deferred
until the main document/widget split makes the common import set clearer.

### Phase 2: Split `lib.rs`

Move implementation out of `lib.rs` in small pieces:

1. geometry primitives
2. visual/style/text primitives
3. document/node tree
4. layout, hit testing, focus, scrolling integration
5. core widget helpers
6. module-local tests

`lib.rs` should end this phase as module declarations, public re-exports, and
crate-level docs only.

### Phase 3: Move Backend-Neutral Modules Into Subtrees

Move files mechanically into their target folders while preserving public module
aliases:

- render contracts into `render/`
- interaction contracts into `interaction/`
- runtime contracts into `runtime/`
- accessibility contracts into `accessibility/`
- diagnostics/testing support into `diagnostics/`

Use small commits and run the full feature matrix after each group.

This phase is complete for the existing backend-neutral modules. The remaining
work is to reduce `src/lib.rs` by moving the large inline document and widget
implementations into the new folders.

### Phase 4: Move Optional Backends

Move optional integrations under `adapters/`:

- `egui_host` -> `adapters/egui`
- `wgpu_renderer` -> `adapters/wgpu`
- `accesskit_winit_adapter` -> `adapters/accesskit_winit`

At this point feature gates should read as adapter choices, not as random
top-level modules.

This phase is complete for the current optional integrations. The old public
paths remain as compatibility aliases for now.

### Phase 5: Decide Public Compatibility

For v6, decide which v5 paths remain as aliases and which paths become
intentional breaking changes. Document every break in `docs/v6_0_migration_guide.md`.

The default should be:

- preserve extremely common paths for one major cycle when cheap;
- break misleading backend-specific paths;
- avoid preserving obscure aliases that make the new structure harder to learn.

## Non-Goals

- Do not split into multiple crates until the module boundaries have survived
  inside one crate.
- Do not reorganize by file size alone.
- Do not use `core` as a dumping ground.
- Do not move optional backend code into backend-neutral modules.
- Do not make examples import private implementation modules.

## Success Criteria

- `src/lib.rs` is below 1,000 lines and mostly declarative.
- A new contributor can predict where a module belongs from the ownership map.
- Feature-gated adapters are physically isolated.
- Widgets are not implemented partly in `lib.rs` and partly in `widget_ext`.
- Docs and examples use the v6 grouped paths.
- Existing public compatibility is intentional and documented.
- The release matrix still passes:

```bash
cargo fmt --all -- --check
cargo check --locked --no-default-features --all-targets
cargo test --locked --no-default-features
cargo check --locked --all-features --all-targets
cargo test --locked --all-features -- --list
cargo check --locked --all-features --examples
cargo doc --locked --all-features --no-deps
```
