# Honknet Engine 1.0.0-rc.1

Complete implementation candidate of a setting-agnostic authoritative 2D multiplayer engine.
The repository contains native Rust server/headless clients, and apps/honknet-web as the sole game client. It also features
ECS, parallel scheduler, maps/grids, physics, binary networking, replication, prediction,
wgpu rendering, retained UI, audio, content tools, persistence, replays, administration,
Studio sources, CLI, SDK, templates, tests and deployment files.

This archive is intended for compilation and defect discovery. It is not production-certified
until all release gates in `docs/RELEASE_GATES.md` pass on target hardware.

```bash
cargo check --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo run -p honknet-server
```
