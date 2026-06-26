/**
 * @propchain/sdk — Gas Estimation Utility
 *
 * Provides automatic gas estimation with configurable safety buffers based
 * on observed network congestion. Buffers are applied as percentages on top
 * of the raw estimate returned by contract dry-runs.
 *
 * @module utils/gas
 */

// ============================================================================
// Types
// ============================================================================

/** Congestion level of the network, inferred from recent block utilisation. */
export enum CongestionLevel {
  Low = 'Low',
  Medium = 'Medium',
  High = 'High',
  Extreme = 'Extreme',
}

/** Result of a gas estimation with applied safety buffer. */
export interface GasEstimate {
  /** Raw gas required as reported by the contract dry-run. */
  rawGas: bigint;
  /** Final gas limit with safety buffer applied. */
  gasWithBuffer: bigint;
  /** The buffer percentage that was applied (0-100). */
  bufferPercent: number;
  /** The congestion level that was detected or provided. */
  congestionLevel: CongestionLevel;
}

/** Options for configuring gas estimation behaviour. */
export interface GasEstimationOptions {
  /**
   * Fixed buffer percentage to apply on top of the raw estimate.
   * If provided, congestion-based lookup is skipped.
   * Defaults to automatic congestion-based selection.
   */
  fixedBufferPercent?: number;

  /**
   * Override the detected congestion level.
   * Useful for testing or when congestion data is already known.
   */
  congestionOverride?: CongestionLevel;

  /**
   * Custom buffer percentages per congestion level.
   * Merged with the built-in defaults.
   */
  buffersByLevel?: Partial<Record<CongestionLevel, number>>;
}

// ============================================================================
// Default Buffers
// ============================================================================

/**
 * Default safety buffer percentages per congestion level.
 *
 * These values are deliberately conservative:
 * - Low: +10%   (calm network, small margin)
 * - Medium: +20%  (moderate load, safer margin)
 * - High: +35%   (congested, generous headroom)
 * - Extreme: +50%  (very congested, maximum headroom)
 */
export const DEFAULT_BUFFERS: Record<CongestionLevel, number> = {
  [CongestionLevel.Low]: 10,
  [CongestionLevel.Medium]: 20,
  [CongestionLevel.High]: 35,
  [CongestionLevel.Extreme]: 50,
};

// ============================================================================
// Congestion Detection
// ============================================================================

/**
 * Infers the current network congestion level from the provided fill ratio.
 *
 * @param blockFillRatio - Fraction of block capacity used (0.0 – 1.0).
 *   Pass `api.rpc.chain.getHeader()` derived fill ratios here.
 * @returns Inferred {@link CongestionLevel}
 *
 * @example
 * ```typescript
 * const level = congestionFromFillRatio(0.75); // => CongestionLevel.High
 * ```
 */
export function congestionFromFillRatio(blockFillRatio: number): CongestionLevel {
  if (blockFillRatio < 0 || blockFillRatio > 1) {
    throw new RangeError(`blockFillRatio must be between 0 and 1, got ${blockFillRatio}`);
  }
  if (blockFillRatio < 0.4) return CongestionLevel.Low;
  if (blockFillRatio < 0.7) return CongestionLevel.Medium;
  if (blockFillRatio < 0.9) return CongestionLevel.High;
  return CongestionLevel.Extreme;
}

// ============================================================================
// Core Estimation Function
// ============================================================================

/**
 * Applies a configurable safety buffer to a raw gas estimate.
 *
 * When `fixedBufferPercent` is provided, it is used directly.
 * Otherwise, the buffer is selected from the congestion-level table
 * (using `congestionOverride` or `CongestionLevel.Medium` as default).
 *
 * @param rawGas - Raw gas required from a contract dry-run.
 * @param options - Estimation configuration.
 * @returns A {@link GasEstimate} with the buffered gas limit.
 *
 * @example
 * ```typescript
 * // Fixed buffer
 * const est = applyGasBuffer(500_000n, { fixedBufferPercent: 15 });
 * // est.gasWithBuffer === 575_000n
 *
 * // Congestion-based buffer
 * const est2 = applyGasBuffer(500_000n, {
 *   congestionOverride: CongestionLevel.High,
 * });
 * // est2.bufferPercent === 35
 * // est2.gasWithBuffer === 675_000n
 * ```
 */
export function applyGasBuffer(
  rawGas: bigint,
  options: GasEstimationOptions = {},
): GasEstimate {
  if (rawGas < 0n) {
    throw new RangeError(`rawGas must be >= 0, got ${rawGas}`);
  }

  const mergedBuffers: Record<CongestionLevel, number> = {
    ...DEFAULT_BUFFERS,
    ...options.buffersByLevel,
  };

  let bufferPercent: number;
  let congestionLevel: CongestionLevel;

  if (options.fixedBufferPercent !== undefined) {
    bufferPercent = options.fixedBufferPercent;
    congestionLevel = options.congestionOverride ?? CongestionLevel.Medium;
  } else {
    congestionLevel = options.congestionOverride ?? CongestionLevel.Medium;
    bufferPercent = mergedBuffers[congestionLevel];
  }

  if (bufferPercent < 0 || bufferPercent > 200) {
    throw new RangeError(`bufferPercent must be between 0 and 200, got ${bufferPercent}`);
  }

  const buffer = (rawGas * BigInt(Math.round(bufferPercent))) / 100n;
  const gasWithBuffer = rawGas + buffer;

  return {
    rawGas,
    gasWithBuffer,
    bufferPercent,
    congestionLevel,
  };
}

/**
 * Convenience function: estimates gas from a raw value and a fill ratio.
 *
 * Combines congestion detection and buffer application in a single call.
 *
 * @param rawGas - Raw gas required from dry-run.
 * @param blockFillRatio - Current block fill ratio (0.0 – 1.0).
 * @param options - Additional overrides (e.g. `buffersByLevel`).
 * @returns A {@link GasEstimate} tuned to the observed congestion level.
 *
 * @example
 * ```typescript
 * const est = estimateGasWithCongestion(1_000_000n, 0.8);
 * // Block at 80% → High congestion → +35% buffer
 * // est.gasWithBuffer === 1_350_000n
 * ```
 */
export function estimateGasWithCongestion(
  rawGas: bigint,
  blockFillRatio: number,
  options: Omit<GasEstimationOptions, 'congestionOverride'> = {},
): GasEstimate {
  const level = congestionFromFillRatio(blockFillRatio);
  return applyGasBuffer(rawGas, { ...options, congestionOverride: level });
}
