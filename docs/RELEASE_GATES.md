# Public release gates

Релиз движка разрешён только после выполнения всех пунктов:

1. `npm ci`, validation, typecheck, tests и production builds.
2. `cargo fmt`, Clippy с `-D warnings`, Rust tests и release build.
3. Linux, Windows и macOS CI.
4. Чистая Docker build и healthcheck.
5. Runtime integration test: server + минимум два клиента.
6. 200-client connection/load test.
7. 6-hour soak test с целевой сущностной нагрузкой.
8. malformed packets, message flood, reconnect storm и duplicate identity tests.
9. persistence crash recovery и backup restore.
10. SBOM, checksums, signed release artifacts и reproducible dependency lockfiles.
11. Security review всех protocol handlers и script commands.
12. Published benchmark report с hardware specification и p95/p99 metrics.

Ни один source archive без этих результатов не маркируется как production-certified.
