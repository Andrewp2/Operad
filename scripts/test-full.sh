#!/usr/bin/env bash
set -euo pipefail

cargo check --all-targets --all-features
cargo check --no-default-features
cargo test --all-features "$@"
