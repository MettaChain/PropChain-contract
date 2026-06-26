/**
 * Gas Estimation Tests (#634)
 *
 * Tests for the automatic gas-estimation utility with congestion-based
 * safety buffers.
 */

import { describe, it, expect } from 'vitest';
import {
  applyGasBuffer,
  estimateGasWithCongestion,
  congestionFromFillRatio,
  CongestionLevel,
  DEFAULT_BUFFERS,
} from '../src/utils/gas';
import type { GasEstimate } from '../src/utils/gas';

// ============================================================================
// congestionFromFillRatio
// ============================================================================

describe('congestionFromFillRatio', () => {
  it('returns Low for fill ratio below 0.4', () => {
    expect(congestionFromFillRatio(0)).toBe(CongestionLevel.Low);
    expect(congestionFromFillRatio(0.0)).toBe(CongestionLevel.Low);
    expect(congestionFromFillRatio(0.39)).toBe(CongestionLevel.Low);
  });

  it('returns Medium for fill ratio 0.4 – 0.69', () => {
    expect(congestionFromFillRatio(0.4)).toBe(CongestionLevel.Medium);
    expect(congestionFromFillRatio(0.55)).toBe(CongestionLevel.Medium);
    expect(congestionFromFillRatio(0.699)).toBe(CongestionLevel.Medium);
  });

  it('returns High for fill ratio 0.7 – 0.89', () => {
    expect(congestionFromFillRatio(0.7)).toBe(CongestionLevel.High);
    expect(congestionFromFillRatio(0.8)).toBe(CongestionLevel.High);
    expect(congestionFromFillRatio(0.899)).toBe(CongestionLevel.High);
  });

  it('returns Extreme for fill ratio >= 0.9', () => {
    expect(congestionFromFillRatio(0.9)).toBe(CongestionLevel.Extreme);
    expect(congestionFromFillRatio(1.0)).toBe(CongestionLevel.Extreme);
  });

  it('throws RangeError for values outside [0, 1]', () => {
    expect(() => congestionFromFillRatio(-0.1)).toThrow(RangeError);
    expect(() => congestionFromFillRatio(1.01)).toThrow(RangeError);
  });
});

// ============================================================================
// DEFAULT_BUFFERS
// ============================================================================

describe('DEFAULT_BUFFERS', () => {
  it('has a buffer for every congestion level', () => {
    for (const level of Object.values(CongestionLevel)) {
      expect(DEFAULT_BUFFERS[level]).toBeDefined();
      expect(typeof DEFAULT_BUFFERS[level]).toBe('number');
    }
  });

  it('buffers increase with congestion severity', () => {
    expect(DEFAULT_BUFFERS[CongestionLevel.Low]).toBeLessThan(
      DEFAULT_BUFFERS[CongestionLevel.Medium],
    );
    expect(DEFAULT_BUFFERS[CongestionLevel.Medium]).toBeLessThan(
      DEFAULT_BUFFERS[CongestionLevel.High],
    );
    expect(DEFAULT_BUFFERS[CongestionLevel.High]).toBeLessThan(
      DEFAULT_BUFFERS[CongestionLevel.Extreme],
    );
  });
});

// ============================================================================
// applyGasBuffer
// ============================================================================

