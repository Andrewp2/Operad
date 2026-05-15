#!/usr/bin/env bash
set -euo pipefail

cargo check --all-targets --all-features
cargo test --lib --all-features "$@"
cargo check --no-default-features
