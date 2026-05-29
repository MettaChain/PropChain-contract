/**
 * @propchain/sdk — Error Handling Utilities
 *
 * Custom error classes, error decoding from on-chain dispatch errors,
 * and helpers for interpreting contract revert results.
 *
 * @module utils/errors
 */

import { PropertyRegistryError, PropertyTokenError, OracleErrorCode } from '../types';

// ============================================================================
// Error Categories
// ============================================================================

/**
 * Error categories matching the on-chain `ErrorCategory` enum.
 */
export enum ErrorCategory {
  PropertyRegistry = 'PropertyRegistry',
  PropertyToken = 'PropertyToken',
  Oracle = 'Oracle',
  Escrow = 'Escrow',
  Bridge = 'Bridge',
  Compliance = 'Compliance',
  Governance = 'Governance',
  Unknown = 'Unknown',
}

// ============================================================================
// Custom Error Class
// ============================================================================

/**
 * Typed error class for PropChain contract errors.
 *
 * Extends the standard `Error` with structured fields for error code,
 * category, description, and original on-chain error variant.
 *
 * @example
 * ```typescript
 * try {
 *   await client.propertyRegistry.getProperty(999);
 * } catch (err) {
 *   if (err instanceof PropChainError) {
 *     console.log('Code:', err.errorCode);
 *     console.log('Category:', err.category);
 *     console.log('Description:', err.description);
 *   }
 * }
 * ```
 */
export class PropChainError extends Error {
  /** Numeric error code from the contract */
  public readonly errorCode: number;
  /** Human-readable error description */
  public readonly description: string;
  /** Error category (which contract module) */
  public readonly category: ErrorCategory;
  /** Original on-chain error variant name (e.g. 'PropertyNotFound') */
  public readonly variant: string;

  constructor(
    variant: string,
    errorCode: number,
    description: string,
    category: ErrorCategory,
  ) {
    super(`[${category}] ${variant}: ${description}`);
    this.name = 'PropChainError';
    this.variant = variant;
    this.errorCode = errorCode;
    this.description = description;
    this.category = category;
  }
}

/**
 * Error thrown when the SDK cannot connect to the blockchain node.
 */
export class ConnectionError extends Error {
  public readonly endpoint: string;
  public readonly attempts: number;

  constructor(endpoint: string, attempts: number, cause?: Error) {
    super(`Failed to connect to ${endpoint} after ${attempts} attempt(s)`);
    this.name = 'ConnectionError';
    this.endpoint = endpoint;
    this.attempts = attempts;
    if (cause) {
      Object.defineProperty(this, 'cause', { value: cause });
    }
  }
}

/**
 * Error thrown when a transaction fails.
 */
export class TransactionError extends Error {
  public readonly txHash?: string;
  public readonly dispatchError?: string;

  constructor(message: string, txHash?: string, dispatchError?: string) {
    super(message);
    this.name = 'TransactionError';
    this.txHash = txHash;
    this.dispatchError = dispatchError;
  }
}

/**
 * Error thrown when gas estimation fails.
 */
export class GasEstimationError extends Error {
  public readonly method: string;

  constructor(method: string, cause?: Error) {
    super(`Failed to estimate gas for method: ${method}`);
    this.name = 'GasEstimationError';
    this.method = method;
    if (cause) {
      Object.defineProperty(this, 'cause', { value: cause });
    }
  }
}

// ============================================================================
// Error Decoding
// ============================================================================

/**
 * Description map for PropertyRegistry error variants.
 */
const REGISTRY_ERROR_DESCRIPTIONS: Record<string, string> = {
  PropertyNotFound: 'Property does not exist in the registry',
  Unauthorized: 'Caller is not authorized for this operation',
  InvalidMetadata: 'Property metadata is invalid or malformed',
  NotCompliant: 'Recipient is not compliant with regulatory requirements',
  ComplianceCheckFailed: 'Call to the compliance registry contract failed',
  EscrowNotFound: 'Escrow does not exist',
  EscrowAlreadyReleased: 'Escrow has already been released',
  BadgeNotFound: 'Badge does not exist for this property',
  InvalidBadgeType: 'Badge type is invalid',
  BadgeAlreadyIssued: 'Badge has already been issued to this property',
  NotVerifier: 'Caller is not an authorized verifier',
  AppealNotFound: 'Appeal does not exist',
  InvalidAppealStatus: 'Appeal status does not allow this operation',
  ComplianceRegistryNotSet: 'Compliance registry contract address has not been configured',
  OracleError: 'Oracle contract returned an error',
  ContractPaused: 'Contract is currently paused',
  AlreadyPaused: 'Contract is already paused',
  NotPaused: 'Contract is not currently paused',
  ResumeRequestAlreadyActive: 'A resume request is already in progress',
  ResumeRequestNotFound: 'No active resume request exists',
  InsufficientApprovals: 'Not enough approvals to complete the operation',
  AlreadyApproved: 'Caller has already approved this operation',
  NotAuthorizedToPause: 'Caller is not authorized to pause the contract',
  ZeroAddress: 'Provided address is the zero address (all zeros)',
  StringTooLong: 'Input string exceeds maximum allowed length',
  StringEmpty: 'Input string is empty when a value is required',
  ValueOutOfBounds: 'Numeric value is out of acceptable bounds',
  BatchSizeExceeded: 'Input batch exceeds the configured max_batch_size',
  SelfTransferNotAllowed: 'Cannot transfer or approve to yourself',
  InvalidRange: 'Range is invalid (min > max)',
};

