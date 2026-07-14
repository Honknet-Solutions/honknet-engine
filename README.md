# Space Station 15 — Honknet Engine v1 RC

Самостоятельный серверный 2D multiplayer engine для длительных станционных сессий.

## Стек

- Rust: ECS, сервер, сеть, физика, карты, ресурсы, persistence и admin API.
- TypeScript: серверные игровые скрипты, клиент, UI и Honknet Studio.
- YAML: прототипы, карты, component schemas, HUI и behavior graphs.
- FTL: локализация.
- PNG/WebP/RSI: графика.

## Быстрый запуск

```bash
npm install
npm run build
cargo test --workspace
cargo run -p honknet-server
```

Во втором терминале:

```bash
npm run dev:client
```

Studio:

```bash
npm run dev:studio
```

