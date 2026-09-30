# Application integration

Operad 10 uses one `runtime::Application<State>` definition and one
`runtime::RuntimeHooks<State>` surface for native and browser hosts. State,
`update`, `view`, and callbacks can live in ordinary platform-independent Rust.
Choose window or canvas options only at launch. The showcase and runtime editor
examples demonstrate this split.

The view callback takes `(&State, UiSize, &mut ViewContext)`. Add an unused third
argument to views that build a whole document. The context belongs to the
runtime session; application state remains ordinary Rust data.

## Selective rebuilding

Wrap expensive sections in `views.section`. Include every application value
the section reads in its comparable inputs:

```rust
use operad::{LayoutStyle, TextStyle, UiDocument, UiNode, UiSize};
use operad::runtime::ViewContext;

fn view(title: &String, viewport: UiSize, views: &mut ViewContext<'_>) -> UiDocument {
    let mut document = UiDocument::new(LayoutStyle::column()
        .with_size(viewport.width, viewport.height));
    let root = document.root();
    views.section(&mut document, root, "title", title, |title, _| {
        let mut panel = UiDocument::new(LayoutStyle::column());
        panel.add_child(panel.root(), UiNode::text(
            "label", title, TextStyle::default(), LayoutStyle::default()));
        panel
    });
    document
}
```

Use `Clone + PartialEq` input structs for panels with several dependencies,
including theme and child inputs. A revision counter works when it advances
for every relevant change. Keep the builder's meaning stable and avoid reading
changing values outside these inputs. Builders may borrow application data
while running, but cached inputs must own their data. Builders can be skipped,
so application side effects belong in `update` or hooks.

The root of the returned document becomes the named section. Its node IDs are
local; `section` returns the root ID in the destination document. Internal
accessibility relations, stacking references, and layout constraints are
remapped. Named portals stay local to a section; app-overlay portals remain
viewport sized. Sibling section names must be unique, including ordinary node
names. Renaming or reparenting starts a new lifetime. Removal evicts the cache.

`AppOverlay` and `Named` portals retain the source parent supplied to
`add_portal_child`. Their layout uses the destination host, while input,
accessibility, and retained identity follow the source. A menu opened inside a
modal therefore remains inside its input boundary even when drawn outside the
dialog. Hiding or disabling the source also affects its portals. Changing the
source starts a new interaction lifetime, including after section reuse.
`GlobalAppOverlay` and `GlobalNamed` use the destination host for ownership;
use them for independent surfaces. `UiNode::parent()` reports the layout parent,
and `logical_parent()` reports the parent used for interaction and identity.
`LayoutSnapshot::portal_owner` preserves the distinction for layout animation.
Composite menu buttons, menu bars, and dropdowns attach their owned popup to the
trigger control; nested submenus attach to the row that opens them. The supplied
parent still determines local placement and portal fallback. This also applies
to `Parent` popups, so hiding or disabling a trigger affects its popup even when
both are physically siblings. Explicit global portals keep independent ownership.
Manual widget action helpers use the same ownership and modal boundary as
automatic dispatch, and check that both the event target and its owner remain
enabled. Drag-and-drop resolves source and destination owners before rejecting
self-drops, including drops onto the source's owned portal descendants.

Tooltips inherit through logical ownership and stop at the active modal boundary.
Automatic tooltips prefer available focus help and fall back to hover help. Their
rendered overlay remains owned by the source control, including in the
accessibility tree. Tooltip names are scoped to their owning node, so a name
used in another panel does not suppress that control's help. Explicit tooltip
triggers use `HelpItemState` to decide whether to show help for disabled controls.

Tooltip anchors follow painted control bounds, including animation transforms.
`tooltip_box_from_request` takes its anchor, viewport, and cursor in logical
window coordinates; its size uses authored UI units. Size and spacing follow
UI scaling once; side selection and position clamping use those scaled dimensions.
Tooltip placement is a layout constraint, so it also uses the correct scale when
the host applies scaling after view construction or reuses a cached section.
Automatic cursor placement uses the latest pointer position, including on
frames that reuse the application view.

Menu, select, context-menu, and command-palette popups accept `AnchoredPopup`
anchors and viewport bounds in logical coordinates relative to the destination
portal host: window coordinates for app overlays, local coordinates for parent
or named portals. Widget sizes and `PopupPlacement` offsets and margins use
authored UI units. Placement accounts for UI scaling before flipping and
constraining a popup, then converts the result for layout. The
`UiNodeLayoutConstraint::AnchoredPopup` constraint resolves at layout time, so
hosts can apply scale after constructing a view and cached sections retain the
original placement inputs. Menus shortened by
the viewport remain scrollable; command palettes scroll their results while
keeping the search field visible. The lower-level `place_popup` helper expects
all lengths in the same units, and `popup_panel` takes an authored layout rect.

Scrollable menus reveal their active row after layout, including when the host
applies scale after view construction. Selection and content geometry changes
request a new reveal. Otherwise, manual scrolling survives both full document
rebuilds and cached sections; an unchanged selection does not pull the scroll
position back. Explicitly authored offsets take precedence. Removing a menu
ends this retained state, so reopening it reveals its active row again.

Menu selection and submenu opening require every item along the path to be
enabled. If application data disables or removes an ancestor of the active item,
navigation resumes among that ancestor's siblings; activation cannot select a
different command as a side effect of recovery. Rebuilding hides popups beneath
disabled branches and disabled menu triggers, including retained menu-bar state.
Read-only `menu_item_at_path` and `menu_items_at_path` still expose disabled data.

