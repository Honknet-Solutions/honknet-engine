# Build verification

Проверено в среде сборки архива:

- `npm run validate` — успешно;
- `npm run typecheck` — успешно для клиента, script host, SDK, Studio и example module;
- `npm run build` — успешно;
- production-сборка PixiJS/Vite клиента — успешно;
- production-сборка Honknet Studio — успешно;
- asset manifest — создан;
- YAML/RSI/prototype/map validation — успешно.

Rust toolchain в среде формирования архива отсутствовал, поэтому `cargo fmt`, `cargo clippy`, `cargo test` и `cargo build --release` должны быть выполнены в целевой среде командой `./verify.sh` или `./verify.ps1`.
