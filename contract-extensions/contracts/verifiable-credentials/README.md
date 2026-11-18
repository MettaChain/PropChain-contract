# verifiable-credentials

Verifiable credentials: issuers (appointed by the admin) issue KYC / property-ownership / accredited-investor /
insurance-active credentials to subjects; anyone verifies validity; only the issuer revokes; credentials expire after
an expiry block (`0` = never). Closes #1438.

Only a hash of the off-chain credential document is stored. The contract does not call the `identity` contract on-chain.
`verify_credential` is a read-only view; `verify_and_log` also emits `CredentialVerified`.

```bash
cargo test --manifest-path contract-extensions/contracts/verifiable-credentials/Cargo.toml
```
