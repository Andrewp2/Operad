# Operad 4.0 Migration Guide

Operad `4.0.0` keeps the v3 document/widget contracts and introduces the first
explicit renderer expansion: optional `wgpu` support.

If you consume `operad` from v3 today, keep your existing UI contracts and add
the new feature only where needed.

## Upgrade from `3.0.0` to `4.0.0`

1. Update dependency metadata.

```toml
operad = { git = "https://github.com/Andrewp2/Operad.git", tag = "v4.0.0", features = ["widgets", "wgpu"] }
```

Use the local workspace path during active development as needed.

2. Keep all existing widget and layout calls; no required API removals are part of
   this release.
3. During migration, legacy style construction can use:
   - `LayoutStyle::from_taffy_style(taffy_style)` when migrating call sites, or
   - direct `taffy::Style` arguments in `UiNode::container(...)`, `text`, `canvas`,
     `image`, and `scene` constructors.
   - `UiDocument::new(taffy_style)` and `set_node_style(id, taffy_style)`.
4. Widget-option compatibility for legacy style literals:
   - `widgets::label(..., style)` and `widgets::scroll_area(..., style)` accept
     legacy `taffy::Style` directly via `Into<LayoutStyle>`.
   - Widget option structs (`ButtonOptions`, `CheckboxOptions`, `SliderOptions`,
     `TextInputOptions`, `ComboBoxOptions`) support `.with_layout(taffy_style)` to
     convert legacy layout while keeping existing field-based setup.

5. Choose a renderer path:
- For GPU rendering, construct a `WgpuRenderer` for snapshot/offscreen frames or a
  `WgpuSurfaceRenderer` for window/app-owned texture targets.
- If you pass the renderer back into `ScenarioHarness`/frame helpers, use the
  `run_frame_with_measurer_and_renderer` variant.

6. Choose egui compatibility intentionally:
- `egui` enables host/input/platform compatibility for existing egui-hosted apps.
- `egui-renderer-compat` enables the legacy egui `PaintList` painter. New renderer
  validation should use `wgpu`.

## New behavior to verify

- Enable `wgpu` tests in CI for backend conformance checks.
- Confirm WGPU snapshot output directly where deterministic coverage matters.
- Confirm target feature sets (especially `pollster` and `wgpu`) are added only where
  the crate actually compiles GPU paths.

## Feature note

`wgpu` is feature-gated in `operad`:
- `features = ["wgpu"]` enables the new module and exported renderer types.
- Without this feature, backend-neutral document, layout, and paint behavior is
  unchanged.
- `features = ["text-cosmic"]` enables the `CosmicTextMeasurer`.
- `features = ["egui-renderer-compat"]` is only for migrations that still need
  the legacy egui painter.

## Rollout recommendation

- Land `4.0.0` in downstream projects one at a time. Treat downstream probes as
  application-owned adoption work rather than an Operad crate release blocker.
