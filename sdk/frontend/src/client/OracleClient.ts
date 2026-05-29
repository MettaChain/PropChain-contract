/**
 * @propchain/sdk — Oracle Client
 *
 * Typed wrapper for the PropChain Property Valuation Oracle contract.
 * Provides methods for querying property valuations, market data,
 * and managing oracle sources.
 *
 * @module client/OracleClient
 */

import type { ApiPromise } from '@polkadot/api';
import { ContractPromise } from '@polkadot/api-contract';
import { Abi } from '@polkadot/api-contract';
import type { KeyringPair } from '@polkadot/keyring/types';

import type {
  PropertyValuation,
  ValuationWithConfidence,
  VolatilityMetrics,
  PropertyType,
  OracleSource,
  TxResult,
  ContractEvent,
  ClientOptions,
  TxProgressCallback,
} from '../types';
import { TxProgressStatus } from '../types';
import { decodeContractError, TransactionError, GasEstimationError } from '../utils/errors';
import { decodeTransactionEvents } from '../utils/events';

export type Signer = KeyringPair | string;

/**
 * Client for interacting with the PropChain Property Valuation Oracle.
 *
 * @example
 * ```typescript
 * const oracle = client.oracle;
 *
 * // Get current valuation
 * const valuation = await oracle.getValuation(propertyId);
 * console.log('Value:', valuation.valuation, 'Confidence:', valuation.confidenceScore);
 *
 * // Get valuation with confidence interval
 * const detailed = await oracle.getValuationWithConfidence(propertyId);
 * console.log('Range:', detailed.confidenceInterval);
 * ```
 */
export class OracleClient {
  private readonly contract: ContractPromise;
  private readonly api: ApiPromise;
  private readonly abi: Abi;
  private readonly contractAddress: string;
  private readonly options: ClientOptions;

  constructor(api: ApiPromise, contractAddress: string, abi: Abi, options?: ClientOptions) {
    this.api = api;
    this.abi = abi;
    this.contractAddress = contractAddress;
    this.options = options ?? {};
    this.contract = new ContractPromise(api, abi, contractAddress);
  }

  // ==========================================================================
  // Valuation Queries
  // ==========================================================================

  /**
   * Gets the current valuation for a property.
   *
   * @param propertyId - Property to get valuation for
   * @returns The property valuation
   */
  async getValuation(propertyId: number): Promise<PropertyValuation> {
    const result = await this.query('get_valuation', [propertyId]);
    return result as unknown as PropertyValuation;
  }

  /**
   * Gets valuation with detailed confidence metrics.
   *
   * @param propertyId - Property to get valuation for
   * @returns Valuation with confidence interval and volatility
   */
  async getValuationWithConfidence(
    propertyId: number,
  ): Promise<ValuationWithConfidence> {
    const result = await this.query('get_valuation_with_confidence', [propertyId]);
    return result as unknown as ValuationWithConfidence;
  }

  /**
   * Gets historical valuations for a property.
   *
   * @param propertyId - Property to get history for
   * @param limit - Maximum number of historical entries
   * @returns Array of historical valuations
   */
  async getHistoricalValuations(
    propertyId: number,
    limit: number = 10,
  ): Promise<PropertyValuation[]> {
    const result = await this.query('get_historical_valuations', [propertyId, limit]);
    return (result as PropertyValuation[]) ?? [];
  }

  /**
   * Gets market volatility for a property type and location.
   *
   * @param propertyType - Type of property
   * @param location - Geographic location
   * @returns Volatility metrics
   */
  async getMarketVolatility(
    propertyType: PropertyType,
    location: string,
  ): Promise<VolatilityMetrics> {
    const result = await this.query('get_market_volatility', [propertyType, location]);
    return result as unknown as VolatilityMetrics;
  }

  // ==========================================================================
  // Valuation Requests
  // ==========================================================================

  /**
   * Requests a new valuation for a property.
   *
   * @param signer - Requesting account
   * @param propertyId - Property to request valuation for
   * @returns Request ID
   */
  async requestValuation(
    signer: Signer,
    propertyId: number,
    onProgress?: TxProgressCallback,
  ): Promise<{ requestId: number } & TxResult> {
    const txResult = await this.submitTx(signer, 'request_valuation', [propertyId], onProgress);
    return { requestId: 0, ...txResult };
  }

  /**
   * Batch requests valuations for multiple properties.
   *
   * @param signer - Requesting account
   * @param propertyIds - Properties to request valuations for
   */
  async batchRequestValuations(
    signer: Signer,
    propertyIds: number[],
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(signer, 'batch_request_valuations', [propertyIds], onProgress);
  }

  // ==========================================================================
  // Oracle Source Management
  // ==========================================================================

