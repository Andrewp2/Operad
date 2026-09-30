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
| `runtime::session` | Document lifetime, interaction reconciliation, frame history, repaint scheduling, and pending cleanup requests |
| `runtime::repaint` | Monotonic repaint deadlines and independent continuous-rendering policy |
| `runtime::integration` | Portable application hooks, ordered input interception, canvas ownership, and borrowed frame observations |
| Native/web host | Platform event translation, clocks, services, window/canvas configuration, and presentation |
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

The document caches an immutable identity index shared by layout, the session,
and canvas hooks. Adding or removing nodes, appending sections, or taking an
unrestricted mutable node reference invalidates that cache. Setters limited to
style or content preserve it. Existing consumers retain their earlier snapshot,
so a tree mutation cannot rewrite the identities needed to reconcile ownership.

On a new document, the session maps focus, pointer gestures and click history,
drag capture, text input, canvas capture, and prior frame history to the new
indices. Removed targets lose ownership; disabled controls also lose focus and
pointer ownership. Removing a captured canvas or active IME target produces
platform cleanup requests. Live
regions retain their identity so unrelated insertions do not reannounce them.
Runtime-owned text composition also follows the field's action binding. Rebinding
the field cancels the original owner and revokes its session before queued input,
even when the node's name and keyboard focus remain unchanged.

Explicit application focus requests, including an explicit blur, apply once to
each newly authored document. Application-authored scroll offsets and animation
inputs take precedence over retained values. A changed animation definition
starts its own runtime state.

## Reuse and invalidation

The session retains the authored document between frames. Application updates,
mutable hooks, and viewport changes invalidate the view. Frame preparation and
platform hooks can return `RuntimeHookResult::unchanged` when they change no
value read by the view. Plain outputs still invalidate conservatively. The
shared session applies this policy on native, web, and application-owned hosts;
an unchanged hook never clears another source's invalidation. Custom hosts call
`invalidate_view` after changing the data used by their view and `refresh_view`
after changing an external text measurement dependency. Input and animation
alone do not rebuild the view.
Scale changes and text styles that affect layout invalidate measured geometry.

The frame pipeline still produces current paint and accessibility output. It
reuses document construction and cached layout when those inputs are unchanged.
Runtime-added tooltips are frame decorations and are removed before retaining
the authored document. Resource uploads remain available after a failed render
and are consumed only when the host acknowledges successful presentation.
Upload payloads use shared immutable `Arc<[u8]>` storage, so document and frame
copies retain the same pixels across retries. Successful presentation releases
the document's pending uploads; any caller-held requests retain their own shared
ownership. WGPU borrows RGBA upload bytes directly and converts BGRA/alpha data
only when preparing the GPU write.
The surface renderer acquires its presentation texture before applying uploads
or preparing GPU geometry. A temporary acquisition failure leaves resource
contents unchanged, so retries can safely include a partial update followed by
a full replacement with a different size or format.

`UiDocument::set_resource` authors a complete resource snapshot. During session
preparation, a linear pass releases earlier pending snapshots and writes that
the new snapshot supersedes. Patches following the snapshot retain their order.
This keeps a changing image's pending pixels bounded by its latest snapshot
and subsequent patches during retries. Snapshot intent survives cached-section
rebuilding and merging; presentation clears the remaining uploads and metadata.
Intervening writes through different handles sharing a textual key prevent
compaction across that boundary, since some renderers alias those handles.
`add_resource_update` records ordered writes. Use it when earlier writes must
remain available for renderer validation or version handling.

The WGPU adapter packs rectangles, rounded rectangles with aligned borders,
circles, round-ended segments, and shadows into one instance stream. The GPU
computes shape distances, pixel coverage, multi-stop linear gradients, and
Gaussian shadows; geometry size does not grow with corner radius or display
scale. Consecutive instances with equal scissors and fragment programs share a draw
while retaining paint order. Fills, borders, gradients, segments, and shadows use
separate programs so software GPUs avoid unrelated fragment work. Border geometry
is derived in the vertex stage; two-stop gradients fetch their colors there. Arbitrary paths and polygon fills still use CPU tessellation; text
uses glyphon, and images use textured instances.

Gradient stops live in a separate storage buffer. Instance and gradient buffers
reuse capacity within device limits. Caller-owned encoders receive staging
copies immediately before their draws, preserving data across deferred frames
and nested layers. sRGB targets linearize fill and border colors before blending.
Shadow blur radius is a visual extent of three Gaussian standard deviations;
rounded shadows use an analytic rectangle integral along straight edges and
interiors outside every corner's four-sigma neighborhood. Affected corners keep
eight-sample GPU quadrature and independent radii, without raster shadow textures.
Primitive and compositor shaders share the corner-distance calculation.

The WGPU adapter keeps app textures and persistent canvas buffers separate from
temporary compositor targets. Temporary targets use private frame indices and
are released when the next frame begins; app resource names never determine
their lifetime. Canvas shader pipelines retain up to 128 recently used variants,
with evicted variants recreated on demand, including for direct canvas draws
outside the UI frame loop.

The view callback receives a session-owned `ViewContext`. Named sections declare
comparable inputs and build isolated authored documents. Equal inputs at the same
section call site reuse the section. Distinct call sites rebuild even when Rust's
diagnostic names for their closure types coincide. This affects construction
reuse; runtime node identity still follows the named path. Rebuilding a parent
can reuse unchanged nested sections. Removing a
section evicts its cache. Section roots join the normal node-name identity path;
internal node references are remapped when appended. Named portals stay local,
while app-overlay hosts retain viewport scope. Reused focus and scroll overrides
do not replay; animation inputs retain their runtime values until the section is
authored again.

Owned portals (`AppOverlay` and `Named`) retain a source-owner reference separately
from their physical layout parent and stacking parent. Input propagation, modal
membership, accessibility ancestry, and stable identity follow that logical
parent. Visibility and enabled state also honor the source; clipping continues
to come from the layout host. Global portals use their host for ownership.
Section snapshots remap portal owners, and layout snapshots carry them so layout
animation and runtime state agree about identity and owner replacement.

Layout retains a Taffy tree across document revisions and reconciles nodes by
the same unambiguous identities as interaction state. It updates styles, text
contexts, and child lists only when changed. Surviving text contexts remap their
document-local IDs without discarding valid measurements. Removed nodes and
their measurement contexts are released. Rebuilds with the same names and
logical parents reuse the immutable identity index. Paint-only rebuilds also
retain computed geometry when tree structure, layout styles, text, clipping,
opacity, and scale match. Unchanged sizing can also survive a rebuild that
needs fresh scroll or portal positioning. Layout constraints use normal
reconciliation. Viewport changes and explicit measurement
refreshes still invalidate geometry. Paint and accessibility still run for
the complete frame; section reuse copies authored nodes and does not cache GPU
paint output. See `docs/runtime-integration.md` for dependency and nesting rules.

Hosts use `RuntimeSessionOptions` to select accessibility capabilities, rendering
settings, and layout animation. Rendering's accessibility preferences also
govern host output and reduced-motion behavior, avoiding two competing settings.

## Frame scheduling

Native, web, and headless hosts share `RuntimeRepaintScheduler`. Delayed requests
coalesce to the earliest absolute deadline. An earlier input frame does not
consume a future deadline, and disabling continuous rendering does not cancel
one-shot requests. Timer wakeups remain separate from repaint deadlines so
canceling a timer removes only that timer's work.

A host calls `RuntimeSession::begin_frame` once, before its document passes.
This consumes work due at frame start. Requests produced by callbacks or
presentation remain pending for the next frame. Successful presentation is
acknowledged with `frame_presented`; `frame_failed` preserves uploads and
requests another attempt. Active document animations also require another frame.

The host merges `next_frame_delay` with its application tick deadline. Native
uses `Wait`/`WaitUntil` and browser uses one animation callback or timeout; no
callback is scheduled while idle. Input, canvas/window resizing, DPI changes,
and asynchronous responses wake the platform adapter. The browser adapter owns
its callback, timeout, resize-observer, and resolution-query handles; embedded
canvas geometry comes from the canvas instead of the enclosing browser window.
Frame hooks do not themselves create a recurring
clock. Application updates drain platform requests before the rebuilt view,
and asynchronous responses apply before view construction.
The browser frame entry point samples its own runtime-relative clock for
metrics, application ticks, and animation deltas, including synchronous input
frames before composition. Callers cannot supply a page-relative timestamp.

WebGPU device loss and permanent frame errors are terminal for a browser
session. This applies to both scheduled frames and synchronous rendering before
text composition. The host cancels scheduled
frames and timers, disconnects geometry observers, stops ordinary input and
asynchronous delivery, and releases pointer capture, pointer lock, and native
text input. Task receivers close so producers can stop cooperatively. The
close-request hook remains available to protect unsaved changes.
The host logs the failure once and updates the configured status element,
recreating it if the startup page removed it. `without_status` disables the DOM
message. Restarting rendering requires reloading the page; the runtime does not
automatically reconstruct application-owned GPU resources.

Native device loss wakes the event loop even when the application is idle and
returns a structured `DeviceLost` failure from the runner. Callers can inspect
the `ErrorReport` through the returned error's `source()`. It does not retry a
destroyed device. The surface renderer reconfigures an outdated surface even
when its size has not changed, and refreshes a suboptimal surface after its
current texture is released. Temporary acquisition failures use
`RenderError::SurfaceUnavailable`, which the native host can retry while
retaining pending uploads. The browser likewise retries only this temporary
error. Other backend failures terminate the native runner or browser session;
a lost surface requires recreation rather than repeatedly acquiring from it.
Failed presentation uses a shared retry interval: 16 ms initially, doubling to
a 250 ms cap, measured from completion of the failed attempt. Successful
presentation resets it. Native and browser hosts honor the deadline for input,
animation, tick, and idle wakeups. Synchronous browser composition-key handling
can prepare current surrounding text during the wait and defer presentation,
preserving pending uploads without extending the deadline or replaying input.

`invalidate_view` and repaint scheduling are intentionally distinct: a callback
may make the description stale for its next use without requiring a new frame.
Custom hosts must wake their platform loop when queuing input or repaint work.

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
- `scripts/check-web-showcase.mjs` verifies WebGPU completion and nonblank canvas
  pixels before exercising showcase input. On Linux, run it under `xvfb-run -a`
  or with an existing X display; Chrome remains headless and uses software Vulkan.
  `OPERAD_WEB_SHOWCASE_UAT=1` includes checkbox wheel, text, drag, and scrolling
  workflows. `OPERAD_WEB_CHECKBOX_PROBE=1` runs only the checkbox workflow: wheel
  input and middle/right clicks preserve values and open windows, while primary
  clicks still toggle them.
- `runtime::repaint` tests deadline persistence, in-frame requests, failed
  presentation, continuous-mode cancellation, and equivalent coalescing across
  4,802 request sequences and initial states.
- `runtime::tasks` tests UI-thread delivery, queue limits, shutdown, startup
  wakeups, concurrent completion during a handler, finite batches, and stale
  result rejection through the application-owned task registry.
- Build `--example runtime_task`, then run `python3 scripts/check-native-task.py`
  on a desktop or under a virtual display to verify that worker success and
  failure wake the idle native event loop and update its final document.
- Build `--features inspector --example native_pointer`, then run
  `python3 scripts/check-native-pointer.py` to exercise the real showcase through
  X11 input on an isolated Xvfb display. It checks checkbox values and window
  visibility across wheel directions, non-primary buttons, and ordinary clicks.
  The script requires Xvfb and xdotool; `XVFB_BIN` can select an unpacked Xvfb.
- Build `--example runtime_ime`, then run `python3 scripts/check-native-ime.py`
  to exercise real Pinyin preedit, commit, undo, and cancel in an isolated X11
  display. It requires Xephyr, Fcitx5/Pinyin, D-Bus, xdotool, and xprop. Run under
  `xvfb-run -a` to keep the nested display off the desktop. The probe waits for
  processed focus/selection and active Pinyin, with a private runtime directory
  that prevents the engine from discovering the desktop's Wayland socket. The browser
  fixture runs through `OPERAD_WEB_IME_PROBE=1` with Chrome's native IME API;
  CI also checks Unicode ranges, reordering, focus changes, disabled/removed
  owners, password/multiline input, candidate geometry, and idle rendering.
