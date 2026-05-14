# Operad 7.0 Release Checklist

Goal: ship `7.0.0` as the release that makes Operad easier to adopt for normal
application screens: cleaner modules, a documented prelude, normalized widgets,
centralized overlay ordering, and useful performance instrumentation.

## Alpha 1 Structure Gate

- [x] `src/lib.rs` is below 1,000 lines and contains module declarations plus
  public re-exports.
- [x] Retained document primitives live in `src/core/document.rs`.
- [x] Widget module wiring lives in `src/widgets/mod.rs`.
- [x] `operad::prelude` exists for normal application imports.
- [x] The v7 migration guide records the current public path policy.
- [x] The v7 branch carries the published `6.1.0` WGPU 29/Glyphon 0.11 baseline.
- [x] Widget module tests have been moved out of `core::document` and into
  `src/widgets/tests.rs`.
- [x] Compatibility aliases have an explicit keep, deprecate, or remove decision.

## Required CI Gates

These should pass on a normal Linux runner before every v7 milestone:

- [x] Format: `cargo fmt --all -- --check`
- [x] No-default compile: `cargo check --locked --no-default-features --all-targets`
- [x] No-default lib tests: `cargo test --locked --no-default-features --lib`
- [x] All-features compile: `cargo check --locked --all-features --all-targets`
- [x] All-features test enumeration: `cargo test --locked --all-features -- --list`
- [x] Example compile: `cargo check --locked --all-features --examples`
- [x] Docs: `cargo doc --locked --all-features --no-deps`
- [x] Package verification dry run: `cargo package --locked`

## Widget And Showcase Gates

- [x] Alpha 2 starts widget normalization with public builders for text-style
  labels, standalone images, separators/spacers, spinners, radio controls,
  toggle switches, visual drag values, generic grids, and panel containers.
- [x] Alpha 5 adds backend-neutral link, hyperlink, and selectable-label
  builders so egui-style text controls no longer require manual node assembly.
- [x] Alpha 6 adds text-input convenience builders for common editor, search,
  password, single-line, multiline, and text-area configurations.
- [x] Alpha 8 adds small, icon, image, toggle, and reset button convenience
  builders.
- [x] Alpha 9 adds form section, row, field label, help text, validation
  message, and error-summary widget helpers.
- [x] Alpha 10 adds central, top, bottom, side, left, right, group, frame,
  sides, columns, indented-section, resize-handle, and resize-container widget
  helpers.
- [x] Alpha 11 adds compact color button, swatch button, color display, and
  RGB/RGBA/SRGB/SRGBA/HSVA/OKLCH color-edit button helpers.
- [x] Alpha 12 adds menu-button state, trigger builders, image menu buttons,
  image-and-text menu buttons, submenu item helpers, and anchored submenu popup
  composition.
- [x] Alpha 13 adds selectable-value and angle-drag helper builders over the
  existing selectable-label and drag-value primitives.
- [x] Alpha 14 adds premultiplied/unmultiplied RGBA and SRGBA color buttons,
  `Color32`-style color buttons, and standalone HSV 2D picker fields.
- [x] Alpha 15 adds `area` and `scene` widget helpers for absolute regions and
  renderer-neutral scene primitives.
- [x] Alpha 16 adds widget-level visible/enabled wrappers, fixed/minimum
  allocation helpers, painter allocation, and programmatic scroll helpers.
- [x] Alpha 17 adds global theme preference buttons and a dark-theme switch.
- [x] Alpha 18 adds form action buttons and field traversal helpers for
  submit/apply/cancel/reset workflows.
- [x] Alpha 19 adds tooltip hover/focus trigger resolution and
  reduced-motion-aware animation presets.
- [ ] Showcase code uses public Operad widget APIs only.
- [ ] Showcase contains no test harness, screenshot, stress, or hidden diagnostic
  code.
- [ ] Every shipped widget has normal, hovered, pressed, toggled, disabled,
  focused, selected, min-size, max-size, and overlay-open visual coverage where
  applicable.
- [ ] Widget defaults cover margins, alignment, hover, pressed, disabled, focus,
  keyboard, clipboard, scrolling, and accessibility behavior without per-example
  patching.
- [ ] Text input caret placement, selection, clipboard, deletion, placeholder
  sizing, keyboard navigation, and IME behavior are validated.
- [ ] Scrollbars can be clicked, dragged, and kept visually aligned with the
  content they control.

## Overlay And Surface Gates

- [x] Alpha 4 adds reusable collapsing-header, tooltip-box, and modal-dialog
  widget builders backed by shared accessibility and overlay contracts.
- [x] Alpha 7 adds reusable drag source and drop zone widget builders backed by
  the shared drag/drop descriptors and platform drag-start contract.
- [ ] Paint ordering, hit testing, focus, popups, drag capture, and accessibility
  traversal use one effective ordering model.
- [ ] Floating windows, popups, menus, tooltips, command palettes, modals, and
  toast overlays share reusable layering rules.
- [x] Floating windows support close, collapse, resize, minimum size, drag,
  focus, and keyboard behavior through library primitives.
- [ ] Combo boxes, submenus, tooltips, and popups do not cause layout shifts
  unless the caller requests inline layout.

## Performance Gates

- [x] Alpha 3 starts performance diagnostics with named frame-pipeline stages
  and cache hit/miss/eviction snapshots.
- [ ] Frame diagnostics report tree rebuild, diffing, layout, text shaping, hit
  testing, paint-list generation, batching, uploads, and backend draw time.
- [ ] Cache diagnostics report layout, shaped text, image, canvas texture, and
  display-list reuse.
- [ ] Stress probes live outside customer-facing examples.
- [ ] Showcase performance has a documented budget and a repeatable measurement
  command.

## Release Sign-Off

- [ ] `Cargo.toml` and `Cargo.lock` both report the intended v7 release version.
- [ ] `CHANGELOG.md` has a `7.0.0` entry covering breaking changes and migration
  paths.
- [ ] README describes Operad as the current library, not as version archaeology.
- [ ] Feature flags are documented and no-default builds stay backend-neutral.
- [ ] Migration guide documents every intentionally changed public path.
- [ ] Package contents do not include stale internal planning docs as current
  product guidance.
