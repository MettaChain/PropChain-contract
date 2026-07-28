// Data types for the metadata contract (Issue #101 - extracted from lib.rs)

pub type PropertyId = u64;
pub type MetadataVersion = u32;
pub type IpfsCid = String;

/// Core property metadata with extensible fields
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct AdvancedPropertyMetadata {
    pub property_id: PropertyId,
    pub version: MetadataVersion,
    pub core: CoreMetadata,
    pub ipfs_resources: IpfsResources,
    pub multimedia: MultimediaContent,
    pub legal_documents: Vec<LegalDocumentRef>,
    pub custom_attributes: Vec<MetadataAttribute>,
    pub content_hash: Hash,
    pub created_at: u64,
    pub updated_at: u64,
    pub created_by: AccountId,
    pub is_finalized: bool,
}

/// Core property information (required fields)
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct CoreMetadata {
    pub name: String,
    pub location: String,
    pub size_sqm: u64,
    pub property_type: MetadataPropertyType,
    pub valuation: u128,
    pub legal_description: String,
    pub coordinates: Option<(i64, i64)>,
    pub year_built: Option<u32>,
    pub bedrooms: Option<u8>,
    pub bathrooms: Option<u8>,
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
    pub metadata_cid: Option<IpfsCid>,
    pub documents_cid: Option<IpfsCid>,
    pub images_cid: Option<IpfsCid>,
    pub legal_docs_cid: Option<IpfsCid>,
    pub virtual_tour_cid: Option<IpfsCid>,
    pub floor_plans_cid: Option<IpfsCid>,
}

/// Multimedia content references
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct MultimediaContent {
    pub images: Vec<MediaItem>,
    pub videos: Vec<MediaItem>,
    pub virtual_tours: Vec<MediaItem>,
    pub floor_plans: Vec<MediaItem>,
}

/// Individual media item reference
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct MediaItem {
    pub content_ref: String,
    pub description: String,
    pub mime_type: String,
    pub file_size: u64,
    pub content_hash: Hash,
    pub uploaded_at: u64,
}

/// Legal document reference
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct LegalDocumentRef {
    pub document_id: u64,
    pub document_type: LegalDocType,
    pub ipfs_cid: IpfsCid,
    pub content_hash: Hash,
    pub issuer: String,
    pub issue_date: u64,
    pub expiry_date: Option<u64>,
    pub is_verified: bool,
    pub verified_by: Option<AccountId>,
}

/// Legal document types
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
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
    HoaDocument,
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
    pub key: String,
    pub value: MetadataValue,
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
    pub snapshot_cid: Option<IpfsCid>,
}
