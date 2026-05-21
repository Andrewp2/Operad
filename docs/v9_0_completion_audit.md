# Operad 9.0 Completion Audit

This audit maps the v9 release candidate to repository artifacts. It is the
release-readiness checklist for the v9 branch.

Status key:

- `Done`: reusable API, documentation, and regression coverage exist for the
  roadmap item.
- `Partial`: reusable pieces exist, but the full behavior still needs more
  product integration or examples.
- `Gap`: no complete Operad-owned artifact exists yet.

## Roadmap Themes

| Theme | Status | Current artifact | Remaining gap |
| --- | --- | --- | --- |
| Shader lab and element materials | Done | Element material records, shader preset declarations, WGSL-backed canvas/frame/button previews, shader fallback reporting, media examples, and the Shader Lab showcase demonstrate shader effects, clip/hit shapes, paint outsets, and declared geometry effects. | Real geometry deformation remains a renderer-backed material contract rather than layout mutation. |
| Popup and overlay stacking | Done | Popup, dropdown, menu, tooltip, modal, toast, and anchored overlay surfaces share portal ownership and nearest stacking-context behavior so child popups remain above later siblings inside the owning window. | Keep adding focused regressions for newly found anchored-placement cases. |
| Scroll containers | Done | Scroll areas reserve scrollbar gutters, hide unnecessary scrollbars, expand on hover/active state, support draggable thumbs, route ctrl/shift wheel to horizontal scroll, and have synthetic end-scroll coverage. | Native touch and inertial scroll tuning remains host-specific. |
| Text input and selection | Done | Editable, read-only, multiline, code, password, and scrollable text inputs share selection ownership, caret geometry, focus/blur behavior, newline handling, and web/native shaping paths. | IME composition details still need backend-specific expansion. |
| Image and media resources | Done | Built-in icons, user-supplied image handles, decoded PNG/JPEG/BMP resources, missing-image rendering, image-backed checkbox marks, tint variants, and media grids are covered by public widgets and showcase examples. | Advanced animated image/video resources remain future work. |
| Widget option coverage | Done | Checkboxes support checked, unchecked, disabled, indeterminate, larger hit targets, custom check colors, and image marks; radio/toggle widgets expose label/no-label, custom colors, shape/length/thumb options; numeric inputs support drag/edit, unit ranges, and adjustable sensitivity. | Keep examples concise as option sets grow. |
| Layout and window sizing | Done | Intrinsic sizing, flex/grid composition, window minimums, organized window packing/minimization, no-clipping checks, and showcase layout tests cover the common shrink/overflow regressions. | Continue moving demo-specific layout assumptions into primitives. |
| Theming and styling | Done | App-wide theme switching, dark/light/bubblegum presets, color buttons, stroke/fill controls, corner radius controls, shadow controls, and styling showcase examples exercise design tokens and component states. | More downstream theme snapshots can be added as apps adopt v9. |
| Data, tree, panel, and timeline demos | Done | Editable trees, virtualized trees/tables, sortable/filterable data tables, draggable panel splits, timeline ruler and track scrolling, and focused layout tests cover dense UI surfaces. | Product-specific row and track models remain app-owned. |
| Forms, date picking, and progress | Done | Forms expose submit/apply/reset semantics, validation, date range picking, loading progress with visible logs, and scroll behavior that respects user position. | Backend persistence and domain validation remain app-owned. |
| Web showcase and release infrastructure | Done | The GitHub Pages showcase opens at `/` and `/showcase/`, favicon packaging is included, browser smoke checks run against the web artifact, and release process docs apply to all future versions. | Publishing still waits for pushed CI to pass. |

## Release Gates

| Release gate | Status | Current artifact | Remaining gap |
| --- | --- | --- | --- |
| Showcase examples for each major feature, using public APIs only | Done | Showcase covers Shader Lab, media, forms, overlays, scrolling, panels, timeline, animation/easing, trees, data tables, text input, numeric input, styling, themes, and canvas. | Keep future showcase regressions covered by external tests. |
| Browser showcase runs as WASM/WebGPU | Done | The Pages workflow builds `showcase_web`, assembles `/` and `/showcase/` artifacts, serves the artifact locally, and runs `scripts/check-web-showcase.mjs` against both routes in headless Chrome with WebGPU enabled. | Keep the smoke script focused on browser-only failures that cargo checks cannot see. |
| Replay/state-matrix tests for interactive behavior | Done | `tests/showcase_layout.rs`, widget state matrix tests, synthetic scroll tests, topmost input-consumption checks, popup stacking tests, shader lab checks, and scenario harness helpers cover recent regressions. | Continue adding focused replay cases for every bug found in showcase. |
| Layout/a11y/performance diagnostics for new surfaces | Done | Debug inspector, accessibility overlay, virtualization diagnostics, dominant frame-timing summaries, performance snapshots, and perf smoke tests cover the v9 diagnostic surfaces. | Native-host perf evidence remains opt-in because CI may be headless. |
| No tests embedded in showcase source | Done | Showcase regression tests live in `tests/showcase_layout.rs`; `examples/showcase.rs` remains a teaching example. | Keep new showcase regressions out of the example file. |
| No app-specific domain semantics in public APIs | Done | New v9 APIs use command, layout, animation, canvas, theme, material, media, data, docking, and diagnostic terms rather than product-specific domains. | Audit new examples when adding richer shader/table/panel demos. |
| Package docs distinguish stable APIs from diagnostics | Done | `docs/v9_0_api_stability.md` classifies stable product APIs, experimental diagnostics/devtools, testing/replay helpers, backend-specific APIs, and migration-only surfaces. | Keep the guide current as partial roadmap areas become stable. |

## Next Work

The v9 release gates are covered. Remaining work should be treated as release
polish: run the full suite, audit the public API naming surface, publish only
after pushed CI passes, and keep any new showcase regressions in external
tests.
