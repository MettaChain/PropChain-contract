// Staking helper methods for PropertyToken (Issue #197)
// Included inside `impl PropertyToken` — do not wrap in another impl block.

const STAKE_SCALING: u128 = 1_000_000_000_000;
const REWARD_RATE_PRECISION: u128 = 10_000; // Basis points precision

fn update_stake_acc_reward(&mut self, token_id: TokenId) {
    let total = self.share_total_staked.get(token_id).unwrap_or(0);
    if total == 0 {
        return;
    }
    let now = self.env().block_number() as u64;
    let last = self.share_last_reward_block.get(token_id).unwrap_or(now);
    let blocks = (now as u128).saturating_sub(last as u128);
    if blocks == 0 {
        return;
    }
    let rate = self.share_reward_rate_bps.get(token_id).unwrap_or(0);
    let reward = total
        .saturating_mul(rate)
        .saturating_mul(blocks)
        / REWARD_RATE_PRECISION
        / 5_256_000;
    let acc = self.share_acc_reward_per_share.get(token_id).unwrap_or(0);
    self.share_acc_reward_per_share.insert(
        token_id,
        &acc.saturating_add(reward.saturating_mul(STAKE_SCALING) / total),
    );
    self.share_last_reward_block.insert(token_id, &now);
}

fn pending_stake_rewards(&self, stake: &ShareStakeInfo) -> u128 {
    let acc = self
        .share_acc_reward_per_share
        .get(stake.token_id)
        .unwrap_or(0);
    let base = stake
        .amount
        .saturating_mul(acc.saturating_sub(stake.reward_debt))
        / STAKE_SCALING;
    base.saturating_mul(stake.lock_period.multiplier()) / 100
}

fn governance_weight(&self, voter: AccountId, token_id: TokenId) -> u128 {
    if let Some(stake) = self.share_stakes.get((voter, token_id)) {
        stake
            .amount
            .saturating_mul(stake.lock_period.multiplier())
            / 100
    } else {
        self.balances.get((voter, token_id)).unwrap_or(0)
    }
}
