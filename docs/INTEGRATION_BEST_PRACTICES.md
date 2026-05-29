# PropChain Integration Best Practices

## Overview

This guide documents proven best practices for integrating with PropChain smart contracts. These patterns and principles have been developed through real-world production deployments and community feedback.

---

## Table of Contents

1. [Architecture Best Practices](#architecture-best-practices)
2. [Security Best Practices](#security-best-practices)
3. [Performance Best Practices](#performance-best-practices)
4. [User Experience Best Practices](#user-experience-best-practices)
5. [Testing Best Practices](#testing-best-practices)
6. [Monitoring & Operations](#monitoring--operations)
7. [Code Organization](#code-organization)

---

## Architecture Best Practices

### 1. Layered Architecture Pattern

**Principle**: Separate concerns into distinct layers for maintainability and testability.

**Recommended Structure**:
```typescript
src/
├── api/                    # Blockchain connection layer
│   ├── blockchain.ts       # API initialization
│   └── provider.ts         # RPC provider management
├── contracts/              # Contract abstraction layer
│   ├── registry.ts        # Property registry wrapper
│   ├── escrow.ts          # Escrow contract wrapper
│   └── compliance.ts      # Compliance registry wrapper
├── services/              # Business logic layer
│   ├── propertyService.ts
│   ├── transferService.ts
│   └── complianceService.ts
├── repositories/          # Data access layer
│   ├── propertyRepository.ts
│   └── eventRepository.ts
└── utils/                 # Shared utilities
    ├── formatters.ts
    └── validators.ts
```

**Benefits**:
- Clear separation of concerns
- Easy to test each layer independently
- Simplifies maintenance and updates
- Enables mocking for frontend development

**Example Implementation**:
```typescript
// ✅ GOOD: Layered architecture
class PropertyService {
  constructor(
    private registry: PropertyRegistryContract,
    private repository: PropertyRepository,
    private validator: PropertyValidator
  ) {}

  async registerProperty(metadata: PropertyMetadata): Promise<number> {
    // Business logic layer
    await this.validator.validate(metadata);
    
    // Contract interaction
    const result = await this.registry.register(metadata);
    
    // Data persistence
    await this.repository.cache(result.property);
    
    return result.propertyId;
  }
}

// ❌ BAD: Mixed concerns
async function registerProperty(metadata: any) {
  // Direct contract calls in business logic
  const contract = new ContractPromise(...);
  await contract.tx.registerProperty(...);
  // No validation, no caching, hard to test
}
```

---

### 2. Repository Pattern for Blockchain Data

**Principle**: Abstract blockchain data access behind repository interfaces.

**Implementation**:
```typescript
interface IPropertyRepository {
  getById(id: number): Promise<Property | null>;
  getByOwner(owner: string): Promise<Property[]>;
  save(property: Property): Promise<void>;
  update(id: number, updates: Partial<Property>): Promise<void>;
}

class PropertyRepository implements IPropertyRepository {
  private cache: Map<number, Property> = new Map();

  async getById(id: number): Promise<Property | null> {
    // Check cache first
    const cached = this.cache.get(id);
    if (cached) return cached;

    // Query blockchain
    const property = await this.fetchFromBlockchain(id);
    
    // Cache result
    this.cache.set(id, property);
    
    return property;
  }

  async getByOwner(owner: string): Promise<Property[]> {
    const propertyIds = await this.contract.query.get_properties_by_owner(owner);
    const properties = await Promise.all(
      propertyIds.map(id => this.getById(id))
    );
    return properties.filter((p): p is Property => p !== null);
  }

  private async fetchFromBlockchain(id: number): Promise<Property> {
    const { output } = await this.contract.query.get_property(id);
    return this.transformProperty(output.unwrap());
  }

  private transformProperty(data: any): Property {
    return {
      id: data.id.toNumber(),
      owner: data.owner.toString(),
      metadata: {
        location: data.metadata.location,
        size: data.metadata.size.toNumber(),
        valuation: BigInt(data.metadata.valuation)
      }
    };
  }
}
```

**Benefits**:
- Single source of truth for data access
- Easy to swap blockchain for mock data
- Centralized caching strategy
- Consistent error handling

---

### 3. Event-Driven Architecture

**Principle**: Use blockchain events to drive application state changes.

**Implementation**:
```typescript
class EventDispatcher {
  private listeners: Map<string, Set<EventHandler>> = new Map();

  async subscribeToPropertyEvents(): Promise<void> {
    await this.api.query.system.events(async (events) => {
      events.forEach((record) => {
        const { event } = record;

        if (event.section === 'propertyRegistry') {
          const handlers = this.listeners.get(event.method);
          
          handlers?.forEach(handler => {
            handler({
              type: event.method,
              data: event.data.toHuman(),
              blockHash: record.phase.asApplyExtrinsic.toString(),
              timestamp: Date.now()
            });
          });
        }
      });
    });
  }

  on(eventType: string, handler: EventHandler): void {
    if (!this.listeners.has(eventType)) {
      this.listeners.set(eventType, new Set());
    }
    this.listeners.get(eventType)!.add(handler);
  }

  off(eventType: string, handler: EventHandler): void {
    this.listeners.get(eventType)?.delete(handler);
  }
}

// Usage
const dispatcher = new EventDispatcher();

dispatcher.on('PropertyRegistered', (event) => {
  console.log('New property:', event.data);
  // Update UI, send notification, refresh cache
});

dispatcher.on('PropertyTransferred', (event) => {
  // Handle ownership change
});
```

**Benefits**:
- Real-time updates
- Loose coupling between components
- Easy to add new event handlers
- Better user experience

---

## Security Best Practices

### 1. Input Validation Strategy

**Principle**: Never trust user input - validate at every layer.

**Implementation with Zod**:
```typescript
import { z } from 'zod';

// Define strict schemas
const PropertyMetadataSchema = z.object({
  location: z
    .string()
    .min(1, 'Location is required')
    .max(256, 'Location too long')
    .regex(/^.+, .+$/, 'Must include city and state/country'),
  
  size: z
    .number()
    .positive('Size must be positive')
    .max(10000000, 'Size exceeds maximum'),
  
  valuation: z
    .number()
    .min(1000, 'Minimum valuation is $10')
    .finite('Valuation must be a valid number'),
  
  documents_url: z
    .string()
    .url('Invalid URL format')
    .refine(
      url => url.startsWith('ipfs://') || url.startsWith('https://'),
      'Must be IPFS or HTTPS URL'
    )
    .optional(),
  
  legal_description: z.string().max(10000).optional()
});

type PropertyMetadata = z.infer<typeof PropertyMetadataSchema>;

// Validation service
class ValidationService {
  async validatePropertyMetadata(
    metadata: unknown
  ): Promise<{ valid: boolean; errors: string[] }> {
    try {
      await PropertyMetadataSchema.parseAsync(metadata);
      return { valid: true, errors: [] };
    } catch (error) {
      if (error instanceof z.ZodError) {
        return {
          valid: false,
          errors: error.errors.map(e => e.message)
        };
      }
      throw error;
    }
  }
}

// Usage in service
async function registerProperty(metadata: unknown) {
  const validation = await validationService.validatePropertyMetadata(metadata);
  
  if (!validation.valid) {
    throw new UserInputError('Invalid metadata', validation.errors);
  }
  
  // Safe to proceed
  await contract.tx.registerProperty(metadata);
}
```

**Benefits**:
- Catches errors early
- Clear error messages for users
- Prevents injection attacks
- Type safety with runtime validation

---

### 2. Secure Key Management

**Principle**: Never expose private keys or seed phrases in application code.

**Best Practices**:

#### ✅ DO: Use Wallet Extensions
```typescript
// Let users manage their own keys
const { web3FromAddress } = await import('@polkadot/extension-dapp');
const injector = await web3FromAddress(account.address);

// Extension handles signing securely
await tx.signAndSend(account, { signer: injector.signer });
```

#### ❌ DON'T: Store Private Keys
```typescript
// NEVER do this!
const keyring = new Keyring();
const pair = keyring.addFromSeed(seedPhrase); // Exposed in code!
await tx.signAndSend(pair);
```

#### ✅ DO: Use Environment Variables for Server Keys
```typescript
// .env (never commit to git)
ADMIN_PRIVATE_KEY=your_secure_key_here

// config.ts
const adminKey = process.env.ADMIN_PRIVATE_KEY;

if (!adminKey) {
  throw new Error('ADMIN_PRIVATE_KEY not set');
}
```

---

### 3. Rate Limiting and DoS Prevention

**Principle**: Protect your backend from abuse with rate limiting.

**Implementation**:
```typescript
import rateLimit from 'express-rate-limit';
import RedisStore from 'rate-limit-redis';

// Configure rate limiter
const limiter = rateLimit({
  store: new RedisStore({
    client: redisClient,
    prefix: 'rl:'
  }),
  windowMs: 15 * 60 * 1000, // 15 minutes
  max: 100, // 100 requests per window
  message: 'Too many requests, please try again later',
  standardHeaders: true,
  legacyHeaders: false,
  skipSuccessfulRequests: false,
  keyGenerator: (req) => {
    return req.ip || req.headers['x-forwarded-for'] as string;
  }
});

// Apply to routes
app.use('/api/', limiter);

// Stricter limits for sensitive operations
const transactionLimiter = rateLimit({
  windowMs: 60 * 1000, // 1 minute
  max: 5, // 5 transactions per minute
  message: 'Transaction limit exceeded'
});

app.post('/api/transactions', transactionLimiter, async (req, res) => {
  // Process transaction
});
```

**Benefits**:
- Prevents DDoS attacks
- Reduces infrastructure costs
- Improves service quality for all users
- Protects against accidental loops

---

## Performance Best Practices

### 1. Caching Strategy

**Principle**: Minimize blockchain queries with intelligent caching.

**Multi-Level Cache Implementation**:
```typescript
class CacheManager {
  private l1Cache = new LRUCache<string, any>({ max: 1000 });
  private l2Cache: Redis; // For distributed caching

  constructor(redisUrl: string) {
    this.l2Cache = createClient({ url: redisUrl });
  }

  async get<T>(key: string): Promise<T | null> {
    // L1 cache (in-memory)
    const l1Result = this.l1Cache.get(key);
    if (l1Result) {
      return l1Result as T;
    }

    // L2 cache (Redis)
    try {
      const l2Result = await this.l2Cache.get(key);
      if (l2Result) {
        const parsed = JSON.parse(l2Result);
        this.l1Cache.set(key, parsed); // Populate L1
        return parsed as T;
      }
    } catch (error) {
      console.error('L2 cache error:', error);
    }

    return null;
  }

  async set(key: string, value: any, ttlSeconds: number = 300): Promise<void> {
    // Set in both caches
    this.l1Cache.set(key, value);
    
    try {
      await this.l2Cache.setEx(key, ttlSeconds, JSON.stringify(value));
    } catch (error) {
      console.error('L2 cache set error:', error);
    }
  }

  async invalidate(pattern: string): Promise<void> {
    // Invalidate matching keys
    const keys = await this.l2Cache.keys(`*${pattern}*`);
    if (keys.length > 0) {
      await this.l2Cache.del(keys);
    }
    
    // Clear L1 cache for pattern
    for (const key of this.l1Cache.keys()) {
      if (key.includes(pattern)) {
        this.l1Cache.delete(key);
      }
    }
  }
}

// Usage
const cache = new CacheManager('redis://localhost:6379');

async function getProperty(propertyId: number): Promise<Property> {
  const cacheKey = `property:${propertyId}`;
  
  // Try cache first
  const cached = await cache.get<Property>(cacheKey);
  if (cached) return cached;

  // Query blockchain
  const property = await fetchFromBlockchain(propertyId);
  
  // Cache for 5 minutes
  await cache.set(cacheKey, property, 300);
  
  return property;
}
```

**Cache Invalidation Strategy**:
```typescript
// Invalidate cache on relevant events
eventDispatcher.on('PropertyRegistered', async (event) => {
  await cache.invalidate('properties:*');
  await cache.invalidate(`owner:${event.data.owner}`);
});

eventDispatcher.on('PropertyTransferred', async (event) => {
  const propertyId = event.data.property_id;
  await cache.invalidate(`property:${propertyId}`);
  await cache.invalidate(`owner:${event.data.from}`);
  await cache.invalidate(`owner:${event.data.to}`);
});
```

---

### 2. Batch Operations

**Principle**: Combine multiple operations to reduce overhead.

**Implementation**:
```typescript
class BatchProcessor {
  private queue: Array<() => Promise<any>> = [];
  private processing = false;

  async add<T>(operation: () => Promise<T>): Promise<T> {
    return new Promise((resolve, reject) => {
      this.queue.push(async () => {
        try {
          const result = await operation();
          resolve(result);
        } catch (error) {
          reject(error);
        }
      });

      // Process after short delay to batch more operations
      if (!this.processing) {
        setTimeout(() => this.processBatch(), 100);
      }
    });
  }

  private async processBatch(): Promise<void> {
    if (this.queue.length === 0) return;

    this.processing = true;

    const batch = [...this.queue];
    this.queue = [];

    try {
      // Execute in parallel where possible
      const results = await Promise.all(batch.map(op => op()));
      console.log(`Processed batch of ${results.length} operations`);
    } catch (error) {
      console.error('Batch processing failed:', error);
    } finally {
      this.processing = false;
    }
  }
}

// Usage
const batchProcessor = new BatchProcessor();

// Queue multiple property queries
const propertyPromises = propertyIds.map(id =>
  batchProcessor.add(() => getProperty(id))
);

// All will be processed in single batch
const properties = await Promise.all(propertyPromises);
```

---

### 3. Lazy Loading and Pagination

**Principle**: Load data on-demand, not all at once.

**Implementation**:
```typescript
interface PaginatedResult<T> {
  items: T[];
  total: number;
  page: number;
  pageSize: number;
  hasMore: boolean;
}

class PropertyQueryService {
  async getPropertiesByOwnerPaginated(
    owner: string,
    page: number = 1,
    pageSize: number = 20
  ): Promise<PaginatedResult<Property>> {
    // Get all property IDs (lightweight)
    const allPropertyIds = await this.getAllPropertyIds(owner);
    
    // Calculate pagination
    const start = (page - 1) * pageSize;
    const end = start + pageSize;
    const pagePropertyIds = allPropertyIds.slice(start, end);
    
    // Load only properties for current page
    const properties = await Promise.all(
      pagePropertyIds.map(id => this.getProperty(id))
    );

    return {
      items: properties,
      total: allPropertyIds.length,
      page,
      pageSize,
      hasMore: end < allPropertyIds.length
    };
  }

  private async getAllPropertyIds(owner: string): Promise<number[]> {
    const { output } = await this.contract.query.get_properties_by_owner(
      this.contract.address,
      { gasLimit: -1 },
      owner
    );

    return output.unwrap().toPrimitive() as number[];
  }
}

// React hook example
function useProperties(owner: string) {
  const [properties, setProperties] = useState<Property[]>([]);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(false);

  const loadPage = useCallback(async (pageNum: number) => {
    setLoading(true);
    try {
      const result = await propertyService.getPropertiesByOwnerPaginated(
        owner,
        pageNum,
        20
      );
      
      setProperties(prev => 
        pageNum === 1 ? result.items : [...prev, ...result.items]
      );
    } finally {
      setLoading(false);
    }
  }, [owner]);

  return {
    properties,
    loading,
    loadMore: () => loadPage(page + 1),
    refresh: () => loadPage(1)
  };
}
```

---

## User Experience Best Practices

### 1. Transaction Feedback

**Principle**: Keep users informed throughout transaction lifecycle.

**Implementation**:
```typescript
enum TransactionStatus {
  PENDING_SIGNATURE = 'pending_signature',
  SUBMITTED = 'submitted',
  IN_BLOCK = 'in_block',
  FINALIZED = 'finalized',
  FAILED = 'failed'
}

interface TransactionState {
  status: TransactionStatus;
  hash?: string;
  blockHash?: string;
  confirmations?: number;
  error?: string;
}

function useTransactionTracker() {
  const [state, setState] = useState<TransactionState>({
    status: TransactionStatus.PENDING_SIGNATURE
  });

  const trackTransaction = useCallback(async (tx: any) => {
    setState({ status: TransactionStatus.SUBMITTED });

    try {
      await tx.signAndSend(account, ({ status, events }) => {
        if (status.isInBlock) {
          setState({
            status: TransactionStatus.IN_BLOCK,
            hash: tx.hash.toString(),
            blockHash: status.asInBlock.toString(),
            confirmations: 0
          });

          // Check for failures
          const failed = events.find(
            ({ event }) => event.method === 'ExtrinsicFailed'
          );
          
          if (failed) {
            setState(prev => ({
              ...prev,
              status: TransactionStatus.FAILED,
              error: 'Transaction failed'
            }));
          }
        } else if (status.isFinalized) {
          setState({
            status: TransactionStatus.FINALIZED,
            hash: tx.hash.toString(),
            blockHash: status.asFinalized.toString(),
            confirmations: 1
          });
        }
      });
    } catch (error: any) {
      setState({
        status: TransactionStatus.FAILED,
        error: error.message || 'Transaction failed'
      });
    }
  }, []);

  return { state, trackTransaction };
}

// UI Component
function TransactionProgress({ status, error }: TransactionState) {
  return (
    <div className="transaction-progress">
      <Step 
        label="Sign Transaction" 
        active={status === TransactionStatus.PENDING_SIGNATURE}
        complete={[
          TransactionStatus.SUBMITTED,
          TransactionStatus.IN_BLOCK,
          TransactionStatus.FINALIZED
        ].includes(status)}
      />
      <Step 
        label="Submitting..." 
        active={status === TransactionStatus.SUBMITTED}
        complete={[
          TransactionStatus.IN_BLOCK,
          TransactionStatus.FINALIZED
        ].includes(status)}
      />
      <Step 
        label="Confirming..." 
        active={status === TransactionStatus.IN_BLOCK}
        complete={status === TransactionStatus.FINALIZED}
      />
      <Step 
        label="Complete" 
        active={status === TransactionStatus.FINALIZED}
        complete={status === TransactionStatus.FINALIZED}
      />
      
      {error && <ErrorMessage>{error}</ErrorMessage>}
    </div>
  );
}
```

---

### 2. Error Message Guidelines

**Principle**: Provide clear, actionable error messages.

**Implementation**:
```typescript
class UserFriendlyError extends Error {
  constructor(
    public userMessage: string,
    public technicalDetails?: string,
    public suggestedAction?: string
  ) {
    super(userMessage);
  }
}

function mapContractError(error: any): UserFriendlyError {
  const errorMessage = error.message || String(error);

  const errorMap: Record<string, UserFriendlyError> = {
    'PropertyNotFound': new UserFriendlyError(
      'Property not found',
      errorMessage,
      'Please verify the property ID and try again'
    ),
    'Unauthorized': new UserFriendlyError(
      'Access denied',
      errorMessage,
      'You do not have permission for this action. Please check your account.'
    ),
    'NotCompliant': new UserFriendlyError(
      'Compliance verification required',
      errorMessage,
      'Please complete KYC verification at kyc.propchain.io'
    ),
    'InvalidMetadata': new UserFriendlyError(
      'Invalid property information',
      errorMessage,
      'Please review and correct the property details'
    ),
    'InsufficientBalance': new UserFriendlyError(
      'Insufficient funds',
      errorMessage,
      'Please add more funds to your account for gas fees'
    )
  };

  for (const [key, friendlyError] of Object.entries(errorMap)) {
    if (errorMessage.includes(key)) {
      return friendlyError;
    }
  }

  return new UserFriendlyError(
    'An unexpected error occurred',
    errorMessage,
    'Please try again or contact support if the problem persists'
  );
}

// Usage in UI
try {
  await registerProperty(metadata);
} catch (error) {
  const friendlyError = mapContractError(error);
  
  toast.error(friendlyError.userMessage, {
    description: friendlyError.suggestedAction,
    duration: 5000
  });
  
  // Log technical details for debugging
  console.error('Technical error:', friendlyError.technicalDetails);
}
```

---

## Testing Best Practices

### 1. Mock Blockchain for Testing

**Principle**: Test without depending on live blockchain.

**Implementation**:
```typescript
class MockContract {
  private state: Map<string, any> = new Map();

  async query(method: string, ...args: any[]) {
    const mockMethod = `mock${method.charAt(0).toUpperCase()}${method.slice(1)}`;
    
    if (typeof this[mockMethod] === 'function') {
      return this[mockMethod](...args);
    }

    throw new Error(`No mock for ${method}`);
  }

  async tx(method: string, options: any, ...args: any[]) {
    // Return mock transaction
    return {
      signAndSend: jest.fn().mockResolvedValue({
        hash: '0x' + '1234'.repeat(16),
        status: { isFinalized: true }
      })
    };
  }

  // Mock implementations
  private mockGetProperty(_account: any, _options: any, propertyId: number) {
    const property = this.state.get(`property:${propertyId}`);
    
    return {
      output: {
        isOk: !!property,
        unwrap: () => property,
        toHuman: () => property
      }
    };
  }

  setMockData(key: string, value: any): void {
    this.state.set(key, value);
  }
}

// Usage in tests
describe('PropertyService', () => {
  let mockContract: MockContract;
  let service: PropertyService;

  beforeEach(() => {
    mockContract = new MockContract();
    service = new PropertyService(mockContract as any);

    // Setup mock data
    mockContract.setMockData('property:1', {
      id: 1,
      owner: 'test-account',
      metadata: { location: 'Test St', size: 1000 }
    });
  });

  test('gets property by id', async () => {
    const property = await service.getProperty(1);
    
    expect(property).toBeDefined();
    expect(property.id).toBe(1);
  });
});
```

---

## Monitoring & Operations

### 1. Health Checks

**Principle**: Monitor integration health proactively.

**Implementation**:
```typescript
interface HealthStatus {
  blockchain: {
    connected: boolean;
    latency: number;
    synced: boolean;
  };
  contract: {
    deployed: boolean;
    responsive: boolean;
  };
  wallet: {
    extensionAvailable: boolean;
    accountsAccessible: boolean;
  };
}

class HealthChecker {
  async checkHealth(): Promise<HealthStatus> {
    const [blockchainHealth, contractHealth, walletHealth] = await Promise.all([
      this.checkBlockchainHealth(),
      this.checkContractHealth(),
      this.checkWalletHealth()
    ]);

    return {
      blockchain: blockchainHealth,
      contract: contractHealth,
      wallet: walletHealth
    };
  }

  private async checkBlockchainHealth() {
    const startTime = Date.now();
    
    try {
      const [chain, blockNumber] = await Promise.all([
        api.rpc.system.chain(),
        api.rpc.chain.getBlockNumber()
      ]);

      const latency = Date.now() - startTime;

      return {
        connected: true,
        latency,
        synced: true
      };
    } catch (error) {
      return {
        connected: false,
        latency: -1,
        synced: false
      };
    }
  }

  private async checkContractHealth() {
    try {
      const { output } = await contract.query.ping();
      
      return {
        deployed: true,
        responsive: output?.isOk === true
      };
    } catch (error) {
      return {
        deployed: false,
        responsive: false
      };
    }
  }

  private async checkWalletHealth() {
    try {
      const { web3Enable, web3Accounts } = await import('@polkadot/extension-dapp');
      const extensions = await web3Enable('Health Check');
      
      if (extensions.length === 0) {
        return { extensionAvailable: false, accountsAccessible: false };
      }

      const accounts = await web3Accounts();
      
      return {
        extensionAvailable: true,
        accountsAccessible: accounts.length > 0
      };
    } catch (error) {
      return { extensionAvailable: false, accountsAccessible: false };
    }
  }
}

// Periodic health checks
setInterval(async () => {
  const health = await healthChecker.checkHealth();
  
  if (!health.blockchain.connected) {
    alertAdmins('Blockchain connection lost');
  }
  
  if (!health.contract.responsive) {
    alertAdmins('Contract not responding');
  }
}, 60000); // Check every minute
```

---

## Code Organization

### File Naming Conventions

```typescript
// Contracts
PropertyRegistry.contract.ts
EscrowContract.contract.ts

// Services
property.service.ts
transfer.service.ts
compliance.service.ts

// Repositories
property.repository.ts
event.repository.ts

// Types
property.types.ts
contract.types.ts

// Utilities
formatters.util.ts
validators.util.ts

// Hooks (React)
useWallet.hook.ts
useProperty.hook.ts

// Tests
property.service.test.ts
integration.test.ts
```

### Documentation Standards

```typescript
/**
 * Property Registration Service
 * 
 * Handles property registration workflow including:
 * - Metadata validation
 * - Compliance checking
 * - Contract interaction
 * - Event tracking
 * 
 * @example
 * ```typescript
 * const service = new PropertyRegistrationService(contract);
 * const { propertyId } = await service.register(metadata, signer);
 * ```
 */
class PropertyRegistrationService {
  /**
   * Register a new property
   * 
   * @param metadata - Property metadata following schema
   * @param signer - Account that will own the property
   * @returns Property ID and transaction hash
   * 
   * @throws {ValidationError} If metadata is invalid
   * @throws {ComplianceError} If signer is not compliant
   * @throws {TransactionError} If blockchain transaction fails
   */
  async register(
    metadata: PropertyMetadata,
    signer: InjectedAccount
  ): Promise<{ propertyId: number; hash: string }> {
    // Implementation
  }
}
```

---

## Conclusion

Following these best practices will help you build robust, secure, and performant integrations with PropChain. Remember to:

1. **Start Simple**: Implement basic functionality first, then optimize
2. **Test Thoroughly**: Use mocks and testnets before production
3. **Monitor Continuously**: Set up alerts and health checks
4. **Document Everything**: Help future developers (including yourself)
5. **Stay Updated**: Follow PropChain updates and security advisories

---

**Related Documents**:
- [Complete Integration Guide](./COMPLETE_INTEGRATION_GUIDE.md)
- [Troubleshooting Guide](./INTEGRATION_TROUBLESHOOTING.md)
- [API Reference](./API_GUIDE.md)
- [Security Best Practices](./SECURITY.md)

**Last Updated**: March 27, 2026  
**Version**: 1.0.0  
**Maintained By**: PropChain Development Team
