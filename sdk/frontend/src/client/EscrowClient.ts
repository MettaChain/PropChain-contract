/**
 * @propchain/sdk — Escrow Client
 *
 * Convenience wrapper for escrow operations, providing a focused API
 * for creating, releasing, and refunding property escrows.
 *
 * @module client/EscrowClient
 */

import type { PropertyRegistryClient, Signer } from './PropertyRegistryClient';
import type { EscrowInfo, TxResult, TxProgressCallback } from '../types';

/**
 * Focused client for escrow operations.
 *
 * This is a convenience wrapper around the PropertyRegistryClient's
 * escrow methods, providing a cleaner API for escrow-focused workflows.
 *
 * @example
 * ```typescript
 * const escrow = client.escrow;
 *
 * // Create an escrow for a property sale
 * const { escrowId } = await escrow.create(
 *   signer,
 *   propertyId,
 *   buyerAddress,
 *   sellerAddress,
 *   BigInt(500000_00000000),
 * );
 *
 * // Release after conditions are met
 * await escrow.release(signer, escrowId);
 * ```
 */
export class EscrowClient {
  private readonly registryClient: PropertyRegistryClient;

  constructor(registryClient: PropertyRegistryClient) {
    this.registryClient = registryClient;
  }

  /**
   * Creates a new escrow for a property transfer.
   *
   * @param signer - Buyer's account
   * @param propertyId - Property being purchased
   * @param buyer - Buyer address
   * @param seller - Seller address
   * @param amount - Escrow amount
   * @returns The escrow ID and transaction result
   */
  async create(
    signer: Signer,
    propertyId: number,
    buyer: string,
    seller: string,
    amount: bigint,
    onProgress?: TxProgressCallback,
  ): Promise<{ escrowId: number } & TxResult> {
    return this.registryClient.createEscrow(signer, propertyId, buyer, seller, amount, onProgress);
  }

  /**
   * Releases an escrow, completing the property transfer.
   *
   * @param signer - Authorized account
   * @param escrowId - Escrow to release
   */
  async release(signer: Signer, escrowId: number, onProgress?: TxProgressCallback): Promise<TxResult> {
    return this.registryClient.releaseEscrow(signer, escrowId, onProgress);
  }

  /**
   * Refunds an escrow, returning funds to the buyer.
   *
   * @param signer - Authorized account
   * @param escrowId - Escrow to refund
   */
  async refund(signer: Signer, escrowId: number, onProgress?: TxProgressCallback): Promise<TxResult> {
    return this.registryClient.refundEscrow(signer, escrowId, onProgress);
  }

  /**
   * Gets escrow information by ID.
   *
   * @param escrowId - Escrow ID to look up
   * @returns Escrow details or `null`
   */
  async get(escrowId: number): Promise<EscrowInfo | null> {
    return this.registryClient.getEscrow(escrowId);
  }
}
