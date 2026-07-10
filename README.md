# Space Station 15

**Space Station 15** is a browser-first open-source framework for multiplayer 2D immersive simulation games.

It is developed by **Open Station** as a modular foundation for persistent roleplay sandboxes: cyberpunk cities, space stations, colonies, bunkers, fantasy settlements and other simulation-heavy worlds.

Space Station 15 is **not** a Space Station 14 fork, not a RobustToolbox fork, and not tied to space station mechanics. Space stations, atmospherics and shuttles are optional game modules, not core assumptions.

## Architecture

```text
Browser Client / TypeScript
        ↓
Protocol / schema-first messages
        ↓
Rust Authoritative Server
        ↓
ECS World Simulation
        ↓
Modules / Game Content
```

## Repository layout

```text
apps/
  client/        Browser client
  server/        Rust authoritative server
crates/
  ss15-core/     Core world/ECS primitives
  ss15-protocol/ Shared network message definitions
docs/            Architecture and design docs
modules/         Optional gameplay modules
```

## Current status

Initial clean repository scaffold.

