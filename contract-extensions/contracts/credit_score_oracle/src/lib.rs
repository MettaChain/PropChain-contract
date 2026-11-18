#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod credit_score_oracle {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    const STALE_BLOCKS: u64 = 28_800;

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct ScoreRecord {
        pub score: u32,
        pub staking_component: u32,
        pub governance_component: u32,
        pub repayment_component: u32,
        pub collateral_component: u32,
        pub components_hash: Hash,
        pub submitted_at: BlockNumber,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error { Unauthorized, InvalidScore, NotFound }

    #[ink(storage)]
    pub struct CreditScoreOracle {
        scorekeepers: Mapping<AccountId, bool>,
        admin: AccountId,
        scores: Mapping<AccountId, ScoreRecord>,
        stale_blocks: u64,
    }

    #[ink(event)]
    pub struct ScoreUpdated { #[ink(topic)] account: AccountId, #[ink(topic)] scorekeeper: AccountId, score: u32 }

    impl CreditScoreOracle {
        #[ink(constructor)]
        pub fn new(scorekeepers: Vec<AccountId>) -> Self {
            let mut map = Mapping::default();
            for keeper in scorekeepers { map.insert(keeper, &true); }
            Self { scorekeepers: map, admin: Self::env().caller(), scores: Mapping::default(), stale_blocks: STALE_BLOCKS }
        }

        fn only_scorekeeper(&self) -> Result<(), Error> {
            if !self.scorekeepers.get(self.env().caller()).unwrap_or(false) { return Err(Error::Unauthorized); }
            Ok(())
        }

        #[ink(message)]
        pub fn submit_score(&mut self, account: AccountId, score: u32, components_hash: Hash) -> Result<(), Error> {
            self.only_scorekeeper()?;
            if score > 1000 { return Err(Error::InvalidScore); }
            let staking = score * 30 / 100;
            let governance = score * 20 / 100;
            let repayment = score * 30 / 100;
            let collateral = score * 20 / 100;
            self.scores.insert(account, &ScoreRecord {
                score, staking_component: staking, governance_component: governance, repayment_component: repayment,
                collateral_component: collateral, components_hash, submitted_at: self.env().block_number(),
            });
            self.env().emit_event(ScoreUpdated { account, scorekeeper: self.env().caller(), score });
            Ok(())
        }

        #[ink(message)]
        pub fn get_score(&self, account: AccountId) -> Option<(u32, BlockNumber)> {
            let rec = self.scores.get(account)?;
            let age = self.env().block_number().saturating_sub(rec.submitted_at);
            let degraded = if age > self.stale_blocks {
                let excess = age - self.stale_blocks;
                let penalty = (excess as u32) / 10;
                rec.score.saturating_sub(penalty.min(rec.score))
            } else { rec.score };
            Some((degraded, rec.submitted_at))
        }

        #[ink(message)]
        pub fn get_score_components(&self, account: AccountId) -> Option<(u32, u32, u32, u32)> {
            let rec = self.scores.get(account)?;
            Some((rec.staking_component, rec.governance_component, rec.repayment_component, rec.collateral_component))
        }

        #[ink(message)] pub fn is_scorekeeper(&self, account: AccountId) -> bool { self.scorekeepers.get(account).unwrap_or(false) }
        #[ink(message)] pub fn set_scorekeeper(&mut self, account: AccountId, enabled: bool) -> Result<(), Error> {
            if self.env().caller() != self.admin { return Err(Error::Unauthorized); }
            self.scorekeepers.insert(account, &enabled); Ok(())
        }
        #[ink(message)] pub fn set_stale_blocks(&mut self, blocks: u64) -> Result<(), Error> {
            if self.env().caller() != self.admin { return Err(Error::Unauthorized); }
            self.stale_blocks = blocks; Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> { ink::env::test::default_accounts::<ink::env::DefaultEnvironment>() }
        fn set_caller(c: AccountId) { ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c); }
        fn set_block(b: BlockNumber) { ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(b); }

        #[ink::test]
        fn submit_breakdown_and_decay() {
            let a = accounts(); set_caller(a.alice); set_block(10);
            let mut o = CreditScoreOracle::new(vec![a.alice]);
            o.submit_score(a.bob, 800, Hash::from([7u8;32])).unwrap();
            assert_eq!(o.get_score(a.bob).unwrap().0, 800);
            let (s,g,r,c) = o.get_score_components(a.bob).unwrap();
            assert_eq!(s+g+r+c, 800);
            set_block(10 + STALE_BLOCKS + 100);
            assert!(o.get_score(a.bob).unwrap().0 < 800);
        }
    }
}
