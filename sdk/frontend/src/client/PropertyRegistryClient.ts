/**
 * @propchain/sdk — PropertyRegistry Client
 *
 * Typed wrapper for the PropChain PropertyRegistry smart contract.
 * Provides ergonomic, strongly-typed methods for all on-chain operations
 * including property management, escrow, badges, pause control, and batch ops.
 *
 * @module client/PropertyRegistryClient
 */

import type { ApiPromise } from '@polkadot/api';
import { ContractPromise } from '@polkadot/api-contract';
import { Abi } from '@polkadot/api-contract';
import type { KeyringPair } from '@polkadot/keyring/types';

import type {
  PropertyMetadata,
  PropertyInfo,
  EscrowInfo,
  HealthStatus,
  GlobalAnalytics,
  GasMetrics,
  Badge,
  BadgeType,
  VerificationRequest,
  Appeal,
  PauseInfo,
  TxResult,
  GasEstimation,
  ContractEvent,
  Subscription,
  BatchResult,
  BatchConfig,
  BatchOperationStats,
  PortfolioSummary,
  PortfolioDetails,
  FractionalInfo,
  FeeOperation,
  ClientOptions,
  TxProgressCallback,
  TxProgressStatus,
} from '../types';
import { PropChainError, TransactionError, decodeContractError, GasEstimationError } from '../utils/errors';
import { decodeTransactionEvents, subscribeToNamedEvent } from '../utils/events';
import type { PropChainEventName, PropChainEventMap } from '../types/events';

/**
 * Signer type: either a KeyringPair or an address string (with external signer).
 */
export type Signer = KeyringPair | string;

// ============================================================================
// PropertyRegistryClient
// ============================================================================

/**
 * Client for interacting with the PropChain PropertyRegistry contract.
 *
 * Provides high-level, typed methods for all contract operations,
 * abstracting away gas estimation, result decoding, and event handling.
 *
 * @example
 * ```typescript
 * import { PropChainClient } from '@propchain/sdk';
 *
 * const client = await PropChainClient.create('ws://localhost:9944', {
 *   propertyRegistry: '5Grw...',
 * });
 *
 * // Register a property
 * const result = await client.propertyRegistry.registerProperty(signer, {
 *   location: '123 Main St',
 *   size: 2000,
 *   legalDescription: 'Lot 1 Block 2',
 *   valuation: BigInt(500000_00000000),
 *   documentsUrl: 'ipfs://Qm...',
 * });
 *
 * console.log('Property ID:', result.propertyId);
 * ```
 */
export class PropertyRegistryClient {
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
  // Property Registration & Query
  // ==========================================================================

  /**
   * Registers a new property on-chain.
   *
   * @param signer - Account signing the transaction
   * @param metadata - Property metadata
   * @returns The new property ID and transaction result
   */
  async registerProperty(
    signer: Signer,
    metadata: PropertyMetadata,
    onProgress?: TxProgressCallback,
  ): Promise<{ propertyId: number } & TxResult> {
    const encodedMetadata = this.encodePropertyMetadata(metadata);
    const txResult = await this.submitTx(
      signer,
      'register_property',
      [encodedMetadata],
      onProgress,
    );

    // Extract property ID from events
    const regEvents = txResult.events.filter((e) => e.name === 'PropertyRegistered');
    const propertyId = regEvents.length > 0
      ? (regEvents[0].args.propertyId as number)
      : 0;

    return { propertyId, ...txResult };
  }

  /**
   * Queries a property by its ID.
   *
   * @param propertyId - The property ID to look up
   * @returns The property information, or `null` if not found
   */
  async getProperty(propertyId: number): Promise<PropertyInfo | null> {
    const result = await this.query('get_property', [propertyId]);
    if (!result) return null;
    return this.decodePropertyInfo(result);
  }

  /**
   * Gets all property IDs owned by an account.
   *
   * @param owner - Owner account address
   * @returns Array of property IDs
   */
  async getOwnerProperties(owner: string): Promise<number[]> {
    const result = await this.query('get_owner_properties', [owner]);
    return (result as number[]) ?? [];
  }

