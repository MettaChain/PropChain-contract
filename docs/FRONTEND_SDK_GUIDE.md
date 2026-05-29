# PropChain Frontend SDK Guide

Comprehensive guide for integrating PropChain smart contracts into frontend applications using the `@propchain/sdk` TypeScript SDK.

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [API Reference](#api-reference)
  - [PropChainClient](#propchainclient)
  - [PropertyRegistryClient](#propertyregistryclient)
  - [PropertyTokenClient](#propertytokenclient)
  - [EscrowClient](#escrowclient)
  - [OracleClient](#oracleclient)
- [Type Reference](#type-reference)
- [React Integration](#react-integration)
- [Event Handling](#event-handling)
- [Error Handling](#error-handling)
- [Advanced Usage](#advanced-usage)
- [Testing Guide](#testing-guide)
- [Troubleshooting](#troubleshooting)

---

## Installation

### Install the SDK

```bash
# From the project root
npm install ./sdk/frontend

# Or install dependencies directly
npm install @polkadot/api @polkadot/api-contract @polkadot/extension-dapp
```

### Requirements

- **Node.js** 18.0+
- **TypeScript** 5.0+
- A running Substrate node (local or remote)

---

## Quick Start

### 1. Create a Client

```typescript
import { PropChainClient } from '@propchain/sdk';

const client = await PropChainClient.create(
  'ws://localhost:9944',
  {
    propertyRegistry: '5Grwva...', // Contract address
    propertyToken: '5FHnea...',
  },
);
```

### 2. Register a Property

```typescript
import { createKeyringPair } from '@propchain/sdk';

// Development account (use browser extension in production)
const alice = createKeyringPair('//Alice');

const { propertyId, txHash } = await client.propertyRegistry.registerProperty(
  alice,
  {
    location: '123 Main St, New York, NY',
    size: 2500,
    legalDescription: 'Lot 1, Block 2, City Subdivision',
    valuation: BigInt('50000000000000'), // $500,000 with 8 decimals
    documentsUrl: 'ipfs://QmXoypizj...',
  },
);

console.log(`Property ${propertyId} registered in tx ${txHash}`);
```

### 3. Query a Property

```typescript
const property = await client.propertyRegistry.getProperty(propertyId);
if (property) {
  console.log('Location:', property.metadata.location);
  console.log('Owner:', property.owner);
  console.log('Valuation:', formatValuation(property.metadata.valuation));
}
```

### 4. Subscribe to Events

```typescript
const sub = await client.propertyRegistry.on('PropertyRegistered', (event) => {
  console.log(`New property #${event.propertyId} by ${event.owner}`);
});

// Later: unsubscribe
sub.unsubscribe();
```

### 5. Disconnect

```typescript
await client.disconnect();
```

---

## API Reference

### PropChainClient

Main entry point that manages the connection and sub-clients.

| Method | Returns | Description |
|--------|---------|-------------|
| `PropChainClient.create(wsEndpoint, addresses, options?)` | `Promise<PropChainClient>` | Connect to a node |
| `PropChainClient.fromApi(api, addresses)` | `PropChainClient` | Wrap existing API |
| `.propertyRegistry` | `PropertyRegistryClient` | Property registry sub-client |
| `.propertyToken` | `PropertyTokenClient` | Property token sub-client |
| `.escrow` | `EscrowClient` | Escrow sub-client |
| `.oracle` | `OracleClient` | Oracle sub-client |
| `.disconnect()` | `Promise<void>` | Disconnect |
| `.isConnected` | `boolean` | Connection status |
| `.api` | `ApiPromise` | Raw API access |
| `.getChainName()` | `Promise<string>` | Chain name |
| `.getBlockNumber()` | `Promise<number>` | Current block |

#### ClientOptions

```typescript
interface ClientOptions {
  types?: Record<string, unknown>;   // Custom types
  autoReconnect?: boolean;            // Default: true
  maxReconnectAttempts?: number;      // Default: 5
  connectionTimeout?: number;         // Default: 30000ms
}
```

---

### PropertyRegistryClient

Full API for property management, escrow, badges, batch operations, and admin.

#### Property Operations

| Method | Returns | Description |
|--------|---------|-------------|
| `registerProperty(signer, metadata)` | `Promise<{ propertyId } & TxResult>` | Register property |
| `getProperty(id)` | `Promise<PropertyInfo \| null>` | Query property |
| `getOwnerProperties(owner)` | `Promise<number[]>` | Owner's property IDs |
| `getPropertyCount()` | `Promise<number>` | Total properties |
| `transferProperty(signer, id, to)` | `Promise<TxResult>` | Transfer ownership |
| `updateMetadata(signer, id, metadata)` | `Promise<TxResult>` | Update metadata |
| `approve(signer, id, to)` | `Promise<TxResult>` | Approve transfer |
| `getApproved(id)` | `Promise<string \| null>` | Get approved account |

#### Escrow Operations

| Method | Returns | Description |
|--------|---------|-------------|
| `createEscrow(signer, propertyId, buyer, seller, amount)` | `Promise<{ escrowId } & TxResult>` | Create escrow |
| `releaseEscrow(signer, escrowId)` | `Promise<TxResult>` | Release escrow |
| `refundEscrow(signer, escrowId)` | `Promise<TxResult>` | Refund escrow |
| `getEscrow(escrowId)` | `Promise<EscrowInfo \| null>` | Query escrow |

#### Health & Analytics

| Method | Returns | Description |
|--------|---------|-------------|
| `healthCheck()` | `Promise<HealthStatus>` | Full health status |
| `ping()` | `Promise<boolean>` | Liveness check |
| `getVersion()` | `Promise<number>` | Contract version |
| `getAdmin()` | `Promise<string>` | Admin account |
| `getGlobalAnalytics()` | `Promise<GlobalAnalytics>` | Analytics data |
| `getPortfolioSummary(owner)` | `Promise<PortfolioSummary>` | Portfolio summary |

#### Badge Operations

| Method | Returns | Description |
|--------|---------|-------------|
| `issueBadge(signer, propertyId, type, expiry, url)` | `Promise<TxResult>` | Issue badge |
| `revokeBadge(signer, propertyId, type, reason)` | `Promise<TxResult>` | Revoke badge |
| `getBadge(propertyId, type)` | `Promise<Badge \| null>` | Query badge |
| `requestVerification(signer, propertyId, type, url)` | `Promise<{ requestId } & TxResult>` | Request verification |

#### Batch Operations

| Method | Returns | Description |
|--------|---------|-------------|
| `batchRegisterProperties(signer, metadataList)` | `Promise<{ batchResult } & TxResult>` | Batch register |
| `batchTransferProperties(signer, ids, to)` | `Promise<TxResult>` | Batch transfer |
| `getBatchConfig()` | `Promise<BatchConfig>` | Batch config |
| `getBatchStats()` | `Promise<BatchOperationStats>` | Batch stats |

---

### PropertyTokenClient

ERC-721/1155 compatible token operations plus fractional ownership, governance, marketplace, and bridge.

#### ERC-721 Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `balanceOf(owner)` | `Promise<number>` | Token balance |
| `ownerOf(tokenId)` | `Promise<string \| null>` | Token owner |
| `transferFrom(signer, from, to, tokenId)` | `Promise<TxResult>` | Transfer token |
| `approve(signer, to, tokenId)` | `Promise<TxResult>` | Approve transfer |
| `setApprovalForAll(signer, operator, approved)` | `Promise<TxResult>` | Set operator |
| `isApprovedForAll(owner, operator)` | `Promise<boolean>` | Check operator |
| `totalSupply()` | `Promise<number>` | Total supply |

#### Property Token Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `registerPropertyWithToken(signer, metadata)` | `Promise<{ tokenId } & TxResult>` | Mint NFT |
| `attachLegalDocument(signer, tokenId, hash, type)` | `Promise<TxResult>` | Attach document |
| `verifyCompliance(signer, tokenId, verified)` | `Promise<TxResult>` | Verify compliance |
| `getOwnershipHistory(tokenId)` | `Promise<OwnershipTransfer[]>` | Ownership history |

#### Fractional Ownership

| Method | Returns | Description |
|--------|---------|-------------|
| `issueShares(signer, tokenId, to, amount)` | `Promise<TxResult>` | Issue shares |
| `redeemShares(signer, tokenId, amount)` | `Promise<TxResult>` | Redeem shares |
| `getShareBalance(tokenId, account)` | `Promise<bigint>` | Share balance |
| `depositDividends(signer, tokenId, amount)` | `Promise<TxResult>` | Deposit rent or dividend income |
| `distributeRentalIncome(signer, tokenId, amount)` | `Promise<TxResult>` | Distribute rental income through management agent |
| `withdrawDividends(signer, tokenId)` | `Promise<TxResult>` | Withdraw dividends |

#### Governance

| Method | Returns | Description |
|--------|---------|-------------|
| `createProposal(signer, tokenId, hash, quorum)` | `Promise<{ proposalId } & TxResult>` | Create proposal |
| `vote(signer, tokenId, proposalId, support)` | `Promise<TxResult>` | Vote |
| `executeProposal(signer, tokenId, proposalId)` | `Promise<TxResult>` | Execute proposal |
| `getProposal(tokenId, proposalId)` | `Promise<Proposal \| null>` | Query proposal |

#### Marketplace

| Method | Returns | Description |
|--------|---------|-------------|
| `placeAsk(signer, tokenId, price, amount)` | `Promise<TxResult>` | Place sell order |
| `cancelAsk(signer, tokenId)` | `Promise<TxResult>` | Cancel ask |
| `buyShares(signer, tokenId, seller, shares, payment)` | `Promise<TxResult>` | Buy shares with attached payment |
| `getLastTradePrice(tokenId)` | `Promise<bigint>` | Last trade price |

#### Cross-Chain Bridge

| Method | Returns | Description |
|--------|---------|-------------|
| `initiateBridgeMultisig(signer, tokenId, chain, recipient, sigs, timeout)` | `Promise<{ requestId } & TxResult>` | Initiate bridge |
| `signBridgeRequest(signer, requestId, approve)` | `Promise<TxResult>` | Sign request |
| `executeBridge(signer, requestId)` | `Promise<TxResult>` | Execute bridge |
| `getBridgeStatus(tokenId)` | `Promise<BridgeStatus \| null>` | Bridge status |

---

### EscrowClient

Convenience wrapper for escrow operations.

| Method | Returns | Description |
|--------|---------|-------------|
| `create(signer, propertyId, buyer, seller, amount)` | `Promise<{ escrowId } & TxResult>` | Create escrow |
| `release(signer, escrowId)` | `Promise<TxResult>` | Release |
| `refund(signer, escrowId)` | `Promise<TxResult>` | Refund |
| `get(escrowId)` | `Promise<EscrowInfo \| null>` | Query |

---

### OracleClient

Property valuation oracle interactions.

| Method | Returns | Description |
|--------|---------|-------------|
| `getValuation(propertyId)` | `Promise<PropertyValuation>` | Get valuation |
| `getValuationWithConfidence(propertyId)` | `Promise<ValuationWithConfidence>` | Get with confidence |
| `requestValuation(signer, propertyId)` | `Promise<{ requestId } & TxResult>` | Request update |
| `getMarketVolatility(type, location)` | `Promise<VolatilityMetrics>` | Market volatility |

---

## Type Reference

### Core Types

```typescript
interface PropertyMetadata {
  location: string;
  size: number;
  legalDescription: string;
  valuation: bigint;
  documentsUrl: string;
}

interface PropertyInfo {
  id: number;
  owner: string;
  metadata: PropertyMetadata;
  registeredAt: number;
}

interface TxResult {
  txHash: string;
  blockHash: string;
  blockNumber: number;
  events: ContractEvent[];
  success: boolean;
}
```

### Enums

```typescript
enum PropertyType { Residential, Commercial, Industrial, Land, ... }
enum BadgeType { OwnerVerification, DocumentVerification, ... }
enum ProposalStatus { Open, Executed, Rejected, Closed }
enum BridgeOperationStatus { None, Pending, Locked, InTransit, ... }
enum FeeOperation { RegisterProperty, TransferProperty, ... }
```

See [types/index.ts](../sdk/frontend/src/types/index.ts) for the complete list.

---

## React Integration

### Hooks Pattern

```tsx
import { useState, useEffect, useCallback } from 'react';
import { PropChainClient, PropertyInfo } from '@propchain/sdk';

function usePropChain(wsEndpoint: string, addresses: ContractAddresses) {
  const [client, setClient] = useState<PropChainClient | null>(null);

  useEffect(() => {
    PropChainClient.create(wsEndpoint, addresses).then(setClient);
    return () => { client?.disconnect(); };
  }, [wsEndpoint]);

  return client;
}

function useProperty(client: PropChainClient | null, id: number) {
  const [property, setProperty] = useState<PropertyInfo | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!client) return;
    setLoading(true);
    client.propertyRegistry.getProperty(id)
      .then(setProperty)
      .finally(() => setLoading(false));
  }, [client, id]);

  return { property, loading };
}
```

### Wallet Connection

```tsx
import { connectExtension, getExtensionSigner } from '@propchain/sdk';

function ConnectWallet() {
  const handleConnect = async () => {
    const accounts = await connectExtension('My PropChain dApp');
    const signer = await getExtensionSigner(accounts[0].address);
    // Use signer for transactions
  };

  return <button onClick={handleConnect}>Connect Wallet</button>;
}
```

### Context Provider Pattern

```tsx
import { createContext, useContext, ReactNode } from 'react';
import { PropChainClient } from '@propchain/sdk';

const PropChainContext = createContext<PropChainClient | null>(null);

export function PropChainProvider({
  children,
  wsEndpoint,
  addresses,
}: {
  children: ReactNode;
  wsEndpoint: string;
  addresses: ContractAddresses;
}) {
  const client = usePropChain(wsEndpoint, addresses);
  return (
    <PropChainContext.Provider value={client}>
      {children}
    </PropChainContext.Provider>
  );
}

export function usePropChainClient() {
  const client = useContext(PropChainContext);
  if (!client) throw new Error('Must be used within PropChainProvider');
  return client;
}
```

---

## Event Handling

### Subscribe to All Events

```typescript
import { subscribeToEvents } from '@propchain/sdk';

const sub = await subscribeToEvents(api, contractAddress, abi, (event) => {
  console.log(`${event.name}:`, event.args);
});
```

### Subscribe to Specific Events

```typescript
// Type-safe event subscription
const sub = await client.propertyRegistry.on('PropertyRegistered', (event) => {
  // event is typed as PropertyRegisteredEvent
  console.log('ID:', event.propertyId);
  console.log('Owner:', event.owner);
  console.log('Location:', event.location);
});
```

### Filter Events from Transaction

```typescript
import { filterEvents, extractTypedEvents } from '@propchain/sdk';

const result = await client.propertyRegistry.registerProperty(signer, metadata);
const regEvents = extractTypedEvents(result.events, 'PropertyRegistered');
```

---

## Error Handling

### Catching Typed Errors

```typescript
import { PropChainError, getUserFriendlyMessage } from '@propchain/sdk';

try {
  await client.propertyRegistry.transferProperty(signer, 999, recipient);
} catch (error) {
  if (error instanceof PropChainError) {
    console.log('Category:', error.category);     // 'PropertyRegistry'
    console.log('Variant:', error.variant);         // 'PropertyNotFound'
    console.log('Description:', error.description); // 'Property does not exist...'

    // Display to user
    showToast(getUserFriendlyMessage(error));
  }
}
```

### Error Categories

- `PropertyRegistry` — Registration, transfer, escrow, badge errors
- `PropertyToken` — Token, bridge, governance errors
- `Oracle` — Valuation and data feed errors
- `Unknown` — Unrecognised errors

---

## Advanced Usage

### Gas Estimation

```typescript
const gas = await client.propertyRegistry.estimateGas(
  myAddress,
  'register_property',
  [metadata],
);
console.log('Gas required:', gas.gasRequired);
console.log('Storage deposit:', gas.storageDeposit);
```

### Batch Operations

```typescript
const metadata = [property1, property2, property3];
const result = await client.propertyRegistry.batchRegisterProperties(signer, metadata);
```

### Network Presets

```typescript
import { NETWORKS, connectToNetwork } from '@propchain/sdk';

// Use built-in presets
const api = await connectToNetwork('westend');

// Or access preset configs
console.log(NETWORKS.local.wsEndpoint); // 'ws://127.0.0.1:9944'
```

### Formatting Utilities

```typescript
import {
  formatBalance,
  parseBalance,
  formatValuation,
  truncateAddress,
  relativeTime,
  formatPropertySize,
} from '@propchain/sdk';

formatBalance(BigInt('10000000000000'), 12);        // '10.0000'
parseBalance('10.5', 12);                            // BigInt('10500000000000')
formatValuation(BigInt('50000000000000'));            // '$500,000.00'
truncateAddress('5GrwvaEF5zXb26Fz9r...');            // '5Grwva…utQY'
relativeTime(Date.now() - 300000);                   // '5 minutes ago'
formatPropertySize(25000);                           // '2.50 ha'
```

---

## Testing Guide

### Running SDK Tests

```bash
cd sdk/frontend

# Run all tests
npm test

# Watch mode
npx vitest

# Coverage report
npm run test:coverage
```

### Writing Tests for Your dApp

```typescript
import { describe, it, expect, vi } from 'vitest';

// Mock the SDK
vi.mock('@propchain/sdk', () => ({
  PropChainClient: {
    create: vi.fn().mockResolvedValue({
      propertyRegistry: {
        getProperty: vi.fn().mockResolvedValue({
          id: 1,
          owner: '5Grw...',
          metadata: { location: 'Test', size: 100, valuation: BigInt(100000) },
        }),
      },
    }),
  },
}));

describe('My Property Component', () => {
  it('displays property data', async () => {
    // Test your component using the mocked SDK
  });
});
```

### Integration Tests (with Local Node)

```bash
# 1. Start node
docker-compose up -d

# 2. Deploy contracts
./scripts/deploy.sh --network local

# 3. Set contract addresses
export REGISTRY_ADDRESS=5Grw...
export TOKEN_ADDRESS=5FHn...

# 4. Run integration tests
cd sdk/frontend
npx vitest run __tests__/integration.test.ts
```

---

## Troubleshooting

### Common Issues

| Problem | Solution |
|---------|----------|
| `ConnectionError: Failed to connect` | Ensure Substrate node is running on the correct port |
| `PropChainError: ContractPaused` | The contract is paused — contact admin or wait for resume |
| `TransactionError: Insufficient balance` | Ensure account has enough tokens for gas + value |
| `Unknown contract method` | Update SDK to match deployed contract version |
| `No Polkadot.js extension` | Install from [polkadot.js.org/extension](https://polkadot.js.org/extension/) |

### Debug Logging

```typescript
// Enable Polkadot.js debug logging
import { logger } from '@polkadot/util';
logger.setLevel('debug');
```

### ABI Updates

The SDK ships with placeholder ABIs. For production use:

1. Build contracts: `cargo contract build`
2. Copy the generated `*.contract` / `*.json` files from `target/ink/`
3. Place them in `sdk/frontend/src/abi/`

---

## Example App

See the complete working example at [`sdk/frontend/examples/react-app/`](../sdk/frontend/examples/react-app/).

```bash
cd sdk/frontend/examples/react-app
npm install
npm run dev
```
