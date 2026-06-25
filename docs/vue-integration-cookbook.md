# Vue Integration Cookbook

This guide walks you through integrating the PropChain SDK into a Vue.js application, covering wallet connection, signing, minting, transferring, and oracle reads.

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

### 1. Create a new Vue project
```bash
npm create vue@latest propchain-vue-app
cd propchain-vue-app
```

### 2. Install dependencies
```bash
npm install @propchain/sdk @polkadot/api @polkadot/extension-dapp
```

### 3. Configure SDK
Create `src/lib/propchain.ts` for SDK initialization:

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

```vue
<!-- src/components/ConnectWallet.vue -->
<template>
  <div class="wallet-section">
    <div v-if="account" class="wallet-info">
      <span>Connected: {{ truncateAddress(account) }}</span>
      <button @click="handleDisconnect">Disconnect</button>
    </div>
    <button v-else @click="handleConnect" :disabled="isConnecting">
      {{ isConnecting ? 'Connecting...' : 'Connect Wallet' }}
    </button>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { connectExtension } from '@propchain/sdk';

const emit = defineEmits<{
  (e: 'connect', address: string): void;
  (e: 'disconnect'): void;
}>();

const account = ref<string | null>(null);
const isConnecting = ref(false);

const handleConnect = async () => {
  isConnecting.value = true;
  try {
    const accounts = await connectExtension('PropChain dApp');
    if (accounts.length > 0) {
      account.value = accounts[0].address;
      localStorage.setItem('selectedAccount', accounts[0].address);
      emit('connect', accounts[0].address);
    }
  } catch (err) {
    console.error('Failed to connect wallet:', err);
  } finally {
    isConnecting.value = false;
  }
};

const handleDisconnect = () => {
  account.value = null;
  localStorage.removeItem('selectedAccount');
  emit('disconnect');
};

const truncateAddress = (addr: string) =>
  `${addr.slice(0, 6)}…${addr.slice(-4)}`;

onMounted(() => {
  const savedAccount = localStorage.getItem('selectedAccount');
  if (savedAccount) {
    account.value = savedAccount;
    emit('connect', savedAccount);
  }
});
</script>
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

```vue
<!-- src/components/MintPropertyToken.vue -->
<template>
  <div class="mint-section">
    <h2>Mint Property Token</h2>
    <p v-if="message">{{ message }}</p>
    <button @click="handleMint" :disabled="isMinting">
      {{ isMinting ? 'Minting...' : 'Mint Token' }}
    </button>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { getPropChainClient } from '@/lib/propchain';
import { getExtensionSigner } from '@propchain/sdk';

const props = defineProps<{
  account: string;
}>();

const isMinting = ref(false);
const message = ref<string | null>(null);

const handleMint = async () => {
  isMinting.value = true;
  message.value = null;

  try {
    const client = await getPropChainClient();
    const signer = await getExtensionSigner(props.account);

    const { tokenId } = await client.propertyToken.registerPropertyWithToken(signer, {
      location: '123 Main St, New York, NY',
      size: 2500,
      legalDescription: 'Lot 1, Block 2',
      valuation: BigInt('50000000000000'), // 500,000 units
      documentsUrl: 'ipfs://QmXoypJgWnX9w5QZ5z9x9QZ5z9x9QZ5z9x9QZ5z9x9QZ5z9',
    });

    message.value = `Successfully minted property token #${tokenId}`;
  } catch (err) {
    message.value = `Minting failed: ${err}`;
  } finally {
    isMinting.value = false;
  }
};
</script>
```

## Transferring Tokens

### Create a Transfer Component

```vue
<!-- src/components/TransferPropertyToken.vue -->
<template>
  <div class="transfer-section">
    <h2>Transfer Property Token</h2>
    <p v-if="message">{{ message }}</p>
    <div>
      <label>Token ID:</label>
      <input
        v-model="tokenId"
        type="text"
        placeholder="Enter token ID"
      />
    </div>
    <div>
      <label>To Address:</label>
      <input
        v-model="toAddress"
        type="text"
        placeholder="Enter recipient address"
      />
    </div>
    <button @click="handleTransfer" :disabled="isTransferring">
      {{ isTransferring ? 'Transferring...' : 'Transfer' }}
    </button>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { getPropChainClient } from '@/lib/propchain';
import { getExtensionSigner } from '@propchain/sdk';

const props = defineProps<{
  account: string;
}>();

const toAddress = ref('');
const tokenId = ref('');
const isTransferring = ref(false);
const message = ref<string | null>(null);

const handleTransfer = async () => {
  isTransferring.value = true;
  message.value = null;

  try {
    const client = await getPropChainClient();
    const signer = await getExtensionSigner(props.account);

    await client.propertyToken.transfer(signer, BigInt(tokenId.value), toAddress.value);
    message.value = `Successfully transferred token #${tokenId.value} to ${toAddress.value}`;
  } catch (err) {
    message.value = `Transfer failed: ${err}`;
  } finally {
    isTransferring.value = false;
  }
};
</script>
```

## Oracle Reads

### Create an Oracle Read Component

```vue
<!-- src/components/OracleRead.vue -->
<template>
  <div class="oracle-section">
    <h2>Oracle Property Valuation</h2>
    <p v-if="message">{{ message }}</p>
    <div>
      <label>Property ID:</label>
      <input
        v-model="propertyId"
        type="text"
        placeholder="Enter property ID"
      />
    </div>
    <button @click="handleReadOracle" :disabled="isReading">
      {{ isReading ? 'Reading...' : 'Get Valuation' }}
    </button>
    <div v-if="valuation" class="valuation-result">
      <h3>Valuation Result</h3>
      <p>Estimated Value: {{ formatValuation(valuation.predictedValuation) }}</p>
      <p>Confidence: {{ valuation.confidenceScore }}%</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { getPropChainClient } from '@/lib/propchain';
import { formatValuation } from '@propchain/sdk';

const propertyId = ref('');
const valuation = ref<any>(null);
const isReading = ref(false);
const message = ref<string | null>(null);

const handleReadOracle = async () => {
  isReading.value = true;
  message.value = null;

  try {
    const client = await getPropChainClient();
    const result = await client.oracle.getPropertyValuation(BigInt(propertyId.value));
    valuation.value = result;
    message.value = `Valuation retrieved for property #${propertyId.value}`;
  } catch (err) {
    message.value = `Read failed: ${err}`;
  } finally {
    isReading.value = false;
  }
};
</script>
```

## Putting It All Together

```vue
<!-- src/App.vue -->
<template>
  <div class="app">
    <ConnectWallet @connect="handleConnect" @disconnect="handleDisconnect" />
    <div v-if="account" class="content">
      <MintPropertyToken :account="account" />
      <TransferPropertyToken :account="account" />
      <OracleRead />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import ConnectWallet from '@/components/ConnectWallet.vue';
import MintPropertyToken from '@/components/MintPropertyToken.vue';
import TransferPropertyToken from '@/components/TransferPropertyToken.vue';
import OracleRead from '@/components/OracleRead.vue';

const account = ref<string | null>(null);

const handleConnect = (address: string) => {
  account.value = address;
};

const handleDisconnect = () => {
  account.value = null;
};
</script>
```

## References
- See [INTEGRATION_EXAMPLES.ts](../../sdk/frontend/INTEGRATION_EXAMPLES.ts) for more examples
- Frontend SDK Guide: [FRONTEND_SDK_GUIDE.md](./FRONTEND_SDK_GUIDE.md)