  /**
   * Gets the total number of registered properties.
   */
  async getPropertyCount(): Promise<number> {
    const result = await this.query('property_count', []);
    return (result as number) ?? 0;
  }

  // ==========================================================================
  // Property Transfers & Approvals
  // ==========================================================================

  /**
   * Transfers property ownership to a new account.
   *
   * @param signer - Current owner or approved account
   * @param propertyId - Property to transfer
   * @param to - New owner address
   */
  async transferProperty(
    signer: Signer,
    propertyId: number,
    to: string,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(signer, 'transfer_property', [propertyId, to], onProgress);
  }

  /**
   * Updates the metadata of a registered property.
   *
   * @param signer - Property owner
   * @param propertyId - Property to update
   * @param metadata - New metadata
   */
  async updateMetadata(
    signer: Signer,
    propertyId: number,
    metadata: PropertyMetadata,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    const encoded = this.encodePropertyMetadata(metadata);
    return this.submitTx(signer, 'update_metadata', [propertyId, encoded], onProgress);
  }

  /**
   * Approves an account to transfer a specific property.
   *
   * @param signer - Property owner
   * @param propertyId - Property to approve
   * @param to - Account to approve (or null to clear)
   */
  async approve(
    signer: Signer,
    propertyId: number,
    to: string | null,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(signer, 'approve', [propertyId, to], onProgress);
  }

  /**
   * Gets the approved account for a property.
   *
   * @param propertyId - Property ID
   * @returns Approved account address, or `null`
   */
  async getApproved(propertyId: number): Promise<string | null> {
    const result = await this.query('get_approved', [propertyId]);
    return (result as string) ?? null;
  }

  // ==========================================================================
  // Escrow Operations
  // ==========================================================================

  /**
   * Creates a new escrow for a property transfer.
   *
   * @param signer - Buyer account
   * @param propertyId - Property being escrowed
   * @param buyer - Buyer address
   * @param seller - Seller address
   * @param amount - Escrow amount
   */
  async createEscrow(
    signer: Signer,
    propertyId: number,
    buyer: string,
    seller: string,
    amount: bigint,
    onProgress?: TxProgressCallback,
  ): Promise<{ escrowId: number } & TxResult> {
    const txResult = await this.submitTx(
      signer,
      'create_escrow',
      [propertyId, buyer, seller, amount.toString()],
      onProgress,
    );

    const escrowEvents = txResult.events.filter((e) => e.name === 'EscrowCreated');
    const escrowId = escrowEvents.length > 0
      ? (escrowEvents[0].args.escrowId as number)
      : 0;

    return { escrowId, ...txResult };
  }

  /**
   * Releases an escrow, completing the property transfer.
   *
   * @param signer - Authorized account (seller or admin)
   * @param escrowId - Escrow to release
   */
  async releaseEscrow(
    signer: Signer,
    escrowId: number,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(signer, 'release_escrow', [escrowId], onProgress);
  }

  /**
   * Refunds an escrow, returning funds to the buyer.
   *
   * @param signer - Authorized account
   * @param escrowId - Escrow to refund
   */
  async refundEscrow(
    signer: Signer,
    escrowId: number,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(signer, 'refund_escrow', [escrowId], onProgress);
  }

  /**
   * Gets escrow information by ID.
   *
   * @param escrowId - Escrow ID
   * @returns Escrow info or `null`
   */
  async getEscrow(escrowId: number): Promise<EscrowInfo | null> {
    const result = await this.query('get_escrow', [escrowId]);
    return result ? (result as unknown as EscrowInfo) : null;
  }

  // ==========================================================================
  // Health & Analytics
  // ==========================================================================

  /**
   * Gets the full health status of the contract.
   */
  async healthCheck(): Promise<HealthStatus> {
    const result = await this.query('health_check', []);
    return result as unknown as HealthStatus;
  }

  /**
   * Simple liveness check.
   */
  async ping(): Promise<boolean> {
    const result = await this.query('ping', []);
    return result as boolean;
  }

