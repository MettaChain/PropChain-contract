# Angular Integration Cookbook

This guide walks you through integrating the PropChain SDK into an Angular application, covering wallet connection, signing, minting, transferring, and oracle reads.

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
- Angular CLI 17+
- A PropChain node endpoint (local or public)
- Polkadot.js browser extension

## Project Setup

### 1. Create a new Angular project
```bash
ng new propchain-angular-app --standalone
cd propchain-angular-app
```

### 2. Install dependencies
```bash
npm install @propchain/sdk @polkadot/api @polkadot/extension-dapp
```

### 3. Configure SDK
Create `src/app/lib/propchain.service.ts` for SDK initialization:

```typescript
import { Injectable } from '@angular/core';
import { PropChainClient, getNetworkConfig, NETWORKS } from '@propchain/sdk';

@Injectable({
  providedIn: 'root'
})
export class PropChainService {
  private client: PropChainClient | null = null;

  async getClient(): Promise<PropChainClient> {
    if (!this.client) {
      const config = getNetworkConfig(NETWORKS.LOCAL);
      this.client = await PropChainClient.create(config.wsEndpoint, {
        propertyRegistry: '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', // Replace with your contract addresses
        propertyToken: '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty',
        oracle: '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
      });
    }
    return this.client;
  }
}
```

## Wallet Connection

### Create a Wallet Connection Component

```typescript
// src/app/components/connect-wallet/connect-wallet.component.ts
import { Component, Output, EventEmitter, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { connectExtension } from '@propchain/sdk';

@Component({
  selector: 'app-connect-wallet',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="wallet-section">
      <div *ngIf="account" class="wallet-info">
        <span>Connected: {{ truncateAddress(account) }}</span>
        <button (click)="handleDisconnect()">Disconnect</button>
      </div>
      <button *ngIf="!account" (click)="handleConnect()" [disabled]="isConnecting">
        {{ isConnecting ? 'Connecting...' : 'Connect Wallet' }}
      </button>
    </div>
  `
})
export class ConnectWalletComponent implements OnInit {
  @Output() connect = new EventEmitter<string>();
  @Output() disconnect = new EventEmitter<void>();

  account: string | null = null;
  isConnecting = false;

  ngOnInit(): void {
    const savedAccount = localStorage.getItem('selectedAccount');
    if (savedAccount) {
      this.account = savedAccount;
      this.connect.emit(savedAccount);
    }
  }

  async handleConnect(): Promise<void> {
    this.isConnecting = true;
    try {
      const accounts = await connectExtension('PropChain dApp');
      if (accounts.length > 0) {
        this.account = accounts[0].address;
        localStorage.setItem('selectedAccount', accounts[0].address);
        this.connect.emit(accounts[0].address);
      }
    } catch (err) {
      console.error('Failed to connect wallet:', err);
    } finally {
      this.isConnecting = false;
    }
  }

  handleDisconnect(): void {
    this.account = null;
    localStorage.removeItem('selectedAccount');
    this.disconnect.emit();
  }

  truncateAddress(addr: string): string {
    return `${addr.slice(0, 6)}…${addr.slice(-4)}`;
  }
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

```typescript
// src/app/components/mint-property-token/mint-property-token.component.ts
import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { PropChainService } from '../../lib/propchain.service';
import { getExtensionSigner } from '@propchain/sdk';

@Component({
  selector: 'app-mint-property-token',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="mint-section">
      <h2>Mint Property Token</h2>
      <p *ngIf="message">{{ message }}</p>
      <button (click)="handleMint()" [disabled]="isMinting">
        {{ isMinting ? 'Minting...' : 'Mint Token' }}
      </button>
    </div>
  `
})
export class MintPropertyTokenComponent {
  @Input() account!: string;
  isMinting = false;
  message: string | null = null;

  constructor(private propChainService: PropChainService) {}

  async handleMint(): Promise<void> {
    this.isMinting = true;
    this.message = null;

    try {
      const client = await this.propChainService.getClient();
      const signer = await getExtensionSigner(this.account);

      const { tokenId } = await client.propertyToken.registerPropertyWithToken(signer, {
        location: '123 Main St, New York, NY',
        size: 2500,
        legalDescription: 'Lot 1, Block 2',
        valuation: BigInt('50000000000000'), // 500,000 units
        documentsUrl: 'ipfs://QmXoypJgWnX9w5QZ5z9x9QZ5z9x9QZ5z9x9QZ5z9x9QZ5z9',
      });

      this.message = `Successfully minted property token #${tokenId}`;
    } catch (err) {
      this.message = `Minting failed: ${err}`;
    } finally {
      this.isMinting = false;
    }
  }
}
```

## Transferring Tokens

### Create a Transfer Component

```typescript
// src/app/components/transfer-property-token/transfer-property-token.component.ts
import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { PropChainService } from '../../lib/propchain.service';
import { getExtensionSigner } from '@propchain/sdk';

