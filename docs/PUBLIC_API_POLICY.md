# Public API policy

Honknet использует SemVer для engine и SDK packages. Patch-релизы не ломают публичные контракты. Minor-релизы добавляют совместимые API. Удаление или несовместимое изменение публичного API допускается только в major-релизе после периода deprecation.

Отдельно версионируются:

- engine version;
- wire protocol version;
- content schema version;
- HUI document version.

Игровой release обязан фиксировать точную версию движка и SDK в `honknet.lock`.

Публичными считаются exports пакетов `@honknet/*`, формат `game.toml`, протокол content schemas и документированные Rust crates. Внутренности `apps/server` и `tools/studio/src` не являются стабильным API.
