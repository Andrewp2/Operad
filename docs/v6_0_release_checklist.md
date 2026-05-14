# Operad 6.0 Release Checklist

Goal: ship `6.0.0` as the first native-window/widget release with app-owned GPU
canvas rendering and the v6 module-organization work already in the tree.

## Required CI Gates

These should pass on a normal Linux runner:

- [x] Format: `cargo fmt --all -- --check`
- [x] No-default compile: `cargo check --locked --no-default-features --all-targets`
- [x] No-default lib tests: `cargo test --locked --no-default-features --lib`
- [x] All-features compile: `cargo check --locked --all-features --all-targets`
- [x] All-features test enumeration: `cargo test --locked --all-features -- --list`
- [x] Example compile: `cargo check --locked --all-features --examples`
- [x] Docs: `cargo doc --locked --all-features --no-deps`
- [x] Package verification dry run: `cargo package --locked`

## Canvas Validation

Run this where a WGPU-compatible adapter is available:

- [x] Canvas render pass helper:
  `cargo test --locked --no-default-features --features wgpu canvas_context_render_pass_draws_shader_into_sampled_texture`
- [x] WGPU snapshot parity:
  `cargo test --locked --features wgpu --test wgpu_snapshot_parity -- --nocapture --test-threads=1`

## Release Sign-Off

- [x] `Cargo.toml` and `Cargo.lock` both report `6.0.0` for the `operad`
  package.
- [x] Changelog has a `6.0.0` entry that covers the native runner, widgets,
  app-owned WGPU canvas rendering, and migration notes.
- [x] README install snippet uses `cargo add operad`.
- [x] README canvas example uses `NativeWgpuCanvasRenderRegistry` for
  app-owned WGPU rendering.
- [x] Migration guide explains the v5-to-v6 dependency update and the preferred
  canvas integration path.
- [x] Module-organization note remains accurate for the v6 source layout.
- [x] Public API review confirms WebGL/WebGL2 names are not part of the canvas
  surface.
- [ ] Create the `v6.0.0` tag only after the package dry run and release gates
  are green.
