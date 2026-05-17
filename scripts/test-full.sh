#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all -- --check
cargo check --locked --no-default-features --all-targets
cargo check --locked --all-targets --all-features
cargo test --locked --all-features "$@"
cargo check --locked --target wasm32-unknown-unknown --no-default-features --features web-showcase --example showcase_web
