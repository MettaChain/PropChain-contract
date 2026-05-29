# Interactive API Playground

This guide provides a lightweight, local interactive playground for calling PropChain contract methods directly from documentation.

## What it does

- Allows developers to send contract calls to a local node
- Uses a simplified JSON RPC UI
- Supports method invocation, parameter editing, and response inspection

## Local setup

1. Start a local Substrate node

```bash
# Example using a Substrate-based local node
./scripts/start_local_node.sh
```

2. Deploy the contract to the local node

3. Open `docs/playground.html` in a browser

## How to use the playground

1. Select a contract method
2. Enter the local node RPC endpoint, e.g. `http://127.0.0.1:9944`
3. Provide contract address and method parameters
4. Execute the call and inspect the response in the UI

## Example workflow

```text
RPC endpoint: http://127.0.0.1:9944
Contract address: 5F...abc
Method: swap_exact_base_for_quote
Parameters: { "pair_id": 1, "amount_in": 100, "min_quote_out": 90 }
```

## Development note

This playground is intentionally lightweight and designed for local development and integration testing.
For production, use the full SDK or client library for contract interaction.
