# event-completeness-checker

Checks that every state-changing `#[ink(message)]` emits an event. Closes #1469.

```bash
cargo run --manifest-path contract-extensions/tools/event-completeness-checker/Cargo.toml -- \
  [--config FILE] [--format json|markdown|text] [--fail-on-missing] PATH...
```

A message is *state-changing* when it takes `&mut self`, assigns to `self.*`, or calls a mutating method
(`insert`, `remove`, `take`, `push`, `pop`, `clear`, `set`, `transfer`) on a `self.*` expression. It *emits*
when its body calls `emit_event` or any function whose name starts with `emit`. Statuses: `Emits`,
`MissingEvent`, `View`, `Whitelisted` (config `allow = message` / `allow = Contract::message`).
The check is syntactic; a mutation hidden in a helper is only visible through the message's own `&mut self`.
