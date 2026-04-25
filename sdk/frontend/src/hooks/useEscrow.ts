import { useState, useEffect, useCallback } from 'react';
import { PropChainClient } from '../client/PropChainClient';
import type { EscrowInfo } from '../types';

interface UseEscrowResult {
  escrow: EscrowInfo | null;
  loading: boolean;
  error: Error | null;
  refetch: () => void;
}

export function useEscrow(
  client: PropChainClient | null,
  escrowId: number,
): UseEscrowResult {
  const [escrow, setEscrow] = useState<EscrowInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const fetchEscrow = useCallback(async () => {
    if (!client) return;

    setLoading(true);
    setError(null);

    try {
      const result = await client.propertyRegistry.getEscrow(escrowId);
      setEscrow(result);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    } finally {
      setLoading(false);
    }
  }, [client, escrowId]);

  useEffect(() => {
    fetchEscrow();
  }, [fetchEscrow]);

  return { escrow, loading, error, refetch: fetchEscrow };
}
