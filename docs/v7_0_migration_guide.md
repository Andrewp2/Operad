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

## Current Public Path Status

| Path | V7 status | Notes |
| --- | --- | --- |
| `operad::UiDocument` | Kept | Common root export. |
| `operad::UiNode` | Kept | Common root export. |
| `operad::UiPoint`, `operad::UiRect`, `operad::UiSize` | Kept | Common geometry exports. |
| `operad::widgets::*` | Kept | Preferred widget module. |
| `operad::prelude::*` | Added | Preferred broad app import. |
| `operad::core::document::*` | Added | Owns retained document primitives. |
| `operad::layout`, `operad::renderer`, `operad::input` | Kept for now | Existing flat module aliases remain while v7 compatibility is audited. |
| Backend adapter aliases | Kept for now | WGPU, egui, and AccessKit adapter aliases remain feature-gated. |

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

- Which v5/v6 flat module aliases should be formally deprecated in v7.
- Whether compatibility aliases should emit deprecation warnings before v8.
- Whether the egui renderer compatibility helpers stay at the root, move under
  `operad::adapters::egui`, or remain as root re-exports from the adapter path.
- Whether old internal planning documents should be packaged in crates.io
  releases or moved under an archive folder before v7.
