# Integration Troubleshooting Guide

## Overview

This guide helps you diagnose and resolve common issues when integrating with PropChain smart contracts. Each issue includes symptoms, causes, solutions, and prevention tips.

---

## Quick Reference

| Symptom | Likely Cause | Quick Fix |
|---------|--------------|-----------|
| "Connection refused" | Wrong RPC endpoint | Check network configuration |
| "Account not found" | Wallet not connected | Reconnect wallet |
| "Gas estimation failed" | Invalid parameters | Validate input data |
| "Not compliant" | KYC not completed | Complete KYC verification |
| "Transaction stuck" | Low gas price | Increase gas limit |

---

## Connection Issues

### Issue: Cannot Connect to Blockchain

**Symptoms**:
```javascript
Error: connect ECONNREFUSED 127.0.0.1:9944
// OR
Error: Unable to retrieve chain info
```

**Possible Causes**:
1. Blockchain node not running
2. Wrong RPC endpoint URL
3. Network firewall blocking connection
4. Node syncing or offline

**Solutions**:

#### Solution 1: Verify Node Status
```bash
# Check if local node is running
curl -H "Content-Type: application/json" \
     -d '{"id":1, "jsonrpc":"2.0", "method": "system_health"}' \
     http://localhost:9944

# Expected response
{"jsonrpc":"2.0","result":{"isSyncing":false,"peers":5,"shouldHavePeers":true},"id":1}
```

#### Solution 2: Check Configuration
```typescript
// ✅ Correct configuration
const config = {
  rpcEndpoint: 'ws://localhost:9944', // Local development
  // OR
  rpcEndpoint: 'wss://rpc.propchain.io', // Production
};

// ❌ Common mistakes
const wrongConfig = {
  rpcEndpoint: 'http://localhost:9944', // Wrong protocol
  // OR
  rpcEndpoint: 'wss://localhost:9944', // Wrong port for wss
};
```

#### Solution 3: Test Connection
```typescript
async function testConnection() {
  try {
    const api = await ApiPromise.create({
      provider: new WsProvider('ws://localhost:9944'),
      throwOnConnect: false
    });

    if (!api.isConnected) {
      throw new Error('Connection failed');
    }

    const [chain, nodeName] = await Promise.all([
      api.rpc.system.chain(),
      api.rpc.system.name()
    ]);

    console.log(`✅ Connected to ${chain} via ${nodeName}`);
  } catch (error) {
    console.error('❌ Connection failed:', error.message);
  }
}

testConnection();
```

**Prevention**:
- Use environment variables for RPC endpoints
- Implement automatic reconnection logic
- Have fallback endpoints configured
- Monitor node health regularly

---

### Issue: Intermittent Disconnections

**Symptoms**:
```javascript
API-WS: disconnected from ws://localhost:9944
// Followed by immediate reconnection attempts
```

**Causes**:
1. Network instability
2. Node restarting
3. WebSocket timeout
4. Load balancer issues

**Solutions**:

#### Implement Robust Reconnection
```typescript
class ResilientConnection {
  private api: ApiPromise | null = null;
  private reconnectAttempts = 0;
  private maxReconnects = 5;

  async connectWithRetry(rpcEndpoint: string): Promise<ApiPromise> {
    while (this.reconnectAttempts < this.maxReconnects) {
      try {
        this.api = await ApiPromise.create({
          provider: new WsProvider(rpcEndpoint),
          throwOnConnect: true
        });

        // Set up disconnect handler
        this.api.on('disconnected', () => {
          console.log('Disconnected, attempting to reconnect...');
          this.reconnectAttempts++;
          this.connectWithRetry(rpcEndpoint);
        });

        this.reconnectAttempts = 0; // Reset on success
        return this.api;
      } catch (error) {
        this.reconnectAttempts++;
        
        if (this.reconnectAttempts === this.maxReconnects) {
          throw new Error(`Failed to connect after ${this.maxReconnects} attempts`);
        }

        // Exponential backoff
        const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
        console.log(`Reconnecting in ${delay}ms...`);
        await this.sleep(delay);
      }
    }

    throw new Error('Max reconnection attempts reached');
  }

  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}
```