describe('applyGasBuffer', () => {
  describe('fixed buffer mode', () => {
    it('applies fixed buffer correctly', () => {
      const result: GasEstimate = applyGasBuffer(1_000_000n, {
        fixedBufferPercent: 10,
      });
      expect(result.rawGas).toBe(1_000_000n);
      expect(result.gasWithBuffer).toBe(1_100_000n);
      expect(result.bufferPercent).toBe(10);
    });

    it('applies 0% buffer (no change)', () => {
      const result = applyGasBuffer(500_000n, { fixedBufferPercent: 0 });
      expect(result.gasWithBuffer).toBe(500_000n);
    });

    it('applies 50% buffer', () => {
      const result = applyGasBuffer(200_000n, { fixedBufferPercent: 50 });
      expect(result.gasWithBuffer).toBe(300_000n);
    });

    it('applies 100% buffer (doubles the gas)', () => {
      const result = applyGasBuffer(100_000n, { fixedBufferPercent: 100 });
      expect(result.gasWithBuffer).toBe(200_000n);
    });
  });

  describe('congestion-based buffer mode', () => {
    it('defaults to Medium congestion when no override is given', () => {
      const result = applyGasBuffer(1_000_000n);
      expect(result.congestionLevel).toBe(CongestionLevel.Medium);
      expect(result.bufferPercent).toBe(DEFAULT_BUFFERS[CongestionLevel.Medium]);
    });

    it('applies Low congestion buffer', () => {
      const result = applyGasBuffer(1_000_000n, {
        congestionOverride: CongestionLevel.Low,
      });
      expect(result.bufferPercent).toBe(DEFAULT_BUFFERS[CongestionLevel.Low]);
      const expected = 1_000_000n + (1_000_000n * BigInt(DEFAULT_BUFFERS[CongestionLevel.Low])) / 100n;
      expect(result.gasWithBuffer).toBe(expected);
    });

    it('applies High congestion buffer', () => {
      const result = applyGasBuffer(1_000_000n, {
        congestionOverride: CongestionLevel.High,
      });
      expect(result.bufferPercent).toBe(DEFAULT_BUFFERS[CongestionLevel.High]);
    });

    it('applies Extreme congestion buffer', () => {
      const result = applyGasBuffer(1_000_000n, {
        congestionOverride: CongestionLevel.Extreme,
      });
      expect(result.bufferPercent).toBe(DEFAULT_BUFFERS[CongestionLevel.Extreme]);
    });

    it('supports custom buffersByLevel overrides', () => {
      const result = applyGasBuffer(1_000_000n, {
        congestionOverride: CongestionLevel.High,
        buffersByLevel: { [CongestionLevel.High]: 25 },
      });
      expect(result.bufferPercent).toBe(25);
      expect(result.gasWithBuffer).toBe(1_250_000n);
    });

    it('leaves default levels intact when only one is overridden', () => {
      const result = applyGasBuffer(1_000_000n, {
        congestionOverride: CongestionLevel.Low,
        buffersByLevel: { [CongestionLevel.High]: 99 },
      });
      expect(result.bufferPercent).toBe(DEFAULT_BUFFERS[CongestionLevel.Low]);
    });
  });

  describe('edge cases', () => {
    it('handles rawGas of 0', () => {
      const result = applyGasBuffer(0n, { fixedBufferPercent: 50 });
      expect(result.rawGas).toBe(0n);
      expect(result.gasWithBuffer).toBe(0n);
    });

    it('throws RangeError for negative rawGas', () => {
      expect(() => applyGasBuffer(-1n)).toThrow(RangeError);
    });

    it('throws RangeError for bufferPercent > 200', () => {
      expect(() => applyGasBuffer(1000n, { fixedBufferPercent: 201 })).toThrow(RangeError);
    });

    it('throws RangeError for negative bufferPercent', () => {
      expect(() => applyGasBuffer(1000n, { fixedBufferPercent: -1 })).toThrow(RangeError);
    });

    it('returns correct shape for GasEstimate', () => {
      const result = applyGasBuffer(500n, { fixedBufferPercent: 20 });
      expect(result).toHaveProperty('rawGas');
      expect(result).toHaveProperty('gasWithBuffer');
      expect(result).toHaveProperty('bufferPercent');
      expect(result).toHaveProperty('congestionLevel');
    });
  });
});

// ============================================================================
// estimateGasWithCongestion
// ============================================================================

describe('estimateGasWithCongestion', () => {
  it('selects Low buffer for low fill ratio', () => {
    const result = estimateGasWithCongestion(1_000_000n, 0.2);
    expect(result.congestionLevel).toBe(CongestionLevel.Low);
    expect(result.bufferPercent).toBe(DEFAULT_BUFFERS[CongestionLevel.Low]);
  });

  it('selects Medium buffer for medium fill ratio', () => {
    const result = estimateGasWithCongestion(1_000_000n, 0.55);
    expect(result.congestionLevel).toBe(CongestionLevel.Medium);
  });

  it('selects High buffer for high fill ratio', () => {
    const result = estimateGasWithCongestion(1_000_000n, 0.8);
    expect(result.congestionLevel).toBe(CongestionLevel.High);
    expect(result.gasWithBuffer).toBe(1_350_000n);
  });

  it('selects Extreme buffer for very high fill ratio', () => {
    const result = estimateGasWithCongestion(1_000_000n, 0.95);
    expect(result.congestionLevel).toBe(CongestionLevel.Extreme);
    expect(result.gasWithBuffer).toBe(1_500_000n);
  });

  it('supports custom buffersByLevel', () => {
    const result = estimateGasWithCongestion(1_000_000n, 0.8, {
      buffersByLevel: { [CongestionLevel.High]: 40 },
    });
    expect(result.bufferPercent).toBe(40);
    expect(result.gasWithBuffer).toBe(1_400_000n);
  });

  it('throws for invalid fill ratio', () => {
    expect(() => estimateGasWithCongestion(1_000_000n, 1.5)).toThrow(RangeError);
  });
});
