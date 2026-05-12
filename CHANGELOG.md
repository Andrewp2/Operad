# Changelog

## 4.0.0

- Added optional `wgpu` rendering support behind the `wgpu` feature.
- Added `WgpuRenderer` and `WgpuSurfaceRenderer` exports under the `wgpu` feature.
- Added GPU snapshot parity test coverage in `tests/wgpu_snapshot_parity.rs`,
  covering CPU parity, texture upload, SDF rounded rectangles, glyphon text, and
  paint order across text and geometry.
- Added WGPU no-readback perf coverage for cached text and mixed changing UI
  scenes, with a release-mode 1 ms p95 render budget.
- Added opt-in GPU render-pass timestamp timing via `RenderOptions::collect_gpu_timing`.
- Added glyphon text chunk caching so changing one text run does not force
  preparing every visible text surface each frame.
- Split the legacy egui painter into `egui-renderer-compat`; the `egui`
  feature now represents host/input/platform compatibility rather than the
  renderer backend path.
- Added initial v4 migration guidance and release checklist.
- Added v4 migration-compat constructors so legacy `taffy::Style` inputs can be passed
  into common node/style constructors (`UiNode::container`, `UiNode::text`, etc.) during
  downstream upgrades.
- Expanded migration compatibility in widget entry points by allowing
  `label`/`scroll_area` to accept legacy layout styles and adding `with_layout(...)`
  to widget option structs (`ButtonOptions`, `CheckboxOptions`, `SliderOptions`,
  `TextInputOptions`, `ComboBoxOptions`).

## 3.0.0

- Breaking layout API updates (LayoutStyle ownership migration) and host/rendering
  modernization for v3 contracts.
