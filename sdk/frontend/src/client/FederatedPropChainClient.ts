import type { ApiPromise } from '@polkadot/api';

import type { ClientOptions, ContractAddresses } from '../types';
import { createApi, connectWithRetry } from '../utils/connection';
import { ConnectionError } from '../utils/errors';
import type { BuiltInContractClients, BuiltInContractModuleId } from '../modules/builtin';
import { loadContractModule } from '../modules/loader';

export class FederatedPropChainClient {
  private _api: ApiPromise;
  private readonly _addresses: ContractAddresses;
  private readonly _options: ClientOptions;
  private _connected: boolean = true;
  private readonly _clientCache = new Map<BuiltInContractModuleId, Promise<unknown>>();

  private constructor(api: ApiPromise, addresses: ContractAddresses, options?: ClientOptions) {
    this._api = api;
    this._addresses = addresses;
    this._options = options ?? {};
  }

  static async create(
    wsEndpoint: string,
    addresses: ContractAddresses,
    options?: ClientOptions,
  ): Promise<FederatedPropChainClient> {
    try {
      const api = options?.autoReconnect !== false
        ? await connectWithRetry(
            wsEndpoint,
            options?.maxReconnectAttempts ?? 5,
            1000,
            options?.types as Record<string, unknown> | undefined,
          )
        : await createApi(wsEndpoint, options?.types as Record<string, unknown> | undefined);

      const client = new FederatedPropChainClient(api, addresses, options);

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

  static fromApi(api: ApiPromise, addresses: ContractAddresses, options?: ClientOptions): FederatedPropChainClient {
    return new FederatedPropChainClient(api, addresses, options);
  }

  get api(): ApiPromise {
    return this._api;
  }

  get isConnected(): boolean {
    return this._connected && this._api.isConnected;
  }

  get addresses(): ContractAddresses {
    return { ...this._addresses };
  }

  async disconnect(): Promise<void> {
    this._connected = false;
    await this._api.disconnect();
  }

  async contract<T extends BuiltInContractModuleId>(id: T): Promise<BuiltInContractClients[T]> {
    const existing = this._clientCache.get(id);
    if (existing) {
      return existing as Promise<BuiltInContractClients[T]>;
    }

    const createPromise = (async () => {
      const address = this.getAddressForModule(id);
      const module = await loadContractModule<BuiltInContractClients[T]>(id);
      return module.createClient({ api: this._api, address, options: this._options });
    })();

    this._clientCache.set(id, createPromise);
    return createPromise as Promise<BuiltInContractClients[T]>;
  }

  async propertyRegistry(): Promise<BuiltInContractClients['propertyRegistry']> {
    return this.contract('propertyRegistry');
  }

  async propertyToken(): Promise<BuiltInContractClients['propertyToken']> {
    return this.contract('propertyToken');
  }

  async oracle(): Promise<BuiltInContractClients['oracle']> {
    return this.contract('oracle');
  }

  private getAddressForModule(id: BuiltInContractModuleId): string {
    const address = (() => {
      switch (id) {
        case 'propertyRegistry':
          return this._addresses.propertyRegistry;
        case 'propertyToken':
          return this._addresses.propertyToken;
        case 'oracle':
          return this._addresses.oracle;
      }
    })();

    if (!address) {
      throw new Error(
        `Contract address not provided for "${id}". Pass it in ContractAddresses when creating the client.`,
      );
    }
    return address;
  }
}

