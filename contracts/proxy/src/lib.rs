#![cfg_attr(not(feature = "std"), no_std)]

#[ink::contract]
mod propchain_proxy {
    use ink::storage::Mapping;

    /// Unique storage key for the proxy data to avoid collisions.
    /// bytes4(keccak256("proxy.storage")) = 0xc5f3bc7a
    const PROXY_STORAGE_KEY: u32 = 0xC5F3BC7A;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        UpgradeFailed,
    }

    #[ink(storage)]
    pub struct TransparentProxy {
        /// The address of the current implementation contract.
        code_hash: Hash,
        /// The address of the proxy admin.
        admin: AccountId,
    }

    #[ink(event)]
    pub struct Upgraded {
        #[ink(topic)]
        new_code_hash: Hash,
    }

    #[ink(event)]
    pub struct AdminChanged {
        #[ink(topic)]
        new_admin: AccountId,
    }

    impl TransparentProxy {
        #[ink(constructor)]
        pub fn new(code_hash: Hash) -> Self {
            Self {
                code_hash,
                admin: Self::env().caller(),
            }
        }

        #[ink(message)]
        pub fn upgrade_to(&mut self, new_code_hash: Hash) -> Result<(), Error> {
            self.ensure_admin()?;
            self.code_hash = new_code_hash;
            self.env().emit_event(Upgraded { new_code_hash });
            Ok(())
        }

        #[ink(message)]
        pub fn change_admin(&mut self, new_admin: AccountId) -> Result<(), Error> {
            self.ensure_admin()?;
            self.admin = new_admin;
            self.env().emit_event(AdminChanged { new_admin });
            Ok(())
        }

        #[ink(message)]
        pub fn code_hash(&self) -> Hash {
            self.code_hash
        }

        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
        }

        /// Fallback-like mechanism to forward calls to the logic contract.
        /// This is a simplified version; in a real-world scenario, you might use 
        /// more advanced patterns like ink!'s `delegate_call`.
        #[ink(message, payable, selector = _)]
        pub fn forward(&self) {
            // Forward everything else to implementation
            // Since ink! doesn't have a native "catch-all" fallback like Solidity yet,
            // we use the selector = _ pattern for non-matching calls.
            let selector = self.env().msg_packer().selector();
            let mut result = self.env()
                .call_v1(self.code_hash)
                .delegate()
                .call_flags(ink::env::CallFlags::default())
                .try_invoke()
                .expect("Forwarding failed");

            // We omit the actual return data handling here for brevity as ink!'s dynamic forwarding
            // is still evolving, but this demonstrates the pattern.
        }

        fn ensure_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }
    }
}
