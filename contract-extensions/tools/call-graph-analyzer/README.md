# call-graph-analyzer

Builds the call graph of an ink! contract (issue #1475): which messages call which internal
functions and which external contracts, where the cycles are, how complex each function is and how
deep the call chains go.

```bash
cargo run --release -- path/to/lib.rs                        # text summary
cargo run --release -- --dot graph.dot --json graph.json path/to/contract/src
dot -Tsvg graph.dot -o graph.svg                             # Graphviz rendering
```

* Nodes: messages (blue), constructors (green), internal functions, and external call targets (red).
* Edges: internal calls, and cross-contract calls (red, thick): `build_call` / `build_create`,
  `.invoke()` / `.try_invoke()` / `.call_v1()` / `.delegate_call()`, native `.transfer()`, and any
  method call on a storage field whose type contains `Ref` (a generated contract reference).
* Cycles: strongly connected components and self-calls. A cycle whose functions also make a
  cross-contract call is marked as a potential reentrancy path. `--fail-on-cycles` turns any cycle into exit code 1.
* Complexity: `1 +` each `if`, extra `match` arm, `for`, `while`, `loop`, `&&`, `||` and `?`.
* Deepest chains: the longest acyclic call path from each message; `--top N` limits the listing.

Limits: it is a syntactic analysis. Calls through trait objects, closures or macros are not followed,
and a contract-reference call is only recognised when the storage field's type name contains `Ref`.
