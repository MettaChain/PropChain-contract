/**
 * Utility Function Tests
 *
 * Tests for formatters, error handling, and event utilities.
 */

import { describe, it, expect } from 'vitest';

import {
  formatBalance,
  parseBalance,
  formatValuation,
  truncateAddress,
  formatTimestamp,
  relativeTime,
  formatNumber,
  formatPropertySize,
} from '../src/utils/formatters';

import {
  PropChainError,
  ConnectionError,
  TransactionError,
  ErrorCategory,
  decodeContractError,
  isContractRevert,
  getUserFriendlyMessage,
} from '../src/utils/errors';

import { filterEvents, extractTypedEvents } from '../src/utils/events';
import { NETWORKS, getNetworkConfig } from '../src/utils/connection';

import type { ContractEvent } from '../src/types';

// ============================================================================
// Formatter Tests
// ============================================================================

describe('Formatters', () => {
  describe('formatBalance', () => {
    it('should format balance with default decimals', () => {
      const result = formatBalance(BigInt('10000000000000'), 12);
      expect(result).toBe('10.0000');
    });

    it('should format balance with custom display decimals', () => {
      const result = formatBalance(BigInt('1500000000000'), 12, 2);
      expect(result).toBe('1.50');
    });

    it('should handle zero balance', () => {
      const result = formatBalance(BigInt(0), 12);
      expect(result).toBe('0.0000');
    });

    it('should handle large balances', () => {
      const result = formatBalance(BigInt('1000000000000000'), 12, 2);
      expect(result).toBe('1000.00');
    });
  });

  describe('parseBalance', () => {
    it('should parse integer amount', () => {
      const result = parseBalance('10', 12);
      expect(result).toBe(BigInt('10000000000000'));
    });

    it('should parse decimal amount', () => {
      const result = parseBalance('10.5', 12);
      expect(result).toBe(BigInt('10500000000000'));
    });

    it('should parse with 8 decimals', () => {
      const result = parseBalance('1', 8);
      expect(result).toBe(BigInt('100000000'));
    });

    it('should handle zero', () => {
      const result = parseBalance('0', 12);
      expect(result).toBe(BigInt(0));
    });
  });

  describe('truncateAddress', () => {
    it('should truncate a long address', () => {
      const address = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';
      const result = truncateAddress(address);
      expect(result).toBe('5Grwva…utQY');
    });

    it('should not truncate a short address', () => {
      const address = '5Grwva';
      const result = truncateAddress(address);
      expect(result).toBe('5Grwva');
    });

    it('should support custom start/end lengths', () => {
      const address = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';
      const result = truncateAddress(address, 8, 6);
      expect(result).toBe('5GrwvaEF…GKutQY');
    });
  });

  describe('formatTimestamp', () => {
    it('should format a timestamp to a readable date', () => {
      const result = formatTimestamp(1700000000000);
      expect(result).toContain('2023');
    });
  });

  describe('relativeTime', () => {
    it('should return "just now" for recent times', () => {
      expect(relativeTime(Date.now())).toBe('just now');
    });

    it('should return minutes ago', () => {
      const fiveMinutesAgo = Date.now() - 5 * 60 * 1000;
      expect(relativeTime(fiveMinutesAgo)).toBe('5 minutes ago');
    });

    it('should return hours ago', () => {
      const twoHoursAgo = Date.now() - 2 * 60 * 60 * 1000;
      expect(relativeTime(twoHoursAgo)).toBe('2 hours ago');
    });

    it('should return days ago', () => {
      const threeDaysAgo = Date.now() - 3 * 24 * 60 * 60 * 1000;
      expect(relativeTime(threeDaysAgo)).toBe('3 days ago');
    });

    it('should handle singular form', () => {
      const oneMinuteAgo = Date.now() - 60 * 1000;
      expect(relativeTime(oneMinuteAgo)).toBe('1 minute ago');
    });
  });

  describe('formatNumber', () => {
    it('should format number with thousands separator', () => {
      expect(formatNumber(1234567)).toBe('1,234,567');
    });

    it('should handle small numbers', () => {
      expect(formatNumber(42)).toBe('42');
    });
  });

  describe('formatPropertySize', () => {
    it('should format in sqm for small properties', () => {
      expect(formatPropertySize(2500)).toBe('2,500 sqm');
    });

    it('should format in hectares for large properties', () => {
      expect(formatPropertySize(25000)).toBe('2.50 ha');
    });
  });
});

// ============================================================================
// Error Tests
// ============================================================================

