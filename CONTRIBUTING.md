# Contributing

Перед pull request выполните:

```bash
./verify.sh
```

Игровой контент размещается в отдельном game module. Rust-код добавляется только для engine hot paths и native infrastructure. Builder-файлы должны оставаться человекочитаемыми YAML/FTL и не генерировать исходный Rust/TypeScript.
