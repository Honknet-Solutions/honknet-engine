#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

npm ci
npm run validate
npm run typecheck
npm run build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace --release

echo "Honknet Engine verification completed successfully."
