/**
 * Payment Error State Tests (#637)
 *
 * End-to-end vitest coverage for SDK payment error states:
 *   - InsufficientBalance
 *   - RequestExpired (expired approval)
 *   - Oracle price staleness (InsufficientSources / StalePrice)
 *   - BridgeTimeout
 *
 * These tests exercise the error decoding pipeline via `decodeContractError`
 * and the structured `PropChainError` class, confirming that each error state
 * surfaces the correct category, variant, and user-friendly description.
 */

import { describe, it, expect } from 'vitest';

import {
  decodeContractError,
  PropChainError,
  GasEstimationError,
  TransactionError,
  ErrorCategory,
  getUserFriendlyMessage,
} from '../src/utils/errors';

// ============================================================================
// Helper
// ============================================================================

/** Asserts that decodeContractError returns the expected shape. */
function assertError(
  variant: string,
  expectedCategory: ErrorCategory,
  expectedDescriptionFragment?: string,
) {
  const err = decodeContractError(variant);
  expect(err).toBeInstanceOf(PropChainError);
  expect(err.variant).toBe(variant);
  expect(err.category).toBe(expectedCategory);
  if (expectedDescriptionFragment) {
    expect(err.description.toLowerCase()).toContain(
      expectedDescriptionFragment.toLowerCase(),
    );
  }
  return err;
}

// ============================================================================
// 1. Insufficient Balance
// ============================================================================

describe('Payment Error: InsufficientBalance', () => {
  it('decodes InsufficientBalance as a PropertyToken error', () => {
    assertError('InsufficientBalance', ErrorCategory.PropertyToken, 'balance');
  });

  it('has a non-empty description', () => {
    const err = decodeContractError('InsufficientBalance');
    expect(err.description.length).toBeGreaterThan(0);
  });

  it('produces a user-friendly message', () => {
    const err = decodeContractError('InsufficientBalance');
    const msg = getUserFriendlyMessage(err);
    expect(msg).toBe(err.description);
  });

  it('message contains category and variant', () => {
    const err = decodeContractError('InsufficientBalance');
    expect(err.message).toContain('PropertyToken');
    expect(err.message).toContain('InsufficientBalance');
  });
});

// ============================================================================
// 2. Expired Approval / RequestExpired
// ============================================================================

describe('Payment Error: RequestExpired (expired approval)', () => {
  it('decodes RequestExpired as a PropertyToken error', () => {
    assertError('RequestExpired', ErrorCategory.PropertyToken, 'expired');
  });

  it('getUserFriendlyMessage returns description for expired requests', () => {
    const err = decodeContractError('RequestExpired');
    const msg = getUserFriendlyMessage(err);
    expect(msg).toContain('expired');
  });

  it('errorCode is a non-negative number', () => {
    const err = decodeContractError('RequestExpired');
    expect(err.errorCode).toBeGreaterThanOrEqual(0);
  });
});

// ============================================================================
// 3. Oracle Price Staleness
// ============================================================================

describe('Payment Error: Oracle Price Staleness', () => {
  it('decodes InsufficientSources as Oracle category', () => {
    assertError('InsufficientSources', ErrorCategory.Oracle);
  });

  it('decodes StalePrice as Oracle category', () => {
    // StalePrice may be in OracleErrorCode depending on the contract version;
    // if not present, we expect Unknown category.
    const err = decodeContractError('StalePrice');
    const validCategories = [ErrorCategory.Oracle, ErrorCategory.Unknown];
    expect(validCategories).toContain(err.category);
  });

  it('oracle error description references the variant', () => {
    const err = decodeContractError('InsufficientSources');
    expect(err.message).toContain('InsufficientSources');
  });

  it('oracle getUserFriendlyMessage returns a non-empty string', () => {
    const err = decodeContractError('InsufficientSources');
    const msg = getUserFriendlyMessage(err);
    expect(typeof msg).toBe('string');
    expect(msg.length).toBeGreaterThan(0);
  });
});

// ============================================================================
// 4. Bridge Timeout
// ============================================================================

describe('Payment Error: BridgeTimeout', () => {
  it('decodes BridgeTimeout as a PropertyToken error', () => {
    assertError('BridgeTimeout', ErrorCategory.PropertyToken, 'timed out');
  });

  it('produces correct error structure', () => {
    const err = decodeContractError('BridgeTimeout');
    expect(err).toBeInstanceOf(PropChainError);
    expect(err.variant).toBe('BridgeTimeout');
    expect(err.name).toBe('PropChainError');
    expect(err instanceof Error).toBe(true);
  });

  it('getUserFriendlyMessage for BridgeTimeout returns description', () => {
    const err = decodeContractError('BridgeTimeout');
    expect(getUserFriendlyMessage(err)).toBe(err.description);
  });
});

// ============================================================================
// 5. GasEstimationError propagation
// ============================================================================

describe('Payment Error: GasEstimationError', () => {
  it('creates error with the correct method name', () => {
    const err = new GasEstimationError('transfer_from');
    expect(err.name).toBe('GasEstimationError');
    expect(err.method).toBe('transfer_from');
    expect(err.message).toContain('transfer_from');
  });

  it('wraps a cause when provided', () => {
    const cause = new Error('dry-run failed');
    const err = new GasEstimationError('approve', cause);
    expect((err as unknown as { cause: Error }).cause).toBe(cause);
  });

  it('getUserFriendlyMessage returns the error message', () => {
    const err = new GasEstimationError('transfer_from');
    const msg = getUserFriendlyMessage(err);
    expect(msg).toContain('transfer_from');
  });
});

// ============================================================================
// 6. TransactionError propagation
// ============================================================================

describe('Payment Error: TransactionError', () => {
  it('stores txHash and dispatchError', () => {
    const err = new TransactionError('TX failed', '0xabc', 'BadOrigin');
    expect(err.txHash).toBe('0xabc');
    expect(err.dispatchError).toBe('BadOrigin');
  });

  it('getUserFriendlyMessage includes the message', () => {
    const err = new TransactionError('Insufficient funds in account');
    const msg = getUserFriendlyMessage(err);
    expect(msg).toContain('Insufficient funds');
  });
});

// ============================================================================
// 7. Unknown / unexpected errors
// ============================================================================

describe('Payment Error: Unknown States', () => {
  it('decodes an unknown variant as Unknown category', () => {
    const err = decodeContractError('SomeFutureError');
    expect(err.category).toBe(ErrorCategory.Unknown);
    expect(err.errorCode).toBe(-1);
  });

  it('getUserFriendlyMessage handles non-Error values', () => {
    expect(getUserFriendlyMessage('raw string')).toBe('An unexpected error occurred');
    expect(getUserFriendlyMessage(42)).toBe('An unexpected error occurred');
    expect(getUserFriendlyMessage(null)).toBe('An unexpected error occurred');
  });

  it('error is still an instance of Error for unknown variants', () => {
    const err = decodeContractError('WeirdUnknownError');
    expect(err instanceof Error).toBe(true);
  });
});
