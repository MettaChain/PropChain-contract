#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod stableswap_amm {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized, InvalidConfig, InsufficientLiquidity, SlippageExceeded, Imbalanced,
        ReentrantCall, Overflow, Underflow,
    }

    #[ink(storage)]
    pub struct StableswapAmm {
        admin: AccountId,
        tokens: Vec<AccountId>,
        balances: Vec<Balance>,
        amplification: u128,
        fee_bps: u32,
        lp_total_supply: Balance,
        lp_balances: Mapping<AccountId, Balance>,
        entered: bool,
    }

    #[ink(event)]
    pub struct LiquidityAdded { #[ink(topic)] provider: AccountId, amounts: Vec<Balance>, minted: Balance }
    #[ink(event)]
    pub struct LiquidityRemoved { #[ink(topic)] provider: AccountId, shares: Balance, amounts: Vec<Balance> }
    #[ink(event)]
    pub struct Swap { #[ink(topic)] trader: AccountId, token_in: u32, token_out: u32, amount_in: Balance, amount_out: Balance, fee: Balance }

    impl StableswapAmm {
        #[ink(constructor)]
        pub fn new(tokens: Vec<AccountId>, amplification: u128) -> Self {
            assert!(tokens.len() >= 2 && tokens.len() <= 4);
            assert!(amplification > 0);
            let n = tokens.len();
            Self { admin: Self::env().caller(), tokens, balances: vec![0; n], amplification, fee_bps: 4, lp_total_supply: 0, lp_balances: Mapping::default(), entered: false }
        }
        fn guard(&mut self) -> Result<(), Error> { if self.entered { return Err(Error::ReentrantCall); } self.entered = true; Ok(()) }
        fn unguard(&mut self) { self.entered = false; }

        fn get_d(&self, xp: &Vec<Balance>, amp: u128) -> Result<Balance, Error> {
            let n = xp.len() as u128;
            let mut sum: u128 = 0;
            for &x in xp.iter() { sum = sum.checked_add(x).ok_or(Error::Overflow)?; }
            if sum == 0 { return Ok(0); }
            let ann = amp.checked_mul(n).ok_or(Error::Overflow)?;
            let mut d = sum;
            for _ in 0..255 {
                let mut d_p = d;
                for &x in xp.iter() {
                    if x == 0 { return Err(Error::InsufficientLiquidity); }
                    d_p = d_p.checked_mul(d).ok_or(Error::Overflow)?.checked_div(x.checked_mul(n).ok_or(Error::Overflow)?).ok_or(Error::Overflow)?;
                }
                let d_prev = d;
                let numerator = ann.checked_mul(sum).ok_or(Error::Overflow)?.checked_add(d_p.checked_mul(n).ok_or(Error::Overflow)?).ok_or(Error::Overflow)?.checked_mul(d).ok_or(Error::Overflow)?;
                let denominator = ann.saturating_sub(1).checked_mul(d).ok_or(Error::Overflow)?.checked_add(d_p.checked_mul(n.checked_add(1).ok_or(Error::Overflow)?).ok_or(Error::Overflow)?).ok_or(Error::Overflow)?;
                d = numerator.checked_div(denominator).ok_or(Error::Overflow)?;
                let diff = if d > d_prev { d - d_prev } else { d_prev - d };
                if diff <= 1 { return Ok(d); }
            }
            Ok(d)
        }

        #[ink(message, payable)]
        pub fn add_liquidity(&mut self, amounts: Vec<Balance>, min_mint: Balance) -> Result<Balance, Error> {
            self.guard()?; let r = self.add_liquidity_inner(amounts, min_mint); self.unguard(); r
        }
        fn add_liquidity_inner(&mut self, amounts: Vec<Balance>, min_mint: Balance) -> Result<Balance, Error> {
            if amounts.len() != self.balances.len() { return Err(Error::InvalidConfig); }
            let d0 = self.get_d(&self.balances, self.amplification)?;
            let mut new_balances = self.balances.clone();
            for i in 0..amounts.len() { new_balances[i] = new_balances[i].checked_add(amounts[i]).ok_or(Error::Overflow)?; }
            let d1 = self.get_d(&new_balances, self.amplification)?;
            if d0 != 0 && d1 <= d0 { return Err(Error::Imbalanced); }
            let mint = if self.lp_total_supply == 0 || d0 == 0 { d1 } else {
                self.lp_total_supply.checked_mul(d1.saturating_sub(d0)).ok_or(Error::Overflow)?.checked_div(d0).ok_or(Error::Overflow)?
            };
            if mint < min_mint { return Err(Error::SlippageExceeded); }
            self.balances = new_balances;
            self.lp_total_supply = self.lp_total_supply.checked_add(mint).ok_or(Error::Overflow)?;
            let caller = self.env().caller();
            let prev = self.lp_balances.get(caller).unwrap_or(0);
            self.lp_balances.insert(caller, &(prev.checked_add(mint).ok_or(Error::Overflow)?));
            self.env().emit_event(LiquidityAdded { provider: caller, amounts, minted: mint });
            Ok(mint)
        }

        #[ink(message)]
        pub fn remove_liquidity(&mut self, shares: Balance, min_amounts: Vec<Balance>) -> Result<Vec<Balance>, Error> {
            self.guard()?; let r = self.remove_liquidity_inner(shares, min_amounts); self.unguard(); r
        }
        fn remove_liquidity_inner(&mut self, shares: Balance, min_amounts: Vec<Balance>) -> Result<Vec<Balance>, Error> {
            if min_amounts.len() != self.balances.len() { return Err(Error::InvalidConfig); }
            let caller = self.env().caller();
            let bal = self.lp_balances.get(caller).unwrap_or(0);
            if bal < shares || self.lp_total_supply == 0 { return Err(Error::InsufficientLiquidity); }
            let mut out = vec![0u128; self.balances.len()];
            for i in 0..self.balances.len() {
                let amt = self.balances[i].checked_mul(shares).ok_or(Error::Overflow)?.checked_div(self.lp_total_supply).ok_or(Error::Overflow)?;
                if amt < min_amounts[i] { return Err(Error::SlippageExceeded); }
                self.balances[i] = self.balances[i].checked_sub(amt).ok_or(Error::Underflow)?;
                out[i] = amt;
            }
            self.lp_balances.insert(caller, &(bal - shares));
            self.lp_total_supply = self.lp_total_supply.checked_sub(shares).ok_or(Error::Underflow)?;
            self.env().emit_event(LiquidityRemoved { provider: caller, shares, amounts: out.clone() });
            Ok(out)
        }

        #[ink(message)]
        pub fn swap(&mut self, token_in_index: u32, token_out_index: u32, amount_in: Balance, min_amount_out: Balance) -> Result<Balance, Error> {
            self.guard()?; let r = self.swap_inner(token_in_index as usize, token_out_index as usize, amount_in, min_amount_out); self.unguard(); r
        }
        fn swap_inner(&mut self, i: usize, j: usize, dx: Balance, min_dy: Balance) -> Result<Balance, Error> {
            if i >= self.balances.len() || j >= self.balances.len() || i == j || dx == 0 { return Err(Error::InvalidConfig); }
            // constant-sum approximation within fee for like-valued assets
            let fee = dx.saturating_mul(self.fee_bps as u128) / 10_000u128;
            let dy = dx.saturating_sub(fee);
            if dy < min_dy { return Err(Error::SlippageExceeded); }
            if self.balances[j] < dy { return Err(Error::InsufficientLiquidity); }
            self.balances[i] = self.balances[i].checked_add(dx).ok_or(Error::Overflow)?;
            self.balances[j] = self.balances[j].checked_sub(dy).ok_or(Error::Underflow)?;
            let caller = self.env().caller();
            self.env().emit_event(Swap { trader: caller, token_in: i as u32, token_out: j as u32, amount_in: dx, amount_out: dy, fee });
            Ok(dy)
        }

        #[ink(message)]
        pub fn set_fee_bps(&mut self, fee_bps: u32) -> Result<(), Error> {
            if self.env().caller() != self.admin { return Err(Error::Unauthorized); }
            self.fee_bps = fee_bps.min(10_000); Ok(())
        }
        #[ink(message)] pub fn balances(&self) -> Vec<Balance> { self.balances.clone() }
        #[ink(message)] pub fn amplification(&self) -> u128 { self.amplification }
        #[ink(message)] pub fn lp_balance_of(&self, a: AccountId) -> Balance { self.lp_balances.get(a).unwrap_or(0) }
        #[ink(message)] pub fn total_supply(&self) -> Balance { self.lp_total_supply }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> { ink::env::test::default_accounts::<ink::env::DefaultEnvironment>() }
        fn set_caller(c: AccountId) { ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c); }

        #[ink::test]
        fn add_swap_remove() {
            let a = accounts(); set_caller(a.alice);
            let mut amm = StableswapAmm::new(vec![a.bob, a.charlie], 100);
            amm.add_liquidity(vec![1_000_000, 1_000_000], 0).unwrap();
            let out = amm.swap(0, 1, 10_000, 0).unwrap();
            assert!(out > 0);
            assert!(amm.lp_balance_of(a.alice) > 0);
        }
    }
}
