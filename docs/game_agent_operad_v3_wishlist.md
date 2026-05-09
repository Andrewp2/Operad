# Game Agent Operad V3 Wishlist

This carries over the current game-side wishlist after migrating the game to
Operad 2.0. The game can use Operad for its non-debug UI now, but a few pieces
still remain outside Operad because the library does not yet own the right
abstractions.

## Current Game-Side Gaps

- `src/ui/debug.rs` remains direct egui by design for now. Debug panels, frame
  graph panels, inspector controls, terrain tools, and transform gizmos can stay
  on egui unless there is an explicit future migration.
- `src/ui/egui.rs` still owns the egui frame lifecycle, raw input conversion,
  tessellation, texture deltas, and platform output. Non-debug game surfaces now
  build Operad documents and use Operad's egui backend painter, but the game
  still needs egui as the host renderer.
- `src/ui/automation.rs`, `src/ui/response_ext.rs`, and egui-based UI tests
  still use egui event and test-harness types because the current window
  integration is egui/winit based.
- Menu save thumbnails are still stored as `egui::TextureHandle` in
  `src/ui/menu/mod.rs`. Operad needs a renderer-neutral image or texture handle
  abstraction for application-owned image resources.
- `src/ui/style.rs` still configures egui style data used by the egui host and
  debug layer. A native Operad renderer/theme system could replace this for
  non-debug UI.

## What Would Make V3 Most Useful To The Game

Operad should own enough backend glue that game UI code can stop depending on
egui types for non-debug surfaces. The useful boundary is not "remove egui from
the process"; it is "make egui an optional backend adapter instead of a type
that leaks into game UI state, input, texture handles, tests, and styling."

The game would benefit most from:

- Renderer-neutral input events and platform-output responses.
- Renderer-neutral image, icon, and texture handles.
- A first-class theme/style system that does not require configuring egui
  visuals for Operad-owned UI.
- Snapshot and event-test utilities that do not require egui harness types.
- Explicit layer and z-order policy for mixed host/debug/app UI, so app UI can
  stay behind debug UI without relying on backend-specific ordering.
- A richer paint-list/backend contract that can eventually target the game's
  renderer directly while keeping the egui backend useful during transition.

## Boundary To Preserve

The game should continue to own game semantics: hotbar slots, inventory rules,
map tools, chat behavior, key bindings, debug actions, and renderer settings.
Operad should own the reusable toolkit substrate: layout, input routing, focus,
scrolling, popups, widgets, paint primitives, texture handles, themes,
accessibility metadata, and test/performance harnesses.

That split keeps Operad reusable for Orbifold and semiconductor tools while
still letting the game remove the remaining egui coupling from non-debug UI.