**Prevention**:
- Use connection pooling
- Implement heartbeat mechanism
- Configure proper timeout values
- Use multiple RPC endpoints

---

## Wallet Issues

### Issue: Wallet Not Detected

**Symptoms**:
```javascript
Error: No injected web3 provider found
// OR
window.injectedWeb3 is undefined
```

**Causes**:
1. Polkadot extension not installed
2. Extension not enabled for site
3. Loading order issue
4. Browser compatibility

**Solutions**:

#### Solution 1: Verify Extension Installation
```typescript
async function checkWalletExtension(): Promise<{
  installed: boolean;
  enabled: boolean;
  accounts: number;
}> {
  // Check if extension exists
  const { web3Enable } = await import('@polkadot/extension-dapp');
  
  try {
    const extensions = await web3Enable('Your DApp');
    
    if (extensions.length === 0) {
      return {
        installed: false,
        enabled: false,
        accounts: 0
      };
    }

    const { web3Accounts } = await import('@polkadot/extension-dapp');
    const accounts = await web3Accounts();

    return {
      installed: true,
      enabled: true,
      accounts: accounts.length
    };
  } catch (error) {
    return {
      installed: true,
      enabled: false,
      accounts: 0
    };
  }
}

// Usage
const status = await checkWalletExtension();

if (!status.installed) {
  alert('Please install Polkadot extension from https://polkadot.js.org/extension/');
} else if (!status.enabled) {
  alert('Please enable Polkadot extension for this site');
}
```

#### Solution 2: Proper Loading Order
```typescript
// ✅ CORRECT: Wait for DOM ready
document.addEventListener('DOMContentLoaded', async () => {
  await initializeWallet();
});

// ❌ WRONG: Might run before extension loads
initializeWallet(); // Don't do this at top level
```

**Prevention**:
- Show clear installation instructions
- Detect extension early in app lifecycle
- Provide alternative wallet options
- Test across different browsers

---

### Issue: Transaction Signing Fails

**Symptoms**:
```javascript
Error: Unable to sign transaction
// OR
User rejected the request
```

**Causes**:
1. Account locked in extension
2. Insufficient balance for fees
3. User rejected signing
4. Wrong account selected

**Solutions**:

#### Pre-Transaction Checklist
```typescript
async function preTransactionCheck(
  signerAddress: string,
  estimatedFee: bigint
): Promise<{
  canProceed: boolean;
  errors: string[];
  warnings: string[];
}> {
  const errors: string[] = [];
  const warnings: string[] = [];

  // Check 1: Account exists
  const { web3Accounts } = await import('@polkadot/extension-dapp');
  const accounts = await web3Accounts();
  const account = accounts.find(a => a.address === signerAddress);

  if (!account) {
    errors.push('Selected account not found in wallet');
  }

  // Check 2: Account unlocked
  // (This requires user interaction to verify)

  // Check 3: Sufficient balance
  const api = await ApiPromise.create({ 
    provider: new WsProvider('wss://rpc.propchain.io') 
  });
  
  const { data: balance } = await api.query.system.account(signerAddress);
  const availableBalance = balance.free.toBn();

  if (availableBalance.lt(estimatedFee)) {
    errors.push('Insufficient balance for transaction fees');
  } else if (availableBalance.lt(estimatedFee.muln(2))) {
    warnings.push('Low balance - consider adding more funds');
  }

  return {
    canProceed: errors.length === 0,
    errors,
    warnings
  };
}

// Usage before sending transaction
const checks = await preTransactionCheck(account.address, estimatedFee);

if (!checks.canProceed) {
  alert('Cannot proceed:\n' + checks.errors.join('\n'));
  return;
}

if (checks.warnings.length > 0) {
  const confirm = window.confirm(checks.warnings.join('\n\nContinue?'));
  if (!confirm) return;
}

// Safe to proceed with transaction
```

