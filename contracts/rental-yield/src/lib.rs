#![allow(clippy::clone_on_copy)] // fires inside ink! generated storage code
#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(unexpected_cfgs)]

use ink::storage::Mapping;
pub use propchain_traits::Role;
use propchain_traits::{non_reentrant, AccessControl, ReentrancyError, ReentrancyGuard};

/// Fixed-point precision used by the per-share reward accumulator.
///
/// A value of 1e12 keeps `amount * PRECISION` within `u128` for amounts up to
/// roughly 3.4e26 while still leaving ample precision for proportional splits.
const PRECISION: u128 = 1_000_000_000_000;

/// Rental yield distribution for tokenized property owners.
///
/// Each property tracks a number of shares. Rent is deposited into a property
/// pool and distributed with a per-share accumulator, so the cost of a deposit
/// does not grow with the number of owners. Holders accrue yield in proportion
/// to their shares and pull it with [`RentalYield::claim`].
#[ink::contract]
pub mod rental_yield {
    use super::*;

    /// Errors returned by the contract.
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Caller does not hold a role that permits the operation.
        Unauthorized,
        /// An account argument was the zero address.
        ZeroAddress,
        /// A property with this id has already been registered.
        PropertyAlreadyExists,
        /// No property is registered under this id.
        PropertyNotFound,
        /// The property has no shares allocated, so rent cannot be split.
        NoSharesAllocated,
        /// The requested share total would exceed the property's total supply.
        InsufficientShares,
        /// A zero amount was supplied where a positive amount is required.
        InvalidAmount,
        /// There is no yield available for this holder.
        NoYield,
        /// State-changing operation rejected while the contract is paused.
        ContractPaused,
        /// The contract is already in the requested pause state.
        AlreadyPaused,
        /// The contract is not paused.
        NotPaused,
        /// A protected function was re-entered.
        ReentrantCall,
        /// An arithmetic operation overflowed or underflowed.
        ArithmeticOverflow,
        /// The native token transfer to the caller failed.
        TransferFailed,
    }

    impl From<ReentrancyError> for Error {
        fn from(_: ReentrancyError) -> Self {
            Error::ReentrantCall
        }
    }

    /// Per-property yield accounting.
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct PropertyYield {
        /// Maximum number of shares that may be allocated.
        pub total_shares: u128,
        /// Number of shares currently allocated to holders.
        pub allocated_shares: u128,
        /// Cumulative rent per share, scaled by [`PRECISION`].
        pub reward_per_share: u128,
        /// Lifetime rent deposited for this property.
        pub deposited: u128,
        /// Lifetime rent claimed by holders of this property.
        pub claimed: u128,
        /// Rounding dust that was not attributable to any share.
        pub remainder: u128,
    }

    #[ink(event)]
    pub struct PropertyRegistered {
        #[ink(topic)]
        pub property_id: u64,
        pub total_shares: u128,
    }

    #[ink(event)]
    pub struct SharesUpdated {
        #[ink(topic)]
        pub property_id: u64,
        #[ink(topic)]
        pub holder: AccountId,
        pub shares: u128,
    }

    #[ink(event)]
    pub struct RentDeposited {
        #[ink(topic)]
        pub property_id: u64,
        #[ink(topic)]
        pub depositor: AccountId,
        pub amount: u128,
        pub reward_per_share: u128,
    }

    #[ink(event)]
    pub struct YieldClaimed {
        #[ink(topic)]
        pub property_id: u64,
        #[ink(topic)]
        pub holder: AccountId,
        pub amount: u128,
    }

    #[ink(event)]
    pub struct Paused {
        #[ink(topic)]
        pub by: AccountId,
    }

    #[ink(event)]
    pub struct Unpaused {
        #[ink(topic)]
        pub by: AccountId,
    }

    #[ink(storage)]
    pub struct RentalYield {
        admin: AccountId,
        access_control: AccessControl,
        reentrancy_guard: ReentrancyGuard,
        paused: bool,
        properties: Mapping<u64, PropertyYield>,
        shares: Mapping<(u64, AccountId), u128>,
        reward_debt: Mapping<(u64, AccountId), u128>,
        pending: Mapping<(u64, AccountId), u128>,
        total_deposited: u128,
        total_claimed: u128,
    }

    impl RentalYield {
        /// Deploy the contract. The caller becomes the admin and receives the
        /// `SuperAdmin` and `Admin` roles through the shared access-control
        /// primitive.
        #[allow(clippy::new_without_default)]
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            let block_number = Self::env().block_number();
            let timestamp = Self::env().block_timestamp();

            let mut access_control =
                AccessControl::new(64).expect("cache TTL is non-zero; infallible");
            access_control
                .bootstrap(caller, block_number, timestamp)
                .expect("constructor caller is non-zero; infallible");

            Self {
                admin: caller,
                access_control,
                reentrancy_guard: ReentrancyGuard::new(),
                paused: false,
                properties: Mapping::default(),
                shares: Mapping::default(),
                reward_debt: Mapping::default(),
                pending: Mapping::default(),
                total_deposited: 0,
                total_claimed: 0,
            }
        }

        // ── Views ────────────────────────────────────────────────────────────

        /// Contract admin (set at deployment).
        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
        }

        /// Whether state-changing yield operations are currently paused.
        #[ink(message)]
        pub fn is_paused(&self) -> bool {
            self.paused
        }

        /// Lifetime rent deposited across every property.
        #[ink(message)]
        pub fn total_deposited(&self) -> u128 {
            self.total_deposited
        }

        /// Lifetime rent claimed across every property.
        #[ink(message)]
        pub fn total_claimed(&self) -> u128 {
            self.total_claimed
        }

        /// Yield accounting for a property, or `None` if it is not registered.
        #[ink(message)]
        pub fn get_property(&self, property_id: u64) -> Option<PropertyYield> {
            self.properties.get(property_id)
        }

        /// Shares held by `holder` in a property.
        #[ink(message)]
        pub fn shares_of(&self, property_id: u64, holder: AccountId) -> u128 {
            self.shares.get((property_id, holder)).unwrap_or(0)
        }

        /// Yield currently claimable by `holder` for a property.
        #[ink(message)]
        pub fn claimable(&self, property_id: u64, holder: AccountId) -> u128 {
            let Some(property) = self.properties.get(property_id) else {
                return 0;
            };
            let key = (property_id, holder);
            let shares = self.shares.get(key).unwrap_or(0);
            let debt = self.reward_debt.get(key).unwrap_or(0);
            let pending = self.pending.get(key).unwrap_or(0);

            let gross = shares
                .saturating_mul(property.reward_per_share)
                .saturating_div(PRECISION);
            pending.saturating_add(gross.saturating_sub(debt))
        }

        /// Whether `account` holds `role`, directly or through inheritance.
        #[ink(message)]
        pub fn has_role(&self, account: AccountId, role: Role) -> bool {
            self.access_control.has_role(account, role)
        }

        // ── Access control ───────────────────────────────────────────────────

        /// Grant `role` to `account`. Only an account holding the role's admin
        /// role (by default `Admin`) may call this.
        #[ink(message)]
        pub fn grant_role(&mut self, account: AccountId, role: Role) -> Result<(), Error> {
            Self::ensure_non_zero(account)?;
            let caller = self.env().caller();
            self.access_control
                .grant_role(
                    caller,
                    account,
                    role,
                    self.env().block_number(),
                    self.env().block_timestamp(),
                )
                .map_err(|_| Error::Unauthorized)
        }

        /// Revoke `role` from `account`. Only an account holding the role's
        /// admin role may call this.
        #[ink(message)]
        pub fn revoke_role(&mut self, account: AccountId, role: Role) -> Result<(), Error> {
            Self::ensure_non_zero(account)?;
            let caller = self.env().caller();
            self.access_control
                .revoke_role(
                    caller,
                    account,
                    role,
                    self.env().block_number(),
                    self.env().block_timestamp(),
                )
                .map_err(|_| Error::Unauthorized)
        }

        // ── Administration ───────────────────────────────────────────────────

        /// Register a property and its share supply. Requires the `Manager`
        /// role (admins inherit it).
        #[ink(message)]
        pub fn register_property(
            &mut self,
            property_id: u64,
            total_shares: u128,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            self.ensure_operator(caller)?;
            if total_shares == 0 {
                return Err(Error::InvalidAmount);
            }
            if self.properties.contains(property_id) {
                return Err(Error::PropertyAlreadyExists);
            }

            self.properties.insert(
                property_id,
                &PropertyYield {
                    total_shares,
                    allocated_shares: 0,
                    reward_per_share: 0,
                    deposited: 0,
                    claimed: 0,
                    remainder: 0,
                },
            );

            self.env().emit_event(PropertyRegistered {
                property_id,
                total_shares,
            });
            Ok(())
        }

        /// Set (or replace) a holder's shares in a registered property.
        /// Requires the `Manager` role.
        ///
        /// Any yield accrued before the change is settled first, so the new
        /// share count only applies to future deposits.
        #[ink(message)]
        pub fn set_shares(
            &mut self,
            property_id: u64,
            holder: AccountId,
            shares: u128,
        ) -> Result<(), Error> {
            Self::ensure_non_zero(holder)?;
            let caller = self.env().caller();
            self.ensure_operator(caller)?;

            let mut property = self
                .properties
                .get(property_id)
                .ok_or(Error::PropertyNotFound)?;
            let key = (property_id, holder);

            self.settle(property_id, holder, &property)?;

            let previous = self.shares.get(key).unwrap_or(0);
            let without_previous = property
                .allocated_shares
                .checked_sub(previous)
                .ok_or(Error::ArithmeticOverflow)?;
            let new_allocated = without_previous
                .checked_add(shares)
                .ok_or(Error::ArithmeticOverflow)?;
            if new_allocated > property.total_shares {
                return Err(Error::InsufficientShares);
            }

            let debt = shares
                .checked_mul(property.reward_per_share)
                .ok_or(Error::ArithmeticOverflow)?
                .checked_div(PRECISION)
                .ok_or(Error::ArithmeticOverflow)?;

            self.shares.insert(key, &shares);
            self.reward_debt.insert(key, &debt);
            property.allocated_shares = new_allocated;
            self.properties.insert(property_id, &property);

            self.env().emit_event(SharesUpdated {
                property_id,
                holder,
                shares,
            });
            Ok(())
        }

        /// Pause deposits and claims. Requires the `PauseGuardian` role (admins
        /// inherit it).
        #[ink(message)]
        pub fn pause(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            self.ensure_guardian(caller)?;
            if self.paused {
                return Err(Error::AlreadyPaused);
            }
            self.paused = true;
            self.env().emit_event(Paused { by: caller });
            Ok(())
        }

        /// Resume deposits and claims. Requires the `PauseGuardian` role.
        #[ink(message)]
        pub fn unpause(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            self.ensure_guardian(caller)?;
            if !self.paused {
                return Err(Error::NotPaused);
            }
            self.paused = false;
            self.env().emit_event(Unpaused { by: caller });
            Ok(())
        }

        // ── Yield flows ──────────────────────────────────────────────────────

        /// Deposit the attached native value as rent for `property_id`.
        ///
        /// Requires the `Manager` role. The deposit is split across the
        /// property's currently allocated shares and rounded down per share;
        /// the leftover remains in the contract's `remainder` bucket rather
        /// than being credited to anyone.
        #[ink(message, payable)]
        pub fn deposit_rent(&mut self, property_id: u64) -> Result<u128, Error> {
            let caller = self.env().caller();
            self.ensure_operator(caller)?;
            self.ensure_not_paused()?;

            let amount = self.env().transferred_value();
            if amount == 0 {
                return Err(Error::InvalidAmount);
            }

            let mut property = self
                .properties
                .get(property_id)
                .ok_or(Error::PropertyNotFound)?;
            if property.allocated_shares == 0 {
                return Err(Error::NoSharesAllocated);
            }

            let scaled = amount
                .checked_mul(PRECISION)
                .ok_or(Error::ArithmeticOverflow)?;
            let delta = scaled
                .checked_div(property.allocated_shares)
                .ok_or(Error::ArithmeticOverflow)?;
            let distributed = delta
                .checked_mul(property.allocated_shares)
                .ok_or(Error::ArithmeticOverflow)?
                .checked_div(PRECISION)
                .ok_or(Error::ArithmeticOverflow)?;
            let remainder = amount
                .checked_sub(distributed)
                .ok_or(Error::ArithmeticOverflow)?;

            property.reward_per_share = property
                .reward_per_share
                .checked_add(delta)
                .ok_or(Error::ArithmeticOverflow)?;
            property.deposited = property
                .deposited
                .checked_add(amount)
                .ok_or(Error::ArithmeticOverflow)?;
            property.remainder = property
                .remainder
                .checked_add(remainder)
                .ok_or(Error::ArithmeticOverflow)?;
            self.total_deposited = self
                .total_deposited
                .checked_add(amount)
                .ok_or(Error::ArithmeticOverflow)?;
            self.properties.insert(property_id, &property);

            self.env().emit_event(RentDeposited {
                property_id,
                depositor: caller,
                amount,
                reward_per_share: property.reward_per_share,
            });
            Ok(amount)
        }

        /// Claim the caller's accrued yield for `property_id`.
        ///
        /// The pending balance is settled and zeroed, then the native token is
        /// transferred. The shared reentrancy guard protects the external call.
        #[ink(message)]
        pub fn claim(&mut self, property_id: u64) -> Result<u128, Error> {
            self.ensure_not_paused()?;

            non_reentrant!(self, {
                let caller = self.env().caller();
                let key = (property_id, caller);
                let mut property = self
                    .properties
                    .get(property_id)
                    .ok_or(Error::PropertyNotFound)?;

                self.settle(property_id, caller, &property)?;

                let amount = self.pending.get(key).unwrap_or(0);
                if amount == 0 {
                    return Err(Error::NoYield);
                }

                let new_claimed = property
                    .claimed
                    .checked_add(amount)
                    .ok_or(Error::ArithmeticOverflow)?;
                let new_total_claimed = self
                    .total_claimed
                    .checked_add(amount)
                    .ok_or(Error::ArithmeticOverflow)?;

                // Effects are prepared before the interaction; the guard
                // prevents the recipient from re-entering this message.
                self.env()
                    .transfer(caller, amount)
                    .map_err(|_| Error::TransferFailed)?;

                self.pending.insert(key, &0);
                property.claimed = new_claimed;
                self.properties.insert(property_id, &property);
                self.total_claimed = new_total_claimed;

                self.env().emit_event(YieldClaimed {
                    property_id,
                    holder: caller,
                    amount,
                });
                Ok(amount)
            })
        }

        // ── Internal helpers ─────────────────────────────────────────────────

        fn ensure_non_zero(account: AccountId) -> Result<(), Error> {
            if account == AccountId::from([0u8; 32]) {
                return Err(Error::ZeroAddress);
            }
            Ok(())
        }

        fn ensure_operator(&self, caller: AccountId) -> Result<(), Error> {
            if self.access_control.has_role(caller, Role::Manager) {
                Ok(())
            } else {
                Err(Error::Unauthorized)
            }
        }

        fn ensure_guardian(&self, caller: AccountId) -> Result<(), Error> {
            if self.access_control.has_role(caller, Role::PauseGuardian) {
                Ok(())
            } else {
                Err(Error::Unauthorized)
            }
        }

        fn ensure_not_paused(&self) -> Result<(), Error> {
            if self.paused {
                Err(Error::ContractPaused)
            } else {
                Ok(())
            }
        }

        /// Move a holder's accrued yield into `pending` and reset their debt to
        /// the current accumulator value.
        fn settle(
            &mut self,
            property_id: u64,
            holder: AccountId,
            property: &PropertyYield,
        ) -> Result<(), Error> {
            let key = (property_id, holder);
            let shares = self.shares.get(key).unwrap_or(0);
            let debt = self.reward_debt.get(key).unwrap_or(0);
            let pending = self.pending.get(key).unwrap_or(0);

            let gross = shares
                .checked_mul(property.reward_per_share)
                .ok_or(Error::ArithmeticOverflow)?
                .checked_div(PRECISION)
                .ok_or(Error::ArithmeticOverflow)?;
            let accrued = gross.checked_sub(debt).ok_or(Error::ArithmeticOverflow)?;
            let new_pending = pending
                .checked_add(accrued)
                .ok_or(Error::ArithmeticOverflow)?;

            self.pending.insert(key, &new_pending);
            self.reward_debt.insert(key, &gross);
            Ok(())
        }
    }
}
