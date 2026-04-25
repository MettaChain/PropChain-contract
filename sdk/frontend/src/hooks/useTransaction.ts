import { useState, useCallback } from 'react';
import type { TxResult, TxProgressCallback, TxStatusUpdate } from '../types';
import { TxProgressStatus } from '../types';

interface UseTransactionResult {
  execute: (txFn: (onProgress: TxProgressCallback) => Promise<TxResult>) => Promise<TxResult>;
  status: TxProgressStatus | null;
  txHash: string | null;
  blockHash: string | null;
  isLoading: boolean;
  error: Error | null;
  reset: () => void;
}

export function useTransaction(): UseTransactionResult {
  const [status, setStatus] = useState<TxProgressStatus | null>(null);
  const [txHash, setTxHash] = useState<string | null>(null);
  const [blockHash, setBlockHash] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const reset = useCallback(() => {
    setStatus(null);
    setTxHash(null);
    setBlockHash(null);
    setIsLoading(false);
    setError(null);
  }, []);

  const execute = useCallback(
    async (txFn: (onProgress: TxProgressCallback) => Promise<TxResult>): Promise<TxResult> => {
      setIsLoading(true);
      setError(null);
      setStatus(null);
      setTxHash(null);
      setBlockHash(null);

      const onProgress: TxProgressCallback = (update: TxStatusUpdate) => {
        setStatus(update.status);
        if (update.txHash) setTxHash(update.txHash);
        if (update.blockHash) setBlockHash(update.blockHash);
      };

      try {
        const result = await txFn(onProgress);
        setIsLoading(false);
        return result;
      } catch (err) {
        const e = err instanceof Error ? err : new Error(String(err));
        setError(e);
        setStatus(TxProgressStatus.Error);
        setIsLoading(false);
        throw e;
      }
    },
    [],
  );

  return { execute, status, txHash, blockHash, isLoading, error, reset };
}
