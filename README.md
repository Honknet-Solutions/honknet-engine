# Honknet Engine 0.2.0-rc.1

Honknet Engine — самостоятельный authoritative 2D multiplayer engine для длительных сессий. Репозиторий содержит Rust-сервер, браузерный client runtime, TypeScript SDK, изолированный script host, content pipeline, Honknet Studio, CLI и минимальный пример игры.

## Статус релиза

Это **release candidate исходного SDK**, предназначенный для разработки отдельных игровых репозиториев и закрытых нагрузочных испытаний. Целевой профиль — сотни подключений и десятки тысяч серверных сущностей при ограниченном PVS. Публичная production-сертификация требует прохождения release gates из `docs/RELEASE_GATES.md` на целевом оборудовании.

## Архитектура

- Rust: ECS, authoritative simulation, networking, replication, maps, persistence, assets, auth, metrics и admin API.
- TypeScript: server game modules, client modules, HUI controllers и shared contracts.
- YAML/HUI/HGRAPH/HSM: prototypes, schemas, maps, UI и behavior/state graphs.
- FTL: localization.
- PNG/WebP/RSI: assets.

Движок и игра разделены. `examples/minimal-game` используется только как compatibility fixture. Для нового проекта:

```bash
npm ci
npm run build
node packages/cli/dist/index.js new ../my-game
```

## Проверка исходников

```bash
npm ci --no-audit --no-fund
npm run validate
npm run typecheck
npm test
npm run build

cargo generate-lockfile
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo build --workspace --release --locked
```

Полный локальный gate:

```bash
./verify.sh
```

## Запуск reference fixture

Требования: Node.js 22 и Rust 1.85.1.

```bash
npm ci --no-audit --no-fund
npm run build
cargo generate-lockfile
cargo run -p honknet-server --locked
```

Во втором терминале:

```bash
npm run dev:client
```

Studio:

```bash
npm run dev:studio
```

## Масштабирование

Ключевые ограничения задаются переменными окружения:

- `HONKNET_PVS_RADIUS`
- `HONKNET_MAX_PVS_ENTITIES`
- `HONKNET_SNAPSHOT_RATE`
- `HONKNET_MAX_CONNECTIONS`
- `HONKNET_MAX_CONNECTIONS_PER_IP`
- `HONKNET_WORKER_THREADS`
- `HONKNET_SCRIPT_MAX_TICK_MS`
- `HONKNET_SCRIPT_MAX_COMMANDS`

Синтетический world benchmark:

```bash
HONKNET_SYNTHETIC_ENTITY_COUNT=100000 cargo run -p honknet-server --release
npm run bench:content
```

Сетевой soak test:

```bash
HONKNET_SOAK_CLIENTS=200 \
HONKNET_SOAK_ENTITIES=100000 \
HONKNET_SOAK_DURATION_SECONDS=21600 \
npm run soak:test
```

## Лицензия

Apache-2.0. См. `LICENSE` и `NOTICE`.

## Подтверждение релиза

Точные проверки и неподтверждённые gates перечислены в `VERIFICATION.md`. Исходный архив не маркируется production-certified до прохождения Rust CI, runtime integration и целевого soak test.
