#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all -- --check
cargo check --locked --all-targets --all-features
cargo test --locked --lib --all-features "$@"
cargo check --locked --no-default-features --all-targets
