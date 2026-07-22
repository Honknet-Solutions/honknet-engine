# Readable source build

This archive contains the same Honknet Engine candidate in an expanded,
human-readable source layout.

Formatting work applied:

- Rust source expanded from compressed one-line blocks into regular modules,
  declarations, functions, branches, and method chains.
- TypeScript checked with the TypeScript compiler and formatted into readable
  declarations and control flow.
- JSON and CSS expanded and normalized.
- WGSL shader source expanded into named intermediate calculations.
- `rustfmt.toml` added for canonical formatting on a machine with Rust installed.
- `tools/source_audit.py` rejects large source files that collapse back into only
  a few lines and rejects source lines longer than 240 characters.

Checks completed in the packaging environment:

- source audit: passed;
- compressed-source candidate scan: passed;
- TypeScript type checking for Studio: passed;
- TypeScript type checking for the web shell: passed;
- ZIP integrity test: performed after packaging.

The packaging environment does not contain a Rust toolchain, so Rust compilation
must still be performed with:

```bash
cargo fmt --all
cargo check --workspace --all-features
```
