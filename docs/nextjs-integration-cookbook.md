# Next.js Integration Cookbook

This guide walks you through integrating the PropChain SDK into a Next.js application, covering wallet connection, signing, minting, transferring, and oracle reads.

## Table of Contents
1. [Prerequisites](#prerequisites)
2. [Project Setup](#project-setup)
3. [Wallet Connection](#wallet-connection)
4. [Signing Transactions](#signing-transactions)
5. [Minting Property Tokens](#minting-property-tokens)
6. [Transferring Tokens](#transferring-tokens)
7. [Oracle Reads](#oracle-reads)

## Prerequisites
- Node.js 18+
- npm or yarn
- A PropChain node endpoint (local or public)
- Polkadot.js browser extension

## Project Setup

### 1. Create a new Next.js project
```bash
npx create-next-app@latest propchain-next-app
cd propchain-next-app
```

### 2. Install dependencies
```bash
npm install @propchain/sdk @polkadot/api @polkadot/extension-dapp
```

### 3. Configure SDK
Create `lib/propchain.ts` for SDK initialization:

```typescript
import { PropChainClient, getNetworkConfig, NETWORKS } from '@propchain/sdk';

let client: PropChainClient | null = null;

export async function getPropChainClient() {
  if (!client) {
    const config = getNetworkConfig(NETWORKS.LOCAL);
    client = await PropChainClient.create(config.wsEndpoint, {
      propertyRegistry: '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', // Replace with your contract addresses
      propertyToken: '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty',
      oracle: '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
    });
  }
  return client;
}
```

## Wallet Connection

### Create a Wallet Connection Component

```tsx
// components/ConnectWallet.tsx
'use client';

import { useState, useEffect } from 'react';
import { connectExtension, getExtensionSigner } from '@propchain/sdk';

export default function ConnectWallet() {
  const [account, setAccount] = useState<string | null>(null);
  const [isConnecting, setIsConnecting] = useState(false);

  const handleConnect = async () => {
    setIsConnecting(true);
    try {
      const accounts = await connectExtension('PropChain dApp');
      if (accounts.length > 0) {
        setAccount(accounts[0].address);
        localStorage.setItem('selectedAccount', accounts[0].address);
      }
    } catch (err) {
      console.error('Failed to connect wallet:', err);
    } finally {
      setIsConnecting(false);
    }
  };

  useEffect(() => {
    const savedAccount = localStorage.getItem('selectedAccount');
    if (savedAccount) {
      setAccount(savedAccount);
    }
  }, []);

  const truncateAddress = (addr: string) =>
    `${addr.slice(0, 6)}…${addr.slice(-4)}`;

  return (
    <div>
      {account ? (
        <div className="wallet-info">
          <span>Connected: {truncateAddress(account)}</span>
          <button onClick={() => setAccount(null)}>Disconnect</button>
        </div>
      ) : (
        <button onClick={handleConnect} disabled={isConnecting}>
          {isConnecting ? 'Connecting...' : 'Connect Wallet'}
        </button>
      )}
    </div>
  );
}
```

## Signing Transactions

```typescript
import { getExtensionSigner } from '@propchain/sdk';

export async function getSigner(address: string) {
  const signer = await getExtensionSigner(address);
  return signer;
}
```

## Minting Property Tokens

### Create a Minting Component

```tsx
// components/MintPropertyToken.tsx
'use client';

import { useState } from 'react';
import { getPropChainClient } from '@/lib/propchain';
import { getExtensionSigner } from '@propchain/sdk';

interface MintPropertyTokenProps {
  account: string;
}

export default function MintPropertyToken({ account }: MintPropertyTokenProps) {
  const [isMinting, setIsMinting] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const handleMint = async () => {
    setIsMinting(true);
    setMessage(null);

    try {
      const client = await getPropChainClient();
      const signer = await getExtensionSigner(account);

      const { tokenId } = await client.propertyToken.registerPropertyWithToken(signer, {
        location: '123 Main St, New York, NY',
        size: 2500,
        legalDescription: 'Lot 1, Block 2',
        valuation: BigInt('50000000000000'), // 500,000 units
        documentsUrl: 'ipfs://QmXoypJgWnX9w5QZ5z9x9QZ5z9x9QZ5z9x9QZ5z9x9QZ5z9',
      });

      setMessage(`Successfully minted property token #${tokenId}`);
    } catch (err) {
      setMessage(`Minting failed: ${err}`);
    } finally {
      setIsMinting(false);
    }
  };

  return (
    <div className="mint-section">
      <h2>Mint Property Token</h2>
      {message && <p>{message}</p>}
      <button onClick={handleMint} disabled={isMinting}>
        {isMinting ? 'Minting...' : 'Mint Token'}
      </button>
    </div>
  );
}
```

## Transferring Tokens

### Create a Transfer Component

```tsx
// components/TransferPropertyToken.tsx
'use client';

import { useState } from 'react';
import { getPropChainClient } from '@/lib/propchain';
import { getExtensionSigner } from '@propchain/sdk';

interface TransferPropertyTokenProps {
  account: string;
}

export default function TransferPropertyToken({ account }: TransferPropertyTokenProps) {
  const [toAddress, setToAddress] = useState('');
  const [tokenId, setTokenId] = useState('');
  const [isTransferring, setIsTransferring] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const handleTransfer = async () => {
    setIsTransferring(true);
    setMessage(null);

    try {
      const client = await getPropChainClient();
      const signer = await getExtensionSigner(account);

      await client.propertyToken.transfer(signer, BigInt(tokenId), toAddress);
      setMessage(`Successfully transferred token #${tokenId} to ${toAddress}`);
    } catch (err) {
      setMessage(`Transfer failed: ${err}`);
    } finally {
      setIsTransferring(false);
    }
  };

  return (
    <div className="transfer-section">
      <h2>Transfer Property Token</h2>
      {message && <p>{message}</p>}
      <div>
        <label>Token ID:</label>
        <input
          type="text"
          value={tokenId}
          onChange={(e) => setTokenId(e.target.value)}
          placeholder="Enter token ID"
        />
      </div>
      <div>
        <label>To Address:</label>
        <input
          type="text"
          value={toAddress}
          onChange={(e) => setToAddress(e.target.value)}
          placeholder="Enter recipient address"
        />
      </div>
      <button onClick={handleTransfer} disabled={isTransferring}>
        {isTransferring ? 'Transferring...' : 'Transfer'}
      </button>
    </div>
  );
}
```

## Oracle Reads

### Create an Oracle Read Component

```tsx
// components/OracleRead.tsx
'use client';

import { useState } from 'react';
import { getPropChainClient } from '@/lib/propchain';
import { formatValuation } from '@propchain/sdk';

export default function OracleRead() {
  const [propertyId, setPropertyId] = useState('');
  const [valuation, setValuation] = useState<any>(null);
  const [isReading, setIsReading] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const handleReadOracle = async () => {
    setIsReading(true);
    setMessage(null);

    try {
      const client = await getPropChainClient();
      const result = await client.oracle.getPropertyValuation(BigInt(propertyId));
      setValuation(result);
      setMessage(`Valuation retrieved for property #${propertyId}`);
    } catch (err) {
      setMessage(`Read failed: ${err}`);
    } finally {
      setIsReading(false);
    }
  };

  return (
    <div className="oracle-section">
      <h2>Oracle Property Valuation</h2>
      {message && <p>{message}</p>}
      <div>
        <label>Property ID:</label>
        <input
          type="text"
          value={propertyId}
          onChange={(e) => setPropertyId(e.target.value)}
          placeholder="Enter property ID"
        />
      </div>
      <button onClick={handleReadOracle} disabled={isReading}>
        {isReading ? 'Reading...' : 'Get Valuation'}
      </button>
      {valuation && (
        <div className="valuation-result">
          <h3>Valuation Result</h3>
          <p>Estimated Value: {formatValuation(valuation.predictedValuation)}</p>
          <p>Confidence: {valuation.confidenceScore}%</p>
        </div>
      )}
    </div>
  );
}
```

## Putting It All Together

```tsx
// app/page.tsx
'use client';

import { useState, useEffect } from 'react';
import ConnectWallet from '@/components/ConnectWallet';
import MintPropertyToken from '@/components/MintPropertyToken';
import TransferPropertyToken from '@/components/TransferPropertyToken';
import OracleRead from '@/components/OracleRead';

export default function Home() {
  const [account, setAccount] = useState<string | null>(null);

  useEffect(() => {
    const savedAccount = localStorage.getItem('selectedAccount');
    if (savedAccount) {
      setAccount(savedAccount);
    }
  }, []);

  return (
    <main className="min-h-screen p-8">
      <ConnectWallet />
      {account && (
        <div className="mt-8 space-y-8">
          <MintPropertyToken account={account} />
          <TransferPropertyToken account={account} />
          <OracleRead />
        </div>
      )}
    </main>
  );
}
```

## References
- See [INTEGRATION_EXAMPLES.ts](../../sdk/frontend/INTEGRATION_EXAMPLES.ts) for more examples
- Frontend SDK Guide: [FRONTEND_SDK_GUIDE.md](./FRONTEND_SDK_GUIDE.md)
