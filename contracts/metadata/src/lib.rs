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

    // ========================================================================
    // TYPES
    // ========================================================================

    pub type PropertyId = u64;
    pub type MetadataVersion = u32;
    pub type IpfsCid = String;

    // ========================================================================
    // EXTENSIBLE METADATA SCHEMA
    // ========================================================================

    /// Core property metadata with extensible fields
    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct AdvancedPropertyMetadata {
        /// Property identifier
        pub property_id: PropertyId,
        /// Current version of the metadata
        pub version: MetadataVersion,
        /// Core property information
        pub core: CoreMetadata,
        /// IPFS content identifiers for associated files
        pub ipfs_resources: IpfsResources,
        /// Multimedia content references
        pub multimedia: MultimediaContent,
        /// Legal document references
        pub legal_documents: Vec<LegalDocumentRef>,
        /// Custom extensible attributes (key-value pairs)
        pub custom_attributes: Vec<MetadataAttribute>,
        /// Content hash for integrity verification
        pub content_hash: Hash,
        /// Creation timestamp
        pub created_at: u64,
        /// Last update timestamp
        pub updated_at: u64,
        /// Creator account
        pub created_by: AccountId,
        /// Whether this metadata is finalized (immutable)
        pub is_finalized: bool,
    }

    /// Core property information (required fields)
    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct CoreMetadata {
        /// Property name/title
        pub name: String,
        /// Physical address/location
        pub location: String,
        /// Property size in square meters
        pub size_sqm: u64,
        /// Property type classification
        pub property_type: MetadataPropertyType,
        /// Current valuation in smallest currency unit
        pub valuation: u128,
        /// Legal description of the property
        pub legal_description: String,
        /// Geographic coordinates (latitude * 1e6, longitude * 1e6)
        pub coordinates: Option<(i64, i64)>,
        /// Year built
        pub year_built: Option<u32>,
        /// Number of bedrooms (for residential)
        pub bedrooms: Option<u8>,
        /// Number of bathrooms (for residential)
        pub bathrooms: Option<u8>,
        /// Zoning classification
        pub zoning: Option<String>,
    }

    /// Property type for metadata classification
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum MetadataPropertyType {
        Residential,
        Commercial,
        Industrial,
        Land,
        MultiFamily,
        Retail,
        Office,
        MixedUse,
        Agricultural,
        Hospitality,
    }

    /// IPFS resource links for the property
    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct IpfsResources {
        /// Main metadata JSON on IPFS
        pub metadata_cid: Option<IpfsCid>,
        /// Documents bundle CID
        pub documents_cid: Option<IpfsCid>,
        /// Images bundle CID
        pub images_cid: Option<IpfsCid>,
        /// Legal documents bundle CID
        pub legal_docs_cid: Option<IpfsCid>,
        /// 3D model / virtual tour CID
        pub virtual_tour_cid: Option<IpfsCid>,
        /// Floor plans CID
        pub floor_plans_cid: Option<IpfsCid>,
    }

    /// Multimedia content references
    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct MultimediaContent {
        /// Image references (CID, description, mime_type)
        pub images: Vec<MediaItem>,
        /// Video references
        pub videos: Vec<MediaItem>,
        /// Virtual tour links
        pub virtual_tours: Vec<MediaItem>,
        /// Floor plans
        pub floor_plans: Vec<MediaItem>,
    }

    /// Individual media item reference
    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct MediaItem {
        /// IPFS CID or URL
        pub content_ref: String,
        /// Description of the media item
        pub description: String,
        /// MIME type
        pub mime_type: String,
        /// File size in bytes
        pub file_size: u64,
        /// Content hash for verification
        pub content_hash: Hash,
        /// Upload timestamp
        pub uploaded_at: u64,
    }

    /// Legal document reference
    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct LegalDocumentRef {
        /// Document identifier
        pub document_id: u64,
        /// Document type
        pub document_type: LegalDocType,
        /// IPFS CID for the document
        pub ipfs_cid: IpfsCid,
        /// Content hash for integrity verification
        pub content_hash: Hash,
        /// Issuing authority
        pub issuer: String,
        /// Issue date timestamp
        pub issue_date: u64,
        /// Expiry date timestamp (if applicable)
        pub expiry_date: Option<u64>,
        /// Verification status
        pub is_verified: bool,
        /// Verifier account (if verified)
        pub verified_by: Option<AccountId>,
    }

    /// Legal document types
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    #[allow(clippy::upper_case_acronyms)]
    pub enum LegalDocType {
        Deed,
        Title,
        Survey,
        Inspection,
        Appraisal,
        TaxRecord,
        Insurance,
        ZoningPermit,
        EnvironmentalReport,
        HOADocument,
        LeaseAgreement,
        MortgageDocument,
        Other,
    }

    /// Custom metadata attribute (extensible key-value pair)
    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct MetadataAttribute {
        /// Attribute key/name
        pub key: String,
        /// Attribute value
        pub value: MetadataValue,
        /// Whether this attribute is required
        pub is_required: bool,
    }

    /// Typed metadata values for extensibility
    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum MetadataValue {
        Text(String),
        Number(u128),
        Boolean(bool),
        Date(u64),
        IpfsRef(IpfsCid),
        AccountRef(AccountId),
    }

    /// Metadata version history entry
    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct MetadataVersionEntry {
        pub version: MetadataVersion,
        pub content_hash: Hash,
        pub updated_by: AccountId,
        pub updated_at: u64,
        pub change_description: String,
        /// Previous IPFS CID snapshot (for full historical access)
        pub snapshot_cid: Option<IpfsCid>,
    }

    // ========================================================================
    // ERRORS
    // ========================================================================

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        PropertyNotFound,
        Unauthorized,
        InvalidMetadata,
        MetadataAlreadyFinalized,
        InvalidIpfsCid,
        DocumentNotFound,
        DocumentAlreadyExists,
        VersionConflict,
        RequiredFieldMissing,
        SizeLimitExceeded,
        InvalidContentHash,
        SearchQueryTooLong,
    }

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
    mod tests {
        use super::*;

        fn default_core() -> CoreMetadata {
            CoreMetadata {
                name: String::from("Test Property"),
                location: String::from("123 Main St, City"),
                size_sqm: 500,
                property_type: MetadataPropertyType::Residential,
                valuation: 1_000_000,
                legal_description: String::from("Lot 1, Block A"),
                coordinates: Some((40_712_776, -74_005_974)),
                year_built: Some(2020),
                bedrooms: Some(3),
                bathrooms: Some(2),
                zoning: Some(String::from("R-1")),
            }
        }

        fn default_ipfs_resources() -> IpfsResources {
            IpfsResources {
                metadata_cid: None,
                documents_cid: None,
                images_cid: None,
                legal_docs_cid: None,
                virtual_tour_cid: None,
                floor_plans_cid: None,
            }
        }

        #[ink::test]
        fn create_metadata_works() {
            let mut contract = AdvancedMetadataRegistry::new();
            let result = contract.create_metadata(
                1,
                default_core(),
                default_ipfs_resources(),
                Hash::from([0x01; 32]),
            );
            assert!(result.is_ok());
            assert_eq!(contract.total_properties(), 1);
            assert_eq!(contract.current_version(1), Some(1));
        }

        #[ink::test]
        fn update_metadata_increments_version() {
            let mut contract = AdvancedMetadataRegistry::new();
            contract
                .create_metadata(
                    1,
                    default_core(),
                    default_ipfs_resources(),
                    Hash::from([0x01; 32]),
                )
                .unwrap();

            let mut updated_core = default_core();
            updated_core.valuation = 2_000_000;

            let result = contract.update_metadata(
                1,
                updated_core,
                default_ipfs_resources(),
                Hash::from([0x02; 32]),
                String::from("Valuation update"),
                None,
            );
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 2);
            assert_eq!(contract.current_version(1), Some(2));
        }

        #[ink::test]
        fn finalized_metadata_cannot_be_updated() {
            let mut contract = AdvancedMetadataRegistry::new();
            contract
                .create_metadata(
                    1,
                    default_core(),
                    default_ipfs_resources(),
                    Hash::from([0x01; 32]),
                )
                .unwrap();
            contract.finalize_metadata(1).unwrap();

            let result = contract.update_metadata(
                1,
                default_core(),
                default_ipfs_resources(),
                Hash::from([0x02; 32]),
                String::from("Should fail"),
                None,
            );
            assert_eq!(result, Err(Error::MetadataAlreadyFinalized));
        }

        #[ink::test]
        fn version_history_tracking_works() {
            let mut contract = AdvancedMetadataRegistry::new();
            contract
                .create_metadata(
                    1,
                    default_core(),
                    default_ipfs_resources(),
                    Hash::from([0x01; 32]),
                )
                .unwrap();
            contract
                .update_metadata(
                    1,
                    default_core(),
                    default_ipfs_resources(),
                    Hash::from([0x02; 32]),
                    String::from("Update 1"),
                    None,
                )
                .unwrap();

            let history = contract.get_version_history(1);
            assert_eq!(history.len(), 2);
            assert_eq!(history[0].version, 1);
            assert_eq!(history[1].version, 2);
        }

        #[ink::test]
        fn add_legal_document_works() {
            let mut contract = AdvancedMetadataRegistry::new();
            contract
                .create_metadata(
                    1,
                    default_core(),
                    default_ipfs_resources(),
                    Hash::from([0x01; 32]),
                )
                .unwrap();

            let result = contract.add_legal_document(
                1,
                LegalDocType::Deed,
                String::from("Qm12345678901234567890123456789012345678901234"),
                Hash::from([0x03; 32]),
                String::from("County Records"),
                1700000000,
                None,
            );
            assert!(result.is_ok());

            let docs = contract.get_legal_documents(1);
            assert_eq!(docs.len(), 1);
            assert!(!docs[0].is_verified);
        }

        #[ink::test]
        fn verify_legal_document_works() {
            let mut contract = AdvancedMetadataRegistry::new();
            contract
                .create_metadata(
                    1,
                    default_core(),
                    default_ipfs_resources(),
                    Hash::from([0x01; 32]),
                )
                .unwrap();

            contract
                .add_legal_document(
                    1,
                    LegalDocType::Title,
                    String::from("Qm12345678901234567890123456789012345678901234"),
                    Hash::from([0x03; 32]),
                    String::from("Title Company"),
                    1700000000,
                    None,
                )
                .unwrap();

            // Admin can verify
            let result = contract.verify_legal_document(1, 1);
            assert!(result.is_ok());

            let docs = contract.get_legal_documents(1);
            assert!(docs[0].is_verified);
        }

        #[ink::test]
        fn add_media_item_works() {
            let mut contract = AdvancedMetadataRegistry::new();
            contract
                .create_metadata(
                    1,
                    default_core(),
                    default_ipfs_resources(),
                    Hash::from([0x01; 32]),
                )
                .unwrap();

            let result = contract.add_media_item(
                1,
                0, // image
                String::from("Qm12345678901234567890123456789012345678901234"),
                String::from("Front view"),
                String::from("image/jpeg"),
                1024 * 1024,
                Hash::from([0x04; 32]),
            );
            assert!(result.is_ok());

            let multimedia = contract.get_multimedia(1).unwrap();
            assert_eq!(multimedia.images.len(), 1);
        }

        #[ink::test]
        fn properties_by_type_query_works() {
            let mut contract = AdvancedMetadataRegistry::new();
            contract
                .create_metadata(
                    1,
                    default_core(),
                    default_ipfs_resources(),
                    Hash::from([0x01; 32]),
                )
                .unwrap();

            let residential = contract.get_properties_by_type(MetadataPropertyType::Residential);
            assert_eq!(residential.len(), 1);
            assert_eq!(residential[0], 1);

            let commercial = contract.get_properties_by_type(MetadataPropertyType::Commercial);
            assert_eq!(commercial.len(), 0);
        }

        #[ink::test]
        fn content_hash_verification_works() {
            let mut contract = AdvancedMetadataRegistry::new();
            contract
                .create_metadata(
                    1,
                    default_core(),
                    default_ipfs_resources(),
                    Hash::from([0x01; 32]),
                )
                .unwrap();

            assert_eq!(
                contract.verify_content_hash(1, Hash::from([0x01; 32])),
                Ok(true)
            );
            assert_eq!(
                contract.verify_content_hash(1, Hash::from([0x02; 32])),
                Ok(false)
            );
        }
    }
}
