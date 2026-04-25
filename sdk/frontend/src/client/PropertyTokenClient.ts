/**
 * @propchain/sdk — PropertyToken Client
 *
 * Typed wrapper for the PropChain PropertyToken (ERC-721/1155) smart contract.
 * Covers NFT operations, fractional ownership, governance, marketplace,
 * cross-chain bridge, and compliance.
 *
 * @module client/PropertyTokenClient
 */

import type { ApiPromise } from '@polkadot/api';
import { ContractPromise } from '@polkadot/api-contract';
import { Abi } from '@polkadot/api-contract';
import type { KeyringPair } from '@polkadot/keyring/types';

import type {
  PropertyMetadata,
  PropertyInfo,
  OwnershipTransfer,
  ComplianceInfo,
  DocumentInfo,
  BridgeStatus,
  BridgeMonitoringInfo,
  BridgeTransaction,
  Proposal,
  Ask,
  TaxRecord,
  TxResult,
  GasEstimation,
  ContractEvent,
  Subscription,
  ClientOptions,
  TxProgressCallback,
  TxProgressStatus,
} from '../types';
import { decodeContractError, TransactionError, GasEstimationError } from '../utils/errors';
import { decodeTransactionEvents, subscribeToNamedEvent } from '../utils/events';
import type { PropChainEventName, PropChainEventMap } from '../types/events';

export type Signer = KeyringPair | string;

// ============================================================================
// PropertyTokenClient
// ============================================================================

/**
 * Client for interacting with the PropChain PropertyToken contract.
 *
 * Supports ERC-721/1155 standard operations plus PropChain-specific features:
 * fractional ownership, governance, secondary marketplace, compliance,
 * legal documents, and cross-chain bridging.
 *
 * @example
 * ```typescript
 * const client = await PropChainClient.create('ws://localhost:9944', {
 *   propertyToken: '5Abc...',
 * });
 *
 * // Mint a property token
 * const { tokenId } = await client.propertyToken.registerPropertyWithToken(signer, {
 *   location: '456 Oak Ave',
 *   size: 3500,
 *   legalDescription: 'Lot 5 Block 3',
 *   valuation: BigInt(1000000_00000000),
 *   documentsUrl: 'ipfs://Qm...',
 * });
 * ```
 */
export class PropertyTokenClient {
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
  // ERC-721 Standard Methods
  // ==========================================================================

  /**
   * Returns the number of tokens owned by an account.
   */
  async balanceOf(owner: string): Promise<number> {
    const result = await this.query('balance_of', [owner]);
    return (result as number) ?? 0;
  }

  /**
   * Returns the owner of a token.
   */
  async ownerOf(tokenId: number): Promise<string | null> {
    const result = await this.query('owner_of', [tokenId]);
    return (result as string) ?? null;
  }

  /**
   * Transfers a token from one account to another.
   */
  async transferFrom(
    signer: Signer,
    from: string,
    to: string,
    tokenId: number,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(signer, 'transfer_from', [from, to, tokenId], onProgress);
  }

  /**
   * Approves an account to transfer a specific token.
   */
  async approve(signer: Signer, to: string, tokenId: number, onProgress?: TxProgressCallback): Promise<TxResult> {
    return this.submitTx(signer, 'approve', [to, tokenId], onProgress);
  }

  /**
   * Gets the approved account for a token.
   */
  async getApproved(tokenId: number): Promise<string | null> {
    const result = await this.query('get_approved', [tokenId]);
    return (result as string) ?? null;
  }

  /**
   * Sets or removes an operator for all tokens owned by the caller.
   */
  async setApprovalForAll(
    signer: Signer,
    operator: string,
    approved: boolean,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(signer, 'set_approval_for_all', [operator, approved], onProgress);
  }

  /**
   * Checks if an operator is approved for all tokens of an owner.
   */
  async isApprovedForAll(owner: string, operator: string): Promise<boolean> {
    const result = await this.query('is_approved_for_all', [owner, operator]);
    return (result as boolean) ?? false;
  }