  /**
   * Checks if all critical dependencies are configured.
   */
  async dependenciesHealthy(): Promise<boolean> {
    const result = await this.query('dependencies_healthy', []);
    return result as boolean;
  }

  /**
   * Gets the contract version.
   */
  async getVersion(): Promise<number> {
    const result = await this.query('version', []);
    return result as number;
  }

  /**
   * Gets the admin account address.
   */
  async getAdmin(): Promise<string> {
    const result = await this.query('admin', []);
    return result as string;
  }

  /**
   * Gets global analytics data.
   */
  async getGlobalAnalytics(): Promise<GlobalAnalytics> {
    const result = await this.query('get_global_analytics', []);
    return result as unknown as GlobalAnalytics;
  }

  /**
   * Gets gas usage metrics.
   */
  async getGasMetrics(): Promise<GasMetrics> {
    const result = await this.query('get_gas_metrics', []);
    return result as unknown as GasMetrics;
  }

  /**
   * Gets the portfolio summary for an owner.
   */
  async getPortfolioSummary(owner: string): Promise<PortfolioSummary> {
    const result = await this.query('get_portfolio_summary', [owner]);
    return result as unknown as PortfolioSummary;
  }

  /**
   * Gets detailed portfolio information for an owner.
   */
  async getPortfolioDetails(owner: string): Promise<PortfolioDetails> {
    const result = await this.query('get_portfolio_details', [owner]);
    return result as unknown as PortfolioDetails;
  }

  // ==========================================================================
  // Badge Operations
  // ==========================================================================

  /**
   * Issues a verification badge to a property.
   */
  async issueBadge(
    signer: Signer,
    propertyId: number,
    badgeType: BadgeType,
    expiresAt: number | null,
    metadataUrl: string,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(
      signer,
      'issue_badge',
      [propertyId, badgeType, expiresAt, metadataUrl],
      onProgress,
    );
  }

  /**
   * Revokes a badge from a property.
   */
  async revokeBadge(
    signer: Signer,
    propertyId: number,
    badgeType: BadgeType,
    reason: string,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(signer, 'revoke_badge', [propertyId, badgeType, reason], onProgress);
  }

  /**
   * Gets badge information for a property.
   */
  async getBadge(propertyId: number, badgeType: BadgeType): Promise<Badge | null> {
    const result = await this.query('get_badge', [propertyId, badgeType]);
    return result ? (result as unknown as Badge) : null;
  }

  /**
   * Requests verification for a badge.
   */
  async requestVerification(
    signer: Signer,
    propertyId: number,
    badgeType: BadgeType,
    evidenceUrl: string,
    onProgress?: TxProgressCallback,
  ): Promise<{ requestId: number } & TxResult> {
    const txResult = await this.submitTx(
      signer,
      'request_verification',
      [propertyId, badgeType, evidenceUrl],
      onProgress,
    );
    const events = txResult.events.filter((e) => e.name === 'VerificationRequested');
    const requestId = events.length > 0 ? (events[0].args.requestId as number) : 0;
    return { requestId, ...txResult };
  }

  // ==========================================================================
  // Pause Control
  // ==========================================================================

  /**
   * Pauses the contract (admin/guardian only).
   */
  async pauseContract(
    signer: Signer,
    reason: string,
    autoResumeAt: number | null,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(signer, 'pause_contract', [reason, autoResumeAt], onProgress);
  }

  /**
   * Requests resuming the contract.
   */
  async requestResume(signer: Signer, onProgress?: TxProgressCallback): Promise<TxResult> {
    return this.submitTx(signer, 'request_resume', [], onProgress);
  }

  /**
   * Approves a resume request.
   */
  async approveResume(signer: Signer, onProgress?: TxProgressCallback): Promise<TxResult> {
    return this.submitTx(signer, 'approve_resume', [], onProgress);
  }

  /**
   * Gets the current pause state.
   */
  async getPauseInfo(): Promise<PauseInfo> {
    const result = await this.query('get_pause_info', []);
    return result as unknown as PauseInfo;
  }

