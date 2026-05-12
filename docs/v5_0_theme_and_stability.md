# Operad 5.0 Theme and Design-Token Stability

Operad `5.0.0` treats the backend-neutral theme model as public API for v5
consumers. The stable contract covers token record names, component role/state
resolution, scoped theme inheritance, and accessibility preference adjustments.
It does not guarantee pixel-identical output across renderers or promise that
every runtime path has already been rewired to consume every token.

The machine-readable summary lives in `src/theme_stability.rs`.

## Stability Categories

Operad uses the stability labels from `src/versioning.rs`:

- Stable: semver-protected for the v5 line.
- Backend-specific: public, but behavior or availability depends on renderer or
  feature flags.
- Migration-only: available to help existing consumers move to v5, not the
  preferred surface for new product code.
- Experimental: available for diagnostics or early integration and may change
  before a later v5 minor release.

## Stable Theme Surface

The following records and behaviors are stable in v5:

- `Theme`, `ThemePatch`, `Theme::dark`, `Theme::default`, and the public token
  records they contain.
- `ScopedThemeRegistry`, `ThemeScope`, `ThemeScopeId`, `ThemeScopeKind`, and
  inherited scope resolution.
- Component roles: button, tab, search field, lane header, range item, editor
  lane, property row, menu row, and transport control.
- Component state flags and slots: base, hovered, pressed, focused, selected,
  active, invalid, warning, changed, pending, open, checked, and disabled.
- `Theme::component`, `Theme::resolve_visual`, `Theme::resolve_text`, and
  `Theme::resolve_icon`.
- Accessibility preference adjustment policy for text scale, reduced motion,
  high contrast, forced colors, and reduced transparency.

## Token Categories

These token categories are stable public records:

- Color tokens: canvas, surface, border, divider, text, accent, status,
  selection, focus, overlay, and editor/domain semantic colors.
- Spacing tokens: dense spacing scale plus control, panel, toolbar, row, and
  grid spacing roles.
- Typography tokens: caption, body, label, heading, title, mono, numeric, and
  disabled text roles.
- Radius tokens: corner radius scale from none through pill.
- Stroke tokens: line-width scale and semantic strokes for dividers, surfaces,
  controls, focus, selection, invalid, and warning states.
- Effect tokens: shadows, glows, inset hairlines, and fallback strokes.
- Opacity tokens: overlay, pressed, selected, disabled, muted, scrim, drag
  preview, and focus glow opacity roles.
- Motion tokens: duration scale, curves, tooltip delay, and reduced-motion
  scaling.
- Component tokens: visual, text, icon, and layout records for each stable
  component role.

Stable means consumers can rely on the fields and roles continuing to exist
through compatible v5 releases. It does not freeze exact default color values,
spacing values, or typography metrics forever; compatible changes may tune
defaults when needed for accessibility, consistency, or bug fixes.

## Component States

Component state resolution is stable. Disabled wins first when present. For
non-disabled states, visual resolution prefers invalid, warning, pending,
pressed, focused, active, open, checked, selected, changed, then hovered before
falling back to base. Text and icon state records follow the same public state
slot vocabulary.

Consumers should set explicit component state flags rather than hard-coding
hover, focus, error, or selection colors. That keeps product UI aligned with
high-contrast, forced-colors, and reduced-transparency adjustments.

## Motion and Reduced Motion

Motion tokens are stable API. Reduced motion is a stable policy:

- `AccessibilityPreferences::should_reduce_motion()` causes micro, fast,
  normal, and slow durations to be scaled by `MotionTokens::reduced_motion_scale`.
- Non-positive or non-finite reduced-motion scales collapse animated durations
  to `instant_ms`.
- Standard, emphasized, and exit curves become linear under reduced motion.

New animation code should consume motion tokens and preference-adjusted themes.
Do not create independent duration constants for product-visible interactions
unless the app owns the full accessibility policy for that motion.

## High Contrast and Forced Colors

High contrast and forced colors are stable theme policies. Forced colors imply
the high-contrast path. The policy strengthens borders, uses less-subtle text
roles, makes focus rings opaque, raises overlay scrim opacity, and rebuilds
component tokens from the adjusted color and stroke records.

Reduced transparency is also stable. It removes translucent colors, opacity
roles, effects, typography colors, and component colors where applicable.

Operad owns these token transformations. Platform-specific forced-color palette
mapping is still host/backend integration work, so consumers should treat the
adjusted theme as the v5 contract and perform any OS palette handoff in their
host adapter.

## Backend Caveats

Theme tokens are backend-neutral; rendering is not pixel-identical by contract.

- CPU and WGPU backends may differ in antialiasing, subpixel placement,
  clipping edge behavior, shader/effect approximation, and texture sampling.
- Text metrics and glyph rasterization can vary by text backend and host font
  stack.
- Effect tokens include fallback strokes so backends that cannot render a
  shadow or glow can still preserve affordance and contrast.
- `wgpu` is backend-specific API. It is public when the feature is enabled, but
  availability depends on adapter support and validation environment.
- Existing runtime/widget paths are still being incrementally rewired to consume
  the full token vocabulary. The stable API is the theme contract; not every
  existing widget should be assumed to exercise every token today.

Snapshot tests should compare intended semantics and bounded tolerances rather
than requiring exact pixels across every backend.

## Migration-Only APIs

Migration-only surfaces exist to move existing applications onto v5 without
forcing a renderer rewrite in the same step. The `egui-renderer-compat` feature
is classified as migration-only for theme stability purposes. New product code
should prefer the backend-neutral theme, component state, paint, and renderer
contracts instead of adding new dependencies on compatibility behavior.

Migration-only APIs may change or be removed without a major release once the
v5 migration window closes. Keep them behind local adapters in downstream code.

## Experimental APIs

Debug theme inspection is experimental in v5. `DebugThemeSnapshot` and related
token inspection helpers are useful for audits and diagnostics, but their field
set may expand as runtime wiring and theme coverage improve.

Experimental APIs should not be part of downstream public contracts unless the
downstream crate is prepared to absorb minor-release changes.

## Consumer Guidance

Use stable token categories and component state resolution for product-facing
UI. Keep backend-specific rendering behavior behind renderer adapters. Keep
migration-only compatibility paths local and temporary. Use experimental debug
inspection for tests and audits, not as a product data model.
