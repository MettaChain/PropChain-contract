# gas-estimator

Estimates gas for each `#[ink(message)]` by analysing the message body (issue #1473). It reads
source with `syn`; nothing is compiled or run, so the numbers are **approximations**: use them to
rank messages and spot outliers, then measure real execution.

```bash
cargo run --release -- path/to/lib.rs                       # text report, most expensive first
cargo run --release -- --format json --iterations 50 path/to/lib.rs
cargo run --release -- --config model.conf --set block_gas_limit=800000000000 path/to/lib.rs
```

What is counted, per message: storage reads, storage writes, external calls, event emissions and
other compute. Operations inside `for` / `while` / `loop` are multiplied by the assumed iteration
count (`loop_iterations`, default 10), nested loops multiply. The cost model defaults are of the
order of `pallet-contracts` reference weights and can be overridden (`storage_read`,
`storage_write`, `external_call`, `event_emit`, `compute_op`, `call_overhead`, `block_gas_limit`,
`loop_iterations`) in a `key = value` config file or with `--set`.

Messages above `block_gas_limit` are flagged and the tool exits `1`. Suggestions are printed for loops
that write, read or call, nested loops, many writes, and external calls.

Limits of a static count: a call through a generated contract reference (`self.token.transfer(..)`)
has no syntactic marker, so it counts as compute unless the method is named like a known external
call; loop bounds are assumed, not read from the code; helper functions called from a message are not
followed.
