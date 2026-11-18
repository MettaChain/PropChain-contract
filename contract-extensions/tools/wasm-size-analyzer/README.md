# wasm-size-analyzer

Breaks a compiled ink! contract's WASM binary down by section, lists the largest functions and
prints optimization recommendations. Closes #1470.

```bash
# analyze an existing build
cargo run --manifest-path contract-extensions/tools/wasm-size-analyzer/Cargo.toml -- \
  --wasm target/ink/my_contract.wasm --top 10 --max-size 131072

# build with `cargo contract build --release` first, include source checks and benchmarks
cargo run --manifest-path contract-extensions/tools/wasm-size-analyzer/Cargo.toml -- \
  --build contracts/staking --benchmarks wasm-size-reports

# compare two builds
cargo run --manifest-path contract-extensions/tools/wasm-size-analyzer/Cargo.toml -- --diff old.wasm new.wasm
```

- Exit code is 1 when the contract exceeds `--max-size`; 2 on usage or I/O errors.
- `--benchmarks DIR` reads `*.json` reports in the format written by `scripts/check-wasm-size.sh` (and `*.wasm` files).
- `--source DIR` (implied by `--build`) flags dependencies that are never referenced and heavy `Vec<` use.
- Function names need the `name` custom section; release builds usually strip it, so functions are then reported by index.
- The tool needs `cargo contract` only for `--build`; it has no crate dependencies.
