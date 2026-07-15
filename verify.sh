#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

npm ci
npm run validate
npm run typecheck
npm test
npm run build
cargo generate-lockfile
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo build --workspace --release --locked

echo "Honknet Engine verification completed successfully."
