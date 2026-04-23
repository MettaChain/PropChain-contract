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

        // Register alice as a validator before signing (issue #203)
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.add_validator(accounts.alice).expect("admin can add validator");

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

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = bridge.sign_bridge_request(request_id, true);
        assert!(result.is_ok());
    }

    #[ink::test]
    fn test_non_validator_cannot_sign() {
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
            .expect("initiation should succeed");

        // bob is a bridge operator but NOT a validator — must be rejected
        bridge.add_bridge_operator(accounts.bob).expect("admin can add operator");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let result = bridge.sign_bridge_request(request_id, true);
        assert_eq!(result, Err(Error::Unauthorized));
    }

    #[ink::test]
    fn test_threshold_enforced_at_execution() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Register two validators
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.add_validator(accounts.alice).expect("add validator alice");
        bridge.add_validator(accounts.bob).expect("add validator bob");
        bridge.add_bridge_operator(accounts.bob).expect("add operator bob");

        let metadata = PropertyMetadata {
            location: String::from("Test Property"),
            size: 1000,
            legal_description: String::from("Test"),
            valuation: 100000,
            documents_url: String::from("ipfs://test"),
        };
        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.charlie, 2, Some(50), metadata)
            .expect("initiation should succeed");

        // Only one signature — execution must fail
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.sign_bridge_request(request_id, true).expect("alice signs");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = bridge.execute_bridge(request_id);
        assert_eq!(result, Err(Error::InvalidRequest)); // status not Locked yet

        // Second signature — now threshold met, execution succeeds
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge.sign_bridge_request(request_id, true).expect("bob signs");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = bridge.execute_bridge(request_id);
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
}
