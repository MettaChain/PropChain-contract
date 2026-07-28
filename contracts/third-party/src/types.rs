// Data types for the third-party contract (Issue #101 - extracted from lib.rs)

pub type ServiceId = u32;
pub type RequestId = u64;

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
pub enum ServiceType {
    KycProvider,
    PaymentGateway,
    Monitoring,
    DataOracle,
    LegalSigning,
    TaxService,
    Other,
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
pub enum ServiceStatus {
    Active,
    Inactive,
    Suspended,
    Maintenance,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ServiceConfig {
    pub service_id: ServiceId,
    pub service_type: ServiceType,
    pub name: String,
    pub provider_account: AccountId,
    pub endpoint_url: String,
    pub api_version: String,
    pub status: ServiceStatus,
    pub registered_at: u64,
    pub fees_collected: u128,
    pub fee_percentage: u16,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct KycRequest {
    pub request_id: RequestId,
    pub user: AccountId,
    pub service_id: ServiceId,
    pub reference_id: String,
    pub status: RequestStatus,
    pub initiated_at: u64,
    pub updated_at: u64,
    pub expiry_date: Option<u64>,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct PaymentRequest {
    pub request_id: RequestId,
    pub payer: AccountId,
    pub service_id: ServiceId,
    pub target_contract: AccountId,
    pub operation_type: u8,
    pub fiat_amount: u128,
    pub fiat_currency: String,
    pub equivalent_tokens: u128,
    pub payment_reference: String,
    pub status: RequestStatus,
    pub init_time: u64,
    pub complete_time: Option<u64>,
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
pub enum RequestStatus {
    Pending,
    Processing,
    Approved,
    Rejected,
    Failed,
    Expired,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct KycRecord {
    pub user: AccountId,
    pub provider_id: ServiceId,
    pub verification_level: u8,
    pub verified_at: u64,
    pub expires_at: u64,
    pub is_active: bool,
}