  // ==========================================================================
  // Batch Operations
  // ==========================================================================

  /**
   * Registers multiple properties in a single transaction.
   */
  async batchRegisterProperties(
    signer: Signer,
    metadataList: PropertyMetadata[],
    onProgress?: TxProgressCallback,
  ): Promise<{ batchResult: BatchResult } & TxResult> {
    const encoded = metadataList.map((m) => this.encodePropertyMetadata(m));
    const txResult = await this.submitTx(signer, 'batch_register_properties', [encoded], onProgress);
    return { batchResult: {} as BatchResult, ...txResult };
  }

  /**
   * Transfers multiple properties to the same recipient.
   */
  async batchTransferProperties(
    signer: Signer,
    propertyIds: number[],
    to: string,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(signer, 'batch_transfer_properties', [propertyIds, to], onProgress);
  }

  /**
   * Gets batch operation configuration.
   */
  async getBatchConfig(): Promise<BatchConfig> {
    const result = await this.query('get_batch_config', []);
    return result as unknown as BatchConfig;
  }

  /**
   * Gets batch operation statistics.
   */
  async getBatchStats(): Promise<BatchOperationStats> {
    const result = await this.query('get_batch_operation_stats', []);
    return result as unknown as BatchOperationStats;
  }

  // ==========================================================================
  // Fee Operations
  // ==========================================================================

  /**
   * Gets the dynamic fee for an operation.
   */
  async getDynamicFee(operation: FeeOperation): Promise<bigint> {
    const result = await this.query('get_dynamic_fee', [operation]);
    return BigInt((result as string) ?? '0');
  }

  // ==========================================================================
  // Admin Operations
  // ==========================================================================

  /**
   * Changes the admin account.
   */
  async changeAdmin(signer: Signer, newAdmin: string, onProgress?: TxProgressCallback): Promise<TxResult> {
    return this.submitTx(signer, 'change_admin', [newAdmin], onProgress);
  }

  /**
   * Sets the oracle contract address.
   */
  async setOracle(signer: Signer, oracleAddress: string, onProgress?: TxProgressCallback): Promise<TxResult> {
    return this.submitTx(signer, 'set_oracle', [oracleAddress], onProgress);
  }

  /**
   * Sets the fee manager contract address.
   */
  async setFeeManager(signer: Signer, feeManager: string | null, onProgress?: TxProgressCallback): Promise<TxResult> {
    return this.submitTx(signer, 'set_fee_manager', [feeManager], onProgress);
  }

  // ==========================================================================
  // Event Subscriptions
  // ==========================================================================

  /**
   * Subscribes to a specific contract event.
   *
   * @typeParam E - Event name from the PropChainEventMap
   * @param eventName - The event to listen for
   * @param callback - Called with the typed event payload
   * @returns A subscription handle
   */
  async on<E extends PropChainEventName>(
    eventName: E,
    callback: (event: PropChainEventMap[E]) => void,
  ): Promise<Subscription> {
    return subscribeToNamedEvent(
      this.api,
      this.contractAddress,
      this.abi,
      eventName,
      callback,
    );
  }

  // ==========================================================================
  // Gas Estimation
  // ==========================================================================

  /**
   * Estimates gas for a contract call.
   *
   * @param callerAddress - The caller's address
   * @param method - Contract method name
   * @param args - Method arguments
   * @returns Gas and storage deposit estimation
   */
  async estimateGas(
    callerAddress: string,
    method: string,
    args: unknown[],
  ): Promise<GasEstimation> {
    const message = this.contract.query[method];
    if (!message) {
      throw new Error(`Unknown contract method: ${method}`);
    }

    const result = await message(callerAddress, { gasLimit: -1 }, ...args);

    return {
      gasRequired: BigInt(result.gasRequired?.toString() ?? '0'),
      storageDeposit: BigInt(result.storageDeposit?.toString() ?? '0'),
    };
  }

  // ==========================================================================
  // Internal Helpers
  // ==========================================================================

