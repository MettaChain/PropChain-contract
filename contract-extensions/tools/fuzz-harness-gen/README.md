# fuzz-harness-gen

Generates `proptest` fuzzing harnesses for ink! contracts (issue #1474): one test file per
`#[ink(message)]`, in a small crate you can run with `cargo test`.

```bash
cargo run --release -- --contract ../../../contracts/some/src/lib.rs \
    --out /tmp/some-fuzz --crate-name some_contract --crate-path /abs/path/to/contracts/some \
    --cases 512 --depth 6
cd /tmp/some-fuzz && cargo test
```

Each generated test builds the contract with its first constructor, sets a fuzzed caller, calls the
message up to `--depth` times with generated arguments (catching panics) and, after each call,
checks invariants: every argument-less `&self` message must still run, and if the contract has
`total_supply()` and `balance_of(AccountId)`, the caller's balance must not exceed the total supply.
A panic or a violated invariant fails the property; `proptest` shrinks and prints the failing input.

Options: `--cases` (iterations per property, default 256), `--depth` (calls per case, default 4),
`--max-len` (max length of generated vectors and strings, default 32), `--message NAME` (only that
message; repeatable).

Supported argument types: integers, `bool`, `Balance`, `BlockNumber`, `Timestamp`, `AccountId`,
`Hash`, `String`, `[u8; N]` (N <= 32), and `Vec<T>` / `Option<T>` of those. Anything else uses
`Default::default()` and is noted at the top of the generated file.

The generated crate depends on the contract crate by path and needs the contract to build on the host
(`std` feature); constructors and messages must be `pub`. Whether a particular contract's harness
compiles unchanged depends on its types, so run it once after generating.