**Prevention**:
- Always show fee estimates upfront
- Verify account selection before signing
- Provide clear error messages
- Implement transaction simulation

---

## Contract Interaction Issues

### Issue: Contract Not Found

**Symptoms**:
```javascript
Error: Code hash not found
// OR
Contract does not exist at the specified address
```

**Causes**:
1. Wrong contract address
2. Contract not deployed to network
3. Network mismatch (mainnet vs testnet)
4. ABI/version incompatibility

**Solutions**:

#### Verify Contract Deployment
```typescript
async function verifyContractDeployment(
  api: ApiPromise,
  contractAddress: string
): Promise<{
  exists: boolean;
  codeHash?: string;
  deployer?: string;
  deployedAt?: number;
}> {
  try {
    const { nonce, data } = await api.query.contracts.contractInfoOf(contractAddress);

    if (!data.isSome) {
      return { exists: false };
    }

    const contractInfo = data.unwrap();

    return {
      exists: true,
      codeHash: contractInfo.codeHash.toString(),
      deployer: contractInfo.deployer.toString(),
      deployedAt: contractInfo.deployedBlockNumber?.toNumber() || 0
    };
  } catch (error) {
    return { exists: false };
  }
}

// Usage
const verification = await verifyContractDeployment(api, contractAddress);

if (!verification.exists) {
  console.error('Contract not deployed at this address');
  console.log('Expected address:', contractAddress);
  
  // List known addresses
  console.log('Known addresses:', {
    mainnet: '5GrwvaEF...',
    testnet: '5FHneW46...',
    local: '5FLSigC9...'
  });
}
```

**Prevention**:
- Use configuration files for addresses
- Verify deployment after upload
- Document addresses per network
- Implement address validation

---

### Issue: Gas Estimation Fails

**Symptoms**:
```javascript
Error: Gas estimation failed
// OR
Out of gas
```

**Causes**:
1. Invalid input parameters
2. Contract execution would revert
3. Insufficient account balance
4. Complex operation exceeding limits

**Solutions**:

#### Robust Gas Estimation
```typescript
async function estimateGasWithFallback(
  query: () => Promise<any>,
  defaultValue: bigint = BigInt(1000000000)
): Promise<{
  gasRequired: bigint;
  confidence: 'high' | 'medium' | 'low';
  warning?: string;
}> {
  try {
    const result = await query();

    if (!result.gasRequired.ok) {
      throw new Error(result.gasRequired.err?.toString() || 'Unknown error');
    }

    const gasRequired = result.gasRequired.gasRequired;

    // Add 20% buffer for safety
    const bufferedGas = (gasRequired.toBigInt() * BigInt(6)) / BigInt(5);

    return {
      gasRequired: bufferedGas,
      confidence: 'high'
    };
  } catch (error: any) {
    console.warn('Gas estimation failed, using fallback:', error.message);

    // Try to diagnose the issue
    if (error.message.includes('InvalidMetadata')) {
      return {
        gasRequired: defaultValue,
        confidence: 'low',
        warning: 'Invalid metadata - gas estimate may be inaccurate'
      };
    }

    if (error.message.includes('NotCompliant')) {
      throw new Error('Account not compliant - cannot estimate gas');
    }

    // Use default with low confidence
    return {
      gasRequired: defaultValue,
      confidence: 'low',
      warning: 'Using default gas limit - transaction may fail'
    };
  }
}

// Usage
const { gasRequired, confidence, warning } = await estimateGasWithFallback(
  () => contract.query.registerProperty(
    signer.address,
    { gasLimit: -1 },
    metadata
  ),
  BigInt(500000000) // Default 500M gas
);

if (warning) {
  console.warn('Gas warning:', warning);
}

console.log(`Gas required: ${gasRequired} (confidence: ${confidence})`);
```

