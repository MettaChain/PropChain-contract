/**
 * @propchain/sdk — PropChainClient
 *
 * Main entry point for the PropChain SDK. Manages the blockchain connection
 * and provides access to all contract-specific sub-clients.
 *
 * @module client/PropChainClient
 */

import type { ApiPromise } from '@polkadot/api';
import { Abi } from '@polkadot/api-contract';

import type { ClientOptions, ContractAddresses } from '../types';
import { createApi, connectWithRetry } from '../utils/connection';
import { ConnectionError } from '../utils/errors';
import { PropertyRegistryClient } from './PropertyRegistryClient';
import { PropertyTokenClient } from './PropertyTokenClient';
import { EscrowClient } from './EscrowClient';
import { OracleClient } from './OracleClient';

// Import placeholder ABIs
import propertyRegistryAbiJson from '../abi/property_registry.json';
import propertyTokenAbiJson from '../abi/property_token.json';

// ============================================================================
// PropChainClient
// ============================================================================

/**
 * Main entry point for PropChain SDK.
 *
 * Creates and manages the connection to a Substrate node and provides
 * typed sub-clients for each PropChain contract.
 *
 * @example
 * ```typescript
 * import { PropChainClient } from '@propchain/sdk';
 *
 * // Connect to a local node
 * const client = await PropChainClient.create('ws://localhost:9944', {
 *   propertyRegistry: '5Grwva...',
 *   propertyToken: '5FHnea...',
 * });
 *
 * // Use typed sub-clients
 * const health = await client.propertyRegistry.healthCheck();
 * const balance = await client.propertyToken.balanceOf(myAddress);
 *
 * // Disconnect when done
 * await client.disconnect();
 * ```
 */
export class PropChainClient {
  private _api: ApiPromise;
  private _propertyRegistry: PropertyRegistryClient | null = null;
  private _propertyToken: PropertyTokenClient | null = null;
  private _escrow: EscrowClient | null = null;
  private _oracle: OracleClient | null = null;
  private readonly _addresses: ContractAddresses;
  private readonly _options: ClientOptions;
  private _connected: boolean = true;

  private constructor(api: ApiPromise, addresses: ContractAddresses, options?: ClientOptions) {
    this._api = api;
    this._addresses = addresses;
    this._options = options ?? {};
  }

  // ==========================================================================
  // Factory Methods
  // ==========================================================================

  /**
   * Creates a new PropChainClient connected to a Substrate node.
   *
   * @param wsEndpoint - WebSocket URL of the node
   * @param addresses - Contract addresses for this deployment
   * @param options - Connection options
   * @returns A connected PropChainClient
   *
   * @example
   * ```typescript
   * const client = await PropChainClient.create(
   *   'ws://localhost:9944',
   *   { propertyRegistry: '5Grw...' },
   * );
   * ```
   */
  static async create(
    wsEndpoint: string,
    addresses: ContractAddresses,
    options?: ClientOptions,
  ): Promise<PropChainClient> {
    try {
      const api = options?.autoReconnect !== false
        ? await connectWithRetry(
            wsEndpoint,
            options?.maxReconnectAttempts ?? 5,
            1000,
            options?.types as Record<string, unknown> | undefined,
          )
        : await createApi(wsEndpoint, options?.types as Record<string, unknown> | undefined);

      const client = new PropChainClient(api, addresses, options);

      // Set up disconnect handler
      api.on('disconnected', () => {
        client._connected = false;
      });

      api.on('connected', () => {
        client._connected = true;
      });

      return client;
    } catch (error) {
      throw new ConnectionError(
        wsEndpoint,
        options?.maxReconnectAttempts ?? 5,
        error instanceof Error ? error : undefined,
      );
    }
  }

  /**
   * Creates a PropChainClient from an existing ApiPromise instance.
   *
   * @param api - An already-connected ApiPromise
   * @param addresses - Contract addresses
   * @returns A PropChainClient wrapping the existing API
   */
  static fromApi(api: ApiPromise, addresses: ContractAddresses): PropChainClient {
    return new PropChainClient(api, addresses);
  }

