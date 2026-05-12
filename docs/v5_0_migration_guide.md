# Operad 5.0 Migration Guide

Operad `5.0.0` establishes the v5 public API, docs, and release-gate foundation.
It does not claim the full interaction-runtime roadmap is complete; see
`docs/v5_0_completion_audit.md` for current done, partial, and gap status.

## Upgrade from `4.0.0` to `5.0.0`

1. Update dependency metadata.

```toml
operad = { git = "https://github.com/Andrewp2/Operad.git", tag = "v5.0.0", features = ["widgets", "wgpu"] }
```

Use a local path dependency while validating unreleased downstream changes.

2. Prefer Operad-owned layout primitives for new public code.

New v5-facing code should use `operad::layout` helpers and public layout facade
types for common construction and inspection. Existing `LayoutStyle` and Taffy
conversion paths remain available for migration and advanced layout cases.

3. Annotate internationalization behavior explicitly where product code depends
   on it.

The v5 public surface includes locale identity, text direction, bidi policy,
layout mirroring, localization policy, and dynamic label metadata. Widgets and
renderers do not yet apply every policy end to end, so downstream applications
should treat these as explicit contracts to wire through product UI state.

4. Use API stability markers when consuming new v5 surfaces.

`operad::versioning` classifies APIs as stable, experimental,
backend-specific, or migration-only. Prefer stable APIs for product-facing code
and quarantine experimental/backend-specific calls behind local adapters.
Theme and design-token stability policy is recorded in
`docs/v5_0_theme_and_stability.md` and mirrored by
`operad::theme_stability`.

5. Keep feature gates intentional.

- `widgets` enables the higher-level widget helpers and widget extension tests.
- `wgpu` enables `WgpuRenderer`, `WgpuSurfaceRenderer`, and GPU validation paths.
- `egui` remains host/input/platform compatibility.
- `egui-renderer-compat` remains the legacy egui painter compatibility path.
- `text-cosmic` enables the optional Cosmic Text measurer.
- `accesskit-winit` enables the optional AccessKit bridge for winit hosts.

6. Move shared host and renderer bookkeeping toward v5 contracts.

Use `RuntimeLoopState`, `WidgetActionQueue`, `RetainedWidgetStateStore`,
`TextEditHistory`, `EffectiveGeometry`, `ResourceCache`, `FontRegistry`, and the
scrolling and compositor policy modules for new integration work. These are
backend-neutral contracts; some existing widget and renderer paths still need
incremental rewiring, so prefer thin local adapters while migrating downstream
code.

Use `PaintCompositorLayer` when a paint subtree needs isolated composition for
rounded clipping, masks, opacity, or basic filters. The WGPU backend uses a real
render-to-texture path for those layers when the `wgpu` feature is enabled, and
rich-rect shadows use the backend's soft falloff path. Snapshot color management
is explicitly sRGB-only in v5; treat `ColorManagementLevel` values above sRGB as
future capability records rather than current WGPU snapshot output.
`PaintPath` now exposes fill-rule and stroke cap/join options backed by shared
lyon tessellation; glyphon text supports grayscale glyphs at fractional
positions, while RGB/LCD component subpixel masks are not a v5 claim. True
backdrop filters are also explicit future capability records rather than
current WGPU snapshot behavior.

7. Move async task and form state through explicit lifecycle records.

`TaskRegistry` and `FormState` provide deterministic state transitions for
progress, cancellation, stale result rejection, validation generations,
apply/submit/cancel/reset phases, and accessible error summaries. Hosts still
own their executor and product validators; Operad owns the UI-side contracts.

8. Treat accessibility adapters as explicit host state.

`HeadlessAccessibilityAdapter` is available for tests and non-platform
integration. Winit hosts can enable `accesskit-winit` to convert
`AccessibilityTree` into AccessKit tree updates, publish full trees/focus
changes through the platform adapter, and queue raw AccessKit action requests
for host handling. Other platform adapters should consume the same
request/report types and can publish canvas/editor target summaries through
`AccessibilityAdapterTargetSummary`.

9. Centralize help, context menu, and diagnostic output.

Use `CommandTooltipResolver`, `TooltipRequest`, `ContextMenuRequest`, and
`HelpOverlayRecord` for tooltip and context menu policy. Use
`DiagnosticReport` when tests, hosts, or devtools need one surface for input
routing, widget actions, overlays, accessibility output, effective geometry,
dirty flags, warnings, errors, and render timing.

Context menus can now be opened through `ContextMenuState::open_from_pointer_event`,
`ContextMenuState::open_from_keyboard`, or `ContextMenuState::open_from_key_event`.
The key path treats the dedicated `KeyCode::ContextMenu` key and `Shift+F10`
as keyboard context-menu invocation. The return value includes both the menu
outcome and the policy resolution so disabled/read-only suppression is visible
to hosts and tests.

Editable text remains `text_input` plus app-owned `TextInputState`. For
read-only copyable text, prefer `selectable_text` and
`handle_selectable_text_event`; these wrap `TextInputOptions` as read-only,
selectable, and copyable while suppressing edits, paste, IME commits, and edit
actions. Use `TextInputOptions::read_only()`, `.selectable(false)`, and
`.allow_copy(false)` when a lower-level text input needs explicit interaction
policy.

## Validation

Run the required gates in `docs/v5_0_release_checklist.md`. Basic CI compiles
all feature combinations and enumerates all-feature tests, but WGPU parity and
perf smoke tests should run only on machines with suitable adapters and stable
timing characteristics.
