# Operad

Operad is a renderer-neutral UI toolkit contract layer for Rust applications.
It lets applications describe layout, input, widget state, actions, rendering
intent, accessibility, diagnostics, resources, and host/runtime behavior without
binding product code to one renderer or native window stack.

Version `5.0.0` is the v5 public API, docs, and versioning foundation. The
crate provides backend-neutral records and optional adapters; applications still
own product state, command handlers, persistence, native event loops, and final
business behavior.

## What Is Included

- Retained `UiDocument` trees with layout, hit testing, focus, scrolling,
  effective geometry, accessibility metadata, and renderer-neutral paint output.
- Optional widget helpers behind `features = ["widgets"]` for controls, menus,
  command palettes, text input, selection/copy policy, data widgets, pickers,
  surfaces, toasts, split panes, and editor-oriented primitives.
- Action, command, transaction, selection, form, task, overlay, navigation,
  virtualization, diagnostic, theme, resource, and font lifecycle contracts.
- CPU snapshot testing helpers and optional WGPU rendering paths.
- Optional `glyphon`/`cosmic-text` backed text paths and an optional
  `accesskit-winit` bridge for native accessibility publication.

## Install

```toml
[dependencies]
operad = "5.0.0"
```

Enable only the integrations you need:

```toml
operad = { version = "5.0.0", features = ["widgets", "wgpu"] }
```

Common feature flags:

- `widgets`: domain-neutral widget helpers.
- `wgpu`: WGPU renderer support, including glyphon text rendering.
- `native-window`: native winit/WGPU surface smoke support.
- `accesskit-winit`: AccessKit publication bridge for winit hosts.
- `text-cosmic`: cosmic-text measurement/shaping support.
- `egui`: egui host/input compatibility.
- `audit`: audit-oriented helpers.

## Examples

Render the full v5 showcase UI to a PPM snapshot:

```bash
cargo run --locked --features widgets --example operad_showcase
```

The showcase writes `target/operad_showcase.ppm` by default and reports paint
and accessibility node counts. Set `OPERAD_SHOWCASE_PPM=/path/to/out.ppm` to
choose another output path.

Run the native WGPU host example as an offscreen smoke:

```bash
cargo run --locked --features wgpu --example native_wgpu_host
```

Native OS-surface execution is opt-in because it requires a display and WGPU
adapter:

```bash
OPERAD_RUN_WGPU_EXAMPLE_WINDOW=1 \
OPERAD_WGPU_EXAMPLE_WINDOW_FRAMES=3 \
cargo run --locked --features native-window --example native_wgpu_host
```

## Documentation

Start with the v5 concept map:

- `docs/v5_0_core_concepts.md`
- `docs/v5_0_migration_guide.md`
- `docs/v5_0_theme_and_stability.md`
- `docs/v5_0_completion_audit.md`
- `docs/v5_0_release_checklist.md`

The core rule for consumers is to keep backend-specific APIs behind local host
or renderer adapters. Product-facing code should prefer Operad's stable
backend-neutral records for layout, actions, transactions, accessibility,
diagnostics, resources, and rendering intent.

## Release Validation

The baseline v5 release gates are:

```bash
cargo fmt --all -- --check
cargo check --locked --no-default-features --all-targets
cargo test --locked --no-default-features
cargo check --locked --all-features --all-targets
cargo test --locked --all-features -- --list
cargo check --locked --all-features --examples
cargo doc --locked --all-features --no-deps
cargo package --locked
```

Perf and WGPU validation commands are listed in
`docs/v5_0_release_checklist.md`.

## License

MIT. See `LICENSE`.
