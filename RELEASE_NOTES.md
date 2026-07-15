# Honknet Engine 0.2.0-rc.1 — Standalone SDK

## Engine

- sparse-set ECS storage вместо компонентных heap-map на каждой сущности;
- typed one/two/three-component queries;
- spatial hash для PVS и collision candidates;
- generic spawning из YAML prototypes;
- component-schema validation и replicated dynamic components;
- multi-grid/chunk maps и tile registry;
- server-authoritative movement и dynamic collision;
- versioned protocol v4 with entity revisions and bounded PVS deltas;
- acknowledged baseline deltas и full-state recovery;
- reconnect/duplicate-session protection;
- script-host timeout, world snapshot и command quotas;
- generic script command validation;
- UI session ownership/action routing;
- atomic persistence, backup fallback и traversal protection;
- authentication, rate limits, connection caps и message-size limits;
- health, readiness и Prometheus-compatible metrics.

## Client and Studio

- fixed-tick local prediction with input replay and visual reconciliation;
- client baseline validation and full-state requests;
- shared HUI runtime for game and Studio;
- complete Studio 3 toolset included in the payload;
- deterministic Vitest configuration for CI.

## Operations

- Dockerfile and Compose deployment;
- systemd and nginx examples;
- signed-token issuer;
- multi-client load test;
- runtime verification script;
- production-readiness, security and multiplayer acceptance documentation.

## Distribution model

- engine, SDK, Studio and CLI are packaged as a standalone engine repository;
- game-specific content is kept outside the engine;
- `templates/empty-game` and `honknet new` provide the public project entry point;
- CI and release workflows cover Linux, Windows, macOS, server artifacts, SDK packaging and container publication.

## Release status

This archive is a release candidate. See `VERIFICATION.md` and `docs/RELEASE_GATES.md` before public deployment.
