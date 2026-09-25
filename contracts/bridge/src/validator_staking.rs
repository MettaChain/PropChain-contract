//! #1109: validator staking and slashing.
//!
//! `ValidatorStaking` is stored on the contract
//! ([`crate::PropertyBridge::staking`]) and tracks the credit-based stake of
//! each registered validator. Approvals for bridge requests require a
//! **staked-weight quorum** (`STAKED_QUORUM_BPS` of `total_staked`), not a
//! bare signature count; a validator that flip-flops its vote on a request is
//! slashed `SLASH_PERCENT` of its current stake into the slash pool.
//!
//! The stake is a nominal economic bond held in the ledger (no token
//! transfers): validators self-stake in `PropertyBridge::stake_validator`,
//! withdraw via `withdraw_stake`, and are penalised via `slash_validator` or
//! automatically on a conflicting vote.

use ink::primitives::AccountId;
use ink::storage::Mapping;

/// Minimum stake required for a validator to become economically active.
pub const MIN_VALIDATOR_STAKE: u128 = 10_000_000;
/// Percentage of a validator's stake removed on a slash.
pub const SLASH_PERCENT: u128 = 20;
/// Percentage of `total_staked` required to lock/approve a bridge request.
pub const STAKED_QUORUM_BPS: u128 = 6_000;

/// Errors returned by the staking ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StakeError {
    /// The requested stake amount is below [`MIN_VALIDATOR_STAKE`].
    BelowMinimum,
    /// The account does not hold enough stake for the requested withdrawal.
    InsufficientStake,
}

/// Credit-based validator staking ledger.
#[ink::storage_item]
#[derive(Default)]
pub struct ValidatorStaking {
    /// Current stake held by each account.
    pub stakes: Mapping<AccountId, u128>,
    /// Sum of all stakes; the denominator for the quorum computation.
    pub total_staked: u128,
    /// Accumulated slashed stake, reserved for future redemption or burns.
    pub slash_pool: u128,
}

impl core::fmt::Debug for ValidatorStaking {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ValidatorStaking")
            .field("total_staked", &self.total_staked)
            .field("slash_pool", &self.slash_pool)
            .finish()
    }
}

impl ValidatorStaking {
    /// Credited `amount` stake to `account`.
    pub fn stake(&mut self, account: AccountId, amount: u128) -> Result<(), StakeError> {
        if amount < MIN_VALIDATOR_STAKE {
            return Err(StakeError::BelowMinimum);
        }
        let current = self.stakes.get(account).unwrap_or(0);
        self.stakes.insert(account, &(current + amount));
        self.total_staked = self.total_staked.saturating_add(amount);
        Ok(())
    }

    /// Withdraws `amount` from `account`'s stake.
    pub fn unstake(&mut self, account: AccountId, amount: u128) -> Result<u128, StakeError> {
        let current = self.stakes.get(account).unwrap_or(0);
        if amount == 0 || amount > current {
            return Err(StakeError::InsufficientStake);
        }
        self.stakes.insert(account, &(current - amount));
        self.total_staked = self.total_staked.saturating_sub(amount);
        Ok(amount)
    }

    /// Slashes `SLASH_PERCENT` of `account`'s stake into the slash pool.
    /// Returns the slashed amount (0 when the account holds no stake).
    pub fn slash(&mut self, account: AccountId) -> u128 {
        let current = self.stakes.get(account).unwrap_or(0);
        if current == 0 {
            return 0;
        }
        let penalty = current * SLASH_PERCENT / 100;
        self.stakes.insert(account, &(current - penalty));
        self.total_staked = self.total_staked.saturating_sub(penalty);
        self.slash_pool = self.slash_pool.saturating_add(penalty);
        penalty
    }

    /// Whether `account` currently holds the active-stake threshold.
    pub fn is_active(&self, account: AccountId) -> bool {
        self.stakes.get(account).unwrap_or(0) >= MIN_VALIDATOR_STAKE
    }

    /// The staked weight of `account` (0 for non-stakers).
    pub fn weight(&self, account: AccountId) -> u128 {
        self.stakes.get(account).unwrap_or(0)
    }

    /// Total staked across all validators.
    pub fn total(&self) -> u128 {
        self.total_staked
    }

    /// The slash pool accumulated so far.
    pub fn pool(&self) -> u128 {
        self.slash_pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ink::test]
    fn stake_enforces_minimum() {
        let mut ledger = ValidatorStaking::default();
        let alice = AccountId::from([1u8; 32]);
        assert_eq!(
            ledger.stake(alice, MIN_VALIDATOR_STAKE - 1),
            Err(StakeError::BelowMinimum)
        );
        assert_eq!(ledger.stake(alice, MIN_VALIDATOR_STAKE), Ok(()));
        assert!(ledger.is_active(alice));
        assert_eq!(ledger.weight(alice), MIN_VALIDATOR_STAKE);
        assert_eq!(ledger.total(), MIN_VALIDATOR_STAKE);
    }

    #[ink::test]
    fn unstake_rejects_insufficient() {
        let mut ledger = ValidatorStaking::default();
        let alice = AccountId::from([1u8; 32]);
        ledger.stake(alice, MIN_VALIDATOR_STAKE).unwrap();
        assert_eq!(
            ledger.unstake(alice, MIN_VALIDATOR_STAKE + 1),
            Err(StakeError::InsufficientStake)
        );
        assert_eq!(
            ledger.unstake(alice, MIN_VALIDATOR_STAKE),
            Ok(MIN_VALIDATOR_STAKE)
        );
        assert_eq!(ledger.total(), 0);
        assert!(!ledger.is_active(alice));
    }

    #[ink::test]
    fn slash_removes_percent_into_pool() {
        let mut ledger = ValidatorStaking::default();
        let alice = AccountId::from([1u8; 32]);
        ledger.stake(alice, 100_000_000).unwrap();
        let penalty = ledger.slash(alice);
        assert_eq!(penalty, 20_000_000);
        assert_eq!(ledger.weight(alice), 80_000_000);
        assert_eq!(ledger.pool(), 20_000_000);
        assert_eq!(ledger.total(), 80_000_000);
        // Slashing a non-staker is a no-op.
        assert_eq!(ledger.slash(AccountId::from([2u8; 32])), 0);
    }

    #[ink::test]
    fn quorum_bps_is_percentage_of_total() {
        let mut ledger = ValidatorStaking::default();
        let alice = AccountId::from([1u8; 32]);
        let bob = AccountId::from([2u8; 32]);
        ledger.stake(alice, 100_000_000).unwrap();
        ledger.stake(bob, 100_000_000).unwrap();
        let quorum = ledger.total() * STAKED_QUORUM_BPS / 10_000;
        assert_eq!(quorum, 120_000_000);
    }
}