Unchanged inputs at the same section call site skip the builder. Switching to
another call site rebuilds the section, so conditional panel definitions do not
display each other's cached content. Wrappers can use `#[track_caller]` to forward
the location of their callers. If a single call site chooses builders at runtime,
include that choice in the inputs, just like other captured application values.
When a parent rebuilds, unchanged nested
sections can still reuse their caches. When the parent is reused, its entire
subtree is reused, so the parent's inputs must include anything that could
change a nested section. Viewport and scale changes rebuild all sections.
Custom hosts call `refresh_view` after changing fonts or another external
measurement dependency; `invalidate_view` only announces changed application
inputs. Authored focus and scroll requests in a reused section do not replay.

Taffy's layout and text measurement caches persist by stable node identity.
Changes to a node's style, text, or children invalidate its dependent layout;
reordering preserves measurements while updating positions. The runtime still
assembles a complete document and produces current paint, accessibility, and
hit-testing output. Section reuse copies authored nodes; this is not partial
GPU repainting. Custom intrinsic-size constraints still run when layout is
invalidated.

Rebuilding a description with unchanged names and logical parents also reuses
the identity index. When only paint changes, Operad can retain computed layout
after comparing tree structure, layout styles, text, clipping, opacity, and
scale. Scrolling and owned portals rerun positioning but can retain unchanged
sizing. Layout constraints use normal reconciliation. Resizing and `refresh_view`
still refresh geometry.

`RuntimeObservation::view_build_stats` and
`RuntimeSession::view_build_stats()` report section builders run and sections
reused in the latest document pass. An entirely retained document reports zero
for both. The runtime editor demonstrates a stable toolbar during timeline
edits. The native task probe checks that background completions rebuild only
the result section.

## Application hooks

Frame and platform callbacks invalidate the view conservatively when they
return ordinary outputs. Return `RuntimeHookResult::unchanged(value)` when the
callback changes no value read by the view. For example, draining an outgoing
cursor request queue need not rebuild a document:

```rust
use operad::platform::PlatformRequest;
use operad::runtime::{RuntimeHookResult, RuntimeHooks};

struct App { requests: Vec<PlatformRequest> }

let hooks = RuntimeHooks::new().with_platform_requests(|state: &mut App, _| {
    RuntimeHookResult::unchanged(std::mem::take(&mut state.requests))
});
```

The same result works with `with_platform_service_requests`,
`with_platform_responses`, and `with_before_render`. Use `changed(value)` or
set `view_changed` when a callback edits visible state. An empty request queue
does not imply unchanged state, and sending requests does not imply a changed
view. Playback positions, meters, completed file dialogs, clipboard edits, and
status updates must still refresh their dependent content. An unchanged result
never cancels invalidation caused by another callback, input, or a task.

Native and browser hosts run these callbacks through the shared session.
Custom hosts use `apply_before_render` before building the view,
`take_platform_requests` to collect application platform work, and
`apply_platform_responses` to deliver service results. Execute the requests
with the host's platform services. Repaint requests still control scheduling;
reporting unchanged view inputs does not suppress platform work or animation.

```rust,ignore
fn application() -> operad::runtime::Application<App> {
    operad::runtime::Application::new(App::default(), App::update, App::view)
        .with_hooks(operad::runtime::RuntimeHooks::new()
            .with_keyboard_input(|state: &mut App, input| state.keyboard(input))
            .with_canvas_input(|state, event| state.canvas(event))
            .with_frame_observer(|state, observation| {
                state.inspect(observation.document, observation.frame);
            }))
}
```

Keyboard hooks receive `KeyboardInput`: `event` contains the normalized key and
its generated text, and `focused` borrows the current focused node. Inspect that
node's action binding or text-input metadata when deciding whether a shortcut
should yield to the control. This context includes focus changes from earlier
events in the same batch. Button activation bindings do not produce focus
actions, so do not infer keyboard ownership from application-side focus history.
Give focused editors their selection, clipboard, undo/redo, and navigation keys
before dispatching application shortcuts. Keep unrelated commands available;
a blanket check for any modifier key is too broad. Let the editor handle Escape
while composing before using it to dismiss a palette or dialog.

Browser DOM shortcut listeners must respect the same ownership. A document-level
listener can dispatch an application command before Operad sees the key, even
when the hidden IME editor has focus. Yield editing and composition keys there
as well, or handle application shortcuts through the shared runtime hook.

## Background work

Use `runtime::task_channel` and `RuntimeHooks::with_task_completions` to deliver
typed results from jobs to application state. Sending wakes the host, even when
the UI is idle. The host applies results on the UI thread before computing frame
metrics or building the view. No tick action or continuous repaint is needed.

```rust,ignore
use operad::runtime::{task_channel, RuntimeHooks};

let (sender, receiver) = task_channel(1);
let hooks = RuntimeHooks::new().with_task_completions(
    receiver,
    |state: &mut App, result: Result<String, std::io::Error>| {
        state.text = result.unwrap_or_else(|error| format!("Could not load: {error}"));
    },
);
std::thread::spawn(move || {
    let result = std::fs::read_to_string("notes.txt");
    // This worker sends only one result into a capacity-one channel. The only
    // possible send error is that the application has already closed.
    let _ = sender.try_send(result);
});
// Attach hooks to Application::new(state, update, view).with_hooks(hooks).
```

Operad owns delivery and view invalidation; the application chooses its executor.
On native platforms, senders can cross threads when their messages are `Send`.
Application state and completion handlers can still use `Rc` and other types
that stay on the UI thread. In a browser, use the sender from a local future or
JavaScript callback. A Web Worker can forward results through a callback on the
application's browser thread. CPU-heavy work must run off that thread to keep
the UI responsive.

Each channel has one receiver and a positive capacity. `try_send` does not wait
for space: `TaskSendError::Full(message)` returns the result for the application
to retry or handle as overload. `Closed(message)` means the receiver was dropped.
Accepted messages are delivered once in channel order unless the receiver closes
first. There is no ordering guarantee between channels. Wakeups coalesce while
messages are pending; attaching a host also wakes it for queued startup results.
The runtime snapshots a finite batch from every channel before invoking handlers.
Messages sent by a handler wait for a later batch, and empty batches do not
invalidate the retained view. Keep handlers short; move expensive work into jobs.