  /**
   * Gets the total supply of tokens.
   */
  async totalSupply(): Promise<number> {
    const result = await this.query('total_supply', []);
    return (result as number) ?? 0;
  }

  // ==========================================================================
  // Property Token Operations
  // ==========================================================================

  /**
   * Registers a property and mints a corresponding token.
   */
  async registerPropertyWithToken(
    signer: Signer,
    metadata: PropertyMetadata,
    onProgress?: TxProgressCallback,
  ): Promise<{ tokenId: number } & TxResult> {
    const encoded = this.encodePropertyMetadata(metadata);
    const txResult = await this.submitTx(signer, 'register_property_with_token', [encoded], onProgress);

    const mintEvents = txResult.events.filter((e) => e.name === 'PropertyTokenMinted');
    const tokenId = mintEvents.length > 0
      ? (mintEvents[0].args.tokenId as number)
      : 0;

    return { tokenId, ...txResult };
  }

  /**
   * Gets the property information associated with a token.
   */
  async getTokenProperty(tokenId: number): Promise<PropertyInfo | null> {
    const result = await this.query('get_token_property', [tokenId]);
    return result ? this.decodePropertyInfo(result) : null;
  }

  /**
   * Gets the ownership history for a token.
   */
  async getOwnershipHistory(tokenId: number): Promise<OwnershipTransfer[]> {
    const result = await this.query('get_ownership_history', [tokenId]);
    return (result as OwnershipTransfer[]) ?? [];
  }

  // ==========================================================================
  // Legal Documents & Compliance
  // ==========================================================================

  /**
   * Attaches a legal document to a property token.
   */
  async attachLegalDocument(
    signer: Signer,
    tokenId: number,
    documentHash: string,
    documentType: string,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(
      signer,
      'attach_legal_document',
      [tokenId, documentHash, documentType],
      onProgress,
    );
  }

  /**
   * Gets legal documents for a token.
   */
  async getLegalDocuments(tokenId: number): Promise<DocumentInfo[]> {
    const result = await this.query('get_legal_documents', [tokenId]);
    return (result as DocumentInfo[]) ?? [];
  }

  /**
   * Verifies compliance for a token.
   */
  async verifyCompliance(
    signer: Signer,
    tokenId: number,
    verified: boolean,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(signer, 'verify_compliance', [tokenId, verified], onProgress);
  }

  /**
   * Gets compliance info for a token.
   */
  async getComplianceInfo(tokenId: number): Promise<ComplianceInfo | null> {
    const result = await this.query('get_compliance_info', [tokenId]);
    return result ? (result as unknown as ComplianceInfo) : null;
  }

  // ==========================================================================
  // Fractional Ownership
  // ==========================================================================

  /**
   * Issues fractional shares for a property token.
   */
  async issueShares(
    signer: Signer,
    tokenId: number,
    to: string,
    amount: bigint,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(signer, 'issue_shares', [tokenId, to, amount.toString()], onProgress);
  }

  /**
   * Redeems (burns) fractional shares.
   */
  async redeemShares(
    signer: Signer,
    tokenId: number,
    amount: bigint,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(signer, 'redeem_shares', [tokenId, amount.toString()], onProgress);
  }

  /**
   * Gets the share balance for an account on a specific token.
   */
  async getShareBalance(tokenId: number, account: string): Promise<bigint> {
    const result = await this.query('share_balance_of', [tokenId, account]);
    return BigInt((result as string) ?? '0');
  }

  /**
   * Gets total shares for a token.
   */
  async getTotalShares(tokenId: number): Promise<bigint> {
    const result = await this.query('total_shares', [tokenId]);
    return BigInt((result as string) ?? '0');
  }

  /**
   * Deposits dividends for a property token.
   */
  async depositDividends(
    signer: Signer,
    tokenId: number,
    amount: bigint,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(signer, 'deposit_dividends', [tokenId, amount.toString()], onProgress);
  }