**Prevention**:
- Always validate inputs before estimation
- Use generous gas limits for complex operations
- Implement gas price oracles
- Monitor gas usage patterns

---

## Compliance Issues

### Issue: Not Compliant Error

**Symptoms**:
```javascript
Error: NotCompliant
// OR
Recipient is not compliant with regulatory requirements
```

**Causes**:
1. KYC verification not completed
2. AML check failed or expired
3. Sanctions list match
4. Jurisdiction restrictions

**Solutions**:

#### Check Compliance Status
```typescript
async function diagnoseComplianceIssue(
  account: string,
  contract: ContractPromise
): Promise<{
  isCompliant: boolean;
  issues: string[];
  recommendations: string[];
}> {
  const issues: string[] = [];
  const recommendations: string[] = [];

  try {
    // Check basic compliance
    const { output } = await contract.query.check_account_compliance(
      contract.address,
      { gasLimit: -1 },
      account
    );

    const isCompliant = output?.toPrimitive() as boolean;

    if (!isCompliant) {
      issues.push('Account not marked as compliant in registry');
      
      // Check specific requirements
      const kycStatus = await checkKYCStatus(account);
      const amlStatus = await checkAMLStatus(account);
      const sanctionsStatus = await checkSanctionsList(account);

      if (!kycStatus.verified) {
        issues.push('KYC verification not completed');
        recommendations.push('Complete KYC verification at https://kyc.propchain.io');
      }

      if (!amlStatus.passed) {
        issues.push('AML check failed or expired');
        recommendations.push('Update AML verification');
      }

      if (sanctionsStatus.match) {
        issues.push('Account found on sanctions list');
        recommendations.push('Contact support for resolution');
      }

      if (kycStatus.expired) {
        issues.push('KYC verification has expired');
        recommendations.push('Renew KYC verification');
      }
    }

    return {
      isCompliant,
      issues,
      recommendations
    };
  } catch (error) {
    return {
      isCompliant: false,
      issues: ['Failed to check compliance status'],
      recommendations: ['Try again later or contact support']
    };
  }
}

// Usage
const diagnosis = await diagnoseComplianceIssue(account.address, contract);

if (!diagnosis.isCompliant) {
  console.error('Compliance issues found:');
  diagnosis.issues.forEach(issue => console.error('  -', issue));
  
  console.log('\nRecommended actions:');
  diagnosis.recommendations.forEach(rec => console.log('  -', rec));
}
```

**Prevention**:
- Check compliance before critical operations
- Show compliance status in UI
- Send expiry reminders
- Provide clear KYC instructions

---

## Transaction Issues

### Issue: Transaction Stuck Pending

**Symptoms**:
```javascript
Transaction submitted but never finalizes
// OR
Stuck at "In Block" status
```

**Causes**:
1. Network congestion
2. Gas price too low
3. Transaction pool full
4. Block production issues

**Solutions**:

#### Monitor Transaction Status
```typescript
async function monitorTransaction(
  txHash: string,
  timeoutMs: number = 5 * 60 * 1000 // 5 minutes
): Promise<{
  status: 'finalized' | 'failed' | 'timeout';
  blockHash?: string;
  events?: any[];
}> {
  const startTime = Date.now();

  return new Promise((resolve, reject) => {
    const checkStatus = async () => {
      try {
        const tx = await api.rpc.chain.getBlockHash(0); // Get latest
        const signedBlock = await api.rpc.chain.getBlock(tx);
        
        // Search for transaction in recent blocks
        for (let i = 0; i < 10; i++) {
          const blockHash = await api.rpc.chain.getBlockHash(
            signedBlock.block.header.number.toNumber() - i
          );
          
          const block = await api.rpc.chain.getBlock(blockHash);
          
          // Check if our tx is in this block
          // (Simplified - actual implementation would be more robust)
          
          if (Date.now() - startTime > timeoutMs) {
            resolve({ status: 'timeout' });
            return;
          }
        }

        // Check again in 5 seconds
        setTimeout(checkStatus, 5000);
      } catch (error) {
        reject(error);
      }
    };

    checkStatus();
  });
}

// Alternative: Implement transaction replacement
async function replaceTransaction(
  originalTx: any,
  higherGasPrice: bigint
): Promise<string> {
  // Create new transaction with same nonce but higher gas
  const newTx = {
    ...originalTx,
    gasPrice: higherGasPrice
  };

  return await sendTransaction(newTx);
}
```