Dropping the hooks closes their receivers and discards queued results. It does
not abort external work. Workers can check `sender.is_closed()` to stop early,
but must still handle send errors. For cancellation or superseded requests,
include a `tasks::TaskHandle` in the result. The application-owned `TaskRegistry`
can reject it before changing state:

```rust,ignore
if state.tasks.complete(&handle, "loaded").disposition.applied() {
    state.document = loaded;
}
```

Use `TaskRegistry::fail` for failures and `cancel` to reject a cancelled job's
late results. Starting the same task ID again advances its generation, so an
older completion cannot replace the new result. Cancellation of the actual
worker remains the application's responsibility.

Custom hosts install `hooks.set_task_waker(...)` to schedule a host event. The
waker may run on a producer thread on native platforms; it must not access state
or drain the hooks. On the host thread, call
`session.apply_task_completions(&mut hooks, &mut state)` before calculating
metrics and building the next document. A nonzero return value means a batch
was applied and the view was invalidated; request a frame if draining outside
the frame callback. Built-in native and web runners do this automatically.

## Input and lifetime

### Text composition

Editable text widgets publish a `TextInputSnapshot` on their document node.
The runtime activates the focused editor's input method, updates its surrounding
text and candidate rectangle, and routes `TextCompositionEvent` through the
editor's ordinary `WidgetActionKind::TextEdit`. Apply that action with
`TextInputState::apply_widget_text_edit`, using the editor's `TextInputOptions`.
Handle `WidgetActionKind::Focus` to update the focus styling passed to the view.
An application does not need to issue IME activation requests for these widgets.
Keep each `TextInputState` in application state across view builds. Constructing
one from a string on every build discards its caret, selection, composition, and
undo history. Apply its edit outcome to the domain value when appropriate; use
explicit text replacement when an external change resets the editing session.

Search fields can use the retained editor's `text()` as their filter query,
leaving composition previews out of filtering until commit. Keep an active
search field visible when its query becomes empty: removing it or collapsing
its panel during Cut prevents the next Paste or Undo from reaching the editor.
Library search state can outlive a music project or other independently replaced
domain object; reset it when its own view or query session is replaced.

For names and other drafts committed with Enter, keep the editor tied to its
entity and define what leaving the field does. A focused-draft policy can discard
the draft on blur or Escape while retaining it across ordinary frame rebuilds.
Cancel an active composition before discarding the surrounding draft. Compare a
normalized, nonempty value with the domain before starting a history transaction;
a no-op commit must not clear redo. Undo/redo that replaces entity snapshots is
also an editor-lifetime boundary, including deletion followed by restoration of
the same IDs and names before another frame is rendered.

When replacing the owning document or project, discard its editor drafts and
undo history. Give replacement editors new node identities, for example by
including a project generation in their names. Reusing a node name and action
binding can preserve the old input-method session even when the model was reset
and the displayed value is unchanged. Action IDs can remain stable for command
routing. Advance clipboard lifetime tokens as well so delayed replies cannot
edit a replacement model. A rejected load that leaves the domain intact should
preserve the current editing session.

Synchronize the domain value only when an edit changes text or explicitly
commits or cancels it. Caret navigation, selection, and clipboard reads must not
reformat or replace the editing model. Pointer release finishes a selection
gesture; it does not commit the field's text.
Document-aware action routing emits text edits for text-field pointer gestures,
without an additional button activation for the same edit binding.
This also applies to read-only and selectable text: selecting or copying their
content does not activate their binding as a button. A `TextBox` or `SearchBox`
role identifies a text control even without an IME snapshot; publishing a
`TextInputSnapshot` also identifies an editor for text and focus routing.

Text fields retain the width assigned by their layout. Their content scrolls
inside that viewport instead of increasing the minimum width to fit the whole
value. Focused fields reveal the caret when its position or the viewport changes;
multiline fields also reveal it vertically. The runtime retains their scroll
state across document rebuilds. Pointer selection and IME cursor placement use
the scrolled content geometry. Set `TextInputOptions::focused` from the editor's
focus state so its caret is painted and revealed.

Drive persistence from changes to application data, not from the presence of a
text-edit action. `TextInputOutcome::changed` describes editor text, which may
still be an uncommitted draft. Applying a draft, moving the caret, copying text,
or updating a composition should not serialize and save the entire project.
Report successful domain changes from your application update logic, including
any selection changes that your project format retains, and save those results.

Command palettes use this same editor for their query. Forward the entire routed
edit to `CommandPaletteState::apply_widget_text_edit(items, edit, options)` with
the `CommandPaletteOptions` used by the view; dropping its pointer coordinates
prevents caret placement and drag selection. Update `options.focused` from focus
actions, and forward `outcome.edit` clipboard requests to your platform service.
Deliver pasted text through the same edit path. `outcome.selected` and
`outcome.closed` remain the command-selection and dismissal signals.

Use `PlatformServiceClient` to allocate clipboard requests and match each paste
to its returned request ID. Accept a reply only while the intended editor session
is still current. Closing, replacing, or editing that field must revoke a pending
paste if the reply would otherwise target a different selection or query. Ignore
unrelated, duplicate, and obsolete replies; an empty clipboard adds no text.
Apply composition cancellation to the old editing model without restoring that
model's focus: the callback reports cleanup, not a new focus request.

Both native and web hosts require the application to connect
`with_platform_service_requests` to its client's `drain_requests` and deliver
`with_platform_responses` to that same client. Returning a clipboard action from
an editor alone does not perform the service request. Drain accepted responses
and apply a valid paste through the ordinary text-edit path; keep request matching
and editor-lifetime checks shared between platforms.

The built-in native host retains its clipboard connection until the event loop
exits. Custom Linux hosts must also keep their clipboard owner alive between
requests: opening and dropping a connection for each operation can discard the
copied text before it is pasted. Persistence after the application exits depends
on the desktop's clipboard manager.

