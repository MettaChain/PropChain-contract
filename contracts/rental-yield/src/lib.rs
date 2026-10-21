#![allow(clippy::clone_on_copy)] // fires inside ink! generated storage code
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unexpected_cfgs)]

use ink::storage::Mapping;
use propchain_traits::{non_reentrant, ReentrancyError, ReentrancyGuard};

/// Rental income distribution for tokenized properties.
///
/// Each property has a fixed set of shareholders whose basis-point shares must
/// add up to 10 000 (100%). The contract administrator credits rental income
/// proportionally to those shares, and each shareholder claims their accrued
/// balance independently.
///
/// The implementation deliberately keeps every state transition in one storage
/// pass: shares are stored as a single `Vec` per property, so a distribution
/// reads the shareholder list once instead of performing one lookup per
/// shareholder during a separate configuration pass.
#[ink::contract]
pub mod rental_yield {
    use ink::prelude::vec::Vec;

    use super::*;

    /// Basis-point denominator (10 000 bps = 100%).
    pub const BASIS_POINTS: u32 = 10_000;

    /// Upper bound on shareholders per property.
    ///
    /// The bound keeps duplicate detection and distribution cost predictable.
    /// Kept small enough that the O(n^2) duplicate check remains cheap.
    pub const MAX_SHAREHOLDERS: usize = 32;

