# Honknet 1.0.0-rc.1

Single-product implementation of the authoritative Honknet 2D multiplayer game.
The repository contains one native Rust server, a headless test client, and
`apps/honknet-web` as the sole game client. It also features
ECS, parallel scheduler, maps/grids, physics, binary networking, replication, prediction,
browser rendering, retained UI, audio, fixed game content, persistence, replays,
administration, Studio sources, CLI, tests and deployment files.

Honknet does not load interchangeable games, projects or runtime gameplay plugins. Engine
subsystems remain separate Rust crates, but they compile into one fixed server/client product.

```bash
cargo check --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo run -p honknet-server
```
