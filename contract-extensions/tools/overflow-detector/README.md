# overflow-detector

Syntax-level scan of Rust / ink! sources for unchecked integer arithmetic (`+ - * /`
and the compound-assignment forms) and truncating `as` casts. Closes #1467.

```bash
cargo run --manifest-path contract-extensions/tools/overflow-detector/Cargo.toml -- \
  [--config FILE] [--json] [--fail-on high|medium|low] PATH...
```

- Safe forms (`checked_*`, `saturating_*`, `wrapping_*`, `overflowing_*`) are method calls and are never flagged.
- Risk: `a * b` on two non-constants is **high**; other non-constant `+ - *` and division by a non-constant are **medium**; constant operands are **low**; casts to `u8/u16/u32/i8/i16/i32` are **high**, to `u64/i64/usize/isize` **medium**.
- There is no type information, so these are candidates, not proofs.
- `#[cfg(test)]` modules and `#[test]` functions are skipped.
- Config file: `safe_call = NAME` lines (comments with `#`). Arithmetic inside arguments of a call to `NAME` is treated as safe.
- `--json` prints `{"summary": {...}, "findings": [...]}`. Exit code is 1 when a finding at or above `--fail-on` (default `high`) exists.
