# Verification report — Honknet Engine 0.2.0-rc.1

Archive preparation date: 2026-07-15.

## Completed in the packaging environment

The following commands completed successfully:

```bash
npm ci --no-audit --no-fund
npm run validate
npm run typecheck
npm test
npm run build:sdk
npm run build:cli
npm run build:game
npm run build:scripts
npm run build:client
npm run build:studio
```

Observed results:

- content validation: 5 prototypes, 3 RSI resources and 7 YAML documents;
- strict TypeScript checks: all workspaces passed;
- automated TypeScript tests: 15/15 passed;
- browser client production build: 816 Vite modules;
- Honknet Studio production build: 103 Vite modules;
- public npm package dry-runs: shared, server SDK, client SDK, HUI runtime and CLI passed;
- `honknet new` created a standalone game project successfully;
- 25,000-prototype content benchmark: 6.89 MB YAML parsed at approximately 3,594 prototypes/second in this environment;
- all 28 Rust source files were parsed by the tree-sitter Rust grammar with zero syntax errors;
- source manifest and archive integrity checks passed.

## Not executable in the packaging environment

A Rust toolchain was not installed, and the environment could not resolve `static.rust-lang.org`. Therefore these commands were **not** executed here:

```bash
cargo generate-lockfile
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo build --workspace --release --locked
```

The archive intentionally remains a release candidate until these commands pass in CI or on a clean machine with Rust 1.85.1.

`Cargo.lock` is not present because Cargo could not run in the packaging environment. `verify.sh`, Docker and release CI generate it before using `--locked`. A stable public release must include the generated lockfile in source control and release artifacts.

## Capacity certification still required

The code contains sparse-set component storage, spatial indexing, bounded PVS, entity revisions, incremental script-world updates, rate limits, bounded queues and load-test tooling. These are scale-oriented implementations, not proof that every game module will support a specific player count.

Production certification still requires the gates in `docs/RELEASE_GATES.md`, including:

- release Rust build on Linux, Windows and macOS;
- clean Docker build and runtime integration test;
- 200-client load test;
- six-hour soak test with the target entity count;
- p95/p99 tick-time, memory and bandwidth reports;
- malformed-message, flood, reconnect and persistence-recovery tests;
- security review and signed release artifacts.

Status: **standalone engine SDK release candidate; suitable for game development and controlled testing, not yet production-certified.**