  /**
   * Withdraws accrued dividends.
   */
  async withdrawDividends(signer: Signer, tokenId: number, onProgress?: TxProgressCallback): Promise<TxResult> {
    return this.submitTx(signer, 'withdraw_dividends', [tokenId], onProgress);
  }

  /**
   * Gets accrued dividend balance for an account.
   */
  async getDividendBalance(tokenId: number, account: string): Promise<bigint> {
    const result = await this.query('dividend_balance_of', [tokenId, account]);
    return BigInt((result as string) ?? '0');
  }

  /**
   * Gets the tax record for an account on a token.
   */
  async getTaxRecord(account: string, tokenId: number): Promise<TaxRecord | null> {
    const result = await this.query('get_tax_record', [account, tokenId]);
    return result ? (result as unknown as TaxRecord) : null;
  }

  // ==========================================================================
  // Governance
  // ==========================================================================

  /**
   * Creates a governance proposal for a property token.
   */
  async createProposal(
    signer: Signer,
    tokenId: number,
    descriptionHash: string,
    quorum: bigint,
    onProgress?: TxProgressCallback,
  ): Promise<{ proposalId: number } & TxResult> {
    const txResult = await this.submitTx(
      signer,
      'create_proposal',
      [tokenId, descriptionHash, quorum.toString()],
      onProgress,
    );
    const events = txResult.events.filter((e) => e.name === 'ProposalCreated');
    const proposalId = events.length > 0 ? (events[0].args.proposalId as number) : 0;
    return { proposalId, ...txResult };
  }

  /**
   * Votes on a governance proposal.
   */
  async vote(
    signer: Signer,
    tokenId: number,
    proposalId: number,
    support: boolean,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(signer, 'vote', [tokenId, proposalId, support], onProgress);
  }

  /**
   * Executes a governance proposal.
   */
  async executeProposal(
    signer: Signer,
    tokenId: number,
    proposalId: number,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(signer, 'execute_proposal', [tokenId, proposalId], onProgress);
  }

  /**
   * Gets a governance proposal.
   */
  async getProposal(tokenId: number, proposalId: number): Promise<Proposal | null> {
    const result = await this.query('get_proposal', [tokenId, proposalId]);
    return result ? (result as unknown as Proposal) : null;
  }

  // ==========================================================================
  // Secondary Marketplace
  // ==========================================================================

  /**
   * Places a sell ask on the secondary market.
   */
  async placeAsk(
    signer: Signer,
    tokenId: number,
    pricePerShare: bigint,
    amount: bigint,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(
      signer,
      'place_ask',
      [tokenId, pricePerShare.toString(), amount.toString()],
      onProgress,
    );
  }

  /**
   * Cancels a sell ask.
   */
  async cancelAsk(signer: Signer, tokenId: number, onProgress?: TxProgressCallback): Promise<TxResult> {
    return this.submitTx(signer, 'cancel_ask', [tokenId], onProgress);
  }

  /**
   * Buys shares from a sell ask.
   */
  async buyShares(
    signer: Signer,
    tokenId: number,
    seller: string,
    amount: bigint,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(signer, 'buy_shares', [tokenId, seller, amount.toString()], onProgress);
  }

  /**
   * Gets a sell ask for a token/seller pair.
   */
  async getAsk(tokenId: number, seller: string): Promise<Ask | null> {
    const result = await this.query('get_ask', [tokenId, seller]);
    return result ? (result as unknown as Ask) : null;
  }

  /**
   * Gets the last trade price for a token.
   */
  async getLastTradePrice(tokenId: number): Promise<bigint> {
    const result = await this.query('get_last_trade_price', [tokenId]);
    return BigInt((result as string) ?? '0');
  }

  // ==========================================================================
  // Cross-Chain Bridge
  // ==========================================================================

