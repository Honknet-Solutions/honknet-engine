# Build verification

Detailed results are recorded in `VERIFICATION.md`.

Verified while preparing this archive:

- `npm ci --no-audit --no-fund`;
- content validation;
- strict TypeScript typecheck for every workspace;
- 15/15 TypeScript tests;
- SDK, CLI, reference game and script-host builds;
- production browser-client build: 816 modules;
- production Studio build: 103 modules;
- public npm package dry-runs;
- CLI game-template creation;
- 25,000-prototype content benchmark;
- Rust grammar parse for 28 files with zero syntax errors.

The packaging environment did not contain a downloadable Rust toolchain. Rust formatting, Clippy, unit tests, release compilation and long-running capacity tests are mandatory release gates and are not claimed as completed.
