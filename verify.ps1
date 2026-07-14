$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

npm ci
npm run validate
npm run typecheck
npm run build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace --release

Write-Host "Honknet Engine verification completed successfully."