Palette Up/Down move through results; Home/End move the query caret. An active
composition owns these keys, Enter, and Escape. Drafts are displayed inline, but
matching uses `query()` (committed text) until commit. `set_query` starts a fresh
editing session and clears its previous undo history.

Preedit changes `display_text()` and `composition()`, while `text()` and undo
history remain unchanged. A commit replaces the original selection as one edit;
cancel discards the draft and preserves that selection. Composition selection is
relative to the draft, and `None` hides its caret. All public offsets in
`TextCompositionEvent`, `TextInputSnapshot`, and `TextRange` are UTF-8 bytes;
browser adapters convert the DOM's UTF-16 offsets. Explicit replacement ranges
refer to committed text. Built-in controls paint draft selection and underlines,
including multiline and masked password drafts.

Pointer selection hit-tests the displayed draft, then cancels composition
without changing committed text or undo history. A point in the unchanged
suffix maps back past the original replacement; a point inside the discarded
draft maps to its replacement start, snapped to a grapheme boundary. Direct
event helpers and routed widget actions use the same mapping, including masks.
The direct helper emits an IME update when cancellation alone changes the state.

Input sessions use opaque IDs independent of document indices. Reordering an
editor preserves its active composition. A commit delivered before a focus
change still belongs to the original editor; the OS may commit when it blurs.
Later events for that session cannot edit the new field. Normal focus loss emits
a text-edit cancellation. For runtime-owned sessions, a pointer selection during
composition also revokes the old session, even when focus stays in the same field.
Escape and IME cancellation do the same, while delivering the cancellation to
the original editor. A fresh ID accepts subsequent input; queued commits for
the canceled ID are rejected. This ordering
holds within a batch and across frames, including empty preedit before commit.
`HostInteractionState::text_composition` tracks this input lifetime separately
from the last rendered snapshot. `HostTextCompositionState` distinguishes an
inactive composition, a nonempty preedit, and an empty preedit that may still
commit. Initialize the field with `Default::default()` in an explicit state
literal; activation helpers and event routing maintain it.
If the application clears a previously published draft without a routed commit
or empty preedit, preparing the new document revokes that session before queued
input is processed. Ordinary layout changes preserve it, as does publishing the
result of a valid commit. Project/model replacement still needs the explicit
identity policy described above; a snapshot does not describe an unseen reset.
Hosts managing explicit input IDs remain responsible for renewing them when
composition is canceled and for applying the direct helper's platform requests.
Removing, disabling, or changing a field's action
binding instead reports its original semantic binding through
`with_interaction_cancelled`; apply its `TextEdit`
cancellation to the application's editing model, even when that model is disabled:

```rust,ignore
RuntimeHooks::new().with_interaction_cancelled(|state: &mut App, cancellation| {
    if let WidgetActionKind::TextEdit(edit) = cancellation.kind {
        let editor = state.editor_for_binding(&cancellation.binding);
        editor.input.apply_widget_text_edit(&edit, &editor.options);
    }
})
```

Custom editors attach a snapshot with `UiNode::with_text_input` or
`set_text_input`, and handle composition actions themselves. The caret rectangle
is local to that node in authored UI coordinates. An editor rendered by a child
scene calls `set_text_input_content(Some(TextInputContent { node: content_node,
text_style, mask: None }))` on its focus/action owner; its caret then uses the child's authored
coordinates and displayed font metrics. Built-in fields do this
automatically. Cached view sections remap this document-local reference.
Runtime pointer actions include `WidgetTextEdit::geometry`, with the pointer and
content bounds in the same authored coordinates, plus the displayed text style
and mask. Password fields publish their mask separately from IME sensitivity:
pointer edits measure the mask glyphs and map back to original UTF-8 offsets,
preserving grapheme boundaries. Direct input helpers use the same mapping and
keep platform caret geometry and sensitivity consistent with the displayed field.
Forward the complete edit to `apply_widget_text_edit` so padding, UI scale,
animation, and font metrics match the frame that received the pointer. In
particular, a focus handler may change the next frame's font before applying
the queued pointer edit. Interaction policy still comes from the application.
Stationary moves and releases do not emit another selection edit. The runtime
still ends pointer capture; text-edit pointer phases are not an application
drag-lifetime protocol. A release at a new position updates the selection even
when no intervening move event arrived.
The raw pointer fields remain available for application-level handling. Custom
callers supplying explicit layout metrics to the direct input helper provide
both those metrics and pointer events in the same coordinate system.
The default text field uses its scene's six-unit content inset without extra
layout padding. Custom layout padding is additional space; size the field to
accommodate that padding and the text line.
Scene primitives scale with `UiDocumentScale::ui_scale` around their layout
origin, matching scene intrinsic measurement. Layout boxes already include UI
scale; platform DPI conversion is separate. The runtime accounts for layout,
UI scale, paint transforms, and clipping; the host converts to physical native
coordinates or browser CSS coordinates. Candidate-navigation keys belong to the
input method while composing and do not also trigger application shortcuts.
Editors that explicitly manage `TextImeRequest` sessions retain control of their
snapshots. Custom hosts call `RuntimeSession::apply_text_ime_request` before
executing these requests so subsequent composition routes to the focused editor;
the built-in hosts do this automatically.
Runtime-owned sessions follow both node identity and action binding. Changing
the binding starts a fresh session before further composition can be accepted;
queued events for the former session are discarded. Text, selection, and layout
updates with the same owner retain the session.
On the web, switching between plain and password input replaces the browser's
editable element. An active composition is canceled before replacement; its
committed text and original selection remain available for further editing.