  /**
   * Adds an oracle source (admin only).
   */
  async addSource(signer: Signer, source: OracleSource, onProgress?: TxProgressCallback): Promise<TxResult> {
    return this.submitTx(signer, 'add_source', [source], onProgress);
  }

  /**
   * Removes an oracle source (admin only).
   */
  async removeSource(signer: Signer, sourceId: string, onProgress?: TxProgressCallback): Promise<TxResult> {
    return this.submitTx(signer, 'remove_source', [sourceId], onProgress);
  }

  /**
   * Gets the reputation score for an oracle source.
   */
  async getReputation(sourceId: string): Promise<number | null> {
    const result = await this.query('get_reputation', [sourceId]);
    return (result as number) ?? null;
  }

  /**
   * Checks for anomalies in price data.
   */
  async detectAnomalies(propertyId: number, newValuation: bigint): Promise<boolean> {
    const result = await this.query('detect_anomalies', [
      propertyId,
      newValuation.toString(),
    ]);
    return (result as boolean) ?? false;
  }

  // ==========================================================================
  // Internal Helpers
  // ==========================================================================

  private async query(method: string, args: unknown[]): Promise<unknown> {
    const queryFn = this.contract.query[method];
    if (!queryFn) {
      throw new Error(`Unknown query method: ${method}`);
    }

    const dummyAddress = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';
    const { result, output } = await queryFn(dummyAddress, { gasLimit: -1 }, ...args);

    if (result.isErr) {
      const errorVariant = result.asErr?.toString() ?? 'Unknown';
      throw decodeContractError(errorVariant);
    }

    return output ? output.toJSON() : null;
  }

  private async submitTx(
    signer: Signer,
    method: string,
    args: unknown[],
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    const signerAddress = typeof signer === 'string' ? signer : signer.address;

    const queryFn = this.contract.query[method];
    if (!queryFn) {
      throw new Error(`Unknown contract method: ${method}`);
    }

    const { gasRequired, result: dryRunResult } = await queryFn(
      signerAddress,
      { gasLimit: -1 },
      ...args,
    );

    if (dryRunResult.isErr) {
      const errorVariant = dryRunResult.asErr?.toString() ?? 'Unknown';
      const cause = decodeContractError(errorVariant);
      throw new GasEstimationError(method, cause);
    }

    const txFn = this.contract.tx[method];
    if (!txFn) {
      throw new Error(`Unknown tx method: ${method}`);
    }

    // Apply safety buffer to estimated gas
    const gasLimit = await this.applyGasBuffer(BigInt(gasRequired?.toString() ?? '0'));

    return new Promise<TxResult>((resolve, reject) => {
      const tx = txFn({ gasLimit }, ...args);

      tx.signAndSend(
        signer as KeyringPair,
        {},
        ({ status, events: rawEvents, dispatchError }) => {
          if (status.isReady && onProgress) {
            onProgress({ status: TxProgressStatus.Ready, txHash: tx.hash.toString() });
          } else if (status.isBroadcast && onProgress) {
            onProgress({ status: TxProgressStatus.Broadcast, txHash: tx.hash.toString() });
          } else if (status.isInBlock && onProgress) {
            onProgress({
              status: TxProgressStatus.InBlock,
              txHash: tx.hash.toString(),
              blockHash: status.asInBlock.toString()
            });
          }

          if (dispatchError) {
            if (onProgress) {
              onProgress({
                status: TxProgressStatus.Error,
                txHash: tx.hash.toString(),
                message: dispatchError.toString()
              });
            }
            reject(
              new TransactionError(
                `Transaction failed: ${dispatchError.toString()}`,
                undefined,
                dispatchError.toString(),
              ),
            );
            return;
          }

          if (status.isFinalized) {
            const blockHash = status.asFinalized.toString();
            if (onProgress) {
              onProgress({
                status: TxProgressStatus.Finalized,
                txHash: tx.hash.toString(),
                blockHash
              });
            }

            const decodedEvents: ContractEvent[] = decodeTransactionEvents(
              this.abi,
              rawEvents as unknown as Array<{
                event: { data: Uint8Array; section: string; method: string };
              }>,
              this.contractAddress,
            );

            resolve({
              txHash: tx.hash.toString(),
              blockHash,
              blockNumber: 0,
              events: decodedEvents,
              success: true,
            });
          }
        },
      ).catch(reject);
    });
  }

  /**
   * Applies a safety buffer to the estimated gas required for a transaction.
   */
  private async applyGasBuffer(estimatedGas: bigint): Promise<bigint> {
    const bufferPercentage = this.options.gasBufferPercentage ?? 10;
    const buffer = (estimatedGas * BigInt(bufferPercentage)) / 100n;
    return estimatedGas + buffer;
  }
}
