//! Test Utilities and Fixtures for PropChain Contracts
//!
//! This module provides shared testing utilities, fixtures, and helpers
//! for all contract tests.

#![cfg(feature = "std")]

use ink::env::test::DefaultAccounts;
use ink::env::DefaultEnvironment;
use ink::primitives::AccountId;
use propchain_traits::*;

/// Test account identifiers
pub struct TestAccounts {
    pub alice: AccountId,
    pub bob: AccountId,
    pub charlie: AccountId,
    pub django: AccountId,
    pub eve: AccountId,
}

impl TestAccounts {
    /// Get default test accounts
    pub fn default() -> Self {
        let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
        Self {
            alice: accounts.alice,
            bob: accounts.bob,
            charlie: accounts.charlie,
            django: accounts.django,
            eve: accounts.eve,
        }
    }

    /// Get all accounts as a vector
    pub fn all(&self) -> Vec<AccountId> {
        vec![self.alice, self.bob, self.charlie, self.django, self.eve]
    }
}

/// Property metadata fixtures
pub struct PropertyMetadataFixtures;

impl PropertyMetadataFixtures {
    /// Create a minimal valid property metadata
    pub fn minimal() -> PropertyMetadata {
        PropertyMetadata {
            location: "123 Main St".to_string(),
            size: 1000,
            legal_description: "Test property".to_string(),
            valuation: 100_000,
            documents_url: "ipfs://test".to_string(),
        }
    }

    /// Create a standard property metadata
    pub fn standard() -> PropertyMetadata {
        PropertyMetadata {
            location: "123 Main St, City, State 12345".to_string(),
            size: 2500,
            legal_description: "Lot 123, Block 4, Subdivision XYZ".to_string(),
            valuation: 500_000,
            documents_url: "https://ipfs.io/ipfs/QmTest".to_string(),
        }
    }

    /// Create a large property metadata
    pub fn large() -> PropertyMetadata {
        PropertyMetadata {
            location: "456 Oak Avenue, Metropolitan City, State 67890".to_string(),
            size: 10_000,
            legal_description: "Large commercial property with extensive legal description"
                .to_string(),
            valuation: 5_000_000,
            documents_url: "https://ipfs.io/ipfs/QmLarge".to_string(),
        }
    }

    /// Create property metadata with custom values
    pub fn custom(
        location: String,
        size: u64,
        legal_description: String,
        valuation: u128,
        documents_url: String,
    ) -> PropertyMetadata {
        PropertyMetadata {
            location,
            size,
            legal_description,
            valuation,
            documents_url,
        }
    }

    /// Create property metadata with edge case values
    pub fn edge_cases() -> Vec<PropertyMetadata> {
        vec![
            // Minimum values
            PropertyMetadata {
                location: "A".to_string(),
                size: 1,
                legal_description: "X".to_string(),
                valuation: 1,
                documents_url: "ipfs://min".to_string(),
            },
            // Maximum reasonable values
            PropertyMetadata {
                location: "A".repeat(500),
                size: u64::MAX,
                legal_description: "X".repeat(5000),
                valuation: u128::MAX,
                documents_url: "ipfs://max".to_string(),
            },
            // Special characters
            PropertyMetadata {
                location: "123 Main St, 城市, État 12345".to_string(),
                size: 1000,
                legal_description: "Test with émojis 🏠 and unicode".to_string(),
                valuation: 100_000,
                documents_url: "ipfs://special".to_string(),
            },
        ]
    }
}

/// Test environment helpers
pub struct TestEnv;

impl TestEnv {
    /// Set the caller for the next contract call
    pub fn set_caller(caller: AccountId) {
        ink::env::test::set_caller::<DefaultEnvironment>(caller);
    }

    /// Set the block timestamp
    pub fn set_block_timestamp(timestamp: u64) {
        ink::env::test::set_block_timestamp::<DefaultEnvironment>(timestamp);
    }

    /// Set the transferred value for the next call
    pub fn set_transferred_value(value: u128) {
        ink::env::test::set_value_transferred::<DefaultEnvironment>(value);
    }