  /**
   * Initiates a cross-chain bridge with multi-signature requirement.
   */
  async initiateBridgeMultisig(
    signer: Signer,
    tokenId: number,
    destinationChain: number,
    recipient: string,
    requiredSignatures: number,
    timeoutBlocks: number | null,
    onProgress?: TxProgressCallback,
  ): Promise<{ requestId: number } & TxResult> {
    const txResult = await this.submitTx(
      signer,
      'initiate_bridge_multisig',
      [tokenId, destinationChain, recipient, requiredSignatures, timeoutBlocks],
      onProgress,
    );
    const events = txResult.events.filter((e) => e.name === 'BridgeRequestCreated');
    const requestId = events.length > 0 ? (events[0].args.requestId as number) : 0;
    return { requestId, ...txResult };
  }

  /**
   * Signs a bridge request.
   */
  async signBridgeRequest(
    signer: Signer,
    requestId: number,
    approve: boolean,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(signer, 'sign_bridge_request', [requestId, approve], onProgress);
  }

  /**
   * Executes a bridge after collecting required signatures.
   */
  async executeBridge(signer: Signer, requestId: number, onProgress?: TxProgressCallback): Promise<TxResult> {
    return this.submitTx(signer, 'execute_bridge', [requestId], onProgress);
  }

  /**
   * Gets bridge status for a token.
   */
  async getBridgeStatus(tokenId: number): Promise<BridgeStatus | null> {
    const result = await this.query('get_bridge_status', [tokenId]);
    return result ? (result as unknown as BridgeStatus) : null;
  }

  /**
   * Monitors a bridge request.
   */
  async monitorBridgeStatus(requestId: number): Promise<BridgeMonitoringInfo | null> {
    const result = await this.query('monitor_bridge_status', [requestId]);
    return result ? (result as unknown as BridgeMonitoringInfo) : null;
  }

  /**
   * Gets bridge history for an account.
   */
  async getBridgeHistory(account: string): Promise<BridgeTransaction[]> {
    const result = await this.query('get_bridge_history', [account]);
    return (result as BridgeTransaction[]) ?? [];
  }

  /**
   * Estimates gas for a bridge operation.
   */
  async estimateBridgeGas(tokenId: number, destinationChain: number): Promise<number> {
    const result = await this.query('estimate_bridge_gas', [tokenId, destinationChain]);
    return (result as number) ?? 0;
  }

  // ==========================================================================
  // Property Management
  // ==========================================================================

  /**
   * Sets the property management contract address.
   */
  async setPropertyManagementContract(
    signer: Signer,
    contractAddress: string | null,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(signer, 'set_property_management_contract', [contractAddress], onProgress);
  }

  /**
   * Assigns a management agent to a token.
   */
  async assignManagementAgent(
    signer: Signer,
    tokenId: number,
    agent: string,
    onProgress?: TxProgressCallback,
  ): Promise<TxResult> {
    return this.submitTx(signer, 'assign_management_agent', [tokenId, agent], onProgress);
  }

  /**
   * Clears the management agent for a token.
   */
  async clearManagementAgent(signer: Signer, tokenId: number, onProgress?: TxProgressCallback): Promise<TxResult> {
    return this.submitTx(signer, 'clear_management_agent', [tokenId], onProgress);
  }

  /**
   * Gets the management agent for a token.
   */
  async getManagementAgent(tokenId: number): Promise<string | null> {
    const result = await this.query('get_management_agent', [tokenId]);
    return (result as string) ?? null;
  }

  // ==========================================================================
  // Event Subscriptions
  // ==========================================================================

  /**
   * Subscribes to a specific contract event.
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

  private encodePropertyMetadata(metadata: PropertyMetadata): unknown {
    return {
      location: metadata.location,
      size: metadata.size,
      legal_description: metadata.legalDescription,
      valuation: metadata.valuation.toString(),
      documents_url: metadata.documentsUrl,
    };
  }

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
   */
  private async applyGasBuffer(estimatedGas: bigint): Promise<bigint> {
    const bufferPercentage = this.options.gasBufferPercentage ?? 10;
    const buffer = (estimatedGas * BigInt(bufferPercentage)) / 100n;
    return estimatedGas + buffer;
  }
}
