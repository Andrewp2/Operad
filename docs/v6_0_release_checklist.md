# Operad 6.0 Release Checklist

Goal: ship `6.0.0` as the direct GPU canvas context release while preserving the
v6 module-organization work already in the tree.

## Required CI Gates

These should pass on a normal Linux runner:

- [ ] Format: `cargo fmt --all -- --check`
- [ ] No-default compile: `cargo check --locked --no-default-features --all-targets`
- [ ] No-default lib tests: `cargo test --locked --no-default-features --lib`
- [ ] All-features compile: `cargo check --locked --all-features --all-targets`
- [ ] All-features test enumeration: `cargo test --locked --all-features -- --list`
- [ ] Example compile: `cargo check --locked --all-features --examples`
- [ ] Docs: `cargo doc --locked --all-features --no-deps`
- [ ] Package verification dry run: `cargo package --locked`

## Canvas Validation

Run this where a WGPU-compatible adapter is available:

- [ ] Attached GPU canvas shader pass:
  `cargo test --locked --no-default-features --features wgpu canvas_context_render_pass_draws_shader_into_sampled_texture`
- [ ] WGPU snapshot parity:
  `cargo test --locked --features wgpu --test wgpu_snapshot_parity -- --nocapture --test-threads=1`

## Release Sign-Off

- [ ] `Cargo.toml` and `Cargo.lock` both report `6.0.0` for the `operad`
  package.
- [ ] Changelog has a `6.0.0` entry that calls out direct GPU canvas contexts.
- [ ] README install snippets use `6.0.0`.
- [ ] README canvas example uses `gpu_context`, `get_gpu_context`, and
  `WgpuCanvasRenderPass::fragment(...)`.
- [ ] Migration guide explains the v5-to-v6 dependency update and the preferred
  canvas integration path.
- [ ] Module-organization note remains accurate for the v6 source layout.
- [ ] Public API review confirms WebGL/WebGL2 names are not part of the native
  Operad canvas surface.
- [ ] Create the `v6.0.0` tag only after the package dry run and release gates
  are green.
