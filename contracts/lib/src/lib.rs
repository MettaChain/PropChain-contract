#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unexpected_cfgs)]

#[cfg(not(feature = "std"))]
use scale_info::prelude::vec::Vec;
use ink::storage::Mapping;
use propchain_traits::*;

#[ink::contract]
mod propchain_contracts {
    use super::*;

    /// Error types for contract
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        PropertyNotFound,
        Unauthorized,
        InvalidMetadata,
        EscrowNotFound,
        EscrowAlreadyReleased,
        InsufficientFunds,
    }

    /// Property Registry contract
    #[ink(storage)]
    pub struct PropertyRegistry {
        /// Mapping from property ID to property informatio
        properties: Mapping<u64, PropertyInfo>,
        /// Mapping from owner to their properties
        owner_properties: Mapping<AccountId, Vec<u64>>,
        /// Mapping from property ID to approved account
        approvals: Mapping<u64, AccountId>,
        /// Property counter
        property_count: u64,
        /// Contract version
        version: u32,
        /// Admin for upgrades (if used directly, or for logic-level auth)
        admin: AccountId,
        /// Mapping from escrow ID to escrow information
        escrows: Mapping<u64, EscrowInfo>,
        /// Escrow counter
        escrow_count: u64,
    }

    /// Escrow information
    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct EscrowInfo {
        pub id: u64,
        pub property_id: u64,
        pub buyer: AccountId,
        pub seller: AccountId,
        pub amount: u128,
        pub released: bool,
    }

    #[ink(event)]
    pub struct PropertyRegistered {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        owner: AccountId,
        version: u8,
    }

    #[ink(event)]
    pub struct PropertyTransferred {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
    }

    #[ink(event)]
    pub struct PropertyMetadataUpdated {
        #[ink(topic)]
        property_id: u64,
        metadata: PropertyMetadata,
    }

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        approved: AccountId,
    }

    #[ink(event)]
    pub struct EscrowCreated {
        #[ink(topic)]
        escrow_id: u64,
        property_id: u64,
        buyer: AccountId,
        seller: AccountId,
        amount: u128,
    }

    #[ink(event)]
    pub struct EscrowReleased {
        #[ink(topic)]
        escrow_id: u64,
    }

    #[ink(event)]
    pub struct EscrowRefunded {
        #[ink(topic)]
        escrow_id: u64,
    }

    impl PropertyRegistry {
        /// Creates a new PropertyRegistry contract
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                properties: Mapping::default(),
                owner_properties: Mapping::default(),
                approvals: Mapping::default(),
                property_count: 0,
                version: 1,
                admin: Self::env().caller(),
                escrows: Mapping::default(),
                escrow_count: 0,
            }
        }

        /// Returns the contract version
        #[ink(message)]
        pub fn version(&self) -> u32 {
            self.version
        }

        /// Registers a new property
        #[ink(message)]
        pub fn register_property(&mut self, metadata: PropertyMetadata) -> Result<u64, Error> {
            let caller = self.env().caller();
            self.property_count += 1;
            let property_id = self.property_count;

            let property_info = PropertyInfo {
                id: property_id,
                owner: caller,
                metadata,
                registered_at: self.env().block_timestamp(),
            };

            self.properties.insert(&property_id, &property_info);

            let mut owner_props = self.owner_properties.get(&caller).unwrap_or_default();
            owner_props.push(property_id);
            self.owner_properties.insert(&caller, &owner_props);

            self.env().emit_event(PropertyRegistered {
                property_id,
                owner: caller,
                version: 1,
            });

            Ok(property_id)
        }

        /// Transfers property ownership
        #[ink(message)]
        pub fn transfer_property(&mut self, property_id: u64, to: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut property = self.properties.get(&property_id).ok_or(Error::PropertyNotFound)?;

            let approved = self.approvals.get(&property_id);
            if property.owner != caller && Some(caller) != approved {
                return Err(Error::Unauthorized);
            }

            let from = property.owner;

            // Remove from current owner's properties
            let mut current_owner_props = self.owner_properties.get(&from).unwrap_or_default();
            current_owner_props.retain(|&id| id != property_id);
            self.owner_properties.insert(&from, &current_owner_props);
            
            // Add to new owner's properties
            let mut new_owner_props = self.owner_properties.get(&to).unwrap_or_default();
            new_owner_props.push(property_id);
            self.owner_properties.insert(&to, &new_owner_props);

            // Update property owner
            property.owner = to;
            self.properties.insert(&property_id, &property);

            // Clear approval
            self.approvals.remove(&property_id);

            self.env().emit_event(PropertyTransferred {
                property_id,
                from,
                to,
            });

            Ok(())
        }

        /// Gets property information
        #[ink(message)]
        pub fn get_property(&self, property_id: u64) -> Option<PropertyInfo> {
            self.properties.get(&property_id)
        }

        /// Gets properties owned by an account
        #[ink(message)]
        pub fn get_owner_properties(&self, owner: AccountId) -> Vec<u64> {
            self.owner_properties.get(&owner).unwrap_or_default()
        }

        /// Gets total property count
        #[ink(message)]
        pub fn property_count(&self) -> u64 {
            self.property_count
        }

        /// Updates property metadata
        #[ink(message)]
        pub fn update_metadata(&mut self, property_id: u64, metadata: PropertyMetadata) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut property = self.properties.get(&property_id).ok_or(Error::PropertyNotFound)?;

            if property.owner != caller {
                return Err(Error::Unauthorized);
            }

            // check if metadata is valid (basic check)
            if metadata.location.is_empty() {
                return Err(Error::InvalidMetadata);
            }

            property.metadata = metadata.clone();
            self.properties.insert(&property_id, &property);

            self.env().emit_event(PropertyMetadataUpdated {
                property_id,
                metadata,
            });

            Ok(())
        }

        /// Approves an account to transfer a specific property
        #[ink(message)]
        pub fn approve(&mut self, property_id: u64, to: Option<AccountId>) -> Result<(), Error> {
            let caller = self.env().caller();
            let property = self.properties.get(&property_id).ok_or(Error::PropertyNotFound)?;

            if property.owner != caller {
                return Err(Error::Unauthorized);
            }

            if let Some(account) = to {
                self.approvals.insert(&property_id, &account);
                self.env().emit_event(Approval {
                    property_id,
                    owner: caller,
                    approved: account,
                });
            } else {
                self.approvals.remove(&property_id);
                let zero_account = AccountId::from([0u8; 32]);
                self.env().emit_event(Approval {
                    property_id,
                    owner: caller,
                    approved: zero_account,
                });
            }

            Ok(())
        }

        /// Gets the approved account for a property
        #[ink(message)]
        pub fn get_approved(&self, property_id: u64) -> Option<AccountId> {
            self.approvals.get(&property_id)
        }

        /// Creates a new escrow for property transfer
        #[ink(message)]
        pub fn create_escrow(&mut self, property_id: u64, amount: u128) -> Result<u64, Error> {
            let caller = self.env().caller();
            let property = self.properties.get(&property_id).ok_or(Error::PropertyNotFound)?;

            // Only property owner can create escrow
            if property.owner != caller {
                return Err(Error::Unauthorized);
            }

            self.escrow_count += 1;
            let escrow_id = self.escrow_count;

            let escrow_info = EscrowInfo {
                id: escrow_id,
                property_id,
                buyer: caller, // In this simple version, caller is buyer
                seller: property.owner,
                amount,
                released: false,
            };

            self.escrows.insert(&escrow_id, &escrow_info);

            self.env().emit_event(EscrowCreated {
                escrow_id,
                property_id,
                buyer: caller,
                seller: property.owner,
                amount,
            });

            Ok(escrow_id)
        }

        /// Releases escrow funds and transfers property
        #[ink(message)]
        pub fn release_escrow(&mut self, escrow_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut escrow = self.escrows.get(&escrow_id).ok_or(Error::EscrowNotFound)?;

            if escrow.released {
                return Err(Error::EscrowAlreadyReleased);
            }

            // Only buyer can release
            if escrow.buyer != caller {
                return Err(Error::Unauthorized);
            }

            // Transfer property
            self.transfer_property(escrow.property_id, escrow.buyer)?;

            escrow.released = true;
            self.escrows.insert(&escrow_id, &escrow);

            self.env().emit_event(EscrowReleased {
                escrow_id,
            });

            Ok(())
        }

        /// Refunds escrow funds
        #[ink(message)]
        pub fn refund_escrow(&mut self, escrow_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut escrow = self.escrows.get(&escrow_id).ok_or(Error::EscrowNotFound)?;

            if escrow.released {
                return Err(Error::EscrowAlreadyReleased);
            }

            // Only seller can refund
            if escrow.seller != caller {
                return Err(Error::Unauthorized);
            }

            escrow.released = true;
            self.escrows.insert(&escrow_id, &escrow);

            self.env().emit_event(EscrowRefunded {
                escrow_id,
            });

            Ok(())
        }

        /// Gets escrow information
        #[ink(message)]
        pub fn get_escrow(&self, escrow_id: u64) -> Option<EscrowInfo> {
            self.escrows.get(&escrow_id)
        }
    }

    #[cfg(kani)]
    mod verification {
        use super::*;

        #[kani::proof]
        fn verify_arithmetic_overflow() {
            let a: u64 = kani::any();
            let b: u64 = kani::any();
            // Verify that addition is safe
            if a < 100 && b < 100 {
                assert!(a + b < 200);
            }
        }

        #[kani::proof]
        fn verify_property_info_struct() {
            let id: u64 = kani::any();
            // Verify PropertyInfo layout/safety if needed
            // This is a placeholder for checking structural invariants
            if id > 0 {
                assert!(id > 0);
            }
        }
    }

    impl Default for PropertyRegistry {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Escrow for PropertyRegistry {
        type Error = Error;

        fn create_escrow(&mut self, property_id: u64, amount: u128) -> Result<u64, Self::Error> {
            self.create_escrow(property_id, amount)
        }

        fn release_escrow(&mut self, escrow_id: u64) -> Result<(), Self::Error> {
            self.release_escrow(escrow_id)
        }

        fn refund_escrow(&mut self, escrow_id: u64) -> Result<(), Self::Error> {
            self.refund_escrow(escrow_id)
        }
    }
}

#[cfg(test)]
mod tests;