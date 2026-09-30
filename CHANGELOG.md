# Changelog

## Unreleased

- Let frame preparation and platform callbacks report unchanged view inputs
  with `RuntimeHookResult`, retaining the document while still delivering
  requests and responses. Plain callback outputs invalidate conservatively.
  Native, web, and custom hosts share the session policy. Paint-only document
  rebuilds reuse computed layout when its inputs match; unchanged node names
  and ownership reuse the identity index.

- Render common shapes through one GPU SDF instance stream: rectangles, rounded
  fills and aligned borders, circles, round-ended lines, gradients, and shadows.
  Rounded gradients retain their authored stops; subpixel strokes retain their
  width. Gaussian outer and inset shadows support offsets and signed spread,
  with insets painted above the fill. Clipped gradients skip stop uploads.
  Rounded shadows use a cheaper analytic integral away from corners while
  preserving eight-sample integration near corners.
  Consecutive shapes batch without changing
  paint order. Compositor clips preserve each corner radius. Gradient uploads
  preserve caller-owned encoder lifetimes and
  return recoverable errors when GPU buffer limits are exceeded.

- Keep composite menu and dropdown popups owned by their trigger. Disabling an
  already-built trigger cancels popup focus and presses; hiding it also hides
  the popup. Nested submenus follow their parent row and the entire trigger
  chain. Portal placement and clipping still use the authored destination host;
  explicit global portals retain independent ownership.

- Revalidate retained menu paths against enabled ancestors. Disabled branches
  no longer open submenus, select commands, or leave actionable child popups
  after rebuilding. Keyboard navigation recovers at the first unavailable
  ancestor; menu bars and disabled menu buttons hide obsolete popups and clear
  active/expanded accessibility state. Reopening drops unavailable paths.

- Bound browser verification startup and CDP requests. Stalled responses,
  connection loss, malformed messages, and Chrome launch failures now fail the
  probe and release its process and profile. Request failures identify their
  method and session, and failed connections reject further work.

- Bound overlay ancestry traversal so cyclic parent relationships cannot hang
  pointer routing or bypass an unrelated modal. Dismissal walks parent graphs
  iteratively, preserving closure, ordering, and focus restoration without
  depending on call-stack depth.

- Keep numeric fields, sliders, and drag values inside their configured bounds
  after stepping. Range endpoints remain reachable even when they lie off the
  step grid. Committed, copied, displayed, and accessible values retain enough
  precision to represent the bounded value; active text drafts remain editable.
  Slider `Edits` clamping now applies after stepping while still allowing supplied
  values outside the range. Very large finite numeric values no longer become
  infinite merely because quantization temporarily scales them for rounding.

- Bound interrupted animation storage by the authored morph values instead of
  the number of interruptions. Ordinary hover/press animations no longer retain
  unused morph history. Keyframed polygons preserve their resolved source shapes
  and vertex resampling through repeated retargeting and runtime restoration.

- Keep obsolete form and field validation results stale after reset, replacement,
  or removal and re-addition of a field. Field requests now draw generations from
  the form's retained clock; adding or replacing fields also invalidates pending
  whole-form validation without canceling unrelated field requests.

- Reject non-ASCII hex-color input without panicking on UTF-8 byte boundaries.

- Restore a floating window's stored size and automatic sizing policy when a
  resize is canceled. `FloatingWindowResizeState` now includes `previous_size`;
  custom capture construction should supply the previous stored size or `None`.

- Anchor layout-animation scaling at the interpolated position. Resizing nodes
  away from the origin no longer shifts their painted position unexpectedly.

- Preserve each UI pass when an embedding application records several frames
  before submitting its WGPU command encoder. Geometry and image uploads,
  embedded canvas programs, and composited layers now execute in the caller's
  command order; resizing later text passes no longer changes earlier ones.
  Native and web hosts retain their immediate upload path.

- Recover glyph-atlas space when changing text exhausts retained rasterized
  glyphs. Rebuild the atlas and prepared text together, then retry the complete
  pass once. Small working sets no longer fail after many text changes; a
  working set that still exceeds capacity returns an error without retrying
  indefinitely.

- Invalidate rasterized GPU glyphs when replacing a renderer's font library.
  Replacement fonts can reuse database-local font IDs; changing their outlines
  no longer displays glyphs cached from the previous library.

- Map pointer selection through an active IME draft before canceling it, keeping
  committed text and undo history intact. Plain, multiline, and password fields
  now agree across routed actions and direct helpers. Canceling by pointer or
  leaving the field revokes the old input session before later queued commits;
  returning focus in the same batch cannot revive it. Escape, IME cancellation,
  and application-cleared drafts also revoke obsolete input. Empty preedit
  remains distinct from cancellation so a following valid commit still works.
  `HostInteractionState` adds `text_composition: HostTextCompositionState` to
  retain event lifetime separately from the rendered snapshot; explicit state
  literals initialize it with `Default::default()`.

- Map password pointer selection through the displayed mask, including scrolled
  fields and multi-codepoint graphemes. Runtime actions and direct input helpers
  now agree on caret geometry; direct helpers preserve the field's IME sensitivity.
  `TextInputContent` and `TextInputPointerGeometry` include a `mask` field; use
  `None` for content displayed without masking.

- Keep text fields within their assigned width while revealing the focused
  caret through internal scrolling. Long values no longer enlarge editors over
  neighbouring controls. Multiline fields also scroll vertically; pointer
  selection and IME geometry use the same displayed content coordinates.

- Use one text-control classification for editing, focus transitions, and click
  routing. Read-only and selectable text no longer activate their edit binding
  as a button command. Custom text/search roles work without an IME snapshot;
  editors publishing a snapshot also receive ordinary text and selection edits.

- Preserve fractional browser pointer coordinates through canvas-relative and
  UI-scale conversion. Hits near adjacent controls no longer move into the
  neighboring control because an event coordinate was truncated to an integer.

- Add a pointer observer to runtime hooks for hover previews and cursor state.
  It exposes the current hit and capture owners, respects disabled controls,
  blocking layers, and modals, and preserves normal widget input dispatch.
  Applications can use shared runtime ownership instead of installing a second
  raw pointer listener alongside text fields and canvas controls.

- Preserve displayed font metrics in text-field pointer edits, including edits
  applied after a focus handler changes application styling. Custom editors
  publish `TextInputContent { node, text_style }` with `set_text_input_content`;
  pointer geometry now owns an optional text style and is `Clone`, not `Copy`.
  Stationary moves and releases no longer recalculate selection against a
  restyled field. Releases still end capture, and a release at a new position
  still updates selection.

- Route text-field pointer clicks through text editing without also activating
  the edit binding as a button command. This prevents consumers from performing
  unrelated command side effects, such as saving a project on a caret click.

- Treat text-field pointer selection as a preview instead of committing or
  canceling the text edit when its gesture ends. Routed and direct input helpers
  now agree. Showcase value editors also avoid reformatting unchanged text during
  caret navigation and selection, preserving the user's insertion point.

- Match showcase clipboard replies to the current paste request and editor
  lifetime. Empty or failed reads preserve the selection instead of inserting
  previously copied text. Editing, focus changes, closing windows, and field
  resets revoke obsolete pastes. Valid pastes use normal editing, including
  undo/redo and shader validation, and completed responses are released.

- Keep the native clipboard connection alive between requests and release it
  when the event loop exits. On Linux, copy and cut no longer lose their text
  before a later paste when no clipboard manager retains it. The connection is
  initialized only when a supported clipboard request needs it.

- Align text-input pointer selection and input-method caret geometry with the
  painted content under custom padding and UI scaling. Editors can identify a
  separate content node while retaining focus/action ownership; cached sections
  remap that reference. Routed edits include authored content coordinates.
  Direct input helpers use the same clipped IME geometry as runtime routing.
  Scene primitives now apply UI scale consistently with their measured size.
  Default text fields no longer add layout padding on top of the scene inset,
  keeping ordinary text, placeholders, and carets fully visible.

- Use the shared text editor for command-palette search. Query fields now expose
  IME snapshots, render composition and selections, support caret navigation,
  clipboard requests, and undo/redo. Home/End move the caret; Up/Down navigate
  results. Forward complete edits with `apply_widget_text_edit` for pointer
  selection, handle `outcome.edit` clipboard requests, and supply focus state in
  `CommandPaletteOptions`. Programmatic query replacements reset edit history.

- Reuse shaped text and one render plan when building a text field's intrinsic
  size, IME caret, and scene. Composition without a visible caret also reuses
  that measurement; empty fields measure placeholder text separately.

- Borrow text editing models when building single-line and multiline fields,
  search fields, and code editors. Password fields construct only their masked
  display state. Rebuilding a field no longer copies its retained undo/redo
  history; presentation mode, selection, composition, and password masking are
  unchanged.

- Move text-input arrows and pointer carets across complete Unicode grapheme
  clusters, and make forward Delete remove the containing cluster. Accented
  characters, joined emoji, flags, and Indic text no longer split during these
  operations. Vertical caret columns count graphemes, and CRLF remains one
  navigation step. Explicit selections, IME offsets, and component-wise
  Backspace retain their existing byte/code-point semantics.

- Avoid copying committed text for change detection during caret movement,
  selection, clipboard reads, and composition drafts or cancellation. Editing
  snapshots are captured only for operations that can change committed text;
  document input helpers reuse the resulting change flag. Paste and keyboard
  edits also reuse their undo snapshot.

- Collapse text selections to their start or end on unmodified Left or Right,
  regardless of selection direction. Shift continues extending the active end,
  and navigation preserves committed text and undo history in editable and
  read-only fields.

- Track releases of IME-owned browser keys while another HTML control has focus.
  Returning to Operad no longer swallows the next press of that key, and the
  external control retains its normal keyboard event handling.

- Cancel browser composition when a text field switches between plain and
  password input. Replacing the editable element preserves committed text and
  selection, clears the application draft, and permits a fresh composition.

- Cancel runtime-owned text composition when a focused field's action binding
  changes. Queued events from the former session cannot edit the replacement
  field, and cancellation is delivered once to the original binding.

- Rebuild cached view sections when their call site changes. Conditional panel
  definitions with equal inputs no longer display an earlier branch's content
  when Rust gives their distinct closure types the same diagnostic name.

- Add `UiDocument::set_resource` for complete resource snapshots. Runtime
  preparation releases obsolete pending snapshots and writes, retaining the
  latest image and subsequent patches through failed presentations and cached
  view rebuilds. Ordered uploads remain available through `add_resource_update`.
  The showcase image now uses snapshots, and same-key handle aliases preserve
  their intervening upload order.

- Acquire the presentation surface before applying GPU resource uploads. A
  temporary surface failure no longer changes an image's size or format and
  causes its retained partial uploads to fail on the next attempt. Failed
  acquisition also avoids embedded canvas rendering and geometry preparation.

- Return renderer errors for textures and surfaces that exceed the device's
  dimension limit, and for snapshots whose padded readback exceeds its buffer
  limit. Invalid sizes no longer reach WGPU allocation or surface configuration.
  Dirty-rectangle validation also rejects overflowing endpoints.

- Share image-upload payloads across documents, render requests, and retries
  instead of copying their pixels at every frame preparation. WGPU borrows RGBA
  payloads directly. `ResourceUpdate.bytes` is now `Arc<[u8]>`; constructors accept
  vectors or existing shared buffers. Direct struct construction can convert
  vectors with `.into()`, and callers needing an owned mutable copy can use
  `.to_vec()`.

- Reject WGPU partial texture uploads without an existing base or with a changed
  size or format. Invalid patches leave the existing texture intact instead of
  recreating it and losing pixels outside the patch. Full uploads can replace
  size and format; subsequent patches use the replacement's format.

- Honor explicit transparent clear colors in WGPU snapshots, offscreen textures,
  and caller-owned targets. `RenderOptions::default().clear_color` now contains
  the existing opaque dark background directly; transparent black no longer
  acts as a sentinel for that default. Caller-owned load/clear overrides retain
  precedence over the request color.

- Package only source scripts, excluding generated Python bytecode caches.
  CI now verifies the packaged browser runtime as well as the native default
  build, including its JavaScript text-input bridge.

- Use the runtime clock for synchronous browser text-input frames as well as
  scheduled frames. Composition-key flushing no longer jumps application ticks
  forward, stalls subsequent ticks, or advances animation with the page's
  different time origin.

- Back off repeated temporary surface failures from 16 ms to a 250 ms cap,
  measured after the failed attempt, and reset the delay on success. Native and
  browser input, animation, tick, and idle wakeups respect the retry deadline.
  Synchronous text input can update surrounding text while deferring GPU work
  without losing uploads or postponing recovery. `RuntimeSession::frame_failed`
  now takes elapsed monotonic time at failure; custom hosts use
  `frame_retry_delay` and `frame_deferred` to follow the same contract.

- Stop browser sessions on permanent frame errors, including frames flushed
  synchronously before text composition. Preserve the renderer's error kind
  until deciding whether to retry. Share device-loss cleanup and report a
  terminal failure once, even if the device is subsequently lost.

- Wake an idle native runtime when its graphics device is lost and return a
  structured failure. Retry only temporary surface acquisition failures instead
  of looping on permanent backend errors. Reconfigure outdated surfaces even
  at unchanged dimensions, and refresh suboptimal surfaces after presentation.

- Give keyboard hooks a `KeyboardInput` containing the raw event and a borrowed
  view of the current focused control. Shortcut handlers can yield to controls
  after pointer or Tab navigation, including changes earlier in the same batch.

- Make ordinary controls activate only on primary clicks, while preserving
  keyboard, accessibility, and programmatic activation. Middle, right, back,
  and forward clicks no longer activate controls or report document clicks.
  Custom controls can use `WidgetActionMode::ActivateAnyButton` to receive
  button-specific actions. The showcase no longer needs an application filter.

- Stop browser sessions when their WebGPU device is lost. Cancel frame/timer
  callbacks, release browser pointer and text-input ownership, and suppress
  further input and asynchronous delivery while retaining close-request
  protection. Report the failure once and restore a removed status element
  unless status reporting is disabled. Reloading the page restarts rendering.

- Route scenario replay through the shared gesture and document-input processor.
  Replayed clicks, drags, ignored pointer releases, and wheel events now follow
  runtime behavior. Reports attach actual results to each step, resize events
  take effect in order, and synthetic pointer timing persists between frames.

- Reveal active rows in scrollable menus, selects, and command palettes after
  layout, including viewport-constrained popups and host-applied UI scaling.
  Follow selection and geometry changes while preserving manual scrolling
  across rebuilt and cached views. Explicit application scroll offsets retain
  precedence, including when they are clamped to the available range.

- Borrow shapes and clipping data for point hit queries instead of creating
  owned geometry and copying material/shader data for every candidate. Ordinary
  queries no longer allocate diagnostic rejection lists; explicit diagnostics
  preserve their detailed results.

- Reuse document stacking order across painting, hit testing, wheel routing, and
  modal selection. Structural and style edits invalidate the cache; scrolling,
  animation, clipping, and interaction eligibility remain current.

- Apply UI scaling once when placing menu, select, context-menu, and command-
  palette popups. Flipping and viewport constraints use the scaled dimensions
  and margins at layout time, including host-applied scales and cached sections.
  Viewport-shortened menus keep all items reachable by scrolling;
  command palettes keep their search field visible while results scroll.

- Anchor tooltips to painted control bounds, including animation translation and
  scale. Apply UI scaling once when positioning and clamping tooltip boxes, and
  use the current pointer position for automatic cursor placement. Resolve
  placement during layout so explicit tooltips also support host-applied scaling
  and cached views.

- Resolve tooltip inheritance through logical portal owners and stop at modal
  boundaries. Automatic tooltips prefer available focus help, fall back to hover
  help, and retain their source owner in rendering and accessibility. Tooltip
  names in separate owner subtrees no longer suppress each other's help.

- Preserve cached native views when registered canvas callbacks have no matching
  canvas in the current frame. Matching callbacks still invalidate the view,
  including when they fail after changing application state.

- Borrow shader source, entry points, and constants when looking up cached canvas
  pipelines. Repeated draws no longer allocate copies of pipeline keys; new
  variants retain their own data and continue to use exact equality.

- Limit the WGPU canvas pipeline cache to 128 recently used shader variants.
  Repeated shader edits and specialization changes retire unused pipelines,
  including direct canvas draws outside the UI frame loop. Evicted variants are
  recompiled when needed; cache hits still borrow their descriptors.

- Keep temporary WGPU compositor targets separate from persistent app textures.
  Image and canvas names can no longer collide with generated layers or trigger
  deletion during frame cleanup. Cleanup visits only temporary targets instead
  of scanning every app texture.

- Add `CanvasRenderProgram::uniform_bytes` for per-frame shader data at group 0,
  binding 0. The showcase's shader time and cube rotation now use uniforms instead
  of specialization constants, reusing GPU pipelines during animation and dragging.

- Borrow canvas data when preparing embedded shader programs. Native callback
  preparation skips empty registries and clones requests only for registered
  handlers, avoiding per-frame copies of unrelated shader source and descriptors.

- Discover canvases and images inside nested compositor layers. Render callbacks,
  embedded canvas programs, missing-handler reports, and input-capture plans now
  include those children in paint order. Wrapping a canvas in a layer preserves
  its capture lifetime; removing the layer releases its captures and pointer lock.

- Route manual widget actions and drag-and-drop through logical portal ownership.
  Disabled event targets no longer activate an enabled ancestor or close a dialog.
  Drop routing rejects the resolved drag source and its owned descendants, even
  when the gesture starts on a child or the drop surface lives in a portal.

- Keep host input results and activation animations consistent with confirmed
  click gestures. Drag releases and rejected clicks no longer activate manual
  widget helpers or fire click animations. Paired click gestures must also match
  the document's current press and release target before dispatching an action.

- Check click distance again on pointer release. Missing or intercepted move
  events can no longer turn a distant release into a widget activation or add
  to the double-click count. Existing drags still commit normally.

- Preserve keyboard focus and active text composition across frame preparation
  when a focusable control uses `HitTestBehavior::PassThrough` or `Block`.
  Pointer hit policy no longer overrides keyboard-focus eligibility.

- Extract canvas input-capture plans directly from paint items without cloning
  shader source or context descriptors. Canvases without capture allocate no
  plans, and initial cursor-lock requests scan the input flags directly.

- Keep source ownership when `AppOverlay` and `Named` portals mount elsewhere.
  Menus opened inside a modal now receive pointer, keyboard, and wheel input,
  remain in its accessibility tree, and inherit their source's disabled/hidden
  state. Nested portals and cached sections retain this ownership. Changing the
  source ends the old focus/press lifetime; layout-animation snapshots use the
  same identity. `GlobalAppOverlay` and `GlobalNamed` retain independent ownership.
  `UiNode::logical_parent()` exposes interaction ancestry; `LayoutSnapshot` now
  carries an optional `portal_owner` alongside its physical layout tree.

- Fix dialog outside dismissal over blocked backgrounds and disabled content.
  Pointer dismissal respects transformed bounds, portal descendants, and the active
  modal, and ignores hidden or disabled dialogs. Outside dismissal occurs on pointer
  press, so a later release cannot close another dialog or dismiss after an inside drag.
  `modal_dialog_dismiss_event_from_input_result` now requires the matching
  `UiInputEvent` as its final argument instead of inferring outside clicks from
  widget activations.

- Preserve modal text focus and active IME composition when clicking the blocked
  background. Pointer focus cannot escape through a dialog's ancestors; presses
  without a focusable target retain eligible focus inside the active modal.

- Share one immutable node-identity index between document layout, runtime state
  retention, and canvas hooks. Input batches no longer rebuild all named paths.
  Structural and unrestricted node edits invalidate the cache while retained
  snapshots preserve previous identities for remapping and cancellation.

- Read the target node's paint transform directly for canvas pointer coordinates
  and IME cursor placement. These conversions no longer sort and materialize
  geometry for every node, while retaining clipping and transformed coordinates.

- Route retained canvas input using the current document's interaction policy,
  including channel changes made before the next frame finishes. Keyboard,
  text, and wheel fallback cannot preserve revoked channels or transfer a
  former canvas node's capture to an ancestor with the same surface key.

- Share cursor lock across all active canvas captures. Resizing, reordering,
  replacing, or removing a canvas no longer unlocks the cursor while another
  owner needs it. Release the lock once the last owner leaves; capture diagnostics
  report the cursor requests actually emitted. Avoid cloning capture keys during
  plan lookup and comparison.

- Keep pointer input, wheel scrolling, and action propagation inside the active
  modal dialog. Opening a modal cancels background widget, scrollbar, and canvas
  drags. Background and disabled canvases continue rendering without requesting
  input capture or pointer lock; their authored policies resume when eligible.

- Move keyboard focus into newly active modal dialogs before routing input, and
  restore it by stable identity when dialogs close. Nested dialogs retain their
  return targets; removed or ambiguous controls cannot inherit stale focus.
  Honor explicit focus requests and modal restore policies, and deliver text
  focus changes caused by document preparation before queued input actions.
  Background canvas keyboard capture cannot bypass the modal, and IME focus loss
  cancels the original editing operation once before rejecting stale commits.

- Exclude `display: none` subtrees from accessibility navigation and live-region
  announcements while retaining offscreen controls. Explicit hidden label and
  description references keep their text, and published relations point only to
  exported nodes. Layout audits no longer flag deliberately hidden controls.

- Restrict modal navigation to the topmost visible, enabled dialog using paint
  stacking order. Hidden dialogs no longer trap Tab; closing the front dialog
  exposes the one behind it. Inspector traces use the document's resolved scope.
  Preserve focus across frames for controls that declare focusability through
  accessibility metadata, including dialog containers.

- Choose the next keyboard focus target directly from the document, avoiding
  accessibility-tree cloning, sorting, and repeated tree searches on every Tab.
  Preserve accessibility priorities, input-only fallbacks, modal scope, and wrapping.

- Handle Tab and Shift+Tab as shared document focus navigation, respecting
  accessibility order, disabled controls, and modal scope. Backward traversal
  starts at the last eligible control when nothing is focused. Navigation does
  not emit text edits; custom input hooks and IME candidate keys retain ownership.

- Keep generated key text on `RawKeyboardEvent::with_text`. Keyboard and canvas
  hooks consume the key and its text together; independent text is no longer
  discarded after Enter or because it shares a consumed key's timestamp. Native
  and web hosts emit one raw event per key. Enter and key repeat insert each
  intended newline once, regardless of frame boundaries.
- Raw-input conversion now uses `to_ui_input_events` and
  `to_ui_input_events_with_wheel_scale` iterators. `RawKeyboardEvent` is `Clone`
  rather than `Copy`; replay steps store `converted` and `results` vectors to
  retain every document event from one raw input.

- Apply document input before routing the next event, so batched scrolls, clicks,
  and canvas hooks see current geometry. Capture action offsets and pointer-edit
  coordinates per event, including the geometry retained for later cancellation.
  Unchanged pointer motion reuses cached text layout.
- Custom hosts now pass a mutable document and text measurer to
  `RuntimeSession::process_input` / `process_input_with_hooks` and handle their
  layout `Result`. Finish the same document with `finish_frame`, collect actions
  with `collect_document_widget_actions(&frame)`, and read input results through
  `frame.input_results()`.

- Preserve automatic scrollbar drags across frames and document rebuilds, keeping
  the original axis and thumb grab position. Scrollbar input no longer activates
  the container or starts a widget edit, and canvas hooks cannot steal the drag.
  Removed, hidden, disabled, or no-longer-scrollable owners release capture.

- Store paired document events and gestures in `HostFrameOutput::events`, preserving
  action order and document ownership. `ui_events()` and `gestures()` provide
  read-only iterators; custom hosts can append `HostInputEvent` entries or convert
  a standalone `UiInputEvent` or `GestureEvent` with `.into()`.

- Prevent a second pointer from stealing or canceling an active widget gesture.
  Custom input hooks continue to receive every pointer's events.

- Preserve text-selection updates and release positions when multiple pointer
  events arrive in one frame. Route each edit to that event's press owner;
  the direct text-input helper also applies the final release position.

- Build stacking keys once per sort for hit testing and painting, avoiding key
  clones and heap allocations during comparisons.

- Keep pointer gestures owned by their initiating button. Chorded or intercepted
  releases from other buttons no longer steal clicks or end drags. Releasing a
  click outside its original target no longer activates that target.
  `PointerGestureTracker::pointer_down` now returns `Option<GestureEvent>` and
  ignores additional presses while that pointer owns a gesture.

- Stop wheel gestures from activating checkboxes, buttons, and other ordinary
  action bindings. Wheel input still reaches scrolling and custom input hooks.

- Add selective rebuilding through `ViewContext::section`. Runtime view callbacks
  now take `(&State, UiSize, &mut ViewContext)`. Unchanged inputs reuse authored
  sections and nested caches; stable nodes retain Taffy layout and measurements
  across document rebuilds. View observations expose section build/reuse counts.
  Custom hosts use `refresh_view` to invalidate external measurement dependencies
  and retain the current document before an action-triggered second build.

- Add native and browser IME composition to editable widgets: inline drafts,
  marked selection and underlines, candidate positioning, cancellation, and one
  undoable commit. Shared sessions preserve input ownership across rebuilds and
  reject stale events after focus changes. Browser input uses a native editable
  element and converts DOM UTF-16 ranges to Operad's UTF-8 byte offsets.
- Add bounded typed background completion channels. Native and web hosts wake
  when results arrive, apply them to state on the UI thread, and rebuild the
  view without polling. Applications retain control of executors and cancellation.
- Use one repaint scheduler across native, web, and headless hosts. Delayed
  repaint deadlines survive earlier frames, canceled timers do not leave stray
  repaints, and repaint requests made during a frame survive its presentation.
- Let the web host sleep while idle and wake on input, resize, application
  ticks, animation, explicit repaint requests, and asynchronous service results.
  Reset native wait deadlines after work is consumed or the window is minimized.
- Observe embedded canvas size and display-scale changes while idle, and size
  the UI from its canvas instead of imposing the browser window's dimensions.
- Drain platform requests after application updates and apply asynchronous
  responses before building the view, so idle applications do not need an
  unrelated event to show a response or submit a queued request.
- Custom session hosts should bracket each host frame with `begin_frame` and
  `frame_presented`/`frame_failed`. `coalesce_repaint_requests` now returns a
  vector preserving independent deadlines and continuous-rendering policy.
- Share document preparation, input routing, frame processing, and runtime state
  retention between native and web hosts. Focus, gestures, IME targets, and
  canvas captures follow stable node paths across rebuilds; removing targets
  cancels their runtime ownership.
- Retain documents and layout across redraws, rebuild after application updates
  and viewport changes, and consume resource uploads only after presentation.
  The final document remains available for inspection; frame-owned tooltips are
  removed when preparing the next frame.
- Move invalidation and timing primitives into `core`, and make subsystem
  modules own their implementations instead of re-exporting root declarations.
- Make `diagnostics`, `inspector`, and `test-support` opt-in features. The native
  showcase now requires `--features inspector`; `web-showcase` enables it.
- Consolidate equivalent inspector option and node types into shared panel
  contracts. Panel functions now return `DiagnosticPanelNodes` where they expose
  a root and a row container, and use common timeline, source, record, candidate,
  text, property, node, change, and issue options where their contracts match.
- Import `EmptyResourceResolver` from `renderer`, timing primitives from `core`
  or the crate root, and theme stability metadata from `theme::stability`.

## 9.0.1 - 2026-05-22

- Added direct app-owned WGPU view composition with explicit load/clear
  behavior, capability reporting, caller-owned GPU timing tokens, and
  discard-path documentation for legacy encoder rendering.
- Added shared font injection for the WGPU renderer and `CosmicTextMeasurer`
  through a backend-neutral font library so layout and rendered glyphs can use
  the same app fonts.
- Added a public empty resource resolver for rect/text-only render paths and
  documented resolver/update requirements for images, textures, thumbnails, and
  canvas resources.
- Fixed Shader Lab material-effect layout so painted outsets do not overlap
  controls, shortened visible showcase copy, restored canvas demo dragging, and
  added default split-pane drag-handle cross-axis margins.

## 9.0.0 - 2026-05-21

- Added element material and shader-effect support for UI surfaces, including
  shader-backed canvas, frame, and button previews, paint outsets, clip and hit
  shape controls, geometry-effect declarations, and renderer-safe fallback
  reporting for invalid WGSL.
- Added the Shader Lab showcase for editable WGSL programs, program presets,
  material contract controls, canvas/frame/button targets, and live validation
  without crashing the host when shader compilation fails.
- Hardened popup, dropdown, menu, tooltip, modal, and portal stacking so
  anchored surfaces are layered relative to their owning window and stay above
  later siblings without escaping unrelated floating windows.
- Reworked scroll containers and scrollbars with automatic visibility,
  reserved scrollbar gutters, hover/active visual states, draggable thumbs,
  ctrl/shift horizontal scrolling, end-position tests, and range epsilon fixes.
- Improved text editing and selection ownership, including single active text
  selection, tighter caret and selection geometry, focus blur behavior,
  multiline newline handling, and scrollable code/text editor regions.
- Expanded media and image resources with user-supplied image handles, decoded
  PNG/JPEG/BMP assets, clearer missing-image rendering, icon image variants,
  and showcase coverage for image-backed checkbox marks and media grids.
- Added indeterminate checkbox state, richer radio/toggle customization,
  editable numeric inputs with unit-specific ranges, date range picking,
  progress logs, theme switching, panel resizing, timeline scrolling, editable
  trees, and easing demos.
- Refined showcase layout and organization behavior so windows start closer to
  content size, can be packed/minimized when organized, and keep regressions in
  external tests instead of `examples/showcase.rs`.
- Cleaned up all-target Cargo warnings and extended visual/layout tests for the
  v9 widget surfaces.

## 8.0.1 - 2026-05-18

- Fixed text input selection geometry on the web by using the same non-wrapping
  shaped text layout for paint, caret placement, and selection highlights.
- Aligned the direct `cosmic-text` dependency with `glyphon` so browser and
  native text measurement use the same shaping engine.

## 8.0.0 - 2026-05-18

- Added application-shell APIs for command registries, scoped hotkeys,
  shortcut remapping, command diagnostics, and command palette examples.
- Added the animation state-machine runtime with inputs, triggers, blend
  bindings, topology morph values, debug graph views, and showcase examples for
  timed, scrubbed, boolean, and interaction-driven animation.
- Added host capability profiles and backend diagnostics for native, web, and
  test hosts so applications can gate hotkeys, text editing, canvas editing,
  flycam input, docking, drag/drop, and accessibility behavior explicitly.
- Expanded canvas input support with key release, raw mouse motion, cursor
  capture requests, host-capture diagnostics, WGPU canvas callbacks, and replay
  coverage for editor-style and flycam-style interactions.
- Added reusable diagnostics for layout, paint, intrinsic sizing, accessibility,
  command routing, theme editing, animation, runtime errors, platform responses,
  virtualization, and performance summaries.
- Added interaction recording and replay infrastructure, scenario harnesses,
  long-wheel helpers, topmost input-consumption assertions, and external
  showcase regression tests.
- Added virtualized tree and data-table APIs with stable focus preservation,
  selection/export/sort/filter/resize metadata, sticky regions, diagnostics,
  and showcase coverage.
- Added docking workspace primitives with split panes, drawers, drawer rails,
  persisted layout snapshots, panel reorder targets, floating panel state, and
  domain-neutral dock drag/drop contracts.
- Added layout animation transition records and reduced-motion-aware paint-list
  interpolation while keeping layout output authoritative.
- Hardened widget layout, scroll, overlay, text input, selection, scrollbar,
  color picker, styling, forms, media, drag/drop, and showcase behavior through
  generic primitive fixes instead of showcase-only sizing patches.
- Added the WASM/WebGPU showcase path for GitHub Pages, including root and
  `/showcase/` artifacts, favicon packaging, browser smoke checks, and web
  startup error reporting.
- Documented v8 API stability categories and the reusable release process for
  future Operad releases.

## 7.0.0 - 2026-05-14

- Started the v7 source-organization pass by moving retained document
  primitives into `src/core/document.rs` and widget module wiring into
  `src/widgets/mod.rs`.
- Added `operad::prelude` as the preferred broad import surface for application
  code.
- Added v7 roadmap, migration-guide, and release-checklist drafts so API moves,
  compatibility aliases, widget parity work, and release gates are tracked as
  work happens.
- Carried the released `6.1.0` WGPU baseline forward, including `wgpu` 29.0.3
  and `glyphon` 0.11.0.
- Moved widget-gated tests out of `core::document` and into the widget module
  so the first source split also moves module ownership for widget behavior.
- Added v7 widget builders for text-style labels, standalone images,
  separators/spacers, spinners, radio buttons/groups, toggle switches, visual
  drag values, generic grids, and panel containers so examples can use normal
  Operad primitives instead of hand-building those nodes.
- Added renderer-neutral performance diagnostics that name frame pipeline stages
  and aggregate cache hit, miss, and eviction rates from display-list reuse
  reports.
- Added v7 overlay/widget builders for collapsing headers, tooltip boxes, and
  modal dialogs so examples and applications can use normal Operad surfaces
  instead of hand-building those nodes from low-level containers.
- Added link, hyperlink, and selectable-label builders so interactive text
  controls are available separately from selectable read-only text input.
- Added text-input convenience builders for single-line input, multiline input,
  text areas, code editors, search boxes, and password fields.
- Added generic drag/drop source and drop-zone builders backed by the existing
  drag/drop descriptors and platform drag-start requests.
- Added small, icon, image, toggle, and reset button convenience builders on top
  of the default button primitive.
- Added form section, row, field label, help text, validation message, and error
  summary widget helpers backed by the existing form validation contracts.
- Added container helpers for panels, frames, groups, sides, columns,
  indentation, resize handles, and resize containers.
- Added compact color button, color swatch button, color-format display, and
  RGB/RGBA/SRGB/SRGBA/HSVA/OKLCH color-edit button helpers.
- Added menu-button state, trigger builders, image menu buttons,
  image-and-text menu buttons, submenu item helpers, and anchored submenu popup
  composition.
- Added selectable-value and angle-drag helper builders over the existing
  selectable-label and drag-value primitives.
- Added color conveniences for premultiplied/unmultiplied RGBA and
  SRGBA buttons, `Color32`-style color buttons, and standalone HSV 2D picker
  fields.
- Added `area` and `scene` widget helpers so absolute-positioned regions and
  scene primitives can be built through the normal public widget API.
- Added widget interaction helpers for visible/enabled UI blocks, fixed/minimum
  allocations, scene painter allocations, and programmatic scrolling.
- Added global theme preference button-group and switch builders so applications
  can expose System/Light/Dark theme controls without showcase-specific code.
- Added form action buttons plus field-order traversal helpers for
  submit/apply/cancel/reset workflows.
- Added tooltip trigger resolution and tooltip animation presets to the widget
  layer.
- Added modal dialog dismissal policy, focus-trap helpers, and overlay-frame
  event helpers to the visual modal builder.
- Added drag-image policy and drop-preview state helpers to the drag/drop widget
  layer.
- Added a composed scroll area with aligned vertical/horizontal scrollbar
  helpers.
- Added required frame-stage and cache-domain performance diagnostics to the
  unified diagnostic report surface.
- Documented v7 performance stress probes, budgets, and repeatable smoke
  commands outside the showcase.
- Trimmed package contents so old internal planning docs are not shipped as
  current crate guidance.
- Added a v7 showcase audit covering public API use, hidden harness removal,
  widget state coverage, text input, scrollbar, and overlay validation.

## 6.1.0

- Updated the optional WGPU stack to `wgpu` 29.0.3 and `glyphon` 0.11.0 so
  native-window and GPU canvas consumers can share the same WGPU version as
  downstream renderers that have already moved past WGPU 25.
- Adjusted the WGPU adapter for WGPU 29 surface acquisition, pipeline layout,
  render pass, polling, sampler, and glyphon text-buffer APIs.

## 6.0.0

- Added a native window runner so simple apps can open a window with
  `run_ui_document` or `run_app` instead of wiring winit and WGPU by hand.
- Added a widget library for common controls, including buttons, checkboxes,
  sliders, text input, selection controls, menus, date and color pickers,
  lists, tables, trees, floating windows, toasts, popup panels, and canvas
  surfaces.
- Added app-owned WGPU canvas rendering. Applications can declare a GPU canvas
  in the UI tree, register a renderer with `NativeWgpuCanvasRenderRegistry`,
  and record their own WGPU work into the canvas texture before Operad
  composites the UI.
- Added `WgpuCanvasContext` and `WgpuCanvasRenderPass` under the `wgpu` feature.
  `WgpuCanvasContext::render_pass` remains a convenience helper for simple WGSL
  passes; apps can also create command encoders and render passes directly.
- Added OKLCH support to the color picker and fixed hue endpoint handling so
  dragging hue to the far right no longer snaps the control back to the left.
- Removed the transient WebGL/WebGL2 naming from the canvas API; the public
  surface uses native GPU and WGPU terminology.
- Moved the source tree toward the v6 module organization while preserving
  common v5 public compatibility paths for downstream consumers.
- Updated README, migration notes, and release validation guidance for the v6
  public API.

## 5.0.0

- Added Operad-owned public layout primitives for common API use, with conversion
  paths back to `LayoutStyle` and Taffy for migration and advanced cases.
- Added localization and internationalization policy types for locale identity,
  text direction, bidi behavior, layout mirroring, and dynamic label metadata.
- Carried localization policy through text content, localized labels, paint
  output, and accessibility metadata.
- Added public API stability/versioning marker types so v5 consumers can
  distinguish stable, experimental, backend-specific, and migration-only APIs.
- Added backend-neutral runtime/frame lifecycle contracts, widget action queues,
  retained widget state lifecycle, edit transactions, and selection/history
  helpers for interaction-oriented hosts.
- Added core widget action routing helpers and action bindings for buttons,
  checkboxes, sliders, and text inputs.
- Wired widget text input edits into `TextEditHistory`, including committed
  transactions and keyboard undo/redo for text input state.
- Added async task lifecycle and form validation contracts for progress,
  cancellation, stale async results, dirty/pending state, submit/apply/cancel
  workflows, and accessible error summaries.
- Added shared effective-geometry, advanced scrolling, compositor feature, and
  resource cache lifecycle contracts for renderer and host integration.
- Added font lifecycle contracts for fallback stacks, loaded/missing/failed
  states, generation checks, cache byte accounting, and eviction planning.
- Added headless accessibility adapter contracts, accessibility target
  publication records, error classification, resource/input limits, and release
  guardrails for adapter and renderer failures.
- Added an optional `accesskit-winit` adapter that converts Operad
  `AccessibilityTree` output into AccessKit tree updates and publishes it
  through winit hosts.
- Added touch/stylus/gamepad routing, multi-window routing, navigation/overlay
  contracts, virtualization planning, tooltip/help/context menu policy, unified
  diagnostics, and theme/design-token stability documentation.
- Updated consumer-style probes and render tests to use the new public
  conversion helpers where direct backend layout fields are not required.
- Documented the v5 completion audit, migration posture, release checklist, and
  CI/release gates for fmt, feature-matrix checks, docs, examples, package
  verification, WGPU validation, perf smoke, and semver review.
- Added a bounded native WGPU window smoke mode to `native_wgpu_host` so release
  validation can prove OS-surface presentation without snapshot readback while
  keeping display-dependent execution opt-in.
- Added CPU/WGPU rich-rect gradient rendering coverage and corrected compositor
  quality profiles so fallback and unsupported effects are not overclaimed.
- Added explicit composited paint layers with CPU snapshot reference rendering
  and WGPU render-to-texture composition for rounded clips, rectangular masks,
  opacity, and basic brightness/contrast/saturate/blur filters.
- Added CPU/WGPU soft rich-rect shadow falloff coverage and compositor quality
  planning for basic native shadow blur limits.
- Added WGPU parity coverage for glyphon text inside composited paint layers
  and release-gated the sRGB-only snapshot color-management policy.
- Added explicit compositor quality fallback records for backdrop filters, which
  remain disabled until a backend samples the already-composited framebuffer.
- Added lyon-backed path fill/stroke tessellation with even-odd holes, curved
  concave fills, configurable stroke caps/joins, and WGPU parity coverage.
- Added WGPU parity coverage for fractional grayscale glyph positioning without
  claiming RGB/LCD component subpixel masks for v5.
- Expanded the native WGPU host example document to include buttons, text input,
  popup/menu items, a drag-handle target, and a canvas viewport.

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
- Removed the legacy compatibility painter path from the active renderer
  backend surface.
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
