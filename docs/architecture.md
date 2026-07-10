# Architecture

Space Station 15 is split into three product layers:

1. **Engine Core** — setting-agnostic world, ECS, maps, networking, replication, resources and runtime services.
2. **Game Framework** — reusable gameplay abstractions such as interaction, inventory, chat, permissions and UI RPC.
3. **Game Modules** — optional gameplay packages such as combat, medical, power, atmospherics, vehicles, cyberpunk or space station systems.

The browser client is never authoritative. The Rust server owns the world state and validates all player actions.
