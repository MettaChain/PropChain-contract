import { useState, useEffect, useCallback } from 'react';
import { PropChainClient } from '../client/PropChainClient';
import type { PropertyInfo } from '../types';

interface UsePropertyResult {
  property: PropertyInfo | null;
  loading: boolean;
  error: Error | null;
  refetch: () => void;
}

export function useProperty(
  client: PropChainClient | null,
  propertyId: number,
): UsePropertyResult {
  const [property, setProperty] = useState<PropertyInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const fetchProperty = useCallback(async () => {
    if (!client) return;

    setLoading(true);
    setError(null);

    try {
      const result = await client.propertyRegistry.getProperty(propertyId);
      setProperty(result);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    } finally {
      setLoading(false);
    }
  }, [client, propertyId]);

  useEffect(() => {
    fetchProperty();
  }, [fetchProperty]);

  return { property, loading, error, refetch: fetchProperty };
}
