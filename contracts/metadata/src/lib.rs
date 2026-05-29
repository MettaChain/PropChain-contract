#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unexpected_cfgs)]
#![allow(clippy::new_without_default)]

//! # Advanced Property Metadata Standard
//!
//! Implements a comprehensive metadata standard for property tokens that supports:
//! - Extensible metadata schema with typed fields
//! - IPFS integration for large file storage
//! - Metadata verification and validation
//! - Dynamic metadata update mechanisms
//! - Metadata versioning and history tracking
//! - Multimedia content support (images, videos, tours)
//! - Legal document integration and verification
//! - Metadata management and search capabilities
//!
//! Resolves: https://github.com/MettaChain/PropChain-contract/issues/69

use ink::prelude::string::String;
use ink::prelude::vec::Vec;
use ink::storage::Mapping;

#[ink::contract]
#[allow(clippy::too_many_arguments)]
mod propchain_metadata {
    use super::*;

    // Data types extracted to types.rs (Issue #101)
    include!("types.rs");

    // Error types extracted to errors.rs (Issue #101)
    include!("errors.rs");

    // ========================================================================
    // EVENTS
    // ========================================================================

    #[ink(event)]
    pub struct MetadataCreated {
        #[ink(topic)]
        property_id: PropertyId,
        #[ink(topic)]
        creator: AccountId,
        version: MetadataVersion,
        content_hash: Hash,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct MetadataUpdated {
        #[ink(topic)]
        property_id: PropertyId,
        #[ink(topic)]
        updater: AccountId,
        old_version: MetadataVersion,
        new_version: MetadataVersion,
        content_hash: Hash,
        change_description: String,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct MetadataFinalized {
        #[ink(topic)]
        property_id: PropertyId,
        #[ink(topic)]
        finalized_by: AccountId,
        final_version: MetadataVersion,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct LegalDocumentAdded {
        #[ink(topic)]
        property_id: PropertyId,
        #[ink(topic)]
        document_id: u64,
        document_type: LegalDocType,
        ipfs_cid: IpfsCid,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct LegalDocumentVerified {
        #[ink(topic)]
        property_id: PropertyId,
        #[ink(topic)]
        document_id: u64,
        #[ink(topic)]
        verifier: AccountId,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct MultimediaAdded {
        #[ink(topic)]
        property_id: PropertyId,
        media_type: String,
        content_ref: String,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct MetadataSearched {
        #[ink(topic)]
        searcher: AccountId,
        query: String,
        results_count: u32,
        timestamp: u64,
    }

    // ========================================================================
    // CONTRACT STORAGE
    // ========================================================================

    #[ink(storage)]
    pub struct AdvancedMetadataRegistry {
        /// Contract admin
        admin: AccountId,
        /// Property metadata storage
        metadata: Mapping<PropertyId, AdvancedPropertyMetadata>,
        /// Version history: (property_id, version) -> entry
        version_history: Mapping<(PropertyId, MetadataVersion), MetadataVersionEntry>,
        /// Property owners/authorized updaters
        property_owners: Mapping<PropertyId, AccountId>,
        /// Document verifiers
        verifiers: Mapping<AccountId, bool>,
        /// Property ID index (for search - maps keyword hash to property IDs)
        location_index: Mapping<u32, Vec<PropertyId>>,
        /// Property type index
        type_index: Mapping<u8, Vec<PropertyId>>,
        /// Total properties registered
        total_properties: u64,
        /// Document counter
        document_counter: u64,
        /// Maximum custom attributes per property
        max_custom_attributes: u32,
        /// Maximum media items per category
        max_media_items: u32,
        /// Maximum legal documents per property
        max_legal_documents: u32,
    }

    // ========================================================================
    // IMPLEMENTATION
    // ========================================================================

    impl AdvancedMetadataRegistry {
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            Self {
                admin: caller,
                metadata: Mapping::default(),
                version_history: Mapping::default(),
                property_owners: Mapping::default(),
                verifiers: Mapping::default(),
                location_index: Mapping::default(),
                type_index: Mapping::default(),
                total_properties: 0,
                document_counter: 0,
                max_custom_attributes: 50,
                max_media_items: 100,
                max_legal_documents: 50,
            }
        }

        // ====================================================================
        // METADATA LIFECYCLE
        // ====================================================================

        /// Creates new property metadata with full extensible schema
        #[ink(message)]
        pub fn create_metadata(
            &mut self,
            property_id: PropertyId,
            core: CoreMetadata,
            ipfs_resources: IpfsResources,
            content_hash: Hash,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let timestamp = self.env().block_timestamp();

            // Ensure property doesn't already have metadata
            if self.metadata.contains(property_id) {
                return Err(Error::InvalidMetadata);
            }

            // Validate core metadata
            self.validate_core_metadata(&core)?;

            // Validate IPFS CIDs if provided
            self.validate_ipfs_resources(&ipfs_resources)?;

            let metadata = AdvancedPropertyMetadata {
                property_id,
                version: 1,
                core,
                ipfs_resources,
                multimedia: MultimediaContent {
                    images: Vec::new(),
                    videos: Vec::new(),
                    virtual_tours: Vec::new(),
                    floor_plans: Vec::new(),
                },
                legal_documents: Vec::new(),
                custom_attributes: Vec::new(),
                content_hash,
                created_at: timestamp,
                updated_at: timestamp,
                created_by: caller,
                is_finalized: false,
            };

            // Store metadata
            self.metadata.insert(property_id, &metadata);
            self.property_owners.insert(property_id, &caller);

            // Record version history
            let version_entry = MetadataVersionEntry {
                version: 1,
                content_hash,
                updated_by: caller,
                updated_at: timestamp,
                change_description: String::from("Initial metadata creation"),
                snapshot_cid: None,
            };
            self.version_history
                .insert((property_id, 1), &version_entry);

            // Update indexes
            let property_type_idx = self.property_type_to_index(&metadata.core.property_type);
            let mut type_list = self.type_index.get(property_type_idx).unwrap_or_default();
            type_list.push(property_id);
            self.type_index.insert(property_type_idx, &type_list);

            self.total_properties += 1;

            self.env().emit_event(MetadataCreated {
                property_id,
                creator: caller,
                version: 1,
                content_hash,
                timestamp,
            });

            Ok(())
        }

        /// Updates property metadata with version tracking
        #[ink(message)]
        pub fn update_metadata(
            &mut self,
            property_id: PropertyId,
            core: CoreMetadata,
            ipfs_resources: IpfsResources,
            content_hash: Hash,
            change_description: String,
            snapshot_cid: Option<IpfsCid>,
        ) -> Result<MetadataVersion, Error> {
            let caller = self.env().caller();
            let timestamp = self.env().block_timestamp();

            self.ensure_owner_or_admin(property_id, caller)?;

            let mut metadata = self
                .metadata
                .get(property_id)
                .ok_or(Error::PropertyNotFound)?;

            if metadata.is_finalized {
                return Err(Error::MetadataAlreadyFinalized);
            }

            // Validate
            self.validate_core_metadata(&core)?;
            self.validate_ipfs_resources(&ipfs_resources)?;

            let old_version = metadata.version;
            let new_version = old_version + 1;

            metadata.version = new_version;
            metadata.core = core;
            metadata.ipfs_resources = ipfs_resources;
            metadata.content_hash = content_hash;
            metadata.updated_at = timestamp;

            self.metadata.insert(property_id, &metadata);

            // Record version history
            let version_entry = MetadataVersionEntry {
                version: new_version,
                content_hash,
                updated_by: caller,
                updated_at: timestamp,
                change_description: change_description.clone(),
                snapshot_cid,
            };
            self.version_history
                .insert((property_id, new_version), &version_entry);

            self.env().emit_event(MetadataUpdated {
                property_id,
                updater: caller,
                old_version,
                new_version,
                content_hash,
                change_description,
                timestamp,
            });

            Ok(new_version)
        }

        /// Adds a custom attribute to property metadata
        #[ink(message)]
        pub fn add_custom_attribute(
            &mut self,
            property_id: PropertyId,
            key: String,
            value: MetadataValue,
            is_required: bool,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            self.ensure_owner_or_admin(property_id, caller)?;

            let mut metadata = self
                .metadata
                .get(property_id)
                .ok_or(Error::PropertyNotFound)?;

            if metadata.is_finalized {
                return Err(Error::MetadataAlreadyFinalized);
            }

            if metadata.custom_attributes.len() as u32 >= self.max_custom_attributes {
                return Err(Error::SizeLimitExceeded);
            }

            metadata.custom_attributes.push(MetadataAttribute {
                key,
                value,
                is_required,
            });
            metadata.updated_at = self.env().block_timestamp();

            self.metadata.insert(property_id, &metadata);
            Ok(())
        }

        /// Finalizes metadata making it immutable
        #[ink(message)]
        pub fn finalize_metadata(&mut self, property_id: PropertyId) -> Result<(), Error> {
            let caller = self.env().caller();
            self.ensure_owner_or_admin(property_id, caller)?;

            let mut metadata = self
                .metadata
                .get(property_id)
                .ok_or(Error::PropertyNotFound)?;

            if metadata.is_finalized {
                return Err(Error::MetadataAlreadyFinalized);
            }

            metadata.is_finalized = true;
            metadata.updated_at = self.env().block_timestamp();

            self.metadata.insert(property_id, &metadata);

            self.env().emit_event(MetadataFinalized {
                property_id,
                finalized_by: caller,
                final_version: metadata.version,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        // ====================================================================
        // MULTIMEDIA CONTENT MANAGEMENT
        // ====================================================================

        /// Adds a multimedia item (image, video, tour, floor plan)
        #[ink(message)]
        pub fn add_media_item(
            &mut self,
            property_id: PropertyId,
            media_category: u8, // 0=image, 1=video, 2=virtual_tour, 3=floor_plan
            content_ref: String,
            description: String,
            mime_type: String,
            file_size: u64,
            content_hash: Hash,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            self.ensure_owner_or_admin(property_id, caller)?;

            let mut metadata = self
                .metadata
                .get(property_id)
                .ok_or(Error::PropertyNotFound)?;

            if metadata.is_finalized {
                return Err(Error::MetadataAlreadyFinalized);
            }

            let media_item = MediaItem {
                content_ref: content_ref.clone(),
                description,
                mime_type,
                file_size,
                content_hash,
                uploaded_at: self.env().block_timestamp(),
            };

            let media_type_str = match media_category {
                0 => {
                    if metadata.multimedia.images.len() as u32 >= self.max_media_items {
                        return Err(Error::SizeLimitExceeded);
                    }
                    metadata.multimedia.images.push(media_item);
                    "image"
                }
                1 => {
                    if metadata.multimedia.videos.len() as u32 >= self.max_media_items {
                        return Err(Error::SizeLimitExceeded);
                    }
                    metadata.multimedia.videos.push(media_item);
                    "video"
                }
                2 => {
                    if metadata.multimedia.virtual_tours.len() as u32 >= self.max_media_items {
                        return Err(Error::SizeLimitExceeded);
                    }
                    metadata.multimedia.virtual_tours.push(media_item);
                    "virtual_tour"
                }
                3 => {
                    if metadata.multimedia.floor_plans.len() as u32 >= self.max_media_items {
                        return Err(Error::SizeLimitExceeded);
                    }
                    metadata.multimedia.floor_plans.push(media_item);
                    "floor_plan"
                }
                _ => return Err(Error::InvalidMetadata),
            };

            metadata.updated_at = self.env().block_timestamp();
            self.metadata.insert(property_id, &metadata);

            self.env().emit_event(MultimediaAdded {
                property_id,
                media_type: String::from(media_type_str),
                content_ref,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        // ====================================================================
        // LEGAL DOCUMENT MANAGEMENT
        // ====================================================================

        /// Adds a legal document reference to property metadata
        #[ink(message)]
        pub fn add_legal_document(
            &mut self,
            property_id: PropertyId,
            document_type: LegalDocType,
            ipfs_cid: IpfsCid,
            content_hash: Hash,
            issuer: String,
            issue_date: u64,
            expiry_date: Option<u64>,
        ) -> Result<u64, Error> {
            let caller = self.env().caller();
            self.ensure_owner_or_admin(property_id, caller)?;

            let mut metadata = self
                .metadata
                .get(property_id)
                .ok_or(Error::PropertyNotFound)?;

            if metadata.is_finalized {
                return Err(Error::MetadataAlreadyFinalized);
            }

            if metadata.legal_documents.len() as u32 >= self.max_legal_documents {
                return Err(Error::SizeLimitExceeded);
            }

            self.validate_ipfs_cid(&ipfs_cid)?;

            self.document_counter += 1;
            let document_id = self.document_counter;

            let doc_ref = LegalDocumentRef {
                document_id,
                document_type: document_type.clone(),
                ipfs_cid: ipfs_cid.clone(),
                content_hash,
                issuer,
                issue_date,
                expiry_date,
                is_verified: false,
                verified_by: None,
            };

            metadata.legal_documents.push(doc_ref);
            metadata.updated_at = self.env().block_timestamp();

            self.metadata.insert(property_id, &metadata);

            self.env().emit_event(LegalDocumentAdded {
                property_id,
                document_id,
                document_type,
                ipfs_cid,
                timestamp: self.env().block_timestamp(),
            });

            Ok(document_id)
        }

        /// Verifies a legal document (verifier only)
        #[ink(message)]
        pub fn verify_legal_document(
            &mut self,
            property_id: PropertyId,
            document_id: u64,
        ) -> Result<(), Error> {
            let caller = self.env().caller();

            // Must be admin or authorized verifier
            if caller != self.admin && !self.verifiers.get(caller).unwrap_or(false) {
                return Err(Error::Unauthorized);
            }

            let mut metadata = self
                .metadata
                .get(property_id)
                .ok_or(Error::PropertyNotFound)?;

            let doc = metadata
                .legal_documents
                .iter_mut()
                .find(|d| d.document_id == document_id)
                .ok_or(Error::DocumentNotFound)?;

            doc.is_verified = true;
            doc.verified_by = Some(caller);

            self.metadata.insert(property_id, &metadata);

            self.env().emit_event(LegalDocumentVerified {
                property_id,
                document_id,
                verifier: caller,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        // ====================================================================
        // METADATA VERSIONING & HISTORY
        // ====================================================================

        /// Gets metadata version history for a property
        #[ink(message)]
        pub fn get_version_history(&self, property_id: PropertyId) -> Vec<MetadataVersionEntry> {
            let metadata = match self.metadata.get(property_id) {
                Some(m) => m,
                None => return Vec::new(),
            };

            let mut history = Vec::new();
            for v in 1..=metadata.version {
                if let Some(entry) = self.version_history.get((property_id, v)) {
                    history.push(entry);
                }
            }
            history
        }

        /// Gets a specific version's metadata entry
        #[ink(message)]
        pub fn get_version(
            &self,
            property_id: PropertyId,
            version: MetadataVersion,
        ) -> Option<MetadataVersionEntry> {
            self.version_history.get((property_id, version))
        }

        // ====================================================================
        // QUERY & SEARCH
        // ====================================================================

        /// Gets full metadata for a property
        #[ink(message)]
        pub fn get_metadata(&self, property_id: PropertyId) -> Option<AdvancedPropertyMetadata> {
            self.metadata.get(property_id)
        }

        /// Gets only the core metadata for a property
        #[ink(message)]
        pub fn get_core_metadata(&self, property_id: PropertyId) -> Option<CoreMetadata> {
            self.metadata.get(property_id).map(|m| m.core)
        }

        /// Gets multimedia content for a property
        #[ink(message)]
        pub fn get_multimedia(&self, property_id: PropertyId) -> Option<MultimediaContent> {
            self.metadata.get(property_id).map(|m| m.multimedia)
        }

        /// Gets legal documents for a property
        #[ink(message)]
        pub fn get_legal_documents(&self, property_id: PropertyId) -> Vec<LegalDocumentRef> {
            self.metadata
                .get(property_id)
                .map(|m| m.legal_documents)
                .unwrap_or_default()
        }

        /// Gets properties by type
        #[ink(message)]
        pub fn get_properties_by_type(
            &self,
            property_type: MetadataPropertyType,
        ) -> Vec<PropertyId> {
            let idx = self.property_type_to_index(&property_type);
            self.type_index.get(idx).unwrap_or_default()
        }

        /// Verifies content integrity of metadata
        #[ink(message)]
        pub fn verify_content_hash(
            &self,
            property_id: PropertyId,
            expected_hash: Hash,
        ) -> Result<bool, Error> {
            let metadata = self
                .metadata
                .get(property_id)
                .ok_or(Error::PropertyNotFound)?;
            Ok(metadata.content_hash == expected_hash)
        }

        /// Gets total properties registered
        #[ink(message)]
        pub fn total_properties(&self) -> u64 {
            self.total_properties
        }

        /// Gets current metadata version for a property
        #[ink(message)]
        pub fn current_version(&self, property_id: PropertyId) -> Option<MetadataVersion> {
            self.metadata.get(property_id).map(|m| m.version)
        }

        // ====================================================================
        // ADMIN FUNCTIONS
        // ====================================================================

        /// Adds a document verifier (admin only)
        #[ink(message)]
        pub fn add_verifier(&mut self, verifier: AccountId) -> Result<(), Error> {
            self.ensure_admin()?;
            self.verifiers.insert(verifier, &true);
            Ok(())
        }

        /// Removes a document verifier (admin only)
        #[ink(message)]
        pub fn remove_verifier(&mut self, verifier: AccountId) -> Result<(), Error> {
            self.ensure_admin()?;
            self.verifiers.remove(verifier);
            Ok(())
        }

        /// Updates configuration limits (admin only)
        #[ink(message)]
        pub fn update_limits(
            &mut self,
            max_custom_attributes: u32,
            max_media_items: u32,
            max_legal_documents: u32,
        ) -> Result<(), Error> {
            self.ensure_admin()?;
            self.max_custom_attributes = max_custom_attributes;
            self.max_media_items = max_media_items;
            self.max_legal_documents = max_legal_documents;
            Ok(())
        }

        /// Returns admin account
        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
        }

        // ====================================================================
        // INTERNAL HELPERS
        // ====================================================================

        fn ensure_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        fn ensure_owner_or_admin(
            &self,
            property_id: PropertyId,
            caller: AccountId,
        ) -> Result<(), Error> {
            if caller == self.admin {
                return Ok(());
            }
            let owner = self
                .property_owners
                .get(property_id)
                .ok_or(Error::PropertyNotFound)?;
            if caller != owner {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        fn validate_core_metadata(&self, core: &CoreMetadata) -> Result<(), Error> {
            if core.name.is_empty() || core.location.is_empty() {
                return Err(Error::RequiredFieldMissing);
            }
            if core.size_sqm == 0 {
                return Err(Error::InvalidMetadata);
            }
            if core.legal_description.is_empty() {
                return Err(Error::RequiredFieldMissing);
            }
            Ok(())
        }

        fn validate_ipfs_resources(&self, resources: &IpfsResources) -> Result<(), Error> {
            if let Some(ref cid) = resources.metadata_cid {
                self.validate_ipfs_cid(cid)?;
            }
            if let Some(ref cid) = resources.documents_cid {
                self.validate_ipfs_cid(cid)?;
            }
            if let Some(ref cid) = resources.images_cid {
                self.validate_ipfs_cid(cid)?;
            }
            if let Some(ref cid) = resources.legal_docs_cid {
                self.validate_ipfs_cid(cid)?;
            }
            if let Some(ref cid) = resources.virtual_tour_cid {
                self.validate_ipfs_cid(cid)?;
            }
            if let Some(ref cid) = resources.floor_plans_cid {
                self.validate_ipfs_cid(cid)?;
            }
            Ok(())
        }

        fn validate_ipfs_cid(&self, cid: &str) -> Result<(), Error> {
            if cid.is_empty() {
                return Err(Error::InvalidIpfsCid);
            }
            // CIDv0: starts with "Qm", 46 chars
            if cid.starts_with("Qm") && cid.len() == 46 {
                return Ok(());
            }
            // CIDv1: starts with "b", min 10 chars
            if cid.starts_with('b') && cid.len() >= 10 {
                return Ok(());
            }
            Err(Error::InvalidIpfsCid)
        }

        fn property_type_to_index(&self, pt: &MetadataPropertyType) -> u8 {
            match pt {
                MetadataPropertyType::Residential => 0,
                MetadataPropertyType::Commercial => 1,
                MetadataPropertyType::Industrial => 2,
                MetadataPropertyType::Land => 3,
                MetadataPropertyType::MultiFamily => 4,
                MetadataPropertyType::Retail => 5,
                MetadataPropertyType::Office => 6,
                MetadataPropertyType::MixedUse => 7,
                MetadataPropertyType::Agricultural => 8,
                MetadataPropertyType::Hospitality => 9,
            }
        }
    }

    impl Default for AdvancedMetadataRegistry {
        fn default() -> Self {
            Self::new()
        }
    }

    // ========================================================================
    // UNIT TESTS
    // ========================================================================
    #[cfg(test)]
    mod tests {}
}
