# Сборка и развёртывание

## Требования

- Rust stable с `rustfmt` и `clippy`;
- Node.js 22 LTS или новее;
- npm 10 или новее.

## Полная проверка

Linux/macOS:

```bash
./verify.sh
```

Windows PowerShell:

```powershell
./verify.ps1
```

## Запуск разработки

```bash
npm ci
npm run build
cargo run -p honknet-server
```

Клиент в отдельном терминале:

```bash
npm run dev:client
```

Studio:

```bash
npm run dev:studio
```

## Production

```bash
cargo build --workspace --release
npm run build
```

Сервер запускается из корня проекта, чтобы относительные пути `engine.toml` корректно разрешались:

```bash
RUST_LOG=info ./target/release/honknet-server
```

Для reverse proxy необходимо разрешить WebSocket upgrade на порт `3015`. Статические файлы клиента находятся в `apps/client/dist`, а Studio — в `tools/studio/dist`.
