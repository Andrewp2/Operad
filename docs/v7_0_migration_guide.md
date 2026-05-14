# Operad 7.0 Migration Guide

This document tracks the public API policy for the v7 roadmap. It is intentionally
started before all v7 work is complete so breaking changes, aliases, and
preferred imports are captured as they happen instead of reconstructed at
release time.

## Compatibility Policy

V7 should keep customer-facing churn deliberate:

- Prefer the grouped public modules for new code:
  `operad::core`, `operad::interaction`, `operad::render`, `operad::runtime`,
  `operad::adapters`, `operad::accessibility`, `operad::widgets`,
  `operad::theme`, `operad::diagnostics`, and `operad::prelude`.
- Keep extremely common v6 root exports when they are cheap and clear.
- Keep old flat module aliases for one major cycle when removing them would not
  simplify the API materially.
- Deprecate misleading paths before removing them when possible.
- Break paths immediately only when the old path leaks backend-specific details
  into backend-neutral code, hides feature-gated dependencies, or teaches the
  wrong ownership model.

## Preferred Imports

New application code can start with:

```rust
use operad::prelude::*;
```

Use module imports when a surface needs a narrower contract:

```rust
use operad::core::{UiDocument, UiNode, UiPoint, UiRect, UiSize};
use operad::widgets::{button, checkbox, slider};
```

Renderer, platform, and adapter-specific code should import from the explicit
module that owns the integration:

```rust
use operad::render::{PaintItem, PaintKind, RenderFrameRequest};
use operad::adapters::wgpu;
```

## Baseline From 6.1

The v7 branch starts from the published `6.1.0` WGPU baseline:

- `wgpu` is `29.0.3`.
- `glyphon` is `0.11.0`.
- Native-window and GPU canvas consumers should not need to downgrade WGPU from
  newer downstream renderers just to adopt v7 work.

## Source Layout Changes

The first v7 structure pass moved the retained document implementation out of
`src/lib.rs` into `src/core/document.rs`. The crate root still re-exports these
types, so existing imports such as `operad::UiDocument` and `operad::UiNode`
continue to work.

The inline `widgets` module moved from `src/lib.rs` into `src/widgets/mod.rs`.
The public module remains `operad::widgets`.

## V7 Widget Additions

V7 adds normal widget builders for surfaces that previously required examples
or applications to hand-build low-level document nodes:

- `operad::widgets::collapsing_header`
- `operad::widgets::link`
- `operad::widgets::hyperlink`
- `operad::widgets::selectable_label`
- `operad::widgets::small_button`
- `operad::widgets::icon_button`
- `operad::widgets::image_button`
- `operad::widgets::toggle_button`
- `operad::widgets::reset_button`
- `operad::widgets::singleline_text_input`
- `operad::widgets::multiline_text_input`
- `operad::widgets::text_area`
- `operad::widgets::code_editor`
- `operad::widgets::search_input`
- `operad::widgets::password_input`
- `operad::widgets::tooltip_box`
- `operad::widgets::modal_dialog`
- `operad::widgets::dnd_drag_source`
- `operad::widgets::dnd_drop_zone`
- `operad::widgets::dnd_drag_source_descriptor`
- `operad::widgets::dnd_drop_target_descriptor`
- `operad::widgets::dnd_drag_start_request`

`selectable_label` is a selectable/toggleable label control. It is intentionally
separate from `selectable_text`, which remains a read-only text input variant
for copy and text-selection behavior.

The button convenience builders share `ButtonOptions` with `button`. They are
thin defaults for common cases rather than separate state models.

The text-input convenience builders all use `TextInputState` and
`TextInputOptions`; they configure common defaults without replacing the
existing state/event handling model. `password_input` masks only the rendered
document state. Applications should keep owning the real text state.

The existing tooltip, dialog, and overlay contracts remain available for state,
timing, dismissal, and focus policy. The new surface builders are the preferred
way to create the visual document nodes.

The drag/drop builders use the existing `DragPayload`, `DragOperation`,
`DragSourceDescriptor`, and `DropTargetDescriptor` contracts. Build the visual
surface with the widget helper, then derive the descriptor from the rendered
node when platform drag/drop routing needs bounds.

## Current Public Path Status

| Path | V7 status | Notes |
| --- | --- | --- |
| `operad::UiDocument` | Kept | Common root export. |
| `operad::UiNode` | Kept | Common root export. |
| `operad::UiPoint`, `operad::UiRect`, `operad::UiSize` | Kept | Common geometry exports. |
| `operad::widgets::*` | Kept | Preferred widget module. |
| `operad::prelude::*` | Added | Preferred broad app import. |
| `operad::core::document::*` | Added | Owns retained document primitives. |
| `operad::layout` | Kept for v7 | Common layout helpers remain root-level because existing examples and consumers use them heavily. Prefer `operad::core::layout` once that module path is introduced. |
| `operad::input`, `operad::actions`, `operad::commands`, `operad::navigation`, `operad::overlays`, `operad::transactions` | Kept for v7 | These remain root modules and are also grouped under `operad::interaction`. New docs should prefer the grouped path for interaction concepts. |
| `operad::paint`, `operad::renderer`, `operad::display`, `operad::resource_cache`, `operad::scrolling`, `operad::virtualization` | Kept for v7 | These remain root modules and are also grouped under `operad::render`. New docs should prefer the grouped path for rendering concepts. |
| `operad::host`, `operad::platform`, `operad::runtime`, `operad::windows` | Kept for v7 | Runtime contracts remain available at their v6 paths while `operad::runtime` becomes the preferred grouping. |
| `operad::wgpu_renderer` | Kept for v7 | Feature-gated compatibility module. New WGPU docs should prefer `operad::adapters::wgpu` once that adapter path is added. |
| `operad::egui_host` | Kept for v7 | Feature-gated compatibility module. New egui docs should prefer `operad::adapters::egui` once that adapter path is added. |
| `operad::accesskit_winit_adapter` | Kept for v7 | Feature-gated compatibility module. New accessibility adapter docs should prefer the adapter grouping once it is added. |
| Root type re-exports, for example `operad::PaintItem`, `operad::WidgetAction`, and `operad::RenderFrameRequest` | Kept for v7 | Common type re-exports stay to keep examples short. Module paths should be used when a doc section is about ownership boundaries. |

No v6 public path is deprecated by the Alpha 1 structure pass. V7 can still
deprecate paths later, but each deprecation must update this guide and the
changelog in the same change.

## Current Dependency Status

| Dependency surface | V7 status | Notes |
| --- | --- | --- |
| `wgpu` feature | Kept | Uses WGPU 29 through the v6.1 baseline. |
| `glyphon` through `wgpu` | Kept | Uses Glyphon 0.11, which depends on WGPU 29. |
| `native-window` feature | Kept | Continues to depend on WGPU, winit, and clipboard integration. |
| no-default builds | Required | Must not pull WGPU, winit, glyphon, egui, AccessKit, or clipboard crates. |

## Migration Checklist For V7 Work

When a v7 change moves or renames a public item:

1. Update this guide in the same change.
2. Keep or remove the root export intentionally.
3. Add a short compatibility note to the changelog draft.
4. Update examples to use the preferred path.
5. Run no-default and all-features checks that cover the affected feature gates.

## Not Yet Decided

- Whether old internal planning documents should be packaged in crates.io
  releases or moved under an archive folder before v7.