  /**
   * Performs a read-only query against the contract.
   */
  private async query(method: string, args: unknown[]): Promise<unknown> {
    const queryFn = this.contract.query[method];
    if (!queryFn) {
      throw new Error(`Unknown query method: ${method}`);
    }

    // Use a dummy address for read-only queries
    const dummyAddress = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';
    const { result, output } = await queryFn(dummyAddress, { gasLimit: -1 }, ...args);

    if (result.isErr) {
      const errorVariant = result.asErr?.toString() ?? 'Unknown';
      throw decodeContractError(errorVariant);
    }

    if (output) {
      return output.toJSON();
    }
    return null;
  }

  /**
   * Submits a state-mutating transaction to the contract.
   */
  private async submitTx(
    signer: Signer,
    method: string,
    args: unknown[],
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    const signerAddress = typeof signer === 'string' ? signer : signer.address;

    // Dry-run to estimate gas
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

    // Submit the actual transaction
    const txFn = this.contract.tx[method];
    if (!txFn) {
      throw new Error(`Unknown tx method: ${method}`);
    }

    // Apply safety buffer to estimated gas
    const gasLimit = await this.applyGasBuffer(BigInt(gasRequired?.toString() ?? '0'));

    return new Promise<TxResult>((resolve, reject) => {
      const tx = txFn({ gasLimit }, ...args);

      const signOptions = typeof signer === 'string' ? {} : undefined;

      tx.signAndSend(
        signer as KeyringPair,
        signOptions ?? {},
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
              blockNumber: 0, // Filled from block details if needed
              events: decodedEvents,
              success: true,
            });
          }
        },
      ).catch(reject);
    });
  }

  /**
   * Encodes PropertyMetadata for contract calls.
   */
  private encodePropertyMetadata(metadata: PropertyMetadata): unknown {
    return {
      location: metadata.location,
      size: metadata.size,
      legal_description: metadata.legalDescription,
      valuation: metadata.valuation.toString(),
      documents_url: metadata.documentsUrl,
    };
  }

  /**
   * Decodes raw property info from the contract.
   */
  private decodePropertyInfo(raw: unknown): PropertyInfo {
    const data = raw as Record<string, unknown>;
    const meta = data.metadata as Record<string, unknown>;
    return {
      id: data.id as number,
      owner: data.owner as string,
      metadata: {
        location: meta.location as string,
        size: meta.size as number,
        legalDescription: (meta.legal_description ?? meta.legalDescription) as string,
        valuation: BigInt((meta.valuation as string) ?? '0'),
        documentsUrl: (meta.documents_url ?? meta.documentsUrl) as string,
      },
      registeredAt: data.registered_at as number,
    };
  }

  /**
   * Applies a safety buffer to the estimated gas required for a transaction.
   * If autoAdjustGas is enabled, the buffer scales with network congestion.
   */
  private async applyGasBuffer(estimatedGas: bigint): Promise<bigint> {
    let bufferPercentage = this.options.gasBufferPercentage ?? 10;

    if (this.options.autoAdjustGas) {
      try {
        // Dynamic adjustment based on contract health and metrics
        const health = await this.healthCheck();
        if (health && health.isHealthy) {
          // Increase buffer if we're near the current block's target or if paused/recovering
          if (health.isPaused) {
            bufferPercentage += 20; // Extra safety during maintenance/pause
          }

          const metrics = await this.getGasMetrics();
          if (metrics && metrics.averageOperationGas > 0) {
            const utilizationRatio =
              Number(metrics.lastOperationGas) / metrics.averageOperationGas;
            if (utilizationRatio > 1.5) {
              bufferPercentage += 15; // High volatility detected
            }
          }
        }
      } catch (error) {
        // Fallback to default buffer if metrics lookup fails
        console.warn('Failed to fetch gas metrics for auto-adjustment, using default buffer.');
      }
    }

    const buffer = (estimatedGas * BigInt(bufferPercentage)) / 100n;
    return estimatedGas + buffer;
  }
}
