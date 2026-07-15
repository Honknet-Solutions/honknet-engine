# Разделение Honknet Engine и Space Station 15 по двум репозиториям

Эта версия проекта рассчитана на два отдельных репозитория:

```text
Honknet-Solutions/
├── honknet-engine
└── space-station-15
```

Исходники движка и игры хранятся отдельно. Готовый клиентский или серверный релиз игры при этом собирается единым пакетом с закреплённой версией движка.

## 1. Репозиторий `honknet-engine`

В корень `honknet-engine` помещается **всё содержимое архива движка**:

```text
honknet-engine/
├── .github/
├── apps/
│   ├── client/
│   ├── script-host/
│   └── server/
├── crates/
├── deploy/
├── docs/
├── examples/
│   └── minimal-game/
├── packages/
│   ├── cli/
│   ├── hui-runtime/
│   ├── sdk-client/
│   ├── sdk-server/
│   └── sdk-shared/
├── templates/
│   └── empty-game/
├── tools/
│   └── studio/
├── Cargo.toml
├── package.json
├── package-lock.json
├── rust-toolchain.toml
├── engine.toml
├── Dockerfile
├── docker-compose.yml
└── остальные корневые документы и скрипты
```

В этот репозиторий относятся только универсальные части:

- Rust ECS и authoritative server;
- сеть, replication, PVS, карты, физика и persistence;
- браузерный client runtime;
- TypeScript script host;
- публичные SDK `@honknet/*`;
- HUI runtime;
- Honknet Studio;
- Honknet CLI;
- валидаторы, генераторы, benchmarks и load tests;
- deployment и release infrastructure;
- минимальный fixture и пустой game template.

В `honknet-engine` **не кладутся** профессии, оружие, медицина, игровые карты, фракции, лор, баланс и ассеты конкретной игры.

### Первичная загрузка движка

Linux/macOS:

```bash
git clone https://github.com/Honknet-Solutions/honknet-engine.git
cd honknet-engine

# Скопировать сюда всё содержимое папки honknet-engine из архива.

npm ci --no-audit --no-fund
npm run validate
npm run typecheck
npm test
npm run build

git add .
git commit -m "Initial Honknet Engine 0.2.0-rc.1"
git push origin main
```

Windows PowerShell:

```powershell
git clone https://github.com/Honknet-Solutions/honknet-engine.git
Set-Location honknet-engine

npm ci --no-audit --no-fund
npm run validate
npm run typecheck
npm test
npm run build

git add .
git commit -m "Initial Honknet Engine 0.2.0-rc.1"
git push origin main
```

Для Rust-части:

```bash
rustup toolchain install 1.85.1 --profile minimal --component rustfmt --component clippy
cargo generate-lockfile
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo build --workspace --release --locked
```

После `cargo generate-lockfile` файл `Cargo.lock` необходимо добавить в Git:

```bash
git add Cargo.lock
git commit -m "Lock Rust dependencies"
git push origin main
```

## 2. Репозиторий `space-station-15`

В `space-station-15` помещается только игра:

```text
space-station-15/
├── client/
│   └── src/
├── server/
│   └── src/
├── shared/
│   └── src/
├── content/
│   ├── prototypes/
│   ├── component-schemas/
│   ├── behaviors/
│   └── ui/
├── localization/
├── maps/
├── resources/
├── tests/
├── game.toml
├── honknet.lock
├── package.json
└── package-lock.json
```

Здесь находятся:

- игровая серверная логика;
- клиентские игровые контроллеры;
- shared contracts игры;
- персонажи, предметы и машины;
- медицина, оружие, атмосфера и электричество;
- профессии, режимы и фракции;
- прототипы и component schemas;
- HUI-документы;
- карты;
- RSI/PNG/WebP и звук;
- FTL-локализация;
- игровые integration tests.

### Создание начального содержимого игры

Репозитории рекомендуется держать рядом:

```text
work/
├── honknet-engine/
└── space-station-15/
```

Сначала собрать CLI в репозитории движка:

```bash
cd honknet-engine
npm ci --no-audit --no-fund
npm run build:cli
```

Затем клонировать пустой игровой репозиторий и развернуть template прямо в него:

```bash
cd ..
git clone https://github.com/Honknet-Solutions/space-station-15.git
cd honknet-engine
node packages/cli/dist/index.js new ../space-station-15
```

CLI допускает существующую папку, содержащую только `.git`.

После генерации:

```bash
cd ../space-station-15
git add .
git commit -m "Initialize Space Station 15 game repository"
git push origin main
```

## 3. Подключение SDK движка к игре

### Публичная схема

Для публичной разработки SDK публикуются из `honknet-engine` как версионированные npm-пакеты:

```text
@honknet/shared
@honknet/server
@honknet/client
@honknet/hui-runtime
@honknet/cli
```

Игра фиксирует версии в `honknet.lock` и `package-lock.json`. Внутренние файлы движка напрямую не импортируются.

Порядок публикации пакетов:

```bash
cd honknet-engine
npm ci --no-audit --no-fund
npm run build:sdk
npm run build:cli

npm publish --workspace @honknet/shared --access public
npm publish --workspace @honknet/server --access public
npm publish --workspace @honknet/client --access public
npm publish --workspace @honknet/hui-runtime --access public
npm publish --workspace @honknet/cli --access public
```

После публикации в игре выполняется:

```bash
cd ../space-station-15
npm install
npm run typecheck
npm run build
```

### Локальная разработка до публикации npm-пакетов

Собрать SDK:

```bash
cd honknet-engine
npm ci --no-audit --no-fund
npm run build:sdk
```

Создать локальные npm links:

```bash
npm link --workspace @honknet/shared
npm link --workspace @honknet/server
npm link --workspace @honknet/client
npm link --workspace @honknet/hui-runtime
```

Подключить их в игровом репозитории:

```bash
cd ../space-station-15
npm link @honknet/shared @honknet/server @honknet/client @honknet/hui-runtime
npm install --workspaces --include-workspace-root
npm run typecheck
npm run build
```

Перед production-релизом локальные links заменяются обычными версиями из npm и фиксируются в `package-lock.json`.

## 4. Что нельзя копировать между репозиториями

Не переносить в `space-station-15`:

```text
crates/
apps/server/
apps/script-host/
packages/sdk-*/
packages/hui-runtime/
tools/studio/
```

Не переносить в `honknet-engine`:

```text
контент SS15
боевые системы конкретной игры
игровые карты
игровые ассеты
лоровую локализацию
баланс и роли
```

Общий код переносится в движок только тогда, когда он универсален для других игр и имеет стабильный публичный API.

## 5. Версии и релизы

Движок и игра имеют независимые версии:

```text
Honknet Engine:   0.2.0-rc.1
Space Station 15: 0.1.0
```

`space-station-15/honknet.lock` фиксирует:

- версию движка;
- protocol version;
- content schema version;
- версии всех SDK.

При выпуске игры серверный bundle и клиентский build содержат конкретную закреплённую версию Honknet Engine. Игроку не требуется отдельно устанавливать SDK или исходники движка.

## 6. Итоговое правило

```text
honknet-engine   = универсальная технология и инструменты
space-station-15 = конкретная игра и её контент
```

Разработка разделена между двумя репозиториями. Распространение готовой игры выполняется единым клиентским и серверным релизом.
