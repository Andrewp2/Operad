# Operad 3.0 Migration Guide

Operad `3.0.0` is a breaking release. The upgrade should be treated as a
consumer migration, not a drop-in `2.x` bump.

## Dependency

For git consumers, depend on the `v3.0.0` branch or a pinned commit from that
branch:

```toml
operad = { git = "ssh://git@github.com/Andrewp2/Operad.git", branch = "v3.0.0", features = ["widgets"] }
```

Enable only the adapters you use:

- `widgets`: shared widget builders and widget state contracts.
- `egui`: egui input/output/texture host adapter.
- `text-cosmic`: real text shaping and measurement through `cosmic-text`.
- `audit`: explicit audit feature hook for consumers that gate audit paths.

The default feature set remains intentionally light.

## Required Layout Changes

The largest v3 break is the layout boundary. Public node and widget APIs no
longer store raw `taffy::Style`.

Before:

```rust
use operad::{length, UiNodeStyle};
use taffy::prelude::{Dimension, Size as TaffySize, Style};

let style = UiNodeStyle {
    layout: Style {
        size: TaffySize {
            width: length(240.0),
            height: Dimension::percent(1.0),
        },
        ..Default::default()
    },
    ..Default::default()
};
```

After:

```rust
use operad::{layout, UiNodeStyle};

let style = UiNodeStyle {
    layout: layout::size(layout::px(240.0), layout::percent(1.0)),
    ..Default::default()
};
```

The following APIs now take or store `LayoutStyle`:

- `UiNodeStyle::layout`
- `UiNode::text`, `UiNode::canvas`, `UiNode::image`, and `UiNode::scene`
- Core widget option structs such as `ButtonOptions`, `CheckboxOptions`,
  `SliderOptions`, `TextInputOptions`, and `ComboBoxOptions`
- Extended widget option structs in `widget_ext::menu`, `widget_ext::data`, and
  `widget_ext::surfaces`

Use `operad::layout::*` for common cases:

```rust
layout::fixed(120.0, 32.0)
layout::fill()
layout::row()
layout::column()
layout::centered_row()
layout::absolute(16.0, 24.0, 240.0, 80.0)
layout::with_gap_all(layout::column(), 8.0)
layout::with_padding_all(layout::fixed(320.0, 200.0), 12.0)
layout::with_margin_left(layout::fixed(240.0, 80.0), 16.0)
```

Direct mutation of Taffy fields is no longer public API. Replace it with
assigning a new `LayoutStyle`, or add a reusable helper to Operad when the shape
is broadly useful.

Before:

```rust
document.node_mut(panel).style.layout.size.width = length(320.0);
```

After:

```rust
document.node_mut(panel).style.layout = layout::fixed(320.0, 48.0);
document.invalidate_layout();
```

## Widget Migration Notes

Widget builders remain renderer-neutral, but option structs now use
`LayoutStyle` for layout fields:

```rust
button(
    &mut document,
    parent,
    "save",
    "Save",
    ButtonOptions {
        layout: layout::fixed(96.0, 32.0),
        ..Default::default()
    },
);
```

Text input is now a full state contract rather than only a rendered field. When
integrating host input, route clipboard and IME effects through the platform
request/output helpers instead of mutating application text directly from a
backend callback.

Searchable selects, command palettes, menu lists, dialogs, popovers, data
tables, tree views, tabs, progress indicators, and timeline/ruler helpers all
emit neutral state, IDs, commands, or accessibility metadata. Keep application
semantics in the consumer crate.

## Host And Renderer Migration Notes

Use the host-frame layer when a consumer needs consistent hover, focus, pressed,
wheel target, drag capture, text/IME, shortcut, platform-service, render, and
accessibility state in one frame:

- `HostDocumentFrameRequest`
- `HostDocumentFrameState`
- `process_document_frame`
- `HostDocumentFrameOutput`

Use renderer registries for app-owned image and canvas drawing:

- `ImageRenderRegistry`
- `CanvasRenderRegistry`
- `CanvasRenderOutput`
- `CanvasHitTarget`
- `CanvasHostCaptureState`

The egui adapter is feature-gated behind `egui`. It should translate backend
events and platform output, but consumer UI models should depend on Operad raw
input, host, renderer, and platform contracts instead of egui types.

## Accessibility And Audit Changes

Accessibility metadata is stricter in v3. Existing UI may trigger new audit
warnings for missing labels, actions, invalid value ranges, missing relation
targets, low contrast text, too-small interactive nodes, clipped text, or
focusable nodes missing from traversal.

For interactive controls, prefer setting:

- `AccessibilityRole`
- label or labelled-by relationship
- focusability
- actions with stable IDs and labels
- selected/checked/pressed/expanded/value state where relevant
- value ranges for sliders, scrollbars, meters, and numeric controls

Canvas/editor surfaces can expose app-owned hit targets and screen-reader
summaries without Operad owning the domain model.

## Testing Migration

Consumers should move UI regression coverage toward the public v3 testing
surface:

- `CpuSnapshotRenderer` and `SnapshotAssertions` for deterministic screenshots.
- `ScenarioHarness` and `EventReplay` for E2E input/render frames.
- `LayoutAssertions`, `PaintAssertions`, `RenderAssertions`, and
  `AccessibilityAssertions` for stable-name checks.
- `FrameTimingSeriesAssertions` and `PerformanceAssertions` for budget checks.

The repo's own quality gate for v3 is:

```sh
cargo test --all-features
cargo test --no-default-features
cargo check --examples --all-features
cargo clippy --all-features --all-targets -- -D warnings
cargo fmt -- --check
git diff --check
```

## Migration Checklist

1. Update the dependency to `v3.0.0`.
2. Remove direct `taffy::Style` construction from app UI code.
3. Replace layout literals with `operad::layout::*` helpers.
4. Update widget option `layout` fields to pass `LayoutStyle`.
5. Replace direct layout-field mutation with reassignment plus layout
   invalidation.
6. Route text input clipboard/IME through Operad platform requests.
7. Run the v3 quality gate in Operad and each downstream consumer.
8. Fix new audit warnings or explicitly document why a warning is acceptable.