The native runner supports the preedit/commit lifecycle exposed by winit 0.30.
That interface does not expose synchronous surrounding-text queries or arbitrary
OS replacement-range callbacks. Those deeper native text services remain outside
this implementation. The browser uses a focused textarea (password input for
sensitive text), native composition events, and surrounding-text selection.
See `tests/fixtures/runtime_ime.rs` for a portable application and cleanup hook.
Composition support does not add font coverage: the browser's bundled font set
does not contain CJK glyphs, which currently render as missing-glyph boxes even
though their text and editing ranges are preserved. Native hosts can use system
fonts; application-owned renderers and measurers accept `FontLibrary` data.

Input hooks receive normalized events before ordinary widget dispatch. Returning
`true` consumes an event. Consuming a keyboard press also suppresses its paired
text event. Keyboard hooks run before canvas keyboard hooks. Mutable input callbacks
invalidate the view; immutable frame observations do not. Frame preparation and
platform callbacks can explicitly report unchanged view inputs as described above.

Canvas capture belongs to the stable path of node names. Give siblings unique
names and preserve them when reordering. A captured pointer remains routed to
its canvas outside the canvas bounds, across rebuilt documents, and when the
canvas hook consumed the press. `CanvasInput::local_position` accounts for the
canvas's effective paint transform. Browser hosts also capture the DOM pointer
so release outside the element reaches the runtime.

Layout, runtime retention, and input hooks share the document's cached identity
index. Targeted setters for style, content, and interaction preserve that index;
`node_mut` and `edit_node` invalidate it because they can replace a node. Tree
changes invalidate it automatically, including added sections and removed nodes.

Hit testing, painting, wheel routing, and modal selection share a cached stacking
order. Structural and style edits invalidate it, including appended sections and
removed runtime decorations. Geometry, clipping, transforms, modal availability,
and input eligibility still use current node state, so scrolling, animation, and
enabling or disabling controls take effect without rebuilding the order.

Point hit queries borrow the node's shape and use its current transform and clip
without creating an owned geometry snapshot or copying shader data. Ordinary
queries avoid collecting diagnostic rejection lists; detailed reasons remain
available through the effective-geometry diagnostics.

Canvas coordinate conversion and IME cursor placement read the target node's
transform and clipping directly, without constructing the document's full
geometry list. Captured pointers keep their coordinates outside the canvas;
noninvertible transforms produce `None` for `CanvasInput::local_position`.

Retained fallback capture identifies a specific node and canvas key. Input
channel permissions come from that owner's current `CanvasInteractionPolicy`,
so disabling or enabling keyboard and wheel input takes effect before the frame
finishes. Replacing a canvas with other content ends its fallback ownership;
an ancestor with the same surface key cannot inherit it. Newly introduced owners
acquire their fallback capture when their frame finishes; direct pointer hits
and keyboard focus use the current document immediately.

Canvas pointer lock is shared by the host window. The runtime requests lock and
hides the cursor when the first eligible canvas needs it, and releases lock and
shows the cursor when the last owner leaves. Resizing, reordering, modal changes,
and replacing one lock owner with another do not cycle the cursor lock. Removed
owners stop receiving input during preparation; cursor requests are reconciled
against the complete frame's capture plans in `finish_frame`.

Custom hosts use `CanvasHostCaptureState::sync` and the resulting transition's
`platform_requests` or `platform_service_requests` for this lifecycle. A
transition retains aggregate cursor changes separately from its per-canvas
`changes` list; build it through `sync`. `RenderFrameRequest::canvas_platform_requests`
provides initial acquisition requests only.

Canvas and image discovery on `RenderFrameRequest` visits nested
`PaintCompositorLayer` children in paint order. Render registries, embedded canvas
programs, capture plans, and initial pointer-lock requests therefore see the same
canvas instances. Requests preserve each child's paint-space geometry; the backend
applies outer layer transforms, opacity, and clipping during composition. Adding a
layer around an unchanged canvas does not replace its capture owner.

Use `CanvasRenderProgram::uniform_bytes` for values that change each frame, such as
time, rotation, or camera parameters. The WGPU backend binds the data at WGSL
`@group(0) @binding(0)` and reuses the pipeline when the bytes change. Pack the bytes
to match the shader's uniform layout; buffer allocation is padded to 16-byte
multiples. `constant` supplies specialization constants and can compile a new
pipeline for each new value, so reserve it for shader variants.

For a caller-owned WGPU target, construct `WgpuRenderer::with_device_queue` with
the target's device and queue, then use `render_frame_into_view_with_encoder`.
Image uploads, embedded canvas programs, composited layers, and the UI draw are
recorded into that encoder. Several frames can be recorded before submission;
their geometry and text remain specific to each pass. Submit dependent resource
updates in recording order, including when using separate encoders. Discarding
an encoder also discards its uploads, so resupply those updates before relying
on their contents. Native and web runners submit frames directly.

A pointer gesture belongs to the button that started it. Additional button
presses and releases still reach raw input hooks, but cannot replace or finish
that gesture. A click activates only when released over its original target and
within the configured drag threshold of the press position. A release at or
beyond that threshold cannot click even when move events were omitted or
intercepted. An active drag continues to receive its release outside that target.

The document's active widget press also belongs to one pointer. Other pointers
remain available to custom input hooks, including canvas interactions, without
replacing or canceling the widget's owner. Text selection follows each event's
press owner, so batching a drag and release into one frame preserves selection.

Removing or disabling the owner, or placing it outside the active modal, ends
its edit. Canvas hooks receive a pointer cancel; `node` is `None` when there is
no valid current owner. Widget edits use
`with_interaction_cancelled`, which provides the original action binding and a
cancel-phase action kind with the last delivered geometry. These callbacks let
applications roll back or finish their own transactions without old node IDs.

`UiNodeId` belongs to one document. Use it only while borrowing that document;
store domain IDs or action bindings in application state.

## Hit testing

