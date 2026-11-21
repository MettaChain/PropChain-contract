//! Minimal economic models for property valuation/rental yield (#1514)
//! and lending liquidation risk simulation (#1513).
//!
//! This module is intentionally lean and designed for extension/review.

#[derive(Debug, Clone, PartialEq)]
pub struct YieldResult {
    pub rental_yield_pct: f64,
    pub annual_rent: f64,
    pub annual_expenses: f64,
    pub net_income: f64,
}

/// Project property value over `years` using compound growth.
pub fn project_property_value(initial_value: f64, annual_growth_rate: f64, years: usize) -> Vec<f64> {
    let mut values = Vec::with_capacity(years + 1);
    let mut current = if initial_value.is_finite() { initial_value.max(0.0) } else { 0.0 };
    let rate = if annual_growth_rate.is_finite() { annual_growth_rate } else { 0.0 };
    values.push(current);
    for _ in 0..years {
        current *= 1.0 + rate;
        values.push(current);
    }
    values
}

/// Calculate rental yield and income breakdown for a property.
pub fn calculate_rental_yield(property_value: f64, monthly_rent: f64, expense_ratio: f64) -> YieldResult {
    let value = if property_value.is_finite() && property_value > 0.0 { property_value } else { 0.0 };
    let rent = if monthly_rent.is_finite() && monthly_rent > 0.0 { monthly_rent } else { 0.0 };
    let ratio = if expense_ratio.is_finite() { expense_ratio.clamp(0.0, 1.0) } else { 0.0 };
    let annual_rent = rent * 12.0;
    let annual_expenses = annual_rent * ratio;
    let net_income = annual_rent - annual_expenses;
    let rental_yield_pct = if value > 0.0 { (net_income / value) * 100.0 } else { 0.0 };
    YieldResult { rental_yield_pct, annual_rent, annual_expenses, net_income }
}

/// Distribute net rental yield proportionally to fractional token holders.
pub fn distribute_fractional_yield(net_income: f64, shares: &[f64]) -> Vec<f64> {
    let income = if net_income.is_finite() && net_income > 0.0 { net_income } else { 0.0 };
    let total: f64 = shares.iter().copied().filter(|s| s.is_finite() && *s > 0.0).sum();
    if total <= 0.0 { return vec![0.0; shares.len()]; }
    shares
        .iter()
        .map(|s| {
            let share = if s.is_finite() && *s > 0.0 { *s } else { 0.0 };
            income * (share / total)
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct LendingPosition {
    pub id: String,
    pub debt: f64,
    pub collateral: f64,
    pub liquidation_threshold: f64, // e.g. 0.8 means collateral/debt must stay >= 1/0.8 in simplified form
}

#[derive(Debug, Clone, PartialEq)]
pub struct PositionLiquidation {
    pub id: String,
    pub liquidated: bool,
    pub bad_debt: f64,
    pub liquidator_profit: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiquidationResult {
    pub positions: Vec<PositionLiquidation>,
    pub total_liquidated: f64,
    pub bad_debt: f64,
    pub protocol_health_factor: f64,
    pub liquidator_profit: f64,
}

/// Simulate a liquidation scenario under a collateral price drop.
pub fn simulate_liquidation_scenario(
    positions: &[LendingPosition],
    price_drop_pct: f64,
) -> LiquidationResult {
    let drop = if price_drop_pct.is_finite() { price_drop_pct.clamp(0.0, 1.0) } else { 0.0 };
    let mut out = Vec::with_capacity(positions.len());
    let mut total_liquidated = 0.0;
    let mut bad_debt = 0.0;
    let mut liquidator_profit = 0.0;
    let mut total_collateral = 0.0;
    let mut total_debt = 0.0;

    for p in positions {
        let debt = if p.debt.is_finite() && p.debt > 0.0 { p.debt } else { 0.0 };
        let collateral_after = if p.collateral.is_finite() && p.collateral > 0.0 { p.collateral * (1.0 - drop) } else { 0.0 };
        total_collateral += collateral_after;
        total_debt += debt;
        let threshold = if p.liquidation_threshold.is_finite() && p.liquidation_threshold > 0.0 { p.liquidation_threshold } else { 1.0 };
        // Simplified: liquidate when collateral falls below debt / threshold.
        let min_collateral = debt / threshold;
        if debt > 0.0 && collateral_after < min_collateral {
            let recovered = collateral_after.min(debt);
            let pos_bad_debt = (debt - collateral_after).max(0.0);
            let profit = (collateral_after - debt).max(0.0) * 0.05; // small incentive model
            total_liquidated += recovered;
            bad_debt += pos_bad_debt;
            liquidator_profit += profit;
            out.push(PositionLiquidation { id: p.id.clone(), liquidated: true, bad_debt: pos_bad_debt, liquidator_profit: profit });
        } else {
            out.push(PositionLiquidation { id: p.id.clone(), liquidated: false, bad_debt: 0.0, liquidator_profit: 0.0 });
        }
    }

    let protocol_health_factor = if total_debt > 0.0 { total_collateral / total_debt } else { f64::INFINITY };
    LiquidationResult { positions: out, total_liquidated, bad_debt, protocol_health_factor, liquidator_profit }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projects_value_and_yield() {
        let values = project_property_value(100_000.0, 0.05, 2);
        assert_eq!(values.len(), 3);
        let y = calculate_rental_yield(200_000.0, 1_000.0, 0.2);
        assert!((y.annual_rent - 12_000.0).abs() < 1e-9);
        assert!(y.rental_yield_pct > 0.0);
    }

    #[test]
    fn simulates_liquidation_thresholds() {
        let positions = vec![LendingPosition { id: "a".into(), debt: 100.0, collateral: 150.0, liquidation_threshold: 0.8 }];
        let res = simulate_liquidation_scenario(&positions, 0.5);
        assert_eq!(res.positions[0].liquidated, true);
        assert!(res.total_liquidated >= 0.0);
    }
}
