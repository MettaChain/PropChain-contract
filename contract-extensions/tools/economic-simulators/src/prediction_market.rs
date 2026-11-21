use crate::{rng::Rng, ModelError};

const DEFAULT_SEED: u64 = 0x1518_0001;

/// Inputs for an LMSR market simulation.
#[derive(Clone, Debug, PartialEq)]
pub struct MarketConfig {
    /// LMSR liquidity parameter `b`; larger values reduce price impact.
    pub liquidity: f64,
    pub participants: usize,
    /// Belief that outcome 1 occurs.
    pub outcome_probability: f64,
    pub informed_trader_share_bps: u32,
    pub trade_size: f64,
    pub seed: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MarketResult {
    pub initial_price: f64,
    pub final_price: f64,
    pub outcome_probability: f64,
    pub price_accuracy: f64,
    /// Absolute price movement for an additional trade equal to `trade_size`.
    pub price_impact_per_trade: f64,
    /// Inverse price impact, a simple measure of depth; zero if price impact is zero.
    pub liquidity_depth: f64,
    pub expected_lp_profit_loss: f64,
    /// Error reduction from the initial price to the final price, from 0 to 1.
    pub market_efficiency: f64,
    pub informed_trades: usize,
    pub uninformed_trades: usize,
    pub liquidity: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LiquidityComparison {
    pub liquidity: f64,
    pub price_accuracy: f64,
    pub price_impact_per_trade: f64,
    pub expected_lp_profit_loss: f64,
    pub market_efficiency: f64,
}

/// Simulate a two-outcome LMSR market with default informed-trader assumptions.
pub fn simulate_market(
    liquidity: f64,
    participants: usize,
    outcome_probability: f64,
) -> Result<MarketResult, ModelError> {
    simulate_market_with_config(&MarketConfig {
        liquidity,
        participants,
        outcome_probability,
        informed_trader_share_bps: 3_000,
        trade_size: liquidity * 0.05,
        seed: DEFAULT_SEED,
    })
}

/// Simulate trades, LMSR prices, market-maker cash flow, and final settlement.
pub fn simulate_market_with_config(
    config: &MarketConfig,
) -> Result<MarketResult, ModelError> {
    validate_config(config)?;
    let mut rng = Rng::new(config.seed);
    let mut shares = [0.0_f64; 2];
    let mut collected = 0.0_f64;
    let mut informed_trades = 0usize;
    let mut uninformed_trades = 0usize;
    let initial_price = lmsr_prices(shares, config.liquidity)[1];
    let initial_error = (initial_price - config.outcome_probability).abs();

    for _ in 0..config.participants {
        let informed = rng.chance_bps(config.informed_trader_share_bps);
        let outcome = if informed {
            informed_trades += 1;
            if config.outcome_probability >= 0.5 {
                1
            } else {
                0
            }
        } else {
            uninformed_trades += 1;
            if rng.unit() < 0.5 {
                1
            } else {
                0
            }
        };
        let old_cost = lmsr_cost(shares, config.liquidity);
        shares[outcome] += config.trade_size;
        let new_cost = lmsr_cost(shares, config.liquidity);
        collected += new_cost - old_cost;
    }

    let final_price = lmsr_prices(shares, config.liquidity)[1];
    let price_accuracy = (1.0 - (final_price - config.outcome_probability).abs()).clamp(0.0, 1.0);
    let final_error = (final_price - config.outcome_probability).abs();
    let market_efficiency = if initial_error == 0.0 {
        1.0
    } else {
        (1.0 - final_error / initial_error).clamp(0.0, 1.0)
    };

    let mut depth_probe = shares;
    depth_probe[1] += config.trade_size;
    let price_impact_per_trade =
        (lmsr_prices(depth_probe, config.liquidity)[1] - final_price).abs();
    let liquidity_depth = if price_impact_per_trade == 0.0 {
        f64::INFINITY
    } else {
        config.trade_size / price_impact_per_trade
    };
    let expected_payout =
        shares[1] * config.outcome_probability + shares[0] * (1.0 - config.outcome_probability);
    Ok(MarketResult {
        initial_price,
        final_price,
        outcome_probability: config.outcome_probability,
        price_accuracy,
        price_impact_per_trade,
        liquidity_depth,
        expected_lp_profit_loss: collected - expected_payout,
        market_efficiency,
        informed_trades,
        uninformed_trades,
        liquidity: config.liquidity,
    })
}

/// Run the same market assumptions against alternative liquidity parameters.
pub fn compare_liquidity_parameters(
    config: &MarketConfig,
    liquidity_values: &[f64],
) -> Result<Vec<LiquidityComparison>, ModelError> {
    let mut results = Vec::with_capacity(liquidity_values.len());
    for liquidity in liquidity_values {
        let mut scenario = config.clone();
        scenario.liquidity = *liquidity;
        let result = simulate_market_with_config(&scenario)?;
        results.push(LiquidityComparison {
            liquidity: *liquidity,
            price_accuracy: result.price_accuracy,
            price_impact_per_trade: result.price_impact_per_trade,
            expected_lp_profit_loss: result.expected_lp_profit_loss,
            market_efficiency: result.market_efficiency,
        });
    }
    Ok(results)
}

/// LMSR cost function with log-sum-exp normalization to prevent overflow.
pub fn lmsr_cost(shares: [f64; 2], liquidity: f64) -> f64 {
    let max = (shares[0] / liquidity).max(shares[1] / liquidity);
    liquidity * (max + ((shares[0] / liquidity - max).exp()
        + (shares[1] / liquidity - max).exp())
    .ln())
}

/// LMSR outcome prices; the returned pair sums to one.
pub fn lmsr_prices(shares: [f64; 2], liquidity: f64) -> [f64; 2] {
    let max = (shares[0] / liquidity).max(shares[1] / liquidity);
    let weight_0 = (shares[0] / liquidity - max).exp();
    let weight_1 = (shares[1] / liquidity - max).exp();
    let total = weight_0 + weight_1;
    [weight_0 / total, weight_1 / total]
}

fn validate_config(config: &MarketConfig) -> Result<(), ModelError> {
    if !config.liquidity.is_finite() || config.liquidity <= 0.0 {
        return Err(ModelError::InvalidParameter(
            "liquidity must be finite and positive",
        ));
    }
    if config.participants == 0 {
        return Err(ModelError::EmptyPopulation);
    }
    if !config.outcome_probability.is_finite()
        || !(0.0..=1.0).contains(&config.outcome_probability)
    {
        return Err(ModelError::InvalidParameter(
            "outcome probability must be between 0 and 1",
        ));
    }
    if config.informed_trader_share_bps > 10_000 {
        return Err(ModelError::InvalidParameter(
            "informed trader share must be at most 10,000 bps",
        ));
    }
    if !config.trade_size.is_finite() || config.trade_size <= 0.0 {
        return Err(ModelError::InvalidParameter(
            "trade size must be finite and positive",
        ));
    }
    if !(config.trade_size * config.participants as f64).is_finite()
        || !(config.trade_size * config.participants as f64 / config.liquidity).is_finite()
    {
        return Err(ModelError::InvalidParameter(
            "aggregate trade size must remain within the LMSR numeric range",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lmsr_prices_follow_softmax_and_sum_to_one() {
        let prices = lmsr_prices([0.0, 0.0], 100.0);
        assert!((prices[0] - 0.5).abs() < 1e-12);
        assert!((prices[0] + prices[1] - 1.0).abs() < 1e-12);
        assert!(lmsr_cost([0.0, 100.0], 100.0) > lmsr_cost([0.0, 0.0], 100.0));
    }

    #[test]
    fn informed_flow_improves_price_accuracy_and_liquidity_dampens_impact() {
        let config = MarketConfig {
            liquidity: 100.0,
            participants: 100,
            outcome_probability: 0.8,
            informed_trader_share_bps: 10_000,
            trade_size: 5.0,
            seed: 12,
        };
        let market = simulate_market_with_config(&config).unwrap();
        let comparisons =
            compare_liquidity_parameters(&config, &[100.0, 1_000.0]).unwrap();
        assert!(market.final_price > market.initial_price);
        assert!(market.price_accuracy > 0.5);
        assert!(comparisons[1].price_impact_per_trade < comparisons[0].price_impact_per_trade);
    }

    #[test]
    fn rejects_invalid_liquidity() {
        assert!(matches!(
            simulate_market(0.0, 10, 0.5),
            Err(ModelError::InvalidParameter(_))
        ));
    }
}
