$ErrorActionPreference = "Stop"
python tools/source_audit.py
cargo fmt --all -- --check
cargo check --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --release
npm install
npm run typecheck
npm run build:game-scripts
git diff --exit-code -- game/scripts/dist/game.js
npm run build:studio
npm run build:web
