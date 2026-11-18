#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod dex_multihop_router {
    use ink::env::call::{build_call, ExecutionInput, Selector};
    use ink::env::DefaultEnvironment;
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        InvalidPath,
        SlippageExceeded,
        DeadlineExceeded,
        PoolNotFound,
        SwapFailed,
        ReentrantCall,
    }

    #[ink(storage)]
    pub struct DexMultihopRouter {
        admin: AccountId,
        factory: AccountId,
        /// pair (token_a, token_b) -> pool address (ordered by encoded bytes for simplicity caller must be consistent)
        pools: Mapping<(AccountId, AccountId), AccountId>,
        entered: bool,
    }

    #[ink(event)]
    pub struct SwapExecuted {
        #[ink(topic)]
        sender: AccountId,
        path: Vec<AccountId>,
        amount_in: Balance,
        amount_out: Balance,
    }

    impl DexMultihopRouter {
        #[ink(constructor)]
        pub fn new(factory: AccountId) -> Self {
            Self {
                admin: Self::env().caller(),
                factory,
                pools: Mapping::default(),
                entered: false,
            }
        }

        fn guard(&mut self) -> Result<(), Error> {
            if self.entered { return Err(Error::ReentrantCall); }
            self.entered = true;
            Ok(())
        }
        fn unguard(&mut self) { self.entered = false; }

        #[ink(message)]
        pub fn register_pool(&mut self, token_a: AccountId, token_b: AccountId, pool: AccountId) -> Result<(), Error> {
            if self.env().caller() != self.admin { return Err(Error::Unauthorized); }
            self.pools.insert((token_a, token_b), &pool);
            self.pools.insert((token_b, token_a), &pool);
            Ok(())
        }

        fn pool_for(&self, a: AccountId, b: AccountId) -> Option<AccountId> {
            self.pools.get((a, b))
        }

        /// Calls pool.get_amount_out(amount_in, token_in, token_out) -> Balance
        fn get_amount_out_pool(&self, pool: AccountId, amount_in: Balance, token_in: AccountId, token_out: AccountId) -> Result<Balance, Error> {
            build_call::<DefaultEnvironment>()
                .call(pool)
                .transferred_value(0)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!("get_amount_out")))
                        .push_arg(amount_in)
                        .push_arg(token_in)
                        .push_arg(token_out),
                )
                .returns::<Balance>()
                .invoke()
                .map_err(|_| Error::PoolNotFound)
        }

        /// Calls pool.swap(amount_in, min_out, token_in, token_out, to) -> Balance
        fn swap_pool(&self, pool: AccountId, amount_in: Balance, min_out: Balance, token_in: AccountId, token_out: AccountId, to: AccountId) -> Result<Balance, Error> {
            build_call::<DefaultEnvironment>()
                .call(pool)
                .transferred_value(0)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!("swap")))
                        .push_arg(amount_in)
                        .push_arg(min_out)
                        .push_arg(token_in)
                        .push_arg(token_out)
                        .push_arg(to),
                )
                .returns::<Balance>()
                .invoke()
                .map_err(|_| Error::SwapFailed)
        }

        #[ink(message)]
        pub fn get_amounts_out(&self, amount_in: Balance, path: Vec<AccountId>) -> Result<Vec<Balance>, Error> {
            if path.len() < 2 { return Err(Error::InvalidPath); }
            let mut amounts: Vec<Balance> = Vec::with_capacity(path.len());
            amounts.push(amount_in);
            let mut i = 0;
            while i + 1 < path.len() {
                let a = path[i];
                let b = path[i + 1];
                let pool = self.pool_for(a, b).ok_or(Error::PoolNotFound)?;
                let out = self.get_amount_out_pool(pool, *amounts.last().unwrap_or(&0), a, b)?;
                amounts.push(out);
                i += 1;
            }
            Ok(amounts)
        }

        #[ink(message)]
        pub fn swap_exact_tokens_for_tokens(
            &mut self,
            amount_in: Balance,
            amount_out_min: Balance,
            path: Vec<AccountId>,
            deadline: BlockNumber,
        ) -> Result<Balance, Error> {
            self.guard()?;
            let res = self.swap_inner(amount_in, amount_out_min, path, deadline);
            self.unguard();
            res
        }

        fn swap_inner(
            &mut self,
            amount_in: Balance,
            amount_out_min: Balance,
            path: Vec<AccountId>,
            deadline: BlockNumber,
        ) -> Result<Balance, Error> {
            if self.env().block_number() > deadline {
                return Err(Error::DeadlineExceeded);
            }
            if path.len() < 2 {
                return Err(Error::InvalidPath);
            }
            let sender = self.env().caller();
            let amounts = self.get_amounts_out(amount_in, path.clone())?;
            let amount_out = *amounts.last().unwrap_or(&0);
            if amount_out < amount_out_min {
                return Err(Error::SlippageExceeded);
            }

            let mut i = 0;
            let mut current_in = amount_in;
            while i + 1 < path.len() {
                let a = path[i];
                let b = path[i + 1];
                let pool = self.pool_for(a, b).ok_or(Error::PoolNotFound)?;
                // intermediate min 0 except conceptual last; we already checked final slippage
                let min_out_hop = if i + 2 == path.len() { amount_out_min } else { 0 };
                current_in = self.swap_pool(pool, current_in, min_out_hop, a, b, sender)?;
                i += 1;
            }

            self.env().emit_event(SwapExecuted {
                sender,
                path,
                amount_in,
                amount_out,
            });
            Ok(amount_out)
        }

        #[ink(message)]
        pub fn factory(&self) -> AccountId { self.factory }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
        }
        fn set_caller(c: AccountId) { ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c); }
        fn set_block(b: BlockNumber) { ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(b); }

        #[ink::test]
        fn invalid_path_rejected() {
            let a = accounts();
            let r = DexMultihopRouter::new(a.alice);
            assert_eq!(r.get_amounts_out(100, vec![a.bob]), Err(Error::InvalidPath));
        }

        #[ink::test]
        fn register_pool_admin_only() {
            let a = accounts();
            set_caller(a.alice);
            let mut r = DexMultihopRouter::new(a.alice);
            assert!(r.register_pool(a.bob, a.charlie, a.django).is_ok());
            set_caller(a.bob);
            assert_eq!(r.register_pool(a.bob, a.charlie, a.django), Err(Error::Unauthorized));
        }

        #[ink::test]
        fn deadline_check() {
            let a = accounts();
            set_caller(a.alice);
            let mut r = DexMultihopRouter::new(a.alice);
            set_block(10);
            let res = r.swap_exact_tokens_for_tokens(100, 1, vec![a.bob, a.charlie], 5);
            assert_eq!(res, Err(Error::DeadlineExceeded));
        }
    }
}
