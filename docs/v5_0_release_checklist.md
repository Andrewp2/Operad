# Operad 5.0 Release Checklist

Goal: ship `5.0.0` as the v5 public API/docs/versioning foundation while being
explicit that the broader interaction-runtime roadmap is still tracked in
`docs/v5_0_completion_audit.md`.

## Branch Layout

- [ ] Keep the release-spine branch limited to release metadata, changelog,
  documentation, CI/release automation, and unavoidable version metadata.
- [ ] Merge completed v5 feature branches into the release branch before final
  verification; do not hide incomplete roadmap items behind the version bump.
- [ ] Treat downstream application probes as adoption evidence, not crate release
  blockers, unless they expose an Operad-owned regression.
- [ ] Tag the final green release commit as `v5.0.0` after package verification.

## Required CI Gates

These are the lightweight checks that should pass on an ordinary GitHub-hosted
Linux runner without a GPU:

- [ ] Format: `cargo fmt --all -- --check`
- [ ] No-default compile: `cargo check --locked --no-default-features --all-targets`
- [ ] No-default tests: `cargo test --locked --no-default-features`
- [ ] All-features compile: `cargo check --locked --all-features --all-targets`
- [ ] All-features test enumeration: `cargo test --locked --all-features -- --list`
- [ ] Example compile: `cargo check --locked --all-features --examples`
- [ ] Docs: `cargo doc --locked --all-features --no-deps`
- [ ] Package verification dry run: `cargo package --locked`

`cargo test --all-features -- --list` intentionally enumerates feature-gated
tests without executing WGPU bodies. That keeps basic CI useful without assuming
the runner has a GPU or software Vulkan adapter.

## Release Sign-Off

- [ ] Changelog has a `5.0.0` entry that summarizes landed v5 roadmap work and
  calls out release-gate automation.
- [ ] Core concepts reference covers the v5 contract model, lifecycle,
  ownership boundaries, migration path, and how backend-neutral contracts fit
  together.
- [ ] Migration guide covers the v4-to-v5 dependency update, new public layout
  facade, i18n policy types, API stability markers, state/action/runtime
  contracts, async task/form validation contracts, effective geometry/resource
  cache contracts, font lifecycle contracts, accessibility adapter contracts,
  tooltip/help/context menu policy, unified diagnostics, theme/design-token
  stability docs, core concepts reference, and unchanged feature gates.
- [ ] Completion audit is current and does not mark partial roadmap areas as
  complete.
- [ ] Semver/API stability review is recorded:
  - Review public additions against `src/versioning.rs` stability categories.
  - Confirm intentional breaking changes are listed in the migration guide.
  - If a baseline is available, run `cargo semver-checks check-release` or record
    why manual review was used instead.
- [ ] Examples compile. Runtime probe assertions such as
  `cargo run --locked --example three_consumer_probe` may be run as additional
  adoption evidence after any existing audit-warning expectations are updated.

## Perf Smoke

Run these on a development or release-validation machine. They are not basic CI
requirements because wall-clock budgets can be noisy on shared runners:

- [ ] CPU/widget smoke: `cargo test --locked --features widgets --test perf_smoke -- --nocapture`
- [ ] Release CPU/widget smoke: `cargo test --release --locked --features widgets --test perf_smoke -- --nocapture`

Record the machine class and whether any budget changes are intentional.

## WGPU-Gated Validation

Run these only where a WGPU-compatible adapter is available, such as a developer
workstation, GPU runner, or runner configured with a known-good software adapter:

- [ ] Snapshot parity: `cargo test --locked --features wgpu --test wgpu_snapshot_parity -- --nocapture`
- [ ] WGPU perf smoke: `cargo test --locked --features widgets,wgpu --test perf_smoke -- --nocapture`
- [ ] Release WGPU perf smoke: `cargo test --release --locked --features widgets,wgpu --test perf_smoke -- --nocapture`

Do not make these commands required for the baseline GitHub-hosted CI job unless
the runner image is explicitly provisioned for WGPU.

## Final Package Steps

- [ ] Re-run required CI gates from a clean worktree.
- [ ] Inspect `cargo package --locked --list` for unexpected files.
- [ ] Confirm `Cargo.toml` and `Cargo.lock` both report `5.0.0`.
- [ ] Create the release tag and publish from the verified commit.
