# Архитектура Honknet Engine

## Граница доверия

Сервер Rust является единственным источником истины. TypeScript-код клиента не изменяет здоровье, инвентарь, положение, права или состояние карты. Серверный TypeScript выполняется script-host и возвращает командный буфер, который проверяется Rust-сервером.

## Поток тика

1. Network принимает и валидирует input.
2. Rust simulation применяет authoritative movement и physics.
3. Engine формирует события для server TypeScript module.
4. Script host выполняет game logic и возвращает commands.
5. Rust проверяет и применяет commands.
6. PVS формирует состояние для каждого клиента.
7. Клиент выполняет interpolation и допустимое prediction.

## Языки

- Rust: engine internals.
- TypeScript: game logic, client behavior, UI.
- YAML: prototypes, maps, HUI, behavior graphs, component schemas.
- FTL: localization.
- RSI/PNG/WebP: assets.
