# Honknet 1.0 release gates

- clean Linux, Windows and macOS builds;
- no Clippy warnings and no undocumented unsafe;
- desktop and web clients connect to one authoritative server;
- malformed packet, reconnect, persistence recovery and replay determinism tests;
- 256 clients, 100,000 entities, 30 TPS, six-hour soak;
- p95 below 20 ms and p99 below 30 ms on the declared reference host;
- signed archives, SBOM and reproducible checksums.
