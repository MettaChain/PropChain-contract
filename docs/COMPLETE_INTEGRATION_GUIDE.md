# Complete Integration Guide for PropChain

## Overview

This comprehensive guide walks you through integrating PropChain smart contracts into your applications. Whether you're building a frontend dApp, backend service, or mobile application, this guide provides step-by-step instructions with working code examples.

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Prerequisites and Setup](#prerequisites-and-setup)
3. [Core Integration Steps](#core-integration-steps)
4. [Common Use Cases](#common-use-cases)
5. [Advanced Integration Patterns](#advanced-integration-patterns)
6. [Testing Your Integration](#testing-your-integration)
7. [Troubleshooting](#troubleshooting)
8. [Best Practices](#best-practices)

---

## Quick Start

**5-Minute Integration**:
```bash
# 1. Install dependencies
npm install @polkadot/api @polkadot/api-contract

# 2. Connect and interact
const api = await ApiPromise.create({ 
  provider: new WsProvider('wss://rpc.propchain.io') 
});

// Load contract and register property
const contract = new ContractPromise(api, abi, contractAddress);
await contract.tx.registerProperty({ gasLimit: -1 }, metadata);
```

For detailed instructions, continue reading below.

---

## Prerequisites and Setup

### Required Knowledge

Before integrating PropChain, you should understand:
- **Basic Blockchain Concepts**: Accounts, transactions, gas fees
- **Smart Contracts**: What they are and how they work
- **Web3 Development**: Wallet connections, signing transactions
- **JavaScript/TypeScript**: Modern async/await patterns

### Development Environment

#### 1. Install Node.js and npm

**Required Version**: Node.js 16+ and npm 8+

```bash
# Check current versions
node --version  # Should show v16.x.x or higher
npm --version   # Should show 8.x.x or higher

# Install/update from https://nodejs.org/
```

#### 2. Install Polkadot Tools

```bash
# Polkadot.js extension for browser wallet
# Visit: https://polkadot.js.org/extension/

# For development
npm install --save-dev @types/node
```

#### 3. Set Up Project Structure

```bash
# Create new project
mkdir propchain-dapp
cd propchain-dapp
npm init -y

# Install core dependencies
npm install @polkadot/api @polkadot/api-contract

# Install TypeScript (optional but recommended)
npm install --save-dev typescript ts-node @types/node

# Install additional utilities
npm install bn.js dotenv
```

**Recommended Project Structure**:
```
propchain-dapp/
├── src/
│   ├── contracts/
│   │   ├── abi.json          # Contract ABI
│   │   └── addresses.json    # Deployed addresses
│   ├── services/
│   │   ├── blockchain.ts     # Blockchain connection
│   │   ├── propertyService.ts # Property operations
│   │   └── complianceService.ts
│   ├── components/           # UI components
│   └── utils/               # Helper functions
├── .env                      # Environment variables
└── package.json
```

---

## Core Integration Steps

### Step 1: Connect to Blockchain

#### Basic Connection

```typescript
import { ApiPromise, WsProvider } from '@polkadot/api';

interface ConnectionConfig {
  rpcEndpoint: string;
  maxRetries?: number;
  retryDelay?: number;
}

class BlockchainConnection {
  private api: ApiPromise | null = null;
  private config: ConnectionConfig;

  constructor(config: ConnectionConfig) {
    this.config = config;
  }

  async connect(): Promise<ApiPromise> {
    let retries = 0;
    const maxRetries = this.config.maxRetries || 3;

    while (retries < maxRetries) {
      try {
        const wsProvider = new WsProvider(this.config.rpcEndpoint);
        this.api = await ApiPromise.create({ 
          provider: wsProvider,
          throwOnConnect: false
        });

        // Verify connection
        if (!this.api.isConnected) {
          throw new Error('Failed to connect');
        }

        console.log(`Connected to ${this.config.rpcEndpoint}`);
        return this.api;
      } catch (error) {
        retries++;
        console.error(`Connection attempt ${retries} failed:`, error);
        
        if (retries === maxRetries) {
          throw new Error(`Failed to connect after ${maxRetries} attempts`);
        }
        
        await this.sleep(this.config.retryDelay || 2000);
      }
    }

    throw new Error('Connection failed');
  }

  disconnect() {
    if (this.api) {
      this.api.disconnect();
      console.log('Disconnected from blockchain');
    }
  }

  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}

// Usage
const connection = new BlockchainConnection({
  rpcEndpoint: 'wss://rpc.propchain.io',
  maxRetries: 3,
  retryDelay: 2000
});

try {
  const api = await connection.connect();
  // Use api...
} finally {
  connection.disconnect();
}
```

#### Network Configuration

```typescript
// .env file
PROPCHAIN_MAINNET_RPC=wss://rpc.propchain.io
PROPCHAIN_TESTNET_RPC=wss://testnet.propchain.io
PROPCHAIN_LOCAL_RPC=ws://localhost:9944

// config.ts
export const NETWORK_CONFIG = {
  mainnet: {
    rpc: process.env.PROPCHAIN_MAINNET_RPC,
    chainId: '0x1234...', // Replace with actual chain ID
    explorer: 'https://explorer.propchain.io'
  },
  testnet: {
    rpc: process.env.PROPCHAIN_TESTNET_RPC,
    chainId: '0x5678...',
    explorer: 'https://testnet.explorer.propchain.io'
  },
  local: {
    rpc: process.env.PROPCHAIN_LOCAL_RPC,
    chainId: '0xabcd...',
    explorer: null
  }
};
```

---

### Step 2: Load Smart Contract

#### Contract Loader Service

```typescript
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';
import contractAbi from './contracts/abi.json';
import contractAddresses from './contracts/addresses.json';

interface ContractInstance {
  api: ApiPromise;
  contract: ContractPromise;
  address: string;
}

class ContractLoader {
  private static instance: ContractLoader;
  private contractCache: Map<string, ContractInstance> = new Map();

  private constructor() {}

  static getInstance(): ContractLoader {
    if (!ContractLoader.instance) {
      ContractLoader.instance = new ContractLoader();
    }
    return ContractLoader.instance;
  }

  async loadContract(
    api: ApiPromise,
    network: 'mainnet' | 'testnet' | 'local' = 'testnet'
  ): Promise<ContractInstance> {
    const cacheKey = `${network}-${contractAddresses[network]}`;
    
    // Return cached instance if available
    if (this.contractCache.has(cacheKey)) {
      console.log(`Using cached contract instance for ${network}`);
      return this.contractCache.get(cacheKey)!;
    }

    const address = contractAddresses[network];
    if (!address) {
      throw new Error(`No contract address configured for ${network}`);
    }

    console.log(`Loading contract at ${address} on ${network}`);
    
    const contract = new ContractPromise(api, contractAbi, address);
    
    const instance: ContractInstance = { api, contract, address };
    this.contractCache.set(cacheKey, instance);
    
    return instance;
  }

  clearCache() {
    this.contractCache.clear();
  }
}

// Usage
const loader = ContractLoader.getInstance();
const { contract, address } = await loader.loadContract(api, 'testnet');
console.log(`Contract loaded at: ${address}`);
```

#### Contract Addresses Management

```typescript
// contracts/addresses.json
{
  "mainnet": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
  "testnet": "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
  "local": "5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y"
}

// contracts/abi.json
// Paste the compiled contract ABI here
```

---

### Step 3: Wallet Connection

#### Polkadot.js Extension Integration

```typescript
import { web3Accounts, web3Enable, web3FromAddress } from '@polkadot/extension-dapp';
import { InjectedAccountWithMeta } from '@polkadot/extension-inject/types';

class WalletManager {
  private accounts: InjectedAccountWithMeta[] = [];
  private selectedAccount: string | null = null;

  async enableExtension(): Promise<boolean> {
    try {
      const extensions = await web3Enable('Your DApp Name');
      
      if (extensions.length === 0) {
        console.warn('No Polkadot extensions found');
        return false;
      }

      console.log(`${extensions.length} extension(s) enabled`);
      return true;
    } catch (error) {
      console.error('Failed to enable extension:', error);
      return false;
    }
  }

  async getAccounts(): Promise<InjectedAccountWithMeta[]> {
    if (!await this.enableExtension()) {
      return [];
    }

    this.accounts = await web3Accounts();
    console.log(`Found ${this.accounts.length} account(s)`);
    return this.accounts;
  }

  selectAccount(address: string): void {
    const account = this.accounts.find(acc => acc.address === address);
    
    if (!account) {
      throw new Error('Account not found');
    }

    this.selectedAccount = address;
    console.log(`Selected account: ${address}`);
  }

  async getSigner(address: string) {
    const injector = await web3FromAddress(address);
    return injector.signer;
  }

  getSelectedAccount(): InjectedAccountWithMeta | null {
    if (!this.selectedAccount) return null;
    
    return this.accounts.find(acc => acc.address === this.selectedAccount) || null;
  }
}

// Usage in React/Vue/Angular
const walletManager = new WalletManager();

// Initialize wallet
await walletManager.enableExtension();
const accounts = await walletManager.getAccounts();

// Select first account
if (accounts.length > 0) {
  walletManager.selectAccount(accounts[0].address);
}
```

#### React Hook Example

```typescript
// hooks/useWallet.ts
import { useState, useEffect } from 'react';
import { InjectedAccountWithMeta } from '@polkadot/extension-inject/types';

export function useWallet() {
  const [accounts, setAccounts] = useState<InjectedAccountWithMeta[]>([]);
  const [selectedAccount, setSelectedAccount] = useState<string | null>(null);
  const [isConnecting, setIsConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const connect = async () => {
    setIsConnecting(true);
    setError(null);

    try {
      const { web3Enable, web3Accounts } = await import('@polkadot/extension-dapp');
      
      await web3Enable('Your DApp');
      const allAccounts = await web3Accounts();
      
      setAccounts(allAccounts);
      
      if (allAccounts.length > 0) {
        setSelectedAccount(allAccounts[0].address);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to connect');
    } finally {
      setIsConnecting(false);
    }
  };

  const disconnect = () => {
    setSelectedAccount(null);
    setAccounts([]);
  };

  return {
    accounts,
    selectedAccount,
    isConnecting,
    error,
    connect,
    disconnect,
    isConnected: selectedAccount !== null
  };
}

// Usage in component
function MyComponent() {
  const { accounts, selectedAccount, connect, disconnect, isConnected } = useWallet();

  if (!isConnected) {
    return <button onClick={connect}>Connect Wallet</button>;
  }

  return (
    <div>
      <p>Connected: {selectedAccount}</p>
      <button onClick={disconnect}>Disconnect</button>
    </div>
  );
}
```

---

### Step 4: Execute Transactions

#### Transaction Service

```typescript
import { ContractPromise } from '@polkadot/api-contract';
import { SubmittableExtrinsic } from '@polkadot/api/types';
import { ISubmittableResult } from '@polkadot/types/types';

interface TransactionOptions {
  gasLimit?: bigint;
  value?: bigint;
  nonce?: number;
}

interface TransactionResult {
  hash: string;
  blockHash?: string;
  status: 'submitted' | 'inblock' | 'finalized' | 'error';
  events?: any[];
}

class TransactionService {
  async executeTransaction(
    tx: SubmittableExtrinsic<'promise'>,
    signer: any,
    options: TransactionOptions = {}
  ): Promise<TransactionResult> {
    return new Promise((resolve, reject) => {
      tx.signAndSend(signer, { 
        gasLimit: options.gasLimit || -1,
        value: options.value || 0
      }, (result: ISubmittableResult) => {
        console.log('Transaction status:', result.status.type);

        if (result.status.isInBlock) {
          console.log(`Transaction in block: ${result.status.asInBlock}`);
          
          // Parse events
          const events = this.parseEvents(result);
          resolve({
            hash: tx.hash.toString(),
            blockHash: result.status.asInBlock.toString(),
            status: 'inblock',
            events
          });
        } else if (result.status.isFinalized) {
          console.log(`Transaction finalized: ${result.status.asFinalized}`);
          
          // Check for errors in events
          const errorEvent = result.events.find(
            ({ event }) => event.method === 'ExtrinsicFailed'
          );
          
          if (errorEvent) {
            reject(new Error('Transaction failed'));
          } else {
            resolve({
              hash: tx.hash.toString(),
              blockHash: result.status.asFinalized.toString(),
              status: 'finalized',
              events: this.parseEvents(result)
            });
          }
        }
      }).catch(reject);
    });
  }

  private parseEvents(result: ISubmittableResult): any[] {
    return result.events
      .filter(({ phase }) => phase.isApplyExtrinsic)
      .map(({ event: { method, section, data } }) => ({
        method,
        section,
        data: data.toHuman()
      }));
  }
}

// Usage
const txService = new TransactionService();

async function registerProperty(metadata: any) {
  const result = await txService.executeTransaction(
    contract.tx.registerProperty({ gasLimit: -1 }, metadata),
    accountPair
  );
  
  console.log('Transaction completed:', result);
  return result;
}
```

---

## Common Use Cases

### Use Case 1: Register a Property

#### Complete Example with Validation

```typescript
import { z } from 'zod'; // For validation

// Define schema
const PropertyMetadataSchema = z.object({
  location: z.string().min(1).max(256),
  size: z.number().min(1).max(10000000),
  valuation: z.number().min(1000), // Minimum $10 in cents
  documents_url: z.string().url().optional(),
  legal_description: z.string().optional()
});

type PropertyMetadata = z.infer<typeof PropertyMetadataSchema>;

class PropertyRegistrationService {
  private contract: ContractPromise;
  private txService: TransactionService;

  constructor(contract: ContractPromise) {
    this.contract = contract;
    this.txService = new TransactionService();
  }

  async registerProperty(
    metadata: PropertyMetadata,
    signer: any
  ): Promise<{ propertyId: number; hash: string }> {
    // Validate metadata
    const validatedData = PropertyMetadataSchema.parse(metadata);

    console.log('Registering property:', validatedData);

    try {
      // Estimate gas first
      const { gasRequired } = await this.contract.query.registerProperty(
        signer.address,
        { gasLimit: -1 },
        validatedData
      );

      if (!gasRequired.ok) {
        throw new Error('Gas estimation failed');
      }

      // Execute transaction
      const result = await this.txService.executeTransaction(
        this.contract.tx.registerProperty(
          { gasLimit: gasRequired.gasRequired },
          validatedData
        ),
        signer
      );

      // Extract property ID from events
      const propertyRegisteredEvent = result.events?.find(
        e => e.method === 'PropertyRegistered'
      );

      if (!propertyRegisteredEvent) {
        throw new Error('Property registration event not found');
      }

      const propertyId = parseInt(propertyRegisteredEvent.data.property_id);

      return {
        propertyId,
        hash: result.hash
      };
    } catch (error) {
      console.error('Property registration failed:', error);
      throw this.handleRegistrationError(error);
    }
  }

  private handleRegistrationError(error: any): Error {
    const errorMessage = error.message || String(error);
    
    if (errorMessage.includes('InvalidMetadata')) {
      return new Error('Invalid property metadata. Please check all fields.');
    }
    if (errorMessage.includes('NotCompliant')) {
      return new Error('Account not compliant. Please complete KYC verification.');
    }
    if (errorMessage.includes('InsufficientBalance')) {
      return new Error('Insufficient balance for gas fees.');
    }
    
    return error;
  }
}

// Usage
async function example() {
  const metadata: PropertyMetadata = {
    location: '123 Main Street, Springfield, IL 62701',
    size: 2500,
    valuation: 35000000, // $350,000 in cents
    documents_url: 'ipfs://QmX7Zz9YvPqK8N3mR5wL2bT6cH4dF9gS1aE8uB7vC3nM2k',
    legal_description: 'Lot 15, Block C, Springfield Heights'
  };

  const service = new PropertyRegistrationService(contract);
  
  try {
    const { propertyId, hash } = await service.registerProperty(
      metadata,
      accountPair
    );
    
    console.log(`Property registered! ID: ${propertyId}, TX: ${hash}`);
  } catch (error) {
    console.error('Registration failed:', error.message);
  }
}
```

### Use Case 2: Transfer Property Ownership

```typescript
interface TransferOptions {
  propertyId: number;
  recipient: string;
  price: bigint;
  useEscrow?: boolean;
}

class PropertyTransferService {
  private contract: ContractPromise;
  private txService: TransactionService;

  constructor(contract: ContractPromise) {
    this.contract = contract;
    this.txService = new TransactionService();
  }

  async transferProperty(
    options: TransferOptions,
    signer: any
  ): Promise<{ hash: string }> {
    console.log(`Transferring property ${options.propertyId} to ${options.recipient}`);

    try {
      // Check compliance first
      const isCompliant = await this.checkRecipientCompliance(options.recipient);
      
      if (!isCompliant) {
        throw new Error('Recipient not compliant with KYC/AML requirements');
      }

      // Get property details to verify ownership
      const property = await this.getPropertyDetails(options.propertyId);
      
      if (property.owner !== signer.address) {
        throw new Error('You do not own this property');
      }

      if (options.useEscrow) {
        return await this.transferViaEscrow(options, signer);
      } else {
        return await this.transferDirect(options, signer);
      }
    } catch (error) {
      console.error('Transfer failed:', error);
      throw error;
    }
  }

  private async transferDirect(
    options: TransferOptions,
    signer: any
  ): Promise<{ hash: string }> {
    const result = await this.txService.executeTransaction(
      this.contract.tx.transfer_property(
        { gasLimit: -1 },
        options.recipient,
        options.propertyId
      ),
      signer
    );

    return { hash: result.hash };
  }

  private async transferViaEscrow(
    options: TransferOptions,
    signer: any
  ): Promise<{ hash: string }> {
    // Create escrow
    const escrowResult = await this.txService.executeTransaction(
      this.contract.tx.create_escrow(
        { gasLimit: -1 },
        options.propertyId,
        options.recipient,
        options.price
      ),
      signer
    );

    return { hash: escrowResult.hash };
  }

  private async checkRecipientCompliance(recipient: string): Promise<boolean> {
    const { output } = await this.contract.query.check_account_compliance(
      this.contract.address,
      { gasLimit: -1 },
      recipient
    );

    return output?.toPrimitive() as boolean || false;
  }

  private async getPropertyDetails(propertyId: number): Promise<any> {
    const { output } = await this.contract.query.get_property(
      this.contract.address,
      { gasLimit: -1 },
      propertyId
    );

    if (!output || !output.isOk) {
      throw new Error('Property not found');
    }

    return output.unwrap();
  }
}
```

### Use Case 3: Query Property Information

```typescript
interface PropertySummary {
  id: number;
  owner: string;
  location: string;
  size: number;
  valuation: bigint;
  registeredAt: number;
}

class PropertyQueryService {
  private contract: ContractPromise;
  private cache: Map<number, PropertySummary> = new Map();

  constructor(contract: ContractPromise) {
    this.contract = contract;
  }

  async getProperty(propertyId: number): Promise<PropertySummary> {
    // Check cache first (5 minute cache)
    const cached = this.cache.get(propertyId);
    if (cached) {
      return cached;
    }

    const { output } = await this.contract.query.get_property(
      this.contract.address,
      { gasLimit: -1 },
      propertyId
    );

    if (!output || !output.isOk) {
      throw new Error(`Property ${propertyId} not found`);
    }

    const property = output.unwrap().toHuman();
    const summary: PropertySummary = {
      id: propertyId,
      owner: property.owner,
      location: property.metadata.location,
      size: parseInt(property.metadata.size),
      valuation: BigInt(property.metadata.valuation.replace(/,/g, '')),
      registeredAt: parseInt(property.registered_at)
    };

    // Cache for 5 minutes
    this.cache.set(propertyId, summary);
    setTimeout(() => this.cache.delete(propertyId), 5 * 60 * 1000);

    return summary;
  }

  async getPropertiesByOwner(owner: string): Promise<PropertySummary[]> {
    const { output } = await this.contract.query.get_properties_by_owner(
      this.contract.address,
      { gasLimit: -1 },
      owner
    );

    if (!output || !output.isOk) {
      return [];
    }

    const propertyIds = output.unwrap().toPrimitive() as number[];
    
    // Fetch details in parallel
    const properties = await Promise.all(
      propertyIds.map(id => this.getProperty(id).catch(() => null))
    );

    return properties.filter((p): p is PropertySummary => p !== null);
  }

  async getPropertyValuation(propertyId: number): Promise<bigint> {
    const { output } = await this.contract.query.get_valuation(
      this.contract.address,
      { gasLimit: -1 },
      propertyId
    );

    if (!output || !output.isOk) {
      throw new Error('Valuation not available');
    }

    return BigInt(output.unwrap().toPrimitive());
  }
}
```

---

## Advanced Integration Patterns

### Event Listening and Indexing

```typescript
class EventListener {
  private api: ApiPromise;
  private listeners: Map<string, Function[]> = new Map();

  constructor(api: ApiPromise) {
    this.api = api;
  }

  async listenToPropertyEvents(
    callback: (event: any) => void,
    propertyId?: number
  ): Promise<() => void> {
    const unsubscribe = await this.api.query.system.events((events) => {
      events.forEach((record) => {
        const { event } = record;

        // Filter PropertyRegistry events
        if (event.section !== 'propertyRegistry') {
          return;
        }

        // Optional: filter by specific property
        if (propertyId !== undefined) {
          const eventPropertyId = event.data.find(
            (d: any) => d.toNumber?.() === propertyId
          );
          
          if (!eventPropertyId) {
            return;
          }
        }

        // Call callback with event details
        callback({
          method: event.method,
          section: event.section,
          data: event.data.toHuman(),
          blockHash: record.phase.asApplyExtrinsic.toString()
        });
      });
    });

    // Return unsubscribe function
    return () => unsubscribe();
  }

  async getHistoricalEvents(
    fromBlock: number,
    toBlock: number,
    eventType?: string
  ): Promise<any[]> {
    const events: any[] = [];

    for (let blockNum = fromBlock; blockNum <= toBlock; blockNum++) {
      const blockHash = await this.api.rpc.chain.getBlockHash(blockNum);
      const signedBlock = await this.api.rpc.chain.getBlock(blockHash);

      const allEvents = await this.api.query.system.events.at(blockHash);

      allEvents.forEach((record) => {
        const { event } = record;

        if (event.section === 'propertyRegistry') {
          if (!eventType || event.method === eventType) {
            events.push({
              blockNumber: blockNum,
              method: event.method,
              data: event.data.toHuman(),
              timestamp: signedBlock.block.extrinsics[0]?.method.toHuman()
            });
          }
        }
      });
    }

    return events;
  }
}

// Usage
const eventListener = new EventListener(api);

// Listen to new property registrations
const unsubscribe = await eventListener.listenToPropertyEvents(
  (event) => {
    if (event.method === 'PropertyRegistered') {
      console.log('New property registered:', event.data);
      // Update UI, send notification, etc.
    }
  }
);

// Later: unsubscribe()
```

---

## Testing Your Integration

See dedicated [Testing Guide](./testing-integration.md) for comprehensive testing strategies.

---

## Troubleshooting

See dedicated [Troubleshooting Guide](./integration-troubleshooting.md) for common issues and solutions.

---

## Best Practices

### Security

1. **Validate All Inputs**: Never trust user input without validation
2. **Use Type Safety**: TypeScript prevents many common errors
3. **Implement Rate Limiting**: Protect against abuse
4. **Secure Key Management**: Never expose private keys
5. **Handle Errors Gracefully**: Don't leak sensitive information

### Performance

1. **Cache Aggressively**: Reduce blockchain queries
2. **Batch Operations**: Combine multiple calls when possible
3. **Use WebSockets**: Real-time updates instead of polling
4. **Optimize Gas**: Estimate gas before sending transactions
5. **Lazy Loading**: Load data only when needed

### User Experience

1. **Clear Error Messages**: Help users understand what went wrong
2. **Transaction Status**: Show real-time progress
3. **Confirmation Dialogs**: Confirm important actions
4. **Loading States**: Indicate when waiting for blockchain
5. **Offline Support**: Handle disconnections gracefully

---

## Next Steps

- [Property Registration Tutorial](./tutorials/basic-property-registration.md)
- [Escrow System Guide](./tutorials/escrow-system.md)
- [Cross-Chain Bridging](./tutorials/cross-chain-bridging.md)
- [API Reference](./API_GUIDE.md)

---

**Last Updated**: March 27, 2026  
**Version**: 2.0.0  
**Maintained By**: PropChain Development Team
