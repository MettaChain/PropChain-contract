# upgradeable-factory

Deploys `upgradeable-proxy` instances for registered implementations and controls upgrades (admin or a designated
upgrade manager). Closes #1445.

Storage compatibility is modelled as a **layout id** (storage family) plus a **layout revision**: an upgrade must stay
in the same family and not lower the revision. The factory trusts what the admin registered; it cannot inspect compiled code.
Proxy instantiation and the `upgrade_to` call on the proxy are cross-contract operations that the off-chain engine
cannot run, so they are not unit-tested; the authorisation, registry and compatibility logic is.

```bash
cargo test --manifest-path contract-extensions/contracts/upgradeable-factory/Cargo.toml
```