**Prevention**:
- Use appropriate gas prices
- Monitor network conditions
- Implement transaction acceleration
- Set reasonable timeouts

---

### Issue: Transaction Reverted

**Symptoms**:
```javascript
ExtrinsicFailed event emitted
// OR
Transaction executed but state unchanged
```

**Causes**:
1. Business logic validation failed
2. Insufficient permissions
3. State precondition not met
4. Contract bug or edge case

**Solutions**:

#### Decode Failure Reason
```typescript
async function decodeTransactionFailure(
  result: ISubmittableResult
): Promise<{
  success: boolean;
  error?: string;
  section?: string;
  method?: string;
  documentation?: string;
}> {
  const failedEvent = result.events.find(
    ({ event }) => event.method === 'ExtrinsicFailed'
  );

  if (!failedEvent) {
    return { success: true };
  }

  // Extract dispatch error
  const [dispatchError] = failedEvent.event.data;
  
  if (!dispatchError) {
    return {
      success: false,
      error: 'Unknown failure reason'
    };
  }

  let errorDetails: any = {};

  if (dispatchError.isModule) {
    const decoded = api.registry.findMetaError(dispatchError.asModule);
    errorDetails = {
      section: decoded.section,
      method: decoded.method,
      documentation: `See API docs for ${decoded.section}.${decoded.method}`
    };
  } else if (dispatchError.isToken) {
    errorDetails = {
      section: 'token',
      method: dispatchError.asToken.type,
      documentation: 'Token-related error'
    };
  }

  // Map to human-readable message
  const errorMessage = mapErrorToMessage(errorDetails);

  return {
    success: false,
    error: errorMessage,
    ...errorDetails
  };
}

function mapErrorToMessage(error: any): string {
  const errorMessages: Record<string, string> = {
    'propertyRegistry.PropertyNotFound': 'The specified property does not exist',
    'propertyRegistry.Unauthorized': 'You do not have permission for this action',
    'propertyRegistry.InvalidMetadata': 'Property metadata is invalid or malformed',
    'propertyRegistry.NotCompliant': 'Account does not meet compliance requirements',
    'balances.InsufficientBalance': 'Insufficient balance for this transaction',
    'contracts.Out_Of_Gas': 'Transaction ran out of gas'
  };

  const key = `${error.section}.${error.method}`;
  return errorMessages[key] || `Contract error: ${error.method}`;
}

// Usage
const txResult = await sendTransaction(tx);
const failure = await decodeTransactionFailure(txResult);

if (!failure.success) {
  console.error('Transaction failed:', failure.error);
  console.log('Documentation:', failure.documentation);
  
  // Show user-friendly message
  alert(failure.error);
}
```

**Prevention**:
- Simulate transactions before sending
- Validate all preconditions
- Use dry-run queries
- Implement comprehensive error handling

---

## Performance Issues

### Issue: Slow Query Response

**Symptoms**:
```javascript
Queries taking 5+ seconds to complete
// OR
UI freezes during blockchain queries
```

**Causes**:
1. Too many sequential queries
2. Large dataset fetching
3. Network latency
4. Inefficient query patterns

**Solutions**:

