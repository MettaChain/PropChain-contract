# @propchain/sdk

TypeScript SDK for integrating with PropChain smart contracts on Substrate/Polkadot.

## Features

- 🏠 **Property Registry** — Register, transfer, query, and manage properties
- 🔐 **Escrow** — Secure property transfer escrows with release/refund
- 🪙 **Property Tokens** — ERC-721/1155 compatible NFTs with fractional ownership
- 🗳️ **Governance** — On-chain proposals and voting for fractional holders
- 💹 **Marketplace** — Secondary market for fractional property shares
- ⛓️ **Cross-Chain Bridge** — Multi-signature bridge for cross-chain transfers
- 📊 **Oracle** — Property valuations with confidence scoring
- 🛡️ **Badges** — Property verification badges with appeal system
- 📦 **Batch Operations** — Register/transfer multiple properties in one tx
- 🔔 **Event Subscriptions** — Type-safe real-time event streaming

## Quick Start

```typescript
import { PropChainClient, createKeyringPair, formatValuation } from '@propchain/sdk';

// Connect to a node
const client = await PropChainClient.create('ws://localhost:9944', {
  propertyRegistry: '5Grwva...',
  propertyToken: '5FHnea...',
});

// Register a property
const alice = createKeyringPair('//Alice');
const { propertyId } = await client.propertyRegistry.registerProperty(alice, {
  location: '123 Main St, New York, NY',
  size: 2500,
  legalDescription: 'Lot 1, Block 2',
  valuation: BigInt('50000000000000'),
  documentsUrl: 'ipfs://Qm...',
});

// Query and display
const property = await client.propertyRegistry.getProperty(propertyId);
console.log(formatValuation(property!.metadata.valuation)); // '$500,000.00'

// Subscribe to events
await client.propertyRegistry.on('PropertyRegistered', (event) => {
  console.log(`Property #${event.propertyId} registered by ${event.owner}`);
});
```

## Dynamic Contract Modules (Module Federation)

If you want to load contract clients/ABIs on demand (e.g. microfrontends), use `FederatedPropChainClient`:

```typescript
import { FederatedPropChainClient } from '@propchain/sdk/federation';

const client = await FederatedPropChainClient.create('ws://localhost:9944', {
  propertyRegistry: '5Grwva...',
  propertyToken: '5FHnea...',
});

const registry = await client.contract('propertyRegistry');
const token = await client.contract('propertyToken');
```

## Documentation

See the full [Frontend SDK Guide](../../docs/FRONTEND_SDK_GUIDE.md) for:
- Complete API reference
- React integration patterns (hooks, context)
- Event handling
- Error handling
- Testing guide
- Troubleshooting

## Example App

```bash
cd examples/react-app
npm install
npm run dev
```

## Development

```bash
# Install dependencies
npm install

# Run tests
npm test

# Type check
npm run typecheck

# Build
npm run build
```

## License

MIT
