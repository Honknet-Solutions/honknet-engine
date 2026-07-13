# Space Station 15 — vertical slice

Self-contained Rust authoritative server and TypeScript/PixiJS browser client.

Implemented:

- WebSocket handshake and stable guest identity;
- authoritative 30 TPS server simulation;
- entity/component/system foundations on server and client;
- full-world replication with client lifecycle tracking;
- input sequence ACK, heartbeat, prediction and reconciliation;
- remote-entity interpolation;
- tile map, walls, a working door and collision;
- interactable item pickup and authoritative inventory;
- multiplayer chat;
- content prototypes and a JSON debug map.

## Install

```bash
npm install
cargo test
npm run typecheck
```

## Run

```bash
cargo run -p honknet-server
```

Second terminal:

```bash
npm run dev:client
```

Open the Vite URL in two browser windows. Use WASD or arrows, `E` to interact, and the chat box to send messages.
