# Implementation matrix

| Area | Primary code |
|---|---|
| Core / CVars / IDs | `crates/honknet-core` |
| ECS / lifecycle / storages / dynamic components / relations / commands | `crates/honknet-ecs` |
| Scheduler | `crates/honknet-scheduler` |
| Reflection / macros / serialization | `crates/honknet-reflection`, `honknet-macros`, `honknet-serialization` |
| Maps / grids / chunks / navigation | `crates/honknet-map`, `honknet-transform`, `honknet-spatial` |
| Z-levels / areas / moving grids / docking / transitions | `crates/honknet-map` |
| Physics | `crates/honknet-physics` |
| Network transports / binary protocol | `crates/honknet-net-core`, `honknet-net-transport` |
| Replication / per-client baselines / PVS / owner visibility / budgets | `crates/honknet-replication`, `game`, `crates/honknet-client-runtime` |
| Prediction / rollback | `crates/honknet-prediction` |
| wgpu rendering | `crates/honknet-render`, `apps/honknet-client` |
| Retained HUI | `crates/honknet-ui` |
| Audio | `crates/honknet-audio` |
| Content / VFS / localization | `honknet-resources`, `honknet-prototypes`, `honknet-localization` |
| Persistence / replay | `honknet-persistence`, `honknet-replay` |
| Auth / admin / observability | corresponding crates |
| Sandboxed game scripts | `crates/honknet-script`, `game/scripts` |
| Authoritative gameplay action pipeline | `crates/honknet-net-core`, `game`, `apps/honknet-server` |
| Hands / equipment / containers / grabbing / pulling / carrying / buckling | `game/src/components`, `game/src/systems`, `crates/honknet-net-core` |
| Runtime apps | `apps/` |
| Studio | `apps/honknet-studio` |
| Fixed game bootstrap and gameplay | `game`, `apps/honknet-server` |
| Lobby / jobs / round lifecycle | `game/src/round.rs`, `crates/honknet-net-core`, `apps/honknet-web` |
| Atmosphere / breathing / power networks / pressure-safe doors | `game/src/components`, `game/src/systems` |
| Chemistry / metabolism / surgery | `game/src/components`, `game/src/systems/chemistry`, `game/src/systems/health` |
| Metrics / authenticated administration / persistence / replay | `honknet-observability`, `honknet-admin`, `honknet-persistence`, `honknet-replay`, `apps/honknet-server` |
| CLI and tests | `apps/honknet-cli`, `integration-tests` |