    /// A shareholder and their share of a property's rental income.
    #[derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Share {
        /// Account that receives the distribution.
        pub owner: AccountId,
        /// Share in basis points. Must be greater than zero.
        pub percentage: u32,
    }

    /// Errors returned by every fallible message.
    ///
    /// Variants are field-less so the encoded error stays small (Issue #1367).
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Caller is not the contract administrator.
        Unauthorized,
        /// Property id `0` is reserved and never valid.
        InvalidPropertyId,
        /// Amount or percentage was zero where a positive value is required.
        InvalidAmount,
        /// The property has no configured shareholders.
        PropertyNotFound,
        /// A shareholder list was empty.
        EmptyShares,
        /// A shareholder was given a zero share.
        ZeroShare,
        /// The zero account cannot be a shareholder.
        ZeroShareholder,
        /// The same account appeared more than once.
        DuplicateShareholder,
        /// Shares did not add up to exactly 10 000 basis points.
        InvalidTotalShares,
        /// More shareholders than [`MAX_SHAREHOLDERS`] were supplied.
        TooManyShareholders,
        /// Nothing has accrued for the caller on this property.
        NoIncomeAvailable,
        /// Arithmetic overflowed while computing or crediting a payout.
        ArithmeticOverflow,
        /// Native transfer to the claimant failed.
        TransferFailed,
        /// A reentrant call was detected while claiming.
        ReentrantCall,
    }

    impl From<ReentrancyError> for Error {
        fn from(_: ReentrancyError) -> Self {
            Error::ReentrantCall
        }
    }

    /// Emitted when a property's shareholder split is replaced.
    #[ink(event)]
    pub struct SharesUpdated {
        #[ink(topic)]
        property_id: u64,
        total_bps: u32,
        shareholder_count: u32,
        updated_by: AccountId,
    }

    /// Emitted once per successful rental income distribution.
    ///
    /// A single aggregate event is emitted instead of one event per recipient,
    /// which keeps event gas independent of the number of shareholders
    /// (Issue #1364).
    #[ink(event)]
    pub struct RentalIncomeDistributed {
        #[ink(topic)]
        property_id: u64,
        amount: Balance,
        recipient_count: u32,
    }

    /// Emitted when a shareholder claims accrued rental income.
    #[ink(event)]
    pub struct RentalIncomeClaimed {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        owner: AccountId,
        amount: Balance,
    }

    #[ink(storage)]
    pub struct RentalYield {
        admin: AccountId,
        /// Shareholder list per property. One storage read per distribution.
        shares: Mapping<u64, Vec<Share>>,
        /// Accrued but unclaimed income per `(property, shareholder)`.
        unclaimed: Mapping<(u64, AccountId), Balance>,
        /// Lifetime income distributed per property.
        total_distributed: Mapping<u64, Balance>,
        reentrancy_guard: ReentrancyGuard,
    }

    impl Default for RentalYield {
        fn default() -> Self {
            Self::new()
        }
    }

    impl RentalYield {
        /// Creates the contract with the caller as administrator.
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                shares: Mapping::default(),
                unclaimed: Mapping::default(),
                total_distributed: Mapping::default(),
                reentrancy_guard: ReentrancyGuard::new(),
            }
        }

        /// Returns the current administrator.
        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
        }

        /// Replaces the shareholder split for `property_id`. Admin-only.
        ///
        /// Existing unclaimed balances are preserved: changing the split only
        /// affects future distributions.
        ///
        /// # Errors
        /// - `Error::Unauthorized` if the caller is not the administrator.
        /// - `Error::InvalidPropertyId` if `property_id` is `0`.
        /// - `Error::EmptyShares` if `shares` is empty.
        /// - `Error::TooManyShareholders` if more than [`MAX_SHAREHOLDERS`]
        ///   entries are supplied.
        /// - `Error::ZeroShare` / `Error::ZeroShareholder` for invalid entries.
        /// - `Error::DuplicateShareholder` if an owner appears twice.
        /// - `Error::InvalidTotalShares` if the shares do not sum to 10 000.
        /// - `Error::ArithmeticOverflow` if the basis-point sum overflows.
        #[ink(message)]
        pub fn set_shares(&mut self, property_id: u64, shares: Vec<Share>) -> Result<(), Error> {
            self.ensure_admin()?;
            validate_property_id(property_id)?;
            let total_bps = validate_shares(&shares)?;
            let shareholder_count = shares.len() as u32;
            let updated_by = self.env().caller();

            self.shares.insert(property_id, &shares);
            self.env().emit_event(SharesUpdated {
                property_id,
                total_bps,
                shareholder_count,
                updated_by,
            });
            Ok(())
        }

        /// Credits `amount` of rental income to every shareholder of
        /// `property_id`, proportional to their basis-point share. Admin-only.
        ///
        /// The contract must hold at least `amount` for later claims to
        /// succeed; this message only accrues balances and performs no
        /// external calls. Rounding dust remains in the contract.
        ///
        /// # Errors
        /// - `Error::Unauthorized` if the caller is not the administrator.
        /// - `Error::InvalidPropertyId` if `property_id` is `0`.
        /// - `Error::InvalidAmount` if `amount` is `0`.
        /// - `Error::PropertyNotFound` if the property has no shareholders.
        /// - `Error::ArithmeticOverflow` if a payout or total would overflow.
        #[ink(message)]
        pub fn distribute_rental_income(
            &mut self,
            property_id: u64,
            amount: Balance,
        ) -> Result<(), Error> {
            self.ensure_admin()?;
            validate_property_id(property_id)?;
            if amount == 0 {
                return Err(Error::InvalidAmount);
            }

            // Read the shareholder list once. A `Mapping` per shareholder
            // would require an extra read per entry on every distribution.
            let shares = self
                .shares
                .get(property_id)
                .ok_or(Error::PropertyNotFound)?;
            if shares.is_empty() {
                return Err(Error::PropertyNotFound);
            }

            // Compute every fallible value before the first storage write.
            // Returning `Err` from an ink! message does not roll storage back,
            // so a partial pass here would leave some shareholders credited
            // and others not.
            let mut credits: Vec<(AccountId, Balance)> = Vec::new();
            let mut recipient_count: u32 = 0;
            for share in shares.iter() {
                let owner = share.owner;
                let payout = amount
                    .checked_mul(Balance::from(share.percentage))
                    .ok_or(Error::ArithmeticOverflow)?
                    / Balance::from(BASIS_POINTS);

                // Zero-value payouts are skipped: writing a zero balance
                // costs storage gas and creates no claimable income.
                if payout == 0 {
                    continue;
                }

                let credited = self
                    .unclaimed
                    .get((property_id, owner))
                    .unwrap_or(0)
                    .checked_add(payout)
                    .ok_or(Error::ArithmeticOverflow)?;
                credits.push((owner, credited));
                recipient_count += 1;
            }

            let total = self
                .total_distributed
                .get(property_id)
                .unwrap_or(0)
                .checked_add(amount)
                .ok_or(Error::ArithmeticOverflow)?;

            for (owner, credited) in credits.iter() {
                self.unclaimed.insert((property_id, *owner), credited);
            }
            self.total_distributed.insert(property_id, &total);

            self.env().emit_event(RentalIncomeDistributed {
                property_id,
                amount,
                recipient_count,
            });
            Ok(())
        }

        /// Claims the caller's accrued rental income for `property_id`.
        ///
        /// The native transfer runs first, then the balance is cleared. Returning
        /// `Err` does not roll storage back in ink!, so clearing first would
        /// destroy the claim if the transfer failed. Reentrancy is prevented by
        /// the shared guard, which is entered before the transfer.
        ///
        /// # Errors
        /// - `Error::InvalidPropertyId` if `property_id` is `0`.
        /// - `Error::PropertyNotFound` if the property has no shareholders.
        /// - `Error::NoIncomeAvailable` if the caller has nothing to claim.
        /// - `Error::TransferFailed` if the native transfer fails.
        /// - `Error::ReentrantCall` if a reentrant call is detected.
        #[ink(message)]
        pub fn claim_rental_income(&mut self, property_id: u64) -> Result<Balance, Error> {
            non_reentrant!(self, {
                validate_property_id(property_id)?;
                if self.shares.get(property_id).is_none() {
                    return Err(Error::PropertyNotFound);
                }

                let caller = self.env().caller();
                let key = (property_id, caller);
                let amount = self.unclaimed.get(key).unwrap_or(0);
                if amount == 0 {
                    return Err(Error::NoIncomeAvailable);
                }

                self.env()
                    .transfer(caller, amount)
                    .map_err(|_| Error::TransferFailed)?;
                // Clear only after the transfer succeeded, so a failed
                // transfer leaves the claim intact.
                self.unclaimed.insert(key, &0);

                self.env().emit_event(RentalIncomeClaimed {
                    property_id,
                    owner: caller,
                    amount,
                });
                Ok(amount)
            })
        }

        /// Returns the unclaimed income for `owner` on `property_id`.
        #[ink(message)]
        pub fn get_unclaimed_rental_income(&self, property_id: u64, owner: AccountId) -> Balance {
            self.unclaimed.get((property_id, owner)).unwrap_or(0)
        }

        /// Returns the configured shareholder list for `property_id`.
        #[ink(message)]
        pub fn get_shares(&self, property_id: u64) -> Vec<Share> {
            self.shares.get(property_id).unwrap_or_default()
        }

        /// Returns `owner`'s share in basis points, or `0` if not found.
        #[ink(message)]
        pub fn get_share(&self, property_id: u64, owner: AccountId) -> u32 {
            let shares = self.shares.get(property_id).unwrap_or_default();
            shares
                .iter()
                .find(|share| share.owner == owner)
                .map(|share| share.percentage)
                .unwrap_or(0)
        }

        /// Returns the lifetime income distributed for `property_id`.
        #[ink(message)]
        pub fn get_total_distributed(&self, property_id: u64) -> Balance {
            self.total_distributed.get(property_id).unwrap_or(0)
        }

        fn ensure_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }
    }

    fn validate_property_id(property_id: u64) -> Result<(), Error> {
        if property_id == 0 {
            return Err(Error::InvalidPropertyId);
        }
        Ok(())
    }

    /// Validates a complete shareholder split and returns its total in bps.
    ///
    /// All checks run before any storage write so a rejected update cannot
    /// leave a partially applied split behind.
    fn validate_shares(shares: &[Share]) -> Result<u32, Error> {
        if shares.is_empty() {
            return Err(Error::EmptyShares);
        }
        if shares.len() > MAX_SHAREHOLDERS {
            return Err(Error::TooManyShareholders);
        }

        let zero_account = AccountId::from([0u8; 32]);
        let mut total: u32 = 0;

        for (index, share) in shares.iter().enumerate() {
            if share.owner == zero_account {
                return Err(Error::ZeroShareholder);
            }
            if share.percentage == 0 {
                return Err(Error::ZeroShare);
            }
            if shares[..index]
                .iter()
                .any(|previous| previous.owner == share.owner)
            {
                return Err(Error::DuplicateShareholder);
            }
            total = total
                .checked_add(share.percentage)
                .ok_or(Error::ArithmeticOverflow)?;
        }

        if total != BASIS_POINTS {
            return Err(Error::InvalidTotalShares);
        }
        Ok(total)
    }

    #[cfg(test)]
    mod tests {
        use ink::env::test;
        use scale::Decode as _;

        use super::*;

        type TestEnvironment = ink::env::DefaultEnvironment;

        const FUNDED_BALANCE: Balance = 1_000_000_000;

        fn accounts() -> test::DefaultAccounts<TestEnvironment> {
            test::default_accounts::<TestEnvironment>()
        }

        fn setup() -> (RentalYield, test::DefaultAccounts<TestEnvironment>) {
            let accounts = accounts();
            test::set_caller::<TestEnvironment>(accounts.alice);
            (RentalYield::new(), accounts)
        }

        fn topic_contains<T: scale::Encode>(topics: &[Vec<u8>], value: &T) -> bool {
            let encoded = value.encode();
            topics.iter().any(|topic| topic.starts_with(&encoded))
        }

        fn share(owner: AccountId, percentage: u32) -> Share {
            Share { owner, percentage }
        }

        fn split(accounts: &test::DefaultAccounts<TestEnvironment>) -> Vec<Share> {
            Vec::from([
                share(accounts.bob, 5_000),
                share(accounts.charlie, 3_000),
                share(accounts.django, 2_000),
            ])
        }

        fn set_default_shares(
            contract: &mut RentalYield,
            accounts: &test::DefaultAccounts<TestEnvironment>,
        ) {
            test::set_caller::<TestEnvironment>(accounts.alice);
            contract.set_shares(1, split(accounts)).unwrap();
        }

        fn fund_contract() {
            let callee = test::callee::<TestEnvironment>();
            test::set_account_balance::<TestEnvironment>(callee, FUNDED_BALANCE);
        }

        #[ink::test]
        fn constructor_uses_caller_as_admin() {
            let (contract, accounts) = setup();
            assert_eq!(contract.admin(), accounts.alice);
        }

        #[ink::test]
        fn set_shares_stores_split_and_emits_event() {
            let (mut contract, accounts) = setup();
            let events_before = test::recorded_events().count();

            contract.set_shares(1, split(&accounts)).unwrap();

            assert_eq!(contract.get_shares(1), split(&accounts));
            assert_eq!(contract.get_share(1, accounts.charlie), 3_000);

            let events = test::recorded_events().collect::<Vec<_>>();
            assert_eq!(events.len(), events_before + 1);
            let event = &events[events_before];
            assert!(topic_contains(&event.topics, &1u64));
        }

        #[ink::test]
        fn set_shares_rejects_non_admin() {
            let (mut contract, accounts) = setup();
            test::set_caller::<TestEnvironment>(accounts.bob);

            assert_eq!(
                contract.set_shares(1, split(&accounts)),
                Err(Error::Unauthorized)
            );
        }

        #[ink::test]
        fn set_shares_rejects_zero_property_id() {
            let (mut contract, accounts) = setup();
            assert_eq!(
                contract.set_shares(0, split(&accounts)),
                Err(Error::InvalidPropertyId)
            );
        }

        #[ink::test]
        fn set_shares_rejects_empty_split() {
            let (mut contract, _accounts) = setup();
            assert_eq!(contract.set_shares(1, Vec::new()), Err(Error::EmptyShares));
        }

        #[ink::test]
        fn set_shares_rejects_zero_share() {
            let (mut contract, accounts) = setup();
            let shares = Vec::from([share(accounts.bob, 10_000), share(accounts.charlie, 0)]);
            assert_eq!(contract.set_shares(1, shares), Err(Error::ZeroShare));
        }

        #[ink::test]
        fn set_shares_rejects_zero_shareholder() {
            let (mut contract, accounts) = setup();
            let shares = Vec::from([
                share(accounts.bob, 5_000),
                share(AccountId::from([0u8; 32]), 5_000),
            ]);
            assert_eq!(contract.set_shares(1, shares), Err(Error::ZeroShareholder));
        }

        #[ink::test]
        fn set_shares_rejects_duplicate_shareholder() {
            let (mut contract, accounts) = setup();
            let shares = Vec::from([share(accounts.bob, 5_000), share(accounts.bob, 5_000)]);
            assert_eq!(
                contract.set_shares(1, shares),
                Err(Error::DuplicateShareholder)
            );
        }

        #[ink::test]
        fn set_shares_rejects_total_that_is_not_one_hundred_percent() {
            let (mut contract, accounts) = setup();
            let shares = Vec::from([share(accounts.bob, 6_000), share(accounts.charlie, 3_000)]);
            assert_eq!(
                contract.set_shares(1, shares),
                Err(Error::InvalidTotalShares)
            );

            let shares = Vec::from([share(accounts.bob, 6_000), share(accounts.charlie, 5_000)]);
            assert_eq!(
                contract.set_shares(1, shares),
                Err(Error::InvalidTotalShares)
            );
        }

        #[ink::test]
        fn set_shares_rejects_more_than_the_shareholder_cap() {
            let (mut contract, _accounts) = setup();
            let mut shares = Vec::new();
            for index in 1..=(MAX_SHAREHOLDERS as u8 + 1) {
                shares.push(share(AccountId::from([index; 32]), 1));
            }
            assert_eq!(
                contract.set_shares(1, shares),
                Err(Error::TooManyShareholders)
            );
        }

        #[ink::test]
        fn failed_validation_does_not_write_storage() {
            let (mut contract, accounts) = setup();
            let bad = Vec::from([share(accounts.bob, 6_000), share(accounts.charlie, 3_000)]);

            assert!(contract.set_shares(1, bad).is_err());
            assert!(contract.get_shares(1).is_empty());
        }

        #[ink::test]
        fn distribution_credits_each_shareholder_proportionally() {
            let (mut contract, accounts) = setup();
            set_default_shares(&mut contract, &accounts);

            contract.distribute_rental_income(1, 1_000).unwrap();

            assert_eq!(contract.get_unclaimed_rental_income(1, accounts.bob), 500);
            assert_eq!(
                contract.get_unclaimed_rental_income(1, accounts.charlie),
                300
            );
            assert_eq!(
                contract.get_unclaimed_rental_income(1, accounts.django),
                200
            );
            assert_eq!(contract.get_total_distributed(1), 1_000);
        }

        #[ink::test]
        fn distributions_accumulate() {
            let (mut contract, accounts) = setup();
            set_default_shares(&mut contract, &accounts);

            contract.distribute_rental_income(1, 1_000).unwrap();
            contract.distribute_rental_income(1, 2_000).unwrap();

            assert_eq!(contract.get_unclaimed_rental_income(1, accounts.bob), 1_500);
            assert_eq!(
                contract.get_unclaimed_rental_income(1, accounts.charlie),
                900
            );
            assert_eq!(
                contract.get_unclaimed_rental_income(1, accounts.django),
                600
            );
            assert_eq!(contract.get_total_distributed(1), 3_000);
        }

        #[ink::test]
        fn distribution_emits_one_aggregate_event() {
            let (mut contract, accounts) = setup();
            set_default_shares(&mut contract, &accounts);
            let events_before = test::recorded_events().count();

            contract.distribute_rental_income(1, 1_000).unwrap();

            let events = test::recorded_events().collect::<Vec<_>>();
            assert_eq!(
                events.len(),
                events_before + 1,
                "a distribution must emit one event, not one per recipient"
            );
            let event = &events[events_before];
            assert!(topic_contains(&event.topics, &1u64));
        }

        #[ink::test]
        fn distribution_rounds_down_and_keeps_dust_in_the_contract() {
            let (mut contract, accounts) = setup();
            let shares = Vec::from([share(accounts.bob, 5_000), share(accounts.charlie, 5_000)]);
            contract.set_shares(1, shares).unwrap();

            contract.distribute_rental_income(1, 101).unwrap();

            assert_eq!(contract.get_unclaimed_rental_income(1, accounts.bob), 50);
            assert_eq!(
                contract.get_unclaimed_rental_income(1, accounts.charlie),
                50
            );
            assert_eq!(contract.get_total_distributed(1), 101);
        }

        #[ink::test]
        fn distribution_rejects_non_admin() {
            let (mut contract, accounts) = setup();
            set_default_shares(&mut contract, &accounts);
            test::set_caller::<TestEnvironment>(accounts.bob);

            assert_eq!(
                contract.distribute_rental_income(1, 1_000),
                Err(Error::Unauthorized)
            );
        }

        #[ink::test]
        fn distribution_rejects_zero_amount() {
            let (mut contract, accounts) = setup();
            set_default_shares(&mut contract, &accounts);

            assert_eq!(
                contract.distribute_rental_income(1, 0),
                Err(Error::InvalidAmount)
            );
        }

        #[ink::test]
        fn distribution_rejects_unknown_property() {
            let (mut contract, _accounts) = setup();
            assert_eq!(
                contract.distribute_rental_income(99, 1_000),
                Err(Error::PropertyNotFound)
            );
        }

        #[ink::test]
        fn distribution_rejects_zero_property_id() {
            let (mut contract, _accounts) = setup();
            assert_eq!(
                contract.distribute_rental_income(0, 1_000),
                Err(Error::InvalidPropertyId)
            );
        }

        #[ink::test]
        fn distribution_guards_against_overflow() {
            let (mut contract, accounts) = setup();
            contract
                .set_shares(1, Vec::from([share(accounts.bob, BASIS_POINTS)]))
                .unwrap();

            assert_eq!(
                contract.distribute_rental_income(1, Balance::MAX),
                Err(Error::ArithmeticOverflow)
            );
            assert_eq!(contract.get_unclaimed_rental_income(1, accounts.bob), 0);
        }

        #[ink::test]
        fn claim_transfers_income_and_clears_the_balance() {
            let (mut contract, accounts) = setup();
            set_default_shares(&mut contract, &accounts);
            contract.distribute_rental_income(1, 1_000).unwrap();
            fund_contract();

            let bob_before = test::get_account_balance::<TestEnvironment>(accounts.bob).unwrap();
            test::set_caller::<TestEnvironment>(accounts.bob);

            let claimed = contract.claim_rental_income(1).unwrap();

            assert_eq!(claimed, 500);
            assert_eq!(contract.get_unclaimed_rental_income(1, accounts.bob), 0);
            assert_eq!(
                test::get_account_balance::<TestEnvironment>(accounts.bob).unwrap(),
                bob_before + 500
            );
            // Other shareholders are unaffected.
            assert_eq!(
                contract.get_unclaimed_rental_income(1, accounts.charlie),
                300
            );
        }

        #[ink::test]
        fn claim_emits_an_event() {
            let (mut contract, accounts) = setup();
            set_default_shares(&mut contract, &accounts);
            contract.distribute_rental_income(1, 1_000).unwrap();
            fund_contract();
            let events_before = test::recorded_events().count();

            test::set_caller::<TestEnvironment>(accounts.bob);
            contract.claim_rental_income(1).unwrap();

            let events = test::recorded_events().collect::<Vec<_>>();
            assert_eq!(events.len(), events_before + 1);
            let event = &events[events_before];
            assert!(topic_contains(&event.topics, &1u64));
            assert!(topic_contains(&event.topics, &accounts.bob));
        }

        #[ink::test]
        fn claim_without_income_is_rejected() {
            let (mut contract, accounts) = setup();
            set_default_shares(&mut contract, &accounts);
            test::set_caller::<TestEnvironment>(accounts.bob);

            assert_eq!(
                contract.claim_rental_income(1),
                Err(Error::NoIncomeAvailable)
            );
        }

        #[ink::test]
        fn claim_for_unknown_property_is_rejected() {
            let (mut contract, accounts) = setup();
            test::set_caller::<TestEnvironment>(accounts.bob);

            assert_eq!(
                contract.claim_rental_income(99),
                Err(Error::PropertyNotFound)
            );
        }

        #[ink::test]
        fn claim_rejects_zero_property_id() {
            let (mut contract, accounts) = setup();
            test::set_caller::<TestEnvironment>(accounts.bob);

            assert_eq!(
                contract.claim_rental_income(0),
                Err(Error::InvalidPropertyId)
            );
        }

        #[ink::test]
        fn claim_cannot_be_repeated() {
            let (mut contract, accounts) = setup();
            set_default_shares(&mut contract, &accounts);
            contract.distribute_rental_income(1, 1_000).unwrap();
            fund_contract();
            test::set_caller::<TestEnvironment>(accounts.bob);

            assert_eq!(contract.claim_rental_income(1).unwrap(), 500);
            assert_eq!(
                contract.claim_rental_income(1),
                Err(Error::NoIncomeAvailable)
            );
        }

        #[ink::test]
        fn changing_shares_does_not_erase_accrued_income() {
            let (mut contract, accounts) = setup();
            set_default_shares(&mut contract, &accounts);
            contract.distribute_rental_income(1, 1_000).unwrap();

            contract
                .set_shares(1, Vec::from([share(accounts.bob, BASIS_POINTS)]))
                .unwrap();

            assert_eq!(contract.get_unclaimed_rental_income(1, accounts.bob), 500);
            assert_eq!(
                contract.get_unclaimed_rental_income(1, accounts.charlie),
                300
            );
        }

        #[ink::test]
        fn shares_updated_event_can_be_decoded() {
            let (mut contract, accounts) = setup();
            contract.set_shares(7, split(&accounts)).unwrap();

            let events = test::recorded_events().collect::<Vec<_>>();
            let event = &events[0];
            let decoded = SharesUpdated::decode(&mut &event.data[..]).expect("decode event");
            assert_eq!(decoded.property_id, 7);
            assert_eq!(decoded.total_bps, BASIS_POINTS);
            assert_eq!(decoded.shareholder_count, 3);
            assert_eq!(decoded.updated_by, accounts.alice);
        }
    }
}
