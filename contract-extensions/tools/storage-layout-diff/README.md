# storage-layout-diff

Compares the `#[ink(storage)]` struct of two versions of an ink! contract and
reports changes that would corrupt storage when a proxy is upgraded to the new
version (issue #1471). It parses source with `syn`; nothing is compiled.

```bash
cargo run --release -- --old old/lib.rs --new new/lib.rs            # text report
cargo run --release -- --old old/lib.rs --new new/lib.rs --format json   # for CI
```

| Change | Class |
|---|---|
| field removed | **Breaking** |
| existing fields reordered | **Breaking** |
| field inserted before an existing field | **Breaking** |
| field type changed (not a widening) | **Breaking** |
| integer widened within the same signedness (`u32` -> `u64`) | **Warning** |
| field(s) appended after every existing field | **Safe** |

Exit codes: `0` no breaking change, `1` at least one breaking change, `2` usage or parse error.
Only the first `#[ink(storage)]` struct in each file is compared. Types are compared as written
(whitespace ignored), so a type alias that changes meaning is not detected.

See `docs/REGENERATE_PROXY.md` and `contracts/proxy/` for the upgrade pattern this guards.
