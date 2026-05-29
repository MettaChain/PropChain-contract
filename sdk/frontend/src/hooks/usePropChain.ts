import { useState, useEffect, useCallback } from 'react';
import { PropChainClient } from '../client/PropChainClient';
import type { ContractAddresses, ClientOptions } from '../types';

interface UsePropChainResult {
  client: PropChainClient | null;
  isConnected: boolean;
  error: Error | null;
  disconnect: () => Promise<void>;
}

export function usePropChain(
  wsEndpoint: string,
  addresses: ContractAddresses,
  options?: ClientOptions,
): UsePropChainResult {
  const [client, setClient] = useState<PropChainClient | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let mounted = true;
    let activeClient: PropChainClient | null = null;

    const connect = async () => {
      try {
        const c = await PropChainClient.create(wsEndpoint, addresses, options);
        if (mounted) {
          activeClient = c;
          setClient(c);
          setIsConnected(true);
        } else {
          await c.disconnect();
        }
      } catch (err) {
        if (mounted) {
          setError(err instanceof Error ? err : new Error(String(err)));
        }
      }
    };

    connect();

    return () => {
      mounted = false;
      setIsConnected(false);
      if (activeClient) {
        activeClient.disconnect().catch(() => undefined);
      }
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [wsEndpoint]);

  const disconnect = useCallback(async () => {
    if (client) {
      setIsConnected(false);
      await client.disconnect();
      setClient(null);
    }
  }, [client]);

  return { client, isConnected, error, disconnect };
}
