# Implementation matrix

| Area | Primary code |
|---|---|
| Core / CVars / IDs | `crates/honknet-core` |
| ECS / lifecycle / storages / commands | `crates/honknet-ecs` |
| Scheduler | `crates/honknet-scheduler` |
| Reflection / macros / serialization | `crates/honknet-reflection`, `honknet-macros`, `honknet-serialization` |
| Maps / grids / chunks / navigation | `crates/honknet-map`, `honknet-transform`, `honknet-spatial` |
| Physics | `crates/honknet-physics` |
| Network transports / binary protocol | `crates/honknet-net-core`, `honknet-net-transport` |
| Replication / PVS / budgets | `crates/honknet-replication` |
| Prediction / rollback | `crates/honknet-prediction` |
| wgpu rendering | `crates/honknet-render`, `apps/honknet-client` |
| Retained HUI | `crates/honknet-ui` |
| Audio | `crates/honknet-audio` |
| Content / VFS / localization | `honknet-resources`, `honknet-prototypes`, `honknet-localization` |
| Persistence / replay | `honknet-persistence`, `honknet-replay` |
| Auth / admin / observability / WASM | corresponding crates |
| Runtime apps | `apps/` |
| Studio | `apps/honknet-studio` |
| Fixed game bootstrap and gameplay | `game`, `apps/honknet-server` |
| CLI and tests | `apps/honknet-cli`, `integration-tests` |
