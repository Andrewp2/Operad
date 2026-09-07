# Operad architecture

Operad accepts application-authored UI documents, maintains interaction state
across document revisions, and produces renderer-neutral frames. Application
data and editing models belong to the application. Windowing and GPU resources
belong to the host. Document-local node indices must never serve as persistent
widget identities.

## Refactor requirements

- One shared runtime session owns cross-document interaction state, scroll and
  animation retention, and frame history. Native and web hosts use the same
  reconciliation and frame lifecycle.
- Identity is stable across sibling insertion and reordering. Removal cancels
  interactions, and an unrelated replacement cannot inherit them. Explicit
  application state takes precedence over retained state. Ambiguous identities
  must not silently match another control.
- Document descriptions and cached runtime work have explicit lifetimes.
  Unchanged frames reuse work; changes invalidate the affected work. Verify
  reuse through actual runtime execution, including changes to layout, text,
  scale, scrolling, and animation.
- Foundational geometry, invalidation, and timing types do not depend on testing
  or diagnostic implementations. Module ownership and imports express this
  dependency direction directly.
- Diagnostics and the inspector are optional consumers of runtime information.
  Ordinary applications can build and run without them. Keep a focused set of
  observations and reusable reports rather than overlapping parallel systems.

## Verification

Exercise the shared runtime through insert/reorder/remove/reinsert sequences,
pointer press and drag, keyboard focus, text input, canvas capture, authored
overrides, and independent sessions. Test invalidation/reuse at runtime
boundaries. Compile the supported native and WASM feature combinations, test
the affected core/widget/renderer behavior, and run the repository's full gate
before declaring the architecture refactor complete.

## Ownership

| Owner | Responsibility |
| --- | --- |
| Application | Domain data, text editing models, commands, and the view description |
| `core::document` | Authored nodes, computed geometry, and document-local input |
| `core::identity` | Scoped node identity shared by runtime retention and layout animation |
| `runtime::session` | Document lifetime, interaction reconciliation, frame history, and pending cleanup requests |
| Native/web host | Platform events, clocks, application callbacks, platform services, and presentation |
| `render` and `adapters` | Paint data, resource contracts, and concrete GPU rendering |
| Diagnostics/inspector | Optional observers and views of runtime data |

The category modules declare their implementations. The crate root provides
selected convenience imports; categories do not import their implementations
back from root declarations. `DirtyFlags` and frame timing types live in `core`
and do not require test-support code.

Application-managed helpers in `core::state` do not control the runtime session.
Applications can use them for their own editing models; focus, pointer ownership,
scroll retention, and animation lifetime are the session's responsibility.

## Identity and lifetime

Identity consists of node-name segments from root to node. Names containing a
slash are a single segment. Sibling names must be unique. Duplicate paths and
descendants of ambiguous parents do not inherit runtime state. Reparenting or
renaming a node creates a new identity. Each session is an independent scope.

On a new document, the session maps focus, pointer gestures and click history,
drag capture, text input, canvas capture, and prior frame history to the new
indices. Removed targets lose ownership; disabled controls also lose focus and
pointer ownership. Removing a captured canvas or active IME target produces
platform cleanup requests. Live
regions retain their identity so unrelated insertions do not reannounce them.

Explicit application focus requests, including an explicit blur, apply once to
each newly authored document. Application-authored scroll offsets and animation
inputs take precedence over retained values. A changed animation definition
starts its own runtime state.

## Reuse and invalidation

The session retains the authored document between frames. Application updates,
mutable hooks, and viewport changes invalidate the view. Custom hosts call
`invalidate_view` after changing the data used by their view or their text
measurement environment. Input and animation alone do not rebuild the view.
Scale changes and text styles that affect layout invalidate measured geometry.

The frame pipeline still produces current paint and accessibility output. It
reuses document construction and cached layout when those inputs are unchanged.
Runtime-added tooltips are frame decorations and are removed before retaining
the authored document. Resource uploads remain available after a failed render
and are consumed only when the host acknowledges successful presentation.

Hosts use `RuntimeSessionOptions` to select accessibility capabilities, rendering
settings, and layout animation. Rendering's accessibility preferences also
govern host output and reduced-motion behavior, avoiding two competing settings.

## Optional tooling

Ordinary native and web applications do not enable diagnostics. `diagnostics`
adds snapshots and reports; `inspector` adds their UI; `test-support` adds replay
and assertion harnesses. Production errors and limits remain available without
these features.

Inspector reports keep their specialized data and row builders. Panels sharing
the same layout contract use the common types in
`widgets::ext::diagnostic_panel`: eleven shared contracts replace 156 equivalent
panel-specific option and node types, preserving their fields and defaults.

## Regression evidence

- `src/runtime/session/tests.rs` exercises lifetime, state ownership, input,
  accessibility, cached layout, uploads, and frame-owned decorations.
- `src/render/layout_animation.rs` tests scoped identity and rejects ambiguous
  animation origins.
- Existing runtime, widget, inspector, layout, and GPU snapshot tests exercise
  the migrated implementations. Public-module tests compile the supported type
  paths instead of parsing the spelling of module declarations.
- `scripts/test-full.sh` checks minimal/all-feature builds, the full test suite,
  and the WASM showcase. Re-run it after changes to these boundaries.
