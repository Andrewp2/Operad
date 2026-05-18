# Release Process

Use this process for every Operad release. Replace `X.Y.Z` with the version
being released and `vX.Y.Z` with the matching git tag.

## Release Principles

- Release from `main` unless a dedicated release branch is intentionally used.
- Tag only the exact commit that passed the local release gate.
- Publish only after GitHub Actions has passed for that pushed commit.
- Do not move a tag after the crate has been published. If a published release
  has a defect, ship a patch release instead.
- Keep showcase regressions in external tests, not in `examples/showcase.rs`.
- Keep release commits focused on version metadata, changelog/release notes,
  docs, and intentional release fixes.

## Choose The Version

Use semantic versioning:

- Major release: breaking public API changes or major behavior changes.
- Minor release: compatible new APIs, features, examples, diagnostics, or
  non-breaking behavior improvements.
- Patch release: compatible bug fixes, docs, tests, packaging, or CI fixes.

Before starting, make sure the intended version is not already published:

```bash
cargo search operad
git tag --list 'v*' --sort=-version:refname | head
```

## Prepare The Release Commit

Confirm the current branch and working tree:

```bash
git branch --show-current
git status --short
```

Update release metadata:

- `Cargo.toml`: set `version = "X.Y.Z"`.
- `Cargo.lock`: update it by running Cargo after the version change.
- `CHANGELOG.md`: add an `X.Y.Z` entry.
- Major releases: add or update major-version roadmap, migration, stability, or
  completion docs when relevant.
- README: update any user-facing version or release guidance if it has changed.

The working tree should contain only intentional release changes before the
final gate starts. If unrelated files are present, leave them out of the release
commit or move them into a separate commit first.

## Local Release Gate

Run the standard full gate:

```bash
scripts/test-full.sh
```

Run final release checks that may not all be covered by the normal loop:

```bash
cargo doc --locked --all-features --no-deps
cargo package --locked
cargo publish --dry-run --locked
```

If the release candidate changes a specific public surface late in the process,
rerun the focused suite that covers that surface. For showcase, layout, widgets,
or performance changes, prefer:

```bash
cargo test --locked --all-features --test showcase_layout -- showcase_ --test-threads=1
cargo test --locked --no-default-features --features widgets --test perf_smoke -- --test-threads=1
cargo check --locked --all-features --examples
```

## Commit

Create one release commit after the local gate is green:

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md README.md docs src examples tests web .github scripts
git commit -m "Release vX.Y.Z"
```

If not every listed path changed, `git add` will simply skip unchanged files.
Review the final commit before tagging:

```bash
git show --stat --oneline HEAD
```

## Tag

Tag the exact verified commit:

```bash
git tag -a vX.Y.Z -m "Operad vX.Y.Z"
```

Verify the tag points where expected:

```bash
git show --stat vX.Y.Z
```

## Push And CI

Push the release commit and tag:

```bash
git push origin main
git push origin vX.Y.Z
```

Wait for GitHub Actions to pass:

- Rust CI
- Showcase Pages build
- Browser smoke test for `/`
- Browser smoke test for `/showcase/`
- GitHub Pages deployment

If CI fails before publishing and the tag has not been announced, fix the issue,
rerun the local gate, and recreate the tag on the new release commit:

```bash
git tag -d vX.Y.Z
git push origin :refs/tags/vX.Y.Z
git tag -a vX.Y.Z -m "Operad vX.Y.Z"
git push origin main
git push origin vX.Y.Z
```

Do not use that recovery flow after `cargo publish` succeeds. After publish,
leave the tag immutable and release the next patch version for fixes.

## Publish

Publish only from the same commit that CI validated:

```bash
git status --short
git rev-parse HEAD
git rev-list -n 1 vX.Y.Z
cargo publish --locked
```

The two commit hashes should match. If they do not, stop and resolve the branch
or tag mismatch before publishing.

## Post-Publish Verification

Confirm crates.io sees the new version:

```bash
cargo search operad
```

Then verify:

- docs.rs starts building or has published `X.Y.Z`.
- The GitHub Pages showcase opens at `/`.
- The GitHub Pages showcase opens at `/showcase/`.
- GitHub release notes exist for `vX.Y.Z`, using `CHANGELOG.md` as the source of
  truth.

## Patch Releases

Patch releases follow the same process. Keep them limited to compatible bug
fixes, documentation, tests, packaging, and release infrastructure. Breaking API
changes belong in the next major release plan.
