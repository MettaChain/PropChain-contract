/**
 * @propchain/sdk — Formatting Utilities
 *
 * Helpers for displaying on-chain data in human-friendly formats:
 * balance formatting, address display, timestamp conversion, etc.
 *
 * @module utils/formatters
 */

// ============================================================================
// Balance Formatting
// ============================================================================

/**
 * Formats an on-chain balance (bigint) to a human-readable string.
 *
 * @param value - Raw on-chain balance value
 * @param decimals - Number of decimal places used on-chain (default: 12 for DOT)
 * @param displayDecimals - Number of decimal places to show (default: 4)
 * @returns Formatted balance string
 *
 * @example
 * ```typescript
 * formatBalance(BigInt('10000000000000'), 12); // '10.0000'
 * formatBalance(BigInt('1500000000000'), 12, 2); // '1.50'
 * ```
 */
export function formatBalance(
  value: bigint,
  decimals: number = 12,
  displayDecimals: number = 4,
): string {
  const divisor = BigInt(10) ** BigInt(decimals);
  const integerPart = value / divisor;
  const fractionalPart = value % divisor;

  const fractionalStr = fractionalPart
    .toString()
    .padStart(decimals, '0')
    .slice(0, displayDecimals);

  return `${integerPart.toString()}.${fractionalStr}`;
}

/**
 * Parses a human-readable balance string to an on-chain bigint value.
 *
 * @param humanValue - Human-readable balance (e.g. '10.5')
 * @param decimals - Number of decimal places used on-chain (default: 12)
 * @returns Raw on-chain balance as bigint
 *
 * @example
 * ```typescript
 * parseBalance('10.5', 12); // BigInt('10500000000000')
 * parseBalance('1', 8);     // BigInt('100000000')
 * ```
 */
export function parseBalance(humanValue: string, decimals: number = 12): bigint {
  const parts = humanValue.split('.');
  const integerPart = parts[0] || '0';
  const fractionalPart = (parts[1] || '').padEnd(decimals, '0').slice(0, decimals);

  return BigInt(integerPart) * BigInt(10) ** BigInt(decimals) + BigInt(fractionalPart);
}

/**
 * Formats a valuation amount in USD with 8 decimal places.
 *
 * @param value - On-chain valuation (8 decimals)
 * @param displayDecimals - Number of decimal places to show (default: 2)
 * @returns Formatted USD string
 *
 * @example
 * ```typescript
 * formatValuation(BigInt('50000000000000')); // '$500,000.00'
 * ```
 */
export function formatValuation(value: bigint, displayDecimals: number = 2): string {
  const usdValue = Number(formatBalance(value, 8, displayDecimals + 2));
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: displayDecimals,
    maximumFractionDigits: displayDecimals,
  }).format(usdValue);
}

// ============================================================================
// Address Formatting
// ============================================================================

/**
 * Formats an SS58 address for display (full address).
 *
 * @param accountId - Full SS58 account address
 * @returns The formatted address string
 */
export function formatAddress(accountId: string): string {
  return accountId;
}

/**
 * Truncates an SS58 address for compact UI display.
 *
 * @param address - Full SS58 account address
 * @param startChars - Characters to keep at the start (default: 6)
 * @param endChars - Characters to keep at the end (default: 4)
 * @returns Truncated address string (e.g. '5Grwva…utQY')
 *
 * @example
 * ```typescript
 * truncateAddress('5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY');
 * // '5Grwva…utQY'
 * ```
 */
export function truncateAddress(
  address: string,
  startChars: number = 6,
  endChars: number = 4,
): string {
  if (address.length <= startChars + endChars + 3) {
    return address;
  }
  return `${address.slice(0, startChars)}…${address.slice(-endChars)}`;
}

// ============================================================================
// Timestamp Formatting
// ============================================================================

/**
 * Formats a block timestamp (milliseconds) to a human-readable date string.
 *
 * @param timestamp - Block timestamp in milliseconds
 * @param locale - Locale for formatting (default: 'en-US')
 * @returns Formatted date string
 *
 * @example
 * ```typescript
 * formatTimestamp(1700000000000); // 'Nov 14, 2023, 10:13:20 PM'
 * ```
 */
export function formatTimestamp(timestamp: number, locale: string = 'en-US'): string {
  return new Date(timestamp).toLocaleString(locale, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

/**
 * Converts a block timestamp to a relative time string (e.g. '5 minutes ago').
 *
 * @param timestamp - Block timestamp in milliseconds
 * @returns Relative time string
 *
 * @example
 * ```typescript
 * relativeTime(Date.now() - 60000); // '1 minute ago'
 * ```
 */
export function relativeTime(timestamp: number): string {
  const now = Date.now();
  const diff = now - timestamp;

  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (days > 0) return `${days} day${days > 1 ? 's' : ''} ago`;
  if (hours > 0) return `${hours} hour${hours > 1 ? 's' : ''} ago`;
  if (minutes > 0) return `${minutes} minute${minutes > 1 ? 's' : ''} ago`;
  if (seconds > 0) return `${seconds} second${seconds > 1 ? 's' : ''} ago`;
  return 'just now';
}

// ============================================================================
// Number Formatting
// ============================================================================

/**
 * Formats a large number with comma separators.
 *
 * @param value - Number to format
 * @returns Formatted string (e.g. '1,234,567')
 */
export function formatNumber(value: number | bigint): string {
  return new Intl.NumberFormat('en-US').format(value);
}

/**
 * Formats a property size with appropriate unit.
 *
 * @param sizeSqm - Size in square metres
 * @returns Formatted string (e.g. '2,500 sqm' or '1.5 ha')
 */
export function formatPropertySize(sizeSqm: number): string {
  if (sizeSqm >= 10000) {
    const hectares = sizeSqm / 10000;
    return `${hectares.toFixed(2)} ha`;
  }
  return `${formatNumber(sizeSqm)} sqm`;
}