/**
 * Description map for PropertyToken error variants.
 */
const TOKEN_ERROR_DESCRIPTIONS: Record<string, string> = {
  TokenNotFound: 'Token does not exist',
  Unauthorized: 'Caller is not authorized',
  PropertyNotFound: 'Property does not exist',
  InvalidMetadata: 'Metadata is invalid or malformed',
  DocumentNotFound: 'Document does not exist',
  ComplianceFailed: 'Compliance check failed',
  BridgeNotSupported: 'Bridge functionality not supported',
  InvalidChain: 'Invalid chain ID',
  BridgeLocked: 'Token is locked in bridge',
  InsufficientSignatures: 'Insufficient signatures for bridge operation',
  RequestExpired: 'Bridge request has expired',
  InvalidRequest: 'Invalid bridge request',
  BridgePaused: 'Bridge operations are paused',
  GasLimitExceeded: 'Gas limit exceeded',
  MetadataCorruption: 'Metadata is corrupted',
  InvalidBridgeOperator: 'Invalid bridge operator',
  DuplicateBridgeRequest: 'Duplicate bridge request',
  BridgeTimeout: 'Bridge operation timed out',
  AlreadySigned: 'Already signed this request',
  InsufficientBalance: 'Insufficient balance',
  InvalidAmount: 'Invalid amount',
  ProposalNotFound: 'Proposal not found',
  ProposalClosed: 'Proposal is closed',
  AskNotFound: 'Ask not found',
  BatchSizeExceeded: 'Input batch exceeds maximum allowed size',
};

/**
 * Decodes a contract error variant string into a typed `PropChainError`.
 *
 * @param errorVariant - The error variant name from the contract result
 * @returns A `PropChainError` instance with structured information
 *
 * @example
 * ```typescript
 * const error = decodeContractError('PropertyNotFound');
 * console.log(error.category); // 'PropertyRegistry'
 * console.log(error.description); // 'Property does not exist...'
 * ```
 */
export function decodeContractError(errorVariant: string): PropChainError {
  // Check PropertyRegistry errors
  if (errorVariant in PropertyRegistryError) {
    const desc = REGISTRY_ERROR_DESCRIPTIONS[errorVariant] || 'Unknown registry error';
    return new PropChainError(
      errorVariant,
      Object.keys(PropertyRegistryError).indexOf(errorVariant),
      desc,
      ErrorCategory.PropertyRegistry,
    );
  }

  // Check PropertyToken errors
  if (errorVariant in PropertyTokenError) {
    const desc = TOKEN_ERROR_DESCRIPTIONS[errorVariant] || 'Unknown token error';
    return new PropChainError(
      errorVariant,
      Object.keys(PropertyTokenError).indexOf(errorVariant),
      desc,
      ErrorCategory.PropertyToken,
    );
  }

  // Check Oracle errors
  if (errorVariant in OracleErrorCode) {
    return new PropChainError(
      errorVariant,
      Object.keys(OracleErrorCode).indexOf(errorVariant),
      `Oracle error: ${errorVariant}`,
      ErrorCategory.Oracle,
    );
  }

  // Unknown error
  return new PropChainError(
    errorVariant,
    -1,
    `Unknown contract error: ${errorVariant}`,
    ErrorCategory.Unknown,
  );
}

/**
 * Checks whether a contract query result indicates a revert.
 *
 * @param result - The result from a contract query
 * @returns `true` if the query reverted
 */
export function isContractRevert(result: { isErr?: boolean; asErr?: unknown }): boolean {
  return result.isErr === true;
}

/**
 * User-friendly error message from a PropChainError or generic Error.
 *
 * @param error - The error to convert
 * @returns A user-friendly message string
 */
export function getUserFriendlyMessage(error: unknown): string {
  if (error instanceof PropChainError) {
    return error.description;
  }
  if (error instanceof ConnectionError) {
    return 'Unable to connect to the blockchain. Please check your network connection.';
  }
  if (error instanceof TransactionError) {
    return `Transaction failed: ${error.message}`;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return 'An unexpected error occurred';
}
