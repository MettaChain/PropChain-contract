/**
 * @propchain/sdk — Connection Utilities
 *
 * Helpers for connecting to Substrate/Polkadot nodes with retry logic,
 * preconfigured network presets, and lifecycle management.
 *
 * @module utils/connection
 */

import { ApiPromise, WsProvider } from '@polkadot/api';

import type { NetworkConfig } from '../types';

// ============================================================================
// Network Presets
// ============================================================================

/**
 * Pre-configured network endpoints for common deployments.
 */
export const NETWORKS: Record<string, NetworkConfig> = {
  local: {
    name: 'Local Development',
    wsEndpoint: 'ws://127.0.0.1:9944',
    isTestnet: true,
  },
  westend: {
    name: 'Westend Testnet',
    wsEndpoint: 'wss://westend-rpc.polkadot.io',
    explorerUrl: 'https://westend.subscan.io',
    isTestnet: true,
  },
  polkadot: {
    name: 'Polkadot Mainnet',
    wsEndpoint: 'wss://rpc.polkadot.io',
    explorerUrl: 'https://polkadot.subscan.io',
    isTestnet: false,
  },
  kusama: {
    name: 'Kusama',
    wsEndpoint: 'wss://kusama-rpc.polkadot.io',
    explorerUrl: 'https://kusama.subscan.io',
    isTestnet: false,
  },
};

// ============================================================================
// Connection Functions
// ============================================================================

/**
 * Creates an `ApiPromise` connected to the given WebSocket endpoint.
 *
 * @param wsEndpoint - WebSocket URL of the Substrate node
 * @param types - Optional custom type definitions
 * @returns A connected `ApiPromise`
 *
 * @example
 * ```typescript
 * const api = await createApi('ws://localhost:9944');
 * const chain = await api.rpc.system.chain();
 * console.log(`Connected to ${chain}`);
 * ```
 */
export async function createApi(
  wsEndpoint: string,
  types?: Record<string, unknown>,
): Promise<ApiPromise> {
  const wsProvider = new WsProvider(wsEndpoint);

  const api = await ApiPromise.create({
    provider: wsProvider,
    types: (types ?? {}) as Record<string, string>,
  });

  await api.isReady;
  return api;
}

/**
 * Creates an API connection with exponential backoff retry logic.
 *
 * @param wsEndpoint - WebSocket URL of the Substrate node
 * @param maxRetries - Maximum number of reconnection attempts (default: 5)
 * @param baseDelayMs - Base delay between retries in ms (default: 1000)
 * @param types - Optional custom type definitions
 * @returns A connected `ApiPromise`
 * @throws Error after exhausting all retries
 *
 * @example
 * ```typescript
 * const api = await connectWithRetry('ws://localhost:9944', 3);
 * ```
 */
export async function connectWithRetry(
  wsEndpoint: string,
  maxRetries: number = 5,
  baseDelayMs: number = 1000,
  types?: Record<string, unknown>,
): Promise<ApiPromise> {
  let lastError: Error | undefined;

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      const api = await createApi(wsEndpoint, types);
      return api;
    } catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error));

      if (attempt < maxRetries) {
        const delay = baseDelayMs * Math.pow(2, attempt);
        await sleep(delay);
      }
    }
  }

  throw new Error(
    `Failed to connect to ${wsEndpoint} after ${maxRetries + 1} attempts: ${lastError?.message}`,
  );
}

/**
 * Creates an API connection using a predefined network preset.
 *
 * @param networkName - Name of the network preset (e.g. 'local', 'westend')
 * @returns A connected `ApiPromise`
 * @throws Error if the network name is not found
 *
 * @example
 * ```typescript
 * const api = await connectToNetwork('westend');
 * ```
 */
export async function connectToNetwork(networkName: string): Promise<ApiPromise> {
  const config = NETWORKS[networkName];
  if (!config) {
    const available = Object.keys(NETWORKS).join(', ');
    throw new Error(
      `Unknown network "${networkName}". Available networks: ${available}`,
    );
  }
  return createApi(config.wsEndpoint);
}

/**
 * Executes a Promise-returning operation with exponential backoff retry logic.
 *
 * @param operation - The function to execute
 * @param maxRetries - Maximum number of retries (default: 3)
 * @param baseDelayMs - Base delay between retries in ms (default: 1000)
 * @returns The result of the operation
 * @throws The last error encountered after exhausting all retries
 */
export async function withExponentialBackoff<T>(
  operation: () => Promise<T>,
  maxRetries: number = 3,
  baseDelayMs: number = 1000,
): Promise<T> {
  let attempt = 0;
  let lastError: Error | undefined;

  while (attempt <= maxRetries) {
    try {
      return await operation();
    } catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error));
      
      // Do not retry if we've reached maxRetries
      if (attempt === maxRetries) {
        break;
      }
      
      const delay = baseDelayMs * Math.pow(2, attempt);
      await sleep(delay);
      attempt++;
    }
  }

  throw lastError;
}

/**
 * Gets the network configuration for a preset name.
 *
 * @param networkName - Name of the network preset
 * @returns The NetworkConfig or undefined if not found
 */
export function getNetworkConfig(networkName: string): NetworkConfig | undefined {
  return NETWORKS[networkName];
}

// ============================================================================
// Internal Helpers
// ============================================================================

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