`HitTestBehavior::Auto` follows node input capabilities. Disabled interactive
controls block content behind them while producing no control action. Ordinary
noninteractive decoration passes through. `Block` creates an explicit input
barrier; `PassThrough` excludes that node while allowing its children to retain
their own policies. `hit_test_result` distinguishes a target from a blocker;
`hit_test` returns only an actionable target.

Use `UiDocument::set_node_enabled` to disable a subtree without deleting its
input capabilities. Re-enabling restores them and preserves independently
disabled descendants. Enabled scroll ancestors can still scroll when a wheel
is over an automatically disabled child. An explicit `Block` stops that wheel.

Wheel gestures do not activate widget action bindings. They scroll eligible
containers and remain available to custom input hooks. Applications that
deliberately map wheel input to an action can do so in their input handler.

Ordinary controls use `WidgetActionMode::Activate`, which accepts primary clicks
alongside keyboard, accessibility, and programmatic activation. Middle, right,
and other pointer buttons do not activate them or report a document click.
Custom controls can opt into `WidgetActionMode::ActivateAnyButton` with
`UiNode::with_action_mode`. Those activations preserve the button and modifiers
for the application to interpret. When migrating custom secondary-button actions,
set this mode on their action owner; ordinary controls need no application filter.

An active modal limits pointer targeting and wheel scrolling to its subtree,
even without an authored scrim. Background hits return `HitTestResult::Blocked`
with the modal as the blocker. Action propagation and scroll chaining stop at
the modal boundary. Opening a modal also ends background widget, scrollbar, and
canvas captures. Background canvases continue painting, with interaction
disabled in the emitted paint list so hosts cannot reacquire their capture or
pointer lock. Closing the modal restores eligibility under the authored policy;
it does not resume canceled drags. Disabled canvases use the same paint policy.
Application input hooks still receive raw events before document dispatch.

Use `RuntimeHooks::with_pointer_observer` for application hover previews and
cursor choices that need the runtime's current hit and capture owners. Its
`PointerInput` supplies the raw event and borrowed `hit` and `captured` nodes;
action-bearing nodes resolve to their logical action owner. Hits respect disabled
controls, blocking layers, and modal scope. Capture remains available outside the
owner's bounds and is established after observing the initial press. References
last only for the callback. Updating state conservatively invalidates the view.

The observer cannot consume events or replace capture. Keep clicks and drags in
widget actions, and use `with_canvas_input` for canvas interception. In particular,
avoid a second DOM pointer listener that starts gestures before control ownership
is resolved, or an overlay that replaces action bindings during a drag. Stable
widget identity preserves the runtime's original owner across rebuilt views.

Automatic scrollbars own their pointer interaction independently of the
container's widget action. A thumb keeps its starting axis and grab position
across frames and document rebuilds; its capture ends on release, cancellation,
or loss of a usable scrollbar. Canvas hooks continue to receive ordinary canvas
input while scrollbar drags remain with the document.

`HostFrameOutput::events` keeps each document event paired with its raw gesture.
The runtime applies each event before routing the next one, so a click or canvas
hook after a wheel event sees the scrolled geometry. Unchanged layout uses the
cached path. Each entry's `document_result` stores its input result and actions;
scroll offsets, pointer coordinates, and edit cancellations therefore retain the
values delivered at that event. `UiInputResult::scrollbar_target` suppresses widget
actions for scrollbar input even when the offset does not change.

Use `ui_events()` and `gestures()` to inspect each stream and
`frame.input_results()` to inspect document results. Custom hosts should retain
the pairing; standalone synthetic events can be appended with
`output.events.push(event.into())`.

For host-processed pointer releases, a paired `GestureEvent::Click` must match
the document's pressed and hit target before `UiInputResult::clicked`, widget
activation, or the `activated` animation input can fire. Drag completion and
rejected clicks still release the press and finish text selection or scrollbar
movement. A bare `UiInputEvent::PointerUp` in a host frame does not imply a click;
custom hosts can use the raw-input processing helpers to derive its gesture.
Direct `UiDocument::handle_input` uses matching press/release hits without gesture
recognition.

## Keyboard navigation

Tab moves to the next eligible control and Shift+Tab moves to the previous one.
Traversal follows accessibility focus order, stays inside the active modal, and
wraps at either end. The active modal is the topmost visible, enabled dialog in
paint order, including inherited layers and stacking parents. Hiding or disabling
it exposes the next eligible dialog. Controls can declare focusability through
either input behavior or accessibility metadata; both survive frame boundaries.

For programmatic focus, use the target node returned by the widget builder in
the current document, then set `UiFocusState::focused` through
`UiDocument::set_focus_state`. An action ID is a routing key, not a node name or
a unique widget identity. For example, a command palette's search action can be
`commands.search` while its input node is named `palette.input`. Looking up the
action string as a node name can silently leave the previous field focused.
Resolve the intended control explicitly, and never retain its document-local
node ID across rebuilds.

`HitTestBehavior` controls pointer and wheel routing independently of keyboard
focus. An eligible control with `PassThrough` or `Block` retains authored or
Tab-acquired focus across preparation and rebuilds, including an active text
composition. Disabling the control still ends its focus lifetime.

The session moves focus into a newly active modal before routing input. It keeps
an explicit focus request inside that dialog; otherwise it chooses the first
eligible target in its focus order, which can be the dialog container itself.
Enter or text input cannot reach a background control through retained focus.
Canvas keyboard capture is also restricted to the active dialog, including
fallback capture and canvases that contain a dialog in their subtree.

Pointer focus stays inside the active dialog too. Clicking the blocked background
preserves its current eligible focus and any active text composition. A press
inside the dialog focuses the hit control or its nearest focusable ancestor,
up to the dialog itself.
If there is no such target, the current eligible focus is retained, or the first
eligible target in the dialog is selected. A dialog without focusable targets
leaves focus empty. Outside an active modal, blank-space clicks still clear focus.