describe('Errors', () => {
  describe('PropChainError', () => {
    it('should create error with all fields', () => {
      const error = new PropChainError(
        'PropertyNotFound',
        1001,
        'Property does not exist',
        ErrorCategory.PropertyRegistry,
      );

      expect(error.name).toBe('PropChainError');
      expect(error.variant).toBe('PropertyNotFound');
      expect(error.errorCode).toBe(1001);
      expect(error.description).toBe('Property does not exist');
      expect(error.category).toBe(ErrorCategory.PropertyRegistry);
      expect(error.message).toContain('PropertyNotFound');
      expect(error instanceof Error).toBe(true);
    });
  });

  describe('ConnectionError', () => {
    it('should create with endpoint and attempts', () => {
      const error = new ConnectionError('ws://localhost:9944', 5);
      expect(error.endpoint).toBe('ws://localhost:9944');
      expect(error.attempts).toBe(5);
      expect(error.message).toContain('ws://localhost:9944');
    });
  });

  describe('TransactionError', () => {
    it('should create with optional fields', () => {
      const error = new TransactionError('TX failed', '0xabc', 'Reverted');
      expect(error.txHash).toBe('0xabc');
      expect(error.dispatchError).toBe('Reverted');
    });
  });

  describe('decodeContractError', () => {
    it('should decode PropertyRegistry errors', () => {
      const error = decodeContractError('PropertyNotFound');
      expect(error.category).toBe(ErrorCategory.PropertyRegistry);
      expect(error.description).toContain('Property');
    });

    it('should decode PropertyToken errors', () => {
      const error = decodeContractError('TokenNotFound');
      expect(error.category).toBe(ErrorCategory.PropertyToken);
    });

    it('should decode Oracle errors', () => {
      const error = decodeContractError('InsufficientSources');
      expect(error.category).toBe(ErrorCategory.Oracle);
    });

    it('should handle unknown errors', () => {
      const error = decodeContractError('SomeUnknownError');
      expect(error.category).toBe(ErrorCategory.Unknown);
    });
  });

  describe('isContractRevert', () => {
    it('should return true for error results', () => {
      expect(isContractRevert({ isErr: true })).toBe(true);
    });

    it('should return false for ok results', () => {
      expect(isContractRevert({ isErr: false })).toBe(false);
    });
  });

  describe('getUserFriendlyMessage', () => {
    it('should return description for PropChainError', () => {
      const error = new PropChainError(
        'Unauthorized',
        1002,
        'Not authorized',
        ErrorCategory.PropertyRegistry,
      );
      expect(getUserFriendlyMessage(error)).toBe('Not authorized');
    });

    it('should return generic message for ConnectionError', () => {
      const error = new ConnectionError('ws://localhost:9944', 3);
      expect(getUserFriendlyMessage(error)).toContain('blockchain');
    });

    it('should handle unknown error types', () => {
      expect(getUserFriendlyMessage('something')).toBe('An unexpected error occurred');
    });
  });
});

// ============================================================================
// Event Tests
// ============================================================================

describe('Events', () => {
  describe('filterEvents', () => {
    const events: ContractEvent[] = [
      { name: 'PropertyRegistered', args: { propertyId: 1 } },
      { name: 'PropertyTransferred', args: { propertyId: 1, to: 'addr' } },
      { name: 'PropertyRegistered', args: { propertyId: 2 } },
      { name: 'EscrowCreated', args: { escrowId: 1 } },
    ];

    it('should filter events by name', () => {
      const result = filterEvents(events, 'PropertyRegistered');
      expect(result).toHaveLength(2);
      expect(result[0].args.propertyId).toBe(1);
      expect(result[1].args.propertyId).toBe(2);
    });

    it('should return empty for no matches', () => {
      const result = filterEvents(events, 'BadgeIssued');
      expect(result).toHaveLength(0);
    });
  });

  describe('extractTypedEvents', () => {
    const events: ContractEvent[] = [
      { name: 'PropertyRegistered', args: { propertyId: 1, owner: 'alice' } },
      { name: 'PropertyTransferred', args: { propertyId: 1 } },
      { name: 'PropertyRegistered', args: { propertyId: 2, owner: 'bob' } },
    ];

    it('should extract and type events', () => {
      const registered = extractTypedEvents(events, 'PropertyRegistered');
      expect(registered).toHaveLength(2);
    });
  });
});

// ============================================================================
// Connection Tests
// ============================================================================

describe('Connection', () => {
  describe('NETWORKS', () => {
    it('should have local network preset', () => {
      expect(NETWORKS.local).toBeDefined();
      expect(NETWORKS.local.wsEndpoint).toBe('ws://127.0.0.1:9944');
      expect(NETWORKS.local.isTestnet).toBe(true);
    });

    it('should have westend network preset', () => {
      expect(NETWORKS.westend).toBeDefined();
      expect(NETWORKS.westend.isTestnet).toBe(true);
    });

    it('should have polkadot network preset', () => {
      expect(NETWORKS.polkadot).toBeDefined();
      expect(NETWORKS.polkadot.isTestnet).toBe(false);
    });

    it('should have kusama network preset', () => {
      expect(NETWORKS.kusama).toBeDefined();
    });
  });

  describe('getNetworkConfig', () => {
    it('should return config for known network', () => {
      const config = getNetworkConfig('local');
      expect(config).toBeDefined();
      expect(config?.name).toBe('Local Development');
    });

    it('should return undefined for unknown network', () => {
      expect(getNetworkConfig('unknown')).toBeUndefined();
    });
  });
});
