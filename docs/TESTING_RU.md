# Первый запуск и тестирование

1. Установить Rust 1.88.0, Node.js 22, системные библиотеки Vulkan/ALSA.
2. Выполнить `cargo check --workspace --all-features`.
3. Выполнить `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
4. Выполнить `cargo test --workspace --all-features`.
5. Запустить сервер: `cargo run -p honknet-server`.
6. Запустить desktop-клиент: `cargo run -p honknet-client`.
7. Запустить нагрузочный клиент: `cargo run -p honknet-headless-client -- --ticks 18000`.
8. Studio: `npm install`, затем `npm run studio`.

Все найденные ошибки следует фиксировать вместе с ОС, версией Rust, командой и полным выводом.