    /// Advance block timestamp by specified amount
    pub fn advance_time(seconds: u64) {
        let current = ink::env::block_timestamp::<DefaultEnvironment>();
        ink::env::test::set_block_timestamp::<DefaultEnvironment>(current + seconds);
    }

    /// Reset test environment
    pub fn reset() {
        let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
        ink::env::test::set_caller::<DefaultEnvironment>(accounts.alice);
        ink::env::test::set_block_timestamp::<DefaultEnvironment>(0);
        ink::env::test::set_value_transferred::<DefaultEnvironment>(0);
    }
}

/// Assertion helpers for common test patterns
pub mod assertions {
    use super::*;

    /// Assert that a result is an error with a specific error type
    pub fn assert_error<T, E: PartialEq + core::fmt::Debug>(
        result: Result<T, E>,
        expected_error: E,
    ) {
        match result {
            Ok(_) => panic!("Expected error {:?}, but got Ok", expected_error),
            Err(e) => assert_eq!(
                e, expected_error,
                "Expected error {:?}, but got {:?}",
                expected_error, e
            ),
        }
    }

    /// Assert that a result is Ok and return the value
    pub fn assert_ok<T, E: core::fmt::Debug>(result: Result<T, E>) -> T {
        result.unwrap_or_else(|e| panic!("Expected Ok, but got error: {:?}", e))
    }

    /// Assert that two AccountIds are equal
    pub fn assert_account_eq(actual: AccountId, expected: AccountId, message: &str) {
        assert_eq!(
            actual, expected,
            "{}: expected {:?}, got {:?}",
            message, expected, actual
        );
    }
}

/// Test data generators for property-based testing
pub mod generators {
    use super::*;

    /// Generate a random AccountId for testing
    pub fn random_account_id(seed: u8) -> AccountId {
        let mut bytes = [seed; 32];
        // Simple pseudo-random generation
        for i in 0..32 {
            bytes[i] = seed.wrapping_add(i as u8);
        }
        AccountId::from(bytes)
    }

    /// Generate property metadata with random valid values
    pub fn random_property_metadata(seed: u64) -> PropertyMetadata {
        PropertyMetadata {
            location: format!("Property at seed {}", seed),
            size: 1000 + (seed % 10000),
            legal_description: format!("Legal description for seed {}", seed),
            valuation: 100_000 + (seed as u128 * 1000),
            documents_url: format!("ipfs://seed-{}", seed),
        }
    }

    /// Generate a vector of property metadata
    pub fn generate_properties(count: usize) -> Vec<PropertyMetadata> {
        (0..count)
            .map(|i| random_property_metadata(i as u64))
            .collect()
    }
}

/// Performance testing utilities
pub mod performance {
    use super::*;

    /// Measure execution time of a function
    pub fn measure_time<F, T>(f: F) -> (T, u64)
    where
        F: FnOnce() -> T,
    {
        let start = ink::env::block_timestamp::<DefaultEnvironment>();
        let result = f();
        let end = ink::env::block_timestamp::<DefaultEnvironment>();
        (result, end.saturating_sub(start))
    }

    /// Benchmark a function multiple times
    pub fn benchmark<F, T>(iterations: u32, f: F) -> Vec<u64>
    where
        F: Fn() -> T,
    {
        (0..iterations)
            .map(|_| {
                let (_, time) = measure_time(|| f());
                time
            })
            .collect()
    }
}

#[cfg(test)]
mod test_utils_tests {
    use super::*;

    #[test]
    fn test_accounts_default() {
        let accounts = TestAccounts::default();
        assert_ne!(accounts.alice, accounts.bob);
        assert_eq!(accounts.all().len(), 5);
    }

    #[test]
    fn test_property_metadata_fixtures() {
        let minimal = PropertyMetadataFixtures::minimal();
        assert!(!minimal.location.is_empty());

        let standard = PropertyMetadataFixtures::standard();
        assert!(standard.size > minimal.size);

        let edge_cases = PropertyMetadataFixtures::edge_cases();
        assert_eq!(edge_cases.len(), 3);
    }

    #[test]
    fn test_generators() {
        let account = generators::random_account_id(42);
        assert_ne!(account, AccountId::from([0; 32]));

        let metadata = generators::random_property_metadata(100);
        assert!(metadata.size > 0);
    }
}
