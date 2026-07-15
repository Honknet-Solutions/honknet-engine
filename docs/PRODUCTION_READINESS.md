# Honknet production-readiness gates

This document separates implemented engine capabilities from claims that must be proven in a target deployment. No multiplayer engine is production-ready merely because it builds.

## Implemented foundation

- Authoritative Rust server with a fixed 30 Hz simulation tick.
- Sparse-set component storage and typed ECS queries.
- Spatial hash used by movement collision and PVS candidate selection.
- Versioned WebSocket protocol with handshake, size limits, timeouts and coded errors.
- Input sequencing, server acknowledgements, client prediction, replay and visual reconciliation.
- Per-client full snapshots, ordered deltas and full-state recovery.
- Generic YAML prototypes, inheritance, component schemas and replication modes.
- TypeScript game-module process with a validated command buffer, timeout and command quota.
- HUI runtime shared by the game client and Honknet Studio.
- Atomic JSON persistence with backup recovery and path-traversal protection.
- Optional signed authentication tokens, connection limits and message rate limits.
- Health, readiness and Prometheus-compatible metrics endpoints.
- Container, systemd and reverse-proxy deployment examples.

## Release gates

A public release is approved only after all gates below pass on the target operating system and hardware.

### Build and correctness

```bash
./verify.sh
```

Required result: every command exits with status 0. This includes content validation, strict TypeScript checks, unit tests, production builds, Rust formatting, Clippy with warnings denied, Rust tests and the release server build.

### Runtime integration

```bash
./tools/verify-runtime.sh
```

Required result: the release server starts, `/readyz` succeeds, multiple clients complete the protocol handshake, snapshots and deltas remain consistent, ping works and `/metrics` remains available.

### Soak and capacity

Run at least:

```bash
HONKNET_VERIFY_CLIENTS=30 HONKNET_VERIFY_DURATION_SECONDS=21600 \
  ./tools/verify-runtime.sh
```

Acceptance criteria should be recorded for the actual server hardware. Recommended initial gates:

- no process crash or unhandled panic;
- no client protocol errors;
- no persistent TPS degradation;
- 99th-percentile tick time below the configured tick budget;
- stable RSS after warm-up, with no monotonic growth;
- reconnect, autosave and recovery verified during the soak;
- snapshot bandwidth within the hosting budget.

### Security

- `auth.required = true` for closed or account-backed servers;
- `HONKNET_AUTH_SECRET` is random, at least 32 bytes and stored outside the repository;
- public clients connect through TLS (`wss://`) at a reverse proxy;
- port 3016 is private and never exposed directly to the internet;
- operating-system user is unprivileged;
- save backups are tested;
- rate-limit and malformed-packet tests pass;
- dependencies and the container image are scanned before release.

### Content compatibility

Before mass content work, freeze and version:

- protocol version;
- native component names and fields;
- component-schema format;
- map format;
- HUI document format;
- script event and command contracts.

Any incompatible change must include a migration or a deliberate version bump.
