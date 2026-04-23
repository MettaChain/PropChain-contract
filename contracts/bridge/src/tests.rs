// Unit tests for the bridge contract (Issue #101 - extracted from lib.rs)

#[cfg(test)]
mod tests {
    use super::*;
    use ink::env::{test, DefaultEnvironment};

    fn setup_bridge() -> PropertyBridge {
        let supported_chains = vec![1, 2, 3];
        PropertyBridge::new(supported_chains, 2, 5, 100, 500000)
    }

    #[ink::test]
    fn test_constructor_works() {
        let bridge = setup_bridge();
        let config = bridge.get_config();
        assert_eq!(config.min_signatures_required, 2);
        assert_eq!(config.max_signatures_required, 5);
    }

    #[ink::test]
    fn test_initiate_bridge_multisig() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let metadata = PropertyMetadata {
            location: String::from("Test Property"),
            size: 1000,
            legal_description: String::from("Test"),
            valuation: 100000,
            documents_url: String::from("ipfs://test"),
        };

        let result = bridge.initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(50), metadata);
        assert!(result.is_ok());
    }

    #[ink::test]
    fn test_sign_bridge_request() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let metadata = PropertyMetadata {
            location: String::from("Test Property"),
            size: 1000,
            legal_description: String::from("Test"),
            valuation: 100000,
            documents_url: String::from("ipfs://test"),
        };

        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(50), metadata)
            .expect("Bridge initiation should succeed in test");

        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = bridge.sign_bridge_request(request_id, true);
        assert!(result.is_ok());
    }

    #[ink::test]
    fn test_cross_chain_trade_lifecycle() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.bob);

        let trade_id = bridge
            .register_cross_chain_trade(9, Some(7), 2, accounts.charlie, 50_000, 49_000)
            .expect("cross-chain trade registration should succeed");
        let trade = bridge
            .get_cross_chain_trade(trade_id)
            .expect("trade should be stored");
        assert_eq!(trade.status, CrossChainTradeStatus::Pending);
        assert_eq!(trade.destination_chain, 2);

        bridge
            .attach_bridge_request_to_trade(trade_id, 33)
            .expect("trader can attach bridge request");
        let attached = bridge
            .get_cross_chain_trade(trade_id)
            .expect("attached trade should exist");
        assert_eq!(attached.bridge_request_id, Some(33));

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .settle_cross_chain_trade(trade_id)
            .expect("admin can settle trade");
        let settled = bridge
            .get_cross_chain_trade(trade_id)
            .expect("settled trade should exist");
        assert_eq!(settled.status, CrossChainTradeStatus::Settled);
    }

    #[ink::test]
    fn test_estimate_bridge_gas_respects_chain_profile() {
        let mut bridge = setup_bridge();

        let default_gas = bridge
            .estimate_bridge_gas(1, 2)
            .expect("default chain should be estimable");

        let tuned_chain = ChainBridgeInfo {
            chain_id: 2,
            chain_name: String::from("High-Confirmation"),
            bridge_contract_address: None,
            is_active: true,
            gas_multiplier: 180,
            confirmation_blocks: 24,
            supported_tokens: Vec::new(),
        };
        bridge
            .update_chain_info(2, tuned_chain)
            .expect("admin should update chain profile");

        let updated_gas = bridge
            .estimate_bridge_gas(1, 2)
            .expect("updated chain should be estimable");

        assert!(updated_gas > default_gas);
        assert!(updated_gas <= bridge.get_config().gas_limit_per_bridge);
    }

    #[ink::test]
    fn test_quote_cross_chain_trade_scales_with_amount() {
        let bridge = setup_bridge();

        let small = bridge
            .quote_cross_chain_trade(2, 50_000)
            .expect("small quote should succeed");
        let large = bridge
            .quote_cross_chain_trade(2, 100_000)
            .expect("large quote should succeed");

        assert!(small.total_fee >= small.protocol_fee);
        assert!(large.total_fee > small.total_fee);
        assert!(large.protocol_fee > small.protocol_fee);
    }
}
