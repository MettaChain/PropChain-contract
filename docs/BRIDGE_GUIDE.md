# PropChain Cross-Chain Bridge Guide

## Architecture

The bridge enables property token transfers between Substrate-based chains
using a multi-signature operator model:

```
Source Chain          Bridge Operators           Destination Chain
    |                    |    |    |                    |
    |-- initiate ------->|    |    |                    |
    |                    |-- sign -->                   |
    |                    |    |-- sign -->              |
    |                    |    |    |-- execute -------->|
    |                    |    |    |                    |
```

## Supported Chains

Each chain is configured with:
- **chain_id**: Unique numeric identifier
- **gas_multiplier**: Cost adjustment (100 = 1x, 150 = 1.5x)
- **confirmation_blocks**: Blocks before finality (default: 6)
- **supported_tokens**: Token IDs transferable on this chain

## Bridge Flow

### 1. Initiate

```rust
bridge.initiate_bridge_multisig(
    token_id,           // Property token to transfer
    destination_chain,  // Target chain ID
    recipient,          // Recipient address on target chain
    metadata,           // Optional property metadata to preserve
);
```

### 2. Collect Signatures

Bridge operators sign the request:
```rust
bridge.sign_bridge_request(request_id);
```

### 3. Execute

After reaching `min_signatures`, any operator can execute:
```rust
bridge.execute_bridge(request_id);
```

### 4. Recovery

If a bridge fails, the admin can recover:
```rust
bridge.recover_failed_bridge(request_id, RecoveryAction::RetryBridge);
// or: RecoveryAction::UnlockToken, RefundGas, CancelBridge
```

## Security Measures

- **Multi-sig threshold**: Minimum 2 operator signatures required
- **Timelock**: Requests expire after configurable block count
- **Metadata preservation**: Property data carried across chains
- **Admin recovery**: Failed bridges can be retried or cancelled
- **Chain deactivation**: Compromised chains can be disabled

## Monitoring

| Metric | Source | Alert Threshold |
|--------|--------|-----------------|
| Pending requests | Bridge contract state | > 10 pending |
| Failed bridges | BridgeRecovered events | Any failure |
| Operator signatures | sign_bridge_request events | < min_signatures in 1 hour |
| Chain availability | chain_info.is_active | Any chain deactivated |

## Adding a New Chain

```rust
bridge.update_chain_info(
    new_chain_id,
    ChainBridgeInfo {
        chain_id: new_chain_id,
        chain_name: "NewChain".into(),
        bridge_contract_address: Some(contract_addr),
        is_active: true,
        gas_multiplier: 120,    // 1.2x gas
        confirmation_blocks: 10,
        supported_tokens: vec![token_1, token_2],
    },
);
```
