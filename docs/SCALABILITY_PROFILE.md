# Scalability profile

## Цель

Целевой профиль Honknet 0.2 — до 200 одновременно подключённых игроков и 100 000 серверных сущностей на одном процессе при пространственно ограниченном PVS. Это целевой профиль, а не гарантия для любого игрового модуля.

## Бюджеты

- server tick: 30 Hz;
- snapshot rate: 10–20 Hz;
- default PVS entity cap: 4096;
- script host budget: 12 ms;
- per-client message limit: 256 KiB;
- bounded event queues and rate limits;
- spatial hash for PVS and collision candidates;
- sparse-set component storages;
- incremental script-world deltas;
- entity revisions for snapshot diff detection.

## Условия достижения

Игровой код не должен выполнять полный обход мира на каждом тике. Тяжёлые системы обязаны использовать spatial queries, event-driven dirty sets, fixed budgets и native Rust hot paths. PVS, snapshot rate и entity cap должны подбираться под карту и доступный uplink.

## Обязательное измерение

Перед production deployment измеряются TPS, p50/p95/p99 tick time, RSS, allocation rate, bytes/client/sec, snapshot size, reconnect recovery и script-host latency. Релиз не считается сертифицированным только потому, что исходники собираются.