  // ==========================================================================
  // Sub-Clients
  // ==========================================================================

  /**
   * Gets the PropertyRegistry sub-client.
   *
   * @throws Error if no PropertyRegistry address was provided
   */
  get propertyRegistry(): PropertyRegistryClient {
    if (!this._propertyRegistry) {
      if (!this._addresses.propertyRegistry) {
        throw new Error(
          'PropertyRegistry contract address not provided. Pass it in ContractAddresses when creating the client.',
        );
      }
      const abi = new Abi(propertyRegistryAbiJson);
      this._propertyRegistry = new PropertyRegistryClient(
        this._api,
        this._addresses.propertyRegistry,
        abi,
        this._options,
      );
    }
    return this._propertyRegistry;
  }

  /**
   * Gets the PropertyToken sub-client.
   *
   * @throws Error if no PropertyToken address was provided
   */
  get propertyToken(): PropertyTokenClient {
    if (!this._propertyToken) {
      if (!this._addresses.propertyToken) {
        throw new Error(
          'PropertyToken contract address not provided. Pass it in ContractAddresses when creating the client.',
        );
      }
      const abi = new Abi(propertyTokenAbiJson);
      this._propertyToken = new PropertyTokenClient(
        this._api,
        this._addresses.propertyToken,
        abi,
        this._options,
      );
    }
    return this._propertyToken;
  }

  /**
   * Gets the Escrow sub-client.
   *
   * Uses the PropertyRegistry contract for escrow operations.
   *
   * @throws Error if no PropertyRegistry address was provided
   */
  get escrow(): EscrowClient {
    if (!this._escrow) {
      this._escrow = new EscrowClient(this.propertyRegistry);
    }
    return this._escrow;
  }

  /**
   * Gets the Oracle sub-client.
   *
   * @throws Error if no Oracle address was provided
   */
  get oracle(): OracleClient {
    if (!this._oracle) {
      if (!this._addresses.oracle) {
        throw new Error(
          'Oracle contract address not provided. Pass it in ContractAddresses when creating the client.',
        );
      }
      // Oracle uses a similar ABI structure; in production, load its own ABI
      const abi = new Abi(propertyRegistryAbiJson);
      this._oracle = new OracleClient(
        this._api,
        this._addresses.oracle,
        abi,
        this._options,
      );
    }
    return this._oracle;
  }

  // ==========================================================================
  // Connection Management
  // ==========================================================================

  /**
   * Gets the underlying Polkadot.js API instance.
   */
  get api(): ApiPromise {
    return this._api;
  }

  /**
   * Returns whether the client is currently connected.
   */
  get isConnected(): boolean {
    return this._connected && this._api.isConnected;
  }

  /**
   * Gets the configured contract addresses.
   */
  get addresses(): ContractAddresses {
    return { ...this._addresses };
  }

  /**
   * Disconnects from the blockchain node and cleans up resources.
   */
  async disconnect(): Promise<void> {
    this._connected = false;
    await this._api.disconnect();
  }

  // ==========================================================================
  // Chain Queries
  // ==========================================================================

  /**
   * Gets the chain name.
   */
  async getChainName(): Promise<string> {
    const chain = await this._api.rpc.system.chain();
    return chain.toString();
  }

  /**
   * Gets the current block number.
   */
  async getBlockNumber(): Promise<number> {
    const header = await this._api.rpc.chain.getHeader();
    return header.number.toNumber();
  }

  /**
   * Gets the chain's genesis hash.
   */
  getGenesisHash(): string {
    return this._api.genesisHash.toString();
  }

  /**
   * Gets the chain's runtime version.
   */
  getRuntimeVersion(): { specName: string; specVersion: number } {
    return {
      specName: this._api.runtimeVersion.specName.toString(),
      specVersion: this._api.runtimeVersion.specVersion.toNumber(),
    };
  }
}
