# Release checklist

```bash
npm ci
npm run typecheck
npm run build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

Затем:

1. Запустить server и два browser clients.
2. Проверить движение, prediction, collision, door, item, inventory и chat.
3. Проверить reconnect с прежним identity.
4. Запустить 30 headless clients минимум на 60 минут.
5. Проверить RSS memory и TPS.
6. Проверить invalid packets, oversized chat и input flood.
7. Проверить asset manifest и все RSI.
8. Проверить autosave и recovery после аварийного завершения.