#### Optimize Query Performance
```typescript
class OptimizedQueryService {
  private cache = new LRUCache<string, any>({ max: 1000 });
  private batchQueue = new Map<string, Promise<any>>();

  async getPropertyWithCache(propertyId: number): Promise<any> {
    const cacheKey = `property:${propertyId}`;
    
    // Check cache first
    const cached = this.cache.get(cacheKey);
    if (cached) {
      return cached;
    }

    // Check if already fetching
    const existing = this.batchQueue.get(cacheKey);
    if (existing) {
      return existing;
    }

    // Fetch with batching
    const fetchPromise = this.fetchProperty(propertyId)
      .then(result => {
        this.cache.set(cacheKey, result);
        this.batchQueue.delete(cacheKey);
        return result;
      })
      .catch(error => {
        this.batchQueue.delete(cacheKey);
        throw error;
      });

    this.batchQueue.set(cacheKey, fetchPromise);
    return fetchPromise;
  }

  async fetchMultipleProperties(propertyIds: number[]): Promise<any[]> {
    // Batch into single query if possible
    const { output } = await contract.query.get_properties_batch(
      contract.address,
      { gasLimit: -1 },
      propertyIds
    );

    return output.unwrap().toHuman();
  }

  private async fetchProperty(propertyId: number): Promise<any> {
    const { output } = await contract.query.get_property(
      contract.address,
      { gasLimit: -1 },
      propertyId
    );

    if (!output || !output.isOk) {
      throw new Error('Property not found');
    }

    return output.unwrap().toHuman();
  }
}
```

**Prevention**:
- Implement caching strategies
- Use batch queries
- Paginate large datasets
- Offload to indexer when possible

---

## Build and Deployment Issues

### Issue: TypeScript Compilation Errors

**Symptoms**:
```typescript
error TS2307: Cannot find module '@polkadot/api'
// OR
Type 'bigint' is not assignable to type 'BN'
```

**Solutions**:

#### Fix Type Issues
```json
// tsconfig.json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "lib": ["ES2020"],
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "outDir": "./dist",
    "rootDir": "./src"
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}
```

```json
// package.json dependencies
{
  "dependencies": {
    "@polkadot/api": "^10.0.0",
    "@polkadot/api-contract": "^10.0.0",
    "@polkadot/util": "^12.0.0",
    "@polkadot/util-crypto": "^12.0.0"
  },
  "devDependencies": {
    "@types/node": "^20.0.0",
    "typescript": "^5.0.0"
  }
}
```

**Prevention**:
- Pin dependency versions
- Use consistent Polkadot.js versions
- Run type checking in CI/CD
- Keep dependencies updated

---

## Getting More Help

### Resources

1. **Documentation**:
   - [API Reference](./API_GUIDE.md)
   - [Complete Integration Guide](./COMPLETE_INTEGRATION_GUIDE.md)
   - [Architecture Overview](./SYSTEM_ARCHITECTURE_OVERVIEW.md)

2. **Community Support**:
   - Discord: Real-time developer chat
   - GitHub Issues: Bug reports and feature requests
   - Stack Overflow: Technical Q&A (tag: propchain)

3. **Direct Support**:
   - Email: dev@propchain.io
   - Office Hours: Weekly developer Q&A

### How to Ask for Help

When reporting issues, include:

```markdown
**Issue Description**: Clear description of the problem

**Environment**:
- Node.js version: v18.x.x
- Network: testnet/mainnet/local
- Browser/Platform: Chrome, Firefox, etc.
- Package versions: @polkadot/api@10.x.x

**Steps to Reproduce**:
1. Step 1
2. Step 2
3. Step 3

**Expected Behavior**: What should happen

**Actual Behavior**: What actually happened

**Code Example**: Minimal reproducible example

**Error Messages**: Full error stack trace

**Troubleshooting Attempted**: What you've tried so far
```

---

**Last Updated**: March 27, 2026  
**Version**: 1.0.0  
**Maintained By**: PropChain Development Team
