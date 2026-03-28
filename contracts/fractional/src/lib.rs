#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod fractional {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PortfolioItem {
        pub token_id: u64,
        pub shares: u128,
        pub price_per_share: u128,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PortfolioAggregation {
        pub total_value: u128,
        pub positions: Vec<(u64, u128, u128)>,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct TaxReport {
        pub total_dividends: u128,
        pub total_proceeds: u128,
        pub transactions: u64,
    }

    #[ink(storage)]
    pub struct Fractional {
        last_prices: Mapping<u64, u128>,
    }

    impl Fractional {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                last_prices: Mapping::default(),
            }
        }
    }

    impl Default for Fractional {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Fractional {
        #[ink(message)]
        pub fn set_last_price(&mut self, token_id: u64, price_per_share: u128) {
            self.last_prices.insert(token_id, &price_per_share);
        }

        #[ink(message)]
        pub fn get_last_price(&self, token_id: u64) -> Option<u128> {
            self.last_prices.get(token_id)
        }

        #[ink(message)]
        pub fn aggregate_portfolio(&self, items: Vec<PortfolioItem>) -> PortfolioAggregation {
            let mut total: u128 = 0;
            let mut positions: Vec<(u64, u128, u128)> = Vec::new();
            for it in items.iter() {
                let price = if it.price_per_share > 0 {
                    it.price_per_share
                } else {
                    self.last_prices.get(it.token_id).unwrap_or(0)
                };
                let value = price.saturating_mul(it.shares);
                total = total.saturating_add(value);
                positions.push((it.token_id, it.shares, price));
            }
            PortfolioAggregation {
                total_value: total,
                positions,
            }
        }

        #[ink(message)]
        pub fn summarize_tax(
            &self,
            dividends: Vec<(u64, u128)>,
            proceeds: Vec<(u64, u128)>,
        ) -> TaxReport {
            let mut total_dividends: u128 = 0;
            for d in dividends.iter() {
                total_dividends = total_dividends.saturating_add(d.1);
            }
            let mut total_proceeds: u128 = 0;
            for p in proceeds.iter() {
                total_proceeds = total_proceeds.saturating_add(p.1);
            }
            TaxReport {
                total_dividends,
                total_proceeds,
                transactions: (dividends.len() + proceeds.len()) as u64,
            }
        }
    }
}
