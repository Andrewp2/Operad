# Operad 5.0 Core Concepts

Operad `5.0.0` defines a backend-neutral UI contract layer. The public v5
surface is meant to let applications describe layout, input, state, rendering
intent, accessibility, diagnostics, and resource lifecycles without binding
product code to one renderer or native host.

This document is a concept/reference map. It complements the migration guide,
theme stability notes, and completion audit:

- `docs/v5_0_migration_guide.md` explains how to move v4 consumers onto v5.
- `docs/v5_0_theme_and_stability.md` records theme and token stability policy.
- `docs/v5_0_completion_audit.md` tracks which roadmap gates are done, partial,
  or still pending.

## Contract Model

V5 separates reusable UI contracts from backend execution:

- Applications own product state, domain commands, persistence, networking,
  platform policy choices, and final business semantics.
- Operad owns reusable UI-side records for layout, action routing, retained
  widget state, transactions, forms, overlays, accessibility, resources,
  diagnostics, and renderer-neutral frame output.
- Hosts and renderers own concrete event loops, windows, surfaces, GPU or CPU
  resources, OS clipboard/IME/cursor services, platform accessibility adapters,
  and scheduling integration.

The stable v5 APIs are the records and lifecycle rules that cross those
boundaries. Backend-specific APIs remain public where needed, but consumers
should keep them behind local adapters.

## Core Concepts

### Layout

Use `operad::layout` for new public layout construction. `Layout`,
`LayoutDimension`, `LayoutInsets`, `LayoutSpacing`, alignment, display,
position, and flex records provide an Operad-owned facade for common cases.
`LayoutStyle` and Taffy conversion paths remain available for migration and
advanced layout behavior.

### Identity And State

Widget trees may be rebuilt each frame. Stable widget keys, scopes, retained
state slots, lifecycle reports, keepalive, expiry, and invalidation records
define which transient state survives those rebuilds. Focus, overlays, scroll,
animation, edit state, and cached measurements should be tied to stable identity
rather than to short-lived node allocation.

### Input, Actions, And Commands

Input routing is backend-neutral. Pointer, keyboard, touch, stylus, and gamepad
events are normalized into Operad records before widgets or host logic interpret
them. Widget actions and command descriptors represent UI intent such as
activation, selection, preview, commit, cancellation, open/close, drag phases,
and focus changes. Applications map those records to product behavior.

### Transactions And Selection

Transactions define preview, update, commit, and cancel boundaries for text,
selection, drag, slider, table, and property edits. Selection models provide
shared single, multi, range, active-item, anchor-item, and roving-index
semantics. Undo/redo integration should use committed transaction boundaries,
not every high-frequency input update.

### Forms And Async Work

Form state tracks dirty, pending, validating, submitted, applied, canceled,
reset, and accessible error-summary behavior. Async task records track progress,
completion, cancellation, errors, stale generations, and repaint invalidation.
Hosts still own executors and product validators; Operad owns the UI-side state
machine and reporting contracts.

### Overlays And Navigation

Overlay records centralize popup, menu, dialog, command palette, modal,
non-modal, nested, dismissal, z-order, and focus-restore behavior. Navigation
records define roving focus, active descendants, collection kind, boundary
behavior, and activation/dismissal semantics for menus, listboxes, tabs, trees,
tables, toolbars, and related dense UI controls.

### Rendering, Geometry, And Resources

Renderer-neutral paint, compositor, effective-geometry, scrolling, and
virtualization records describe what the UI needs. Backends decide how to draw
or approximate it. Resource and font contracts track cache identity, generation,
budgeting, fallback, loaded/missing/failed state, stale rejection, and eviction
planning. Pixel parity is not guaranteed across CPU, WGPU, text backends, or
host font stacks.

### Accessibility

Accessibility records are host-facing state, not only test metadata. Operad
defines focus traps, navigable targets, live-region and adapter requests, and
headless adapter behavior. Platform screen-reader publication is backend work,
but platform adapters should consume the same request and report types.

### Themes And Stability

Theme, token, component state, and accessibility preference adjustment contracts
are stable for v5 as described in `docs/v5_0_theme_and_stability.md`.
`operad::versioning` classifies APIs as stable, experimental, backend-specific,
or migration-only. Product-facing code should prefer stable APIs and isolate
experimental or backend-specific use.

### Diagnostics

Diagnostics provide one report surface for input routing, widget actions,
overlay state, accessibility output, effective geometry, render timing, dirty
flags, warnings, errors, and fallback decisions. Tests, hosts, and devtools
should build on this shared vocabulary instead of inventing separate debug
formats for each subsystem.

## Lifecycle

A typical v5 frame follows these conceptual phases:

1. Host receives platform events and converts them into Operad input, window,
   document, resource, and scheduler records.
2. Application rebuilds or updates the UI tree from product state and stable
   widget identity.
3. Operad resolves retained state, input routing, navigation, overlay state,
   transactions, forms, async task reports, accessibility requests, layout, and
   effective geometry.
4. Operad emits action, command, diagnostic, accessibility, repaint, cursor,
   clipboard, IME, drag/drop, resource, and render records.
5. Application applies product semantics for committed actions and commands.
6. Host and renderer execute backend-specific work and schedule the next frame
   only when invalidation requires it.

The exact implementation can be split across host, renderer, and application
layers, but the records crossing those layers should stay backend-neutral where
possible.

## Ownership Boundaries

Applications should own:

- Domain state, command handlers, persistence, networking, and security policy.
- Product validators, permission checks, and data loading.
- Mapping Operad action and command records to business behavior.
- Local adapters that quarantine backend-specific or experimental APIs.

Operad should own:

- Public records for reusable UI behavior and lifecycle transitions.
- Deterministic state machines for forms, tasks, transactions, overlays,
  navigation, selection, resource accounting, diagnostics, and invalidation.
- Backend-neutral layout, theme, accessibility, geometry, compositor, scrolling,
  virtualization, and paint intent.
- Compatibility bridges that help existing consumers migrate without becoming
  the preferred v5 surface for new code.

Hosts and renderers should own:

- Native event loops, windows, surfaces, timers, idle work, and repaint hooks.
- GPU/CPU resource creation, presentation, adapter support, and fallback policy.
- OS clipboard, IME, cursor, drag/drop, accessibility publication, and font
  discovery integration.
- Backend-specific rendering quality and performance tradeoffs.

## Migration Path

New v5 adoption should be incremental:

1. Update dependencies and feature gates as described in
   `docs/v5_0_migration_guide.md`.
2. Use `operad::layout` and theme/token contracts for new public code while
   keeping Taffy and compatibility renderer paths local to migration adapters.
3. Introduce stable widget identity and retained state before moving complex
   focus, overlay, scroll, or edit workflows.
4. Route user intent through action, command, transaction, selection, form, and
   task records instead of app-local hit-result plumbing.
5. Feed diagnostics and accessibility reports from the same frame data used by
   rendering and input routing.
6. Move backend-specific runtime, WGPU, platform accessibility, clipboard, IME,
   cursor, and resource code behind host or renderer adapters.
7. Use the completion audit to distinguish stable contracts from areas where
   existing widget helpers or native backends still need incremental adoption.

## Compatibility Notes

V5 does not require downstream applications to rewrite every renderer or widget
path at once. Migration-only and backend-specific APIs exist so current
applications can move forward while isolating compatibility code. New
product-facing code should prefer the stable backend-neutral contracts and treat
direct backend integration as an adapter concern.
