# Honknet Engine 1.0.0-rc.1 — Complete implementation candidate

This source archive is the first unified test candidate. It intentionally replaces the earlier 0.2/dev foundation archive and carries all engine areas in one workspace.

The archive has passed source/TOML/TypeScript structural checks in the packaging environment. Rust compilation and platform runtime testing were not executable there because the Rust toolchain and external package network were unavailable. Begin with `verify.ps1` on Windows or `./verify.sh` on Linux/macOS and report the complete first failing command.

## Toolchain compatibility fix

- Minimum Rust toolchain updated from 1.85.1 to 1.88.0.
- Docker and GitHub Actions now use Rust 1.88 consistently.
- This resolves MSRV failures from `time 0.3.54`, ICU 2.2, `idna_adapter` and current Wayland dependencies.