For dialogs configured with outside-pointer dismissal, the modal dismissal
helpers act on `UiInputEvent::PointerDown`. Releases, moves, and cancellations do
not request outside dismissal. Disabled controls and portal descendants inside
the dialog count as inside, and the dialog's visible transformed shape determines
its bounds. Background dialogs cannot dismiss while another modal blocks them.
Pass the matching `UiInputEvent` as the final argument to
`modal_dialog_dismiss_event_from_input_result`, along with its `UiInputResult`.
Use the document layout in which the event was handled. The helper handles close
button activations and outside presses; the result alone cannot distinguish
an outside press from a wheel event or movement over the modal barrier.

Closing a modal restores the control that was focused before it opened. Nested
dialogs retain separate return targets by stable identity. Set
`ModalDialogOptions::with_focus_restore` or `AccessibilityMeta::restore_focus` to
choose `FocusRestoreTarget::None` or a specific node instead. Explicit focus
requests on close take precedence. Missing, ambiguous, or ineligible return
targets are discarded; an underlying active modal still receives focus.
Preparation-time text-field focus notifications precede the frame's input
actions, including on frames without keyboard or pointer input. Custom hosts
using only `UiDocument` receive the keyboard guard; `RuntimeSession` owns entry
and restoration across document lifetimes.

With no current focus, forward traversal starts at the first
control and backward traversal starts at the last. Ctrl, Alt, or Meta combined
with Tab remains available for application shortcuts.

This behavior lives in `UiDocument::handle_input`, so native, browser, and custom
session hosts share it. Keyboard and canvas hooks run first; consuming the key
prevents default navigation. Navigation emits text-field focus transitions without
sending Tab as an edit to the newly focused field. The direct text-input helper
also navigates on Tab, while an active composition retains its candidate keys.

## Accessibility visibility

`UiDocument::accessibility_snapshot()` excludes a node and its descendants when
its layout style uses `Display::None`, even before the next layout pass. Controls
that are merely clipped or scrolled out of view remain in the accessibility tree.
Excluded nodes do not participate in its navigation or live-region announcements.

Explicit `labelled_by` and `described_by` references can still use hidden semantic
metadata. The snapshot resolves that text into the visible control's label or
hint when any referenced target is excluded, and removes the corresponding links.
Other published relations contain only exported targets. Authored metadata stays
intact, so showing the target again restores its links; missing authored targets
still produce audit warnings. `AccessibilityMeta::hidden` continues to exclude
only its own semantic node, allowing children of structural wrappers to remain.

## Keyboard text