- Build `--example native_device_loss`, then run
  `xvfb-run -a python3 scripts/check-native-device-loss.py`. A worker destroys a
  real device after the host becomes idle; the runner must wake and return the
  classified failure without rendering another frame or receiving input. A
  second case destroys the device from a canvas callback during rendering.
- `tests/fixtures/runtime_scheduling.rs` runs without a tick action. The browser
  runner's `OPERAD_WEB_SCHEDULING_PROBE=1` mode checks that it sleeps when idle and
  wakes for input, asynchronous service completion, background job success and
  failure, delayed repaint, continuous
  mode, and window/embedded resize. High and fractional DPI changes are checked
  alongside viewport resizing because Chrome emulation omits resolution-query
  notifications for a DPI-only override. CI builds and
  runs this fixture independently of the continuously ticking showcase.
- `OPERAD_WEB_DEVICE_LOSS_PROBE=1` runs the same browser fixture and destroys a
  WebGPU device during acquisition, while idle, continuously repainting, or
  awaiting a timer. It checks that startup loss prevents the first frame,
  browser input is released, scheduled callbacks are cancelled, and later
  background completion/input/resize cannot restart rendering. It also checks
  configured and disabled status reporting and preservation of the close-request hook.
- `OPERAD_WEB_RENDER_FAILURE_PROBE=1` checks the same shutdown contract for an
  invalid renderer configuration, including synchronous composition-key frames.
  A later device loss must not report the stopped session again.
- `OPERAD_WEB_CLOCK_PROBE=1` enables application ticks in the scheduling fixture
  and separates the page and runtime clock origins. Composition-key flushing
  must synchronize surrounding text without an artificial tick burst or stall,
  and the next scheduled frames must not replay the flushed input.

## Application integration and observation

`Application<State>` carries state, update, view, and `RuntimeHooks` on either
platform. Both runners route normalized events through
`RuntimeSession::process_input_with_hooks`; canvas capture remains associated
with stable identity even when an application consumes the press. Document
reconciliation produces semantic cancellations when an edit owner disappears
or becomes ineligible. Applications use those callbacks to close their own
transactions.

Input interception, hit testing, document updates, and action capture run once
per event in order. The next event sees geometry after preceding scroll and
interaction changes; cached layout avoids remeasurement when geometry is
unchanged. `HostInputEvent::document_result` records the result and action values
at that point. Finishing the frame consumes those results without replaying
input or deriving old actions from final geometry. Application updates and view
rebuilds remain a phase after the input batch.

`RuntimeObservation` borrows the final computed document and frame before
submission. It adds no view construction or layout. The session retains the
submitted document, including runtime decorations, for inspection between
frames; it removes those decorations only when preparing the next frame.
Browser inspection reads this retained document. Diagnostics can consume these
observations without owning or advancing the runtime lifecycle.

`runtime::task_channel` transports bounded, typed job results. Hosts install a
waker and drain through `RuntimeSession::apply_task_completions` on their UI
thread before computing metrics or building a view. All channels are snapshotted
before callbacks run, so delivery is finite even when handlers enqueue work.
Only nonempty batches invalidate the view. Applications own executors, task
generations, and cancellation; dropping hooks closes delivery without aborting
external jobs. Native senders are thread-safe for `Send` messages, while browser
senders and their wakeups remain on the browser thread.

`runtime::ime` owns automatic text-input sessions. Editable nodes publish the
displayed text, marked range, selection, and local caret geometry. The session
assigns an opaque focus-lifetime ID, preserves it across document reconciliation,
and retires it on focus loss, removal, or disablement. Hosts translate native
composition and character encodings; application-owned editing models decide
how to apply the resulting text edits. Preedit never changes committed text or
history. Event-order routing prevents a queued commit from reaching a later
focus target. Semantic cancellation callbacks clean up models for removed nodes.

See [runtime integration](docs/runtime-integration.md) for the public contracts
and migration from separate native/web hook types.