@Component({
  selector: 'app-transfer-property-token',
  standalone: true,
  imports: [CommonModule, FormsModule],
  template: `
    <div class="transfer-section">
      <h2>Transfer Property Token</h2>
      <p *ngIf="message">{{ message }}</p>
      <div>
        <label>Token ID:</label>
        <input
          [(ngModel)]="tokenId"
          type="text"
          placeholder="Enter token ID"
        />
      </div>
      <div>
        <label>To Address:</label>
        <input
          [(ngModel)]="toAddress"
          type="text"
          placeholder="Enter recipient address"
        />
      </div>
      <button (click)="handleTransfer()" [disabled]="isTransferring">
        {{ isTransferring ? 'Transferring...' : 'Transfer' }}
      </button>
    </div>
  `
})
export class TransferPropertyTokenComponent {
  @Input() account!: string;
  toAddress = '';
  tokenId = '';
  isTransferring = false;
  message: string | null = null;

  constructor(private propChainService: PropChainService) {}

  async handleTransfer(): Promise<void> {
    this.isTransferring = true;
    this.message = null;

    try {
      const client = await this.propChainService.getClient();
      const signer = await getExtensionSigner(this.account);

      await client.propertyToken.transfer(signer, BigInt(this.tokenId), this.toAddress);
      this.message = `Successfully transferred token #${this.tokenId} to ${this.toAddress}`;
    } catch (err) {
      this.message = `Transfer failed: ${err}`;
    } finally {
      this.isTransferring = false;
    }
  }
}
```

## Oracle Reads

### Create an Oracle Read Component

```typescript
// src/app/components/oracle-read/oracle-read.component.ts
import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { PropChainService } from '../../lib/propchain.service';
import { formatValuation } from '@propchain/sdk';

@Component({
  selector: 'app-oracle-read',
  standalone: true,
  imports: [CommonModule, FormsModule],
  template: `
    <div class="oracle-section">
      <h2>Oracle Property Valuation</h2>
      <p *ngIf="message">{{ message }}</p>
      <div>
        <label>Property ID:</label>
        <input
          [(ngModel)]="propertyId"
          type="text"
          placeholder="Enter property ID"
        />
      </div>
      <button (click)="handleReadOracle()" [disabled]="isReading">
        {{ isReading ? 'Reading...' : 'Get Valuation' }}
      </button>
      <div *ngIf="valuation" class="valuation-result">
        <h3>Valuation Result</h3>
        <p>Estimated Value: {{ formatValuation(valuation.predictedValuation) }}</p>
        <p>Confidence: {{ valuation.confidenceScore }}%</p>
      </div>
    </div>
  `
})
export class OracleReadComponent {
  propertyId = '';
  valuation: any = null;
  isReading = false;
  message: string | null = null;
  formatValuation = formatValuation;

  constructor(private propChainService: PropChainService) {}

  async handleReadOracle(): Promise<void> {
    this.isReading = true;
    this.message = null;

    try {
      const client = await this.propChainService.getClient();
      const result = await client.oracle.getPropertyValuation(BigInt(this.propertyId));
      this.valuation = result;
      this.message = `Valuation retrieved for property #${this.propertyId}`;
    } catch (err) {
      this.message = `Read failed: ${err}`;
    } finally {
      this.isReading = false;
    }
  }
}
```

## Putting It All Together

```typescript
// src/app/app.component.ts
import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ConnectWalletComponent } from './components/connect-wallet/connect-wallet.component';
import { MintPropertyTokenComponent } from './components/mint-property-token/mint-property-token.component';
import { TransferPropertyTokenComponent } from './components/transfer-property-token/transfer-property-token.component';
import { OracleReadComponent } from './components/oracle-read/oracle-read.component';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [
    CommonModule,
    ConnectWalletComponent,
    MintPropertyTokenComponent,
    TransferPropertyTokenComponent,
    OracleReadComponent
  ],
  template: `
    <div class="app">
      <app-connect-wallet (connect)="handleConnect($event)" (disconnect)="handleDisconnect()" />
      <div *ngIf="account" class="content">
        <app-mint-property-token [account]="account" />
        <app-transfer-property-token [account]="account" />
        <app-oracle-read />
      </div>
    </div>
  `
})
export class AppComponent {
  account: string | null = null;

  handleConnect(address: string): void {
    this.account = address;
  }

  handleDisconnect(): void {
    this.account = null;
  }
}
```

## References
- See [INTEGRATION_EXAMPLES.ts](../../sdk/frontend/INTEGRATION_EXAMPLES.ts) for more examples
- Frontend SDK Guide: [FRONTEND_SDK_GUIDE.md](./FRONTEND_SDK_GUIDE.md)
