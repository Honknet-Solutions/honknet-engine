# Game repository contract

Игра хранится отдельно от Honknet Engine и содержит `game.toml`, `honknet.lock`, TypeScript modules, content, resources, maps и localization.

Движок загружает только пути, объявленные игровым manifest. Игра не должна импортировать внутренние файлы engine repository. Допустимы только опубликованные `@honknet/server`, `@honknet/client`, `@honknet/shared` и `@honknet/hui-runtime`.

Reference fixture в `examples/minimal-game` проверяет обратную совместимость API. Empty template в `templates/empty-game` используется командой `honknet new`.
