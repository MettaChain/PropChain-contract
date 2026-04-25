import { useState, useEffect, useCallback } from 'react';
import { PropChainClient } from '../client/PropChainClient';
import type { PortfolioSummary, PortfolioDetails } from '../types';

interface UsePortfolioResult {
  summary: PortfolioSummary | null;
  details: PortfolioDetails | null;
  loading: boolean;
  error: Error | null;
  refetch: () => void;
}

export function usePortfolio(
  client: PropChainClient | null,
  owner: string,
): UsePortfolioResult {
  const [summary, setSummary] = useState<PortfolioSummary | null>(null);
  const [details, setDetails] = useState<PortfolioDetails | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const fetchPortfolio = useCallback(async () => {
    if (!client || !owner) return;

    setLoading(true);
    setError(null);

    try {
      const [s, d] = await Promise.all([
        client.propertyRegistry.getPortfolioSummary(owner),
        client.propertyRegistry.getPortfolioDetails(owner),
      ]);
      setSummary(s);
      setDetails(d);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    } finally {
      setLoading(false);
    }
  }, [client, owner]);

  useEffect(() => {
    fetchPortfolio();
  }, [fetchPortfolio]);

  return { summary, details, loading, error, refetch: fetchPortfolio };
}