Text-input arrow movement and pointer placement use extended grapheme clusters,
so a base character with combining accents or a joined emoji is one navigation
unit. Without a selection, forward Delete removes the containing cluster. `TextInputPosition::column`
counts graphemes; its `byte_index`, explicit selections, and IME ranges still use
UTF-8 byte offsets. Backspace and IME requests to delete surrounding code points
can remove individual components, such as a combining accent. These operation
boundaries follow the distinction in [Unicode text segmentation](https://www.unicode.org/reports/tr29/#Grapheme_Cluster_Boundaries).

Put text generated by a key press on `RawKeyboardEvent::with_text(text)`.
Native and web hosts do this automatically. Keyboard and canvas hooks receive the
key with its text; consuming that event suppresses both. Send text without an
associated normalized key through `RawTextInputEvent`, and keep IME composition
on `RawTextCompositionEvent`. Independent text is never discarded because it
shares a timestamp with a consumed key or follows Enter.

`RawInputEvent::to_ui_input_events()` and
`to_ui_input_events_with_wheel_scale(...)` return an ordered iterator: a press
can produce a key event followed by printable text. Release events produce no
document input. Control characters such as Enter's newline are handled by the
key event, and Ctrl/Meta shortcuts do not insert their attached text.
`RawKeyboardEvent` owns its optional text and implements `Clone`, rather than
`Copy`. Replay step reports keep `converted` and `results` vectors so both
parts of a key press remain under the original step label.

## Measured text fitting

For bounded single-line labels, use `TextStyle::default().ellipsis()`. Layout
and WGPU fit the text with the active measurer and font system; the document
keeps the original text for accessibility and inspection. The default remains
clipping. When the application needs the fitted string, use
`fit_text(&mut measurer, &text_content, width)` or
`CosmicTextMeasurer::fit_text`. The result includes text, size, and whether it
was truncated. An approximate measurer gives an approximate fit; use the actual
font measurer when matching rendered text matters.

Fitting preserves grapheme clusters and uses a logical-end ellipsis. It is
single-line fitting, not multiline clamping. If even the ellipsis cannot fit,
the visible string is empty. Nonmonotonic font advances can yield a conservative
prefix, but the measured result stays within the requested width.

## Observing frames

`with_frame_observer` receives immutable application state plus a borrowed
`RuntimeObservation`: metrics, the laid-out document, and the complete
`HostDocumentFrameOutput` about to be submitted. It runs after the host's action
and rebuild pass, without another view call or layout. The document is the
source for geometry and accessibility; `paint()` returns the same paint list
as the frame's render request. Clone only the information an asynchronous
consumer actually needs.

Publish control geometry, text audits, and visual snapshots from this callback.
Pass its borrowed document and paint list through reporting helpers instead of
calling `view`, `compute_layout`, or `paint_list` again. A separate diagnostic
document repeats frame work and can disagree with the displayed UI's scale,
font measurements, focus, or transient decorations. Apply paint transforms to
both text bounds and estimated glyph geometry when deriving an overlap report.
`PaintItem::clip_rect` is already in final paint coordinates: do not apply the
item's content transform to it. The renderer applies only the separate target
or DPI conversion to that clip. SVG exports must also clip text, including scene
text, instead of letting labels escape their scroll containers.

For zooming an entire application viewport, use `RuntimeHooks::with_scale_factor`
and include display density in the returned factor. Both hosts divide physical
dimensions by that factor before calling the view, and use the same factor for
presentation and input conversion. Keep the document's style scale at one when
using this approach. `with_ui_scale` scales authored lengths and text; it does
not turn a fixed-size root into a responsive viewport. In browser integrations,
custom DOM input uses CSS pixels: divide positions and pixel wheel deltas by
`metrics.scale_factor / metrics.dpi_scale` before applying document-space logic.
Multiply document coordinates by that ratio when publishing browser automation
targets. Line and page wheel deltas are counts, not CSS distances.

The callback runs before submission, which can still fail. It does not certify
that pixels reached the screen. Resource acknowledgement remains tied to
`RuntimeSession::frame_presented`; failure uses `frame_failed(now)`, with elapsed
monotonic time at failure. Temporary presentation failures retry after 16 ms,
doubling to a 250 ms cap until a frame succeeds. Custom hosts must honor
`frame_retry_delay(now)` before attempting presentation, including redraws
requested by input, animation, or application ticks. If input must be processed
earlier (for example, to synchronize surrounding text before composition),
prepare and retain the document, then call `frame_deferred()` without submitting
to the renderer. This retains uploads and work without extending the retry wait.
Renderers returning `RenderError::SurfaceUnavailable` must do so before applying
the request's resource updates. The retained sequence may include patches and
full replacements that change size or format; replaying an already-applied
sequence can make its earlier patches invalid. Operad's surface renderer acquires
the presentation texture before mutating those resources.

For a complete image whose latest state replaces earlier unpresented states,
call `document.set_resource(descriptor, pixels)`. If an image was decoded with
`ResourceUpdate::from_encoded_image`, pass its `descriptor` and `bytes` fields.
The session releases superseded pending pixel buffers while keeping patches
queued after the latest snapshot. It also preserves snapshot intent when a
cached view section is rebuilt or reused. The retained snapshot is validated
by the renderer; superseded snapshots are never submitted or validated.
Use `add_resource_update` for ordered write or version protocols where every
update must be delivered. A later explicit snapshot supersedes earlier pending
writes for its handle. Intervening different handles with the same textual key
preserve the sequence across that boundary. Custom hosts obtain these lifetime
rules through `RuntimeSession::build_document` or `prepare_document`.

Application-owned hosts use `process_input_with_hooks` for the same ordered
routing and `reconcile_input_hooks` after a rebuild to deliver owner cleanup.
`process_input` and `process_input_with_hooks` take a mutable document and a text
measurer, and return a layout `Result`. Pass their output to `finish_frame` with
the same document; it completes layout, paint, and accessibility without replaying
input. `collect_document_widget_actions(&frame)` returns the actions captured
during input processing and does not need the final document. Application updates
and their view rebuild still occur after the input batch.
If cleanup invalidates state, rebuild before final layout. Invoke
`hooks.observe(state, RuntimeObservation::new(metrics, document, frame,
session.view_build_stats()))` before submitting the final frame. Retain that
document with `retain_document`;
`session.document()` then makes it available for read-only inspection between
frames. Inspection must not call the application's view or prepare a replacement
document.

## Headless scenarios

With `test-support`, `ScenarioHarness` routes replay input through the shared
gesture and document-input processor before building a frame. Raw events keep
their pointer IDs, buttons, timestamps, and wheel units. UI-only pointer helpers
represent the primary mouse button; their synthetic clock advances one millisecond
per step and continues between frames. Set `replay_time_millis` to model a delay,
or use raw events for exact timing and pointer metadata.

Each replay step reports only its accepted UI events and actual document results.
Ignored releases have no input result. Scrolling and resizing affect subsequent
events in the same replay, and `raw_scaled` keeps its explicit wheel conversion
independent of the layout viewport. `EventReplay::run` remains a lower-level
document event replay without host gesture recognition.

## Migrating from 9.x

- Replace native/web hook and metric types with `runtime::RuntimeHooks` and
  `runtime::RuntimeMetrics`. Native monitor-based initial sizing moves to
  `NativeWindowOptions::with_initial_size`.
- Add the third `&mut ViewContext<'_>` argument to view callbacks, or wrap an
  existing two-argument view in `|state, viewport, _| state.view(viewport)`.
- Keyboard callbacks receive `KeyboardInput`; its `event` is a `RawKeyboardEvent`
  with Operad key codes and modifiers. Backend event structs stay inside the
  host. Canvas callbacks use `CanvasInput` and raw movement uses `RawMouseMotion`.
- Import `EmptyResourceResolver` from `operad::renderer` in production rendering
  and screenshot code. `operad::testing` requires the `test-support` feature;
  ordinary rendering does not need it.
- Exhaustive `ScenePrimitive` matches must handle `MorphPolygonKeyframes`.
  Coordinate conversion must transform every point in every keyframe, while
  retaining the interpolation amount and styling.
- Exhaustive `RenderError` matches must handle `SurfaceUnavailable`. It signals
  a temporary presentation failure; the hosts retain uploads and retry.
  `Backend` failures terminate the native runner or browser session instead of
  retrying indefinitely. Browser shutdown releases input and pending work while
  preserving the close-request hook; reloading the page restarts rendering.
  Native device loss also ends the runner with a structured `DeviceLost` report.
- `RuntimeSession::frame_failed` now takes the monotonic failure time; use the
  same time origin as `begin_frame`, measured after the unsuccessful attempt.
- Handle `UiInputEvent::PointerCancel` in exhaustive event matches. It clears
  pointer interaction without clicking or committing an edit.
- `RuntimeMetrics::elapsed` is monotonic time since that host started. Do not
  compare it with the browser's `performance.now()` origin.
- Replace application-side rebuilds for geometry or diagnostic snapshots with
  the frame observer. Replace pointer-routing rewrites with stable names and
  runtime capture, and close editing transactions on cancellation.
- Browser closing follows `beforeunload` restrictions. A veto requests the
  browser's confirmation behavior; browsers decide whether to show it.
