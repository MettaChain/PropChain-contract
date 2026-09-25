// Identity Management Dashboard Interface
//
// This module provides a high-level interface for identity management operations
// that can be used by frontend applications and dashboards.
//
// Cross-contract queries are dispatched with explicit message selectors so no
// additional trait surface is required on the registry. Because the off-chain
// test engine cannot execute contract invocations, every message is split into
// a thin dispatching wrapper plus a pure `build_*` constructor that is unit
// tested against seeded registry state.

use ink::env::call::{build_call, ExecutionInput, Selector};
use ink::prelude::string::String;
use ink::prelude::vec::Vec;
use ink::primitives::AccountId;
use super::*;

/// Dashboard interface for identity management operations
pub struct IdentityDashboard {
    registry: AccountId,
}

impl IdentityDashboard {
    /// Issue a read-only cross-contract query against the registry.
    fn query_registry<A: scale::Encode, R: scale::Decode>(
        &self,
        selector: [u8; 4],
        args: A,
    ) -> Option<R> {
        match build_call::<ink::env::DefaultEnvironment>()
            .call(self.registry)
            .exec_input(ExecutionInput::new(Selector::new(selector)).push_arg(&args))
            .returns::<R>()
            .try_invoke()
        {
            Ok(ink::primitives::MessageResult::Ok(value)) => Some(value),
            _ => None,
        }
    }

    fn registry_get_identity(&self, account: AccountId) -> Option<Identity> {
        self.query_registry(ink::selector_bytes!("get_identity"), &(account,))
    }

    fn registry_get_reputation_metrics(&self, account: AccountId) -> Option<ReputationMetrics> {
        self.query_registry(ink::selector_bytes!("get_reputation_metrics"), &(account,))
    }

    fn registry_get_trust_assessment(
        &self,
        assessor: AccountId,
        target: AccountId,
    ) -> Option<TrustAssessment> {
        self.query_registry(
            ink::selector_bytes!("get_trust_assessment"),
            &(assessor, target),
        )
    }

    fn registry_get_cross_chain_verification(
        &self,
        account: AccountId,
        chain_id: ChainId,
    ) -> Option<CrossChainVerification> {
        self.query_registry(
            ink::selector_bytes!("get_cross_chain_verification"),
            &(account, chain_id),
        )
    }

    fn registry_get_supported_chains(&self) -> Vec<ChainId> {
        self.query_registry(ink::selector_bytes!("get_supported_chains"), &())
            .unwrap_or_default()
    }

    fn registry_get_audit_count(&self) -> u64 {
        self.query_registry(ink::selector_bytes!("get_audit_count"), &())
            .unwrap_or(0)
    }

    fn registry_get_account_audit_count(&self, account: AccountId) -> u64 {
        self.query_registry(ink::selector_bytes!("get_account_audit_count"), &(account,))
            .unwrap_or(0)
    }

    /// Create new dashboard interface
    pub fn new(registry_address: AccountId) -> Self {
        Self {
            registry: registry_address,
        }
    }

    /// Get complete identity profile for dashboard display
    pub fn get_identity_profile(&self, account: AccountId) -> Option<IdentityProfile> {
        let identity = self.registry_get_identity(account)?;
        let reputation_metrics = self.registry_get_reputation_metrics(account)?;
        let cross_chain_verifications = self.get_cross_chain_summary(account);
        Some(Self::build_identity_profile(
            account,
            identity,
            reputation_metrics,
            cross_chain_verifications,
        ))
    }

    /// Pure aggregation behind [`Self::get_identity_profile`].
    pub fn build_identity_profile(
        account: AccountId,
        identity: Identity,
        reputation_metrics: ReputationMetrics,
        cross_chain_verifications: Vec<CrossChainSummary>,
    ) -> IdentityProfile {
        IdentityProfile {
            account_id: account,
            did: identity.did_document.did,
            verification_level: identity.verification_level,
            is_verified: identity.is_verified,
            reputation_score: identity.reputation_score,
            trust_score: identity.trust_score,
            verification_expires: identity.verification_expires,
            created_at: identity.created_at,
            last_activity: identity.last_activity,
            reputation_metrics: ReputationProfile {
                total_transactions: reputation_metrics.total_transactions,
                successful_transactions: reputation_metrics.successful_transactions,
                failed_transactions: reputation_metrics.failed_transactions,
                dispute_count: reputation_metrics.dispute_count,
                average_transaction_value: reputation_metrics.average_transaction_value,
                total_value_transacted: reputation_metrics.total_value_transacted,
                success_rate: (reputation_metrics.successful_transactions * 100)
                    .checked_div(reputation_metrics.total_transactions)
                    .unwrap_or(0),
            },
            privacy_settings: identity.privacy_settings,
            cross_chain_verifications,
        }
    }

    /// Get trust assessment summary for counterparty evaluation
    pub fn get_trust_summary(&self, assessor: AccountId, target: AccountId) -> Option<TrustSummary> {
        let trust_assessment = self.registry_get_trust_assessment(assessor, target)?;
        let target_identity = self.registry_get_identity(target)?;
        Some(Self::build_trust_summary(
            target,
            trust_assessment,
            target_identity,
        ))
    }

    /// Pure aggregation behind [`Self::get_trust_summary`].
    pub fn build_trust_summary(
        target_account: AccountId,
        assessment: TrustAssessment,
        target_identity: Identity,
    ) -> TrustSummary {
        TrustSummary {
            target_account,
            trust_score: assessment.trust_score,
            risk_level: assessment.risk_level.clone(),
            verification_level: target_identity.verification_level,
            reputation_score: target_identity.reputation_score,
            is_verified: target_identity.is_verified,
            assessment_expires: assessment.expires_at,
            last_assessed: assessment.assessment_date,
            recommended_actions: Self::recommended_actions_for(&assessment.risk_level),
        }
    }

    /// Get identity verification status and requirements
    pub fn get_verification_status(&self, account: AccountId) -> Option<VerificationStatus> {
        let identity = self.registry_get_identity(account)?;
        Some(Self::build_verification_status(account, identity))
    }

    /// Pure aggregation behind [`Self::get_verification_status`].
    pub fn build_verification_status(
        account: AccountId,
        identity: Identity,
    ) -> VerificationStatus {
        VerificationStatus {
            account_id: account,
            current_level: identity.verification_level,
            is_verified: identity.is_verified,
            verified_at: identity.verified_at,
            expires_at: identity.verification_expires,
            next_required_level: Self::next_verification_level(&identity.verification_level),
            verification_steps: Self::verification_steps(&identity.verification_level),
        }
    }

    /// Get privacy and security settings
    pub fn get_privacy_security_settings(
        &self,
        account: AccountId,
    ) -> Option<PrivacySecuritySettings> {
        let identity = self.registry_get_identity(account)?;
        let supported_chains = self.registry_get_supported_chains();
        let cross_chain_verifications = self.get_cross_chain_count(account);
        Some(Self::build_privacy_security_settings(
            account,
            identity,
            supported_chains,
            cross_chain_verifications,
        ))
    }

    /// Pure aggregation behind [`Self::get_privacy_security_settings`].
    pub fn build_privacy_security_settings(
        account: AccountId,
        identity: Identity,
        supported_chains: Vec<ChainId>,
        cross_chain_verifications: u32,
    ) -> PrivacySecuritySettings {
        PrivacySecuritySettings {
            account_id: account,
            privacy_settings: identity.privacy_settings.clone(),
            social_recovery_enabled: !identity.social_recovery.guardians.is_empty(),
            guardian_count: identity.social_recovery.guardians.len() as u8,
            recovery_threshold: identity.social_recovery.threshold,
            is_recovery_active: identity.social_recovery.is_recovery_active,
            supported_chains,
            cross_chain_verifications,
        }
    }

    /// Get transaction and activity history
    pub fn get_activity_history(&self, account: AccountId, _limit: u32) -> ActivityHistory {
        let reputation_metrics = self.registry_get_reputation_metrics(account);
        Self::build_activity_history(account, reputation_metrics)
    }

    /// Pure aggregation behind [`Self::get_activity_history`]. An account with no
    /// recorded metrics reports zeroes; the previous fallback invented a
    /// `reputation_score` of 500, which is a plausible-looking value for an
    /// account that has never transacted (#1129).
    pub fn build_activity_history(
        account: AccountId,
        reputation_metrics: Option<ReputationMetrics>,
    ) -> ActivityHistory {
        let reputation_metrics = reputation_metrics.unwrap_or_default();

        ActivityHistory {
            account_id: account,
            total_transactions: reputation_metrics.total_transactions,
            successful_transactions: reputation_metrics.successful_transactions,
            failed_transactions: reputation_metrics.failed_transactions,
            dispute_count: reputation_metrics.dispute_count,
            dispute_resolved_count: reputation_metrics.dispute_resolved_count,
            average_transaction_value: reputation_metrics.average_transaction_value,
            total_value_transacted: reputation_metrics.total_value_transacted,
            last_updated: reputation_metrics.last_updated,
            recent_activities: Vec::new(), // Would be populated from event logs
        }
    }

    /// Get dashboard statistics for the whole registry.
    ///
    /// Only figures that the registry can actually answer are reported. The
    /// previous version of this method returned literals for every field
    /// (`total_identities: 0`, `average_reputation_score: 500`,
    /// `supported_chains: 5`), which is how a dashboard ends up showing an
    /// average reputation of 500 on an empty registry (#1129).
    ///
    /// Identity, transaction and reputation aggregates are deliberately absent:
    /// the registry stores identities in `Mapping<AccountId, Identity>` with no
    /// secondary index and no counters, so no message can report them without a
    /// full scan. Adding those counters is a separate change; until it exists
    /// this view reports the supported-chain and audit totals it can read.
    pub fn get_registry_dashboard(&self) -> RegistryDashboard {
        let supported_chains = self.registry_get_supported_chains();
        let audit_entries = self.registry_get_audit_count();
        Self::build_registry_dashboard(supported_chains, audit_entries)
    }

    /// Pure aggregation behind [`Self::get_registry_dashboard`].
    pub fn build_registry_dashboard(
        supported_chains: Vec<ChainId>,
        audit_entries: u64,
    ) -> RegistryDashboard {
        RegistryDashboard {
            supported_chains: supported_chains.len() as u32,
            audit_entries,
        }
    }

    /// Get the dashboard view for a single account.
    ///
    /// Every field is copied from stored registry state: the identity itself,
    /// its reputation metrics, the cross-chain verifications that exist for it,
    /// and the number of audit entries recorded against it. Returns `None` when
    /// the account has no identity, rather than a row of zeroes that reads like
    /// a real profile (#1129).
    pub fn get_account_dashboard(&self, account: AccountId) -> Option<AccountDashboard> {
        let identity = self.registry_get_identity(account)?;
        let reputation_metrics = self
            .registry_get_reputation_metrics(account)
            .unwrap_or_default();
        let supported_chains = self.registry_get_supported_chains();
        let cross_chain_verifications = self.active_cross_chain_verification_count(account);
        let audit_entries = self.registry_get_account_audit_count(account);
        Some(Self::build_account_dashboard(
            account,
            identity,
            reputation_metrics,
            cross_chain_verifications,
            supported_chains.len() as u32,
            audit_entries,
        ))
    }

    /// Pure aggregation behind [`Self::get_account_dashboard`].
    pub fn build_account_dashboard(
        account: AccountId,
        identity: Identity,
        reputation_metrics: ReputationMetrics,
        cross_chain_verifications: u32,
        supported_chains: u32,
        audit_entries: u64,
    ) -> AccountDashboard {
        AccountDashboard {
            account_id: account,
            verification_level: identity.verification_level,
            is_verified: identity.is_verified,
            reputation_score: identity.reputation_score,
            trust_score: identity.trust_score,
            kyc_tier: identity.kyc_tier,
            verification_expires: identity.verification_expires,
            created_at: identity.created_at,
            last_activity: identity.last_activity,
            supported_chains,
            cross_chain_verifications,
            total_transactions: reputation_metrics.total_transactions,
            successful_transactions: reputation_metrics.successful_transactions,
            failed_transactions: reputation_metrics.failed_transactions,
            dispute_count: reputation_metrics.dispute_count,
            dispute_resolved_count: reputation_metrics.dispute_resolved_count,
            average_transaction_value: reputation_metrics.average_transaction_value,
            total_value_transacted: reputation_metrics.total_value_transacted,
            last_updated: reputation_metrics.last_updated,
            audit_entries,
            social_recovery_enabled: !identity.social_recovery.guardians.is_empty(),
            guardian_count: identity.social_recovery.guardians.len() as u8,
            recovery_threshold: identity.social_recovery.threshold,
            is_recovery_active: identity.social_recovery.is_recovery_active,
        }
    }

    // Helper methods
    fn get_cross_chain_summary(&self, account: AccountId) -> Vec<CrossChainSummary> {
        if self.registry_get_identity(account).is_none() {
            return Vec::new();
        }

        let supported_chains = self.registry_get_supported_chains();
        let verifications: Vec<(ChainId, Option<CrossChainVerification>)> = supported_chains
            .iter()
            .map(|chain_id| {
                (
                    *chain_id,
                    self.registry_get_cross_chain_verification(account, *chain_id),
                )
            })
            .collect();

        Self::build_cross_chain_summary(verifications)
    }

    /// Pure aggregation behind [`Self::get_cross_chain_summary`].
    pub fn build_cross_chain_summary(
        verifications: Vec<(ChainId, Option<CrossChainVerification>)>,
    ) -> Vec<CrossChainSummary> {
        verifications
            .into_iter()
            .filter_map(|(chain_id, verification)| {
                verification.map(|v| CrossChainSummary {
                    chain_id,
                    chain_name: Self::chain_name(chain_id),
                    verified_at: v.verified_at,
                    reputation_score: v.reputation_score,
                    is_active: v.is_active,
                })
            })
            .collect()
    }

    fn get_cross_chain_count(&self, account: AccountId) -> u32 {
        self.get_cross_chain_summary(account).len() as u32
    }

    /// Count the cross-chain verifications stored for `account` that have not
    /// been deactivated. Reuses [`Self::build_cross_chain_summary`] so the
    /// dashboard and the summary agree on what counts as active.
    fn active_cross_chain_verification_count(&self, account: AccountId) -> u32 {
        let verifications: Vec<(ChainId, Option<CrossChainVerification>)> = self
            .registry_get_supported_chains()
            .iter()
            .map(|chain_id| {
                (
                    *chain_id,
                    self.registry_get_cross_chain_verification(account, *chain_id),
                )
            })
            .collect();

        Self::build_cross_chain_summary(verifications)
            .iter()
            .filter(|summary| summary.is_active)
            .count() as u32
    }

    fn chain_name(chain_id: ChainId) -> String {
        match chain_id {
            1 => "Ethereum".to_string(),
            2 => "Polkadot".to_string(),
            3 => "Avalanche".to_string(),
            4 => "BSC".to_string(),
            5 => "Polygon".to_string(),
            _ => format!("Chain {}", chain_id),
        }
    }

    fn next_verification_level(current: &VerificationLevel) -> VerificationLevel {
        match current {
            VerificationLevel::None => VerificationLevel::Basic,
            VerificationLevel::Basic => VerificationLevel::Standard,
            VerificationLevel::Standard => VerificationLevel::Enhanced,
            VerificationLevel::Enhanced => VerificationLevel::Premium,
            VerificationLevel::Premium => VerificationLevel::Premium, // Already at highest level
        }
    }

    fn verification_steps(current: &VerificationLevel) -> Vec<String> {
        match current {
            VerificationLevel::None => vec![
                "Create DID document".to_string(),
                "Complete basic identity verification".to_string(),
            ],
            VerificationLevel::Basic => vec![
                "Submit KYC documents".to_string(),
                "Complete identity verification".to_string(),
            ],
            VerificationLevel::Standard => vec![
                "Provide additional verification documents".to_string(),
                "Complete enhanced due diligence".to_string(),
            ],
            VerificationLevel::Enhanced => vec![
                "Submit premium verification documents".to_string(),
                "Complete comprehensive background check".to_string(),
            ],
            VerificationLevel::Premium => vec![], // Already at highest level
        }
    }

    fn recommended_actions_for(risk: &RiskLevel) -> Vec<String> {
        let mut actions = Vec::new();

        match risk {
            RiskLevel::Low => {
                actions.push("Proceed with transaction".to_string());
                actions.push("Standard verification sufficient".to_string());
            }
            RiskLevel::Medium => {
                actions.push("Consider additional verification".to_string());
                actions.push("Use escrow for high-value transactions".to_string());
            }
            RiskLevel::High => {
                actions.push("Require enhanced verification".to_string());
                actions.push("Use multi-signature escrow".to_string());
                actions.push("Consider insurance".to_string());
            }
            RiskLevel::Critical => {
                actions.push("Avoid transaction".to_string());
                actions.push("Report suspicious activity".to_string());
            }
        }

        actions
    }
}

/// Data structures for dashboard display

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct IdentityProfile {
    pub account_id: AccountId,
    pub did: String,
    pub verification_level: VerificationLevel,
    pub is_verified: bool,
    pub reputation_score: u32,
    pub trust_score: u32,
    pub verification_expires: Option<u64>,
    pub created_at: u64,
    pub last_activity: u64,
    pub reputation_metrics: ReputationProfile,
    pub privacy_settings: PrivacySettings,
    pub cross_chain_verifications: Vec<CrossChainSummary>,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ReputationProfile {
    pub total_transactions: u64,
    pub successful_transactions: u64,
    pub failed_transactions: u64,
    pub dispute_count: u64,
    pub average_transaction_value: u128,
    pub total_value_transacted: u128,
    pub success_rate: u64,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct CrossChainSummary {
    pub chain_id: ChainId,
    pub chain_name: String,
    pub verified_at: u64,
    pub reputation_score: u32,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct TrustSummary {
    pub target_account: AccountId,
    pub trust_score: u32,
    pub risk_level: RiskLevel,
    pub verification_level: VerificationLevel,
    pub reputation_score: u32,
    pub is_verified: bool,
    pub assessment_expires: u64,
    pub last_assessed: u64,
    pub recommended_actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct VerificationStatus {
    pub account_id: AccountId,
    pub current_level: VerificationLevel,
    pub is_verified: bool,
    pub verified_at: Option<u64>,
    pub expires_at: Option<u64>,
    pub next_required_level: VerificationLevel,
    pub verification_steps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct PrivacySecuritySettings {
    pub account_id: AccountId,
    pub privacy_settings: PrivacySettings,
    pub social_recovery_enabled: bool,
    pub guardian_count: u8,
    pub recovery_threshold: u8,
    pub is_recovery_active: bool,
    pub supported_chains: Vec<ChainId>,
    pub cross_chain_verifications: u32,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ActivityHistory {
    pub account_id: AccountId,
    pub total_transactions: u64,
    pub successful_transactions: u64,
    pub failed_transactions: u64,
    pub dispute_count: u64,
    pub dispute_resolved_count: u64,
    pub average_transaction_value: u128,
    pub total_value_transacted: u128,
    pub last_updated: u64,
    pub recent_activities: Vec<String>, // Would contain actual activity details
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct RegistryDashboard {
    /// Number of chains the registry accepts cross-chain verifications for.
    pub supported_chains: u32,
    /// Total audit entries recorded registry-wide.
    pub audit_entries: u64,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct AccountDashboard {
    pub account_id: AccountId,
    pub verification_level: VerificationLevel,
    pub is_verified: bool,
    pub reputation_score: u32,
    pub trust_score: u32,
    pub kyc_tier: KycTier,
    pub verification_expires: Option<u64>,
    pub created_at: u64,
    pub last_activity: u64,
    pub supported_chains: u32,
    pub cross_chain_verifications: u32,
    pub total_transactions: u64,
    pub successful_transactions: u64,
    pub failed_transactions: u64,
    pub dispute_count: u64,
    pub dispute_resolved_count: u64,
    pub average_transaction_value: u128,
    pub total_value_transacted: u128,
    pub last_updated: u64,
    pub audit_entries: u64,
    pub social_recovery_enabled: bool,
    pub guardian_count: u8,
    pub recovery_threshold: u8,
    pub is_recovery_active: bool,
}

/// The dashboard's only source of truth is stored registry state, so these
/// tests seed that state directly through the pure `build_*` constructors and
/// assert every field is copied from it. They fail if a literal is ever
/// reintroduced in place of a stored value (#1129).
#[cfg(test)]
mod tests {
    use super::*;

    fn account(byte: u8) -> AccountId {
        AccountId::from([byte; 32])
    }

    fn did_document(did: &str) -> DIDDocument {
        DIDDocument {
            did: did.to_string(),
            public_key: vec![1, 2, 3],
            verification_method: "Ed25519".to_string(),
            service_endpoint: None,
            created_at: 1_000,
            updated_at: 1_000,
            version: 1,
        }
    }

    fn identity() -> Identity {
        Identity {
            account_id: account(0x0a),
            did_document: did_document("did:propchain:0x0a"),
            reputation_score: 742,
            verification_level: VerificationLevel::Enhanced,
            kyc_tier: KycTier::Tier2_Standard,
            trust_score: 61,
            is_verified: true,
            verified_at: Some(2_000),
            verification_expires: Some(9_000),
            social_recovery: SocialRecoveryConfig {
                guardians: vec![account(0xb1), account(0xb2)],
                threshold: 2,
                recovery_period: 100,
                timelock_period: 50,
                last_recovery_attempt: None,
                is_recovery_active: false,
                recovery_approvals: Vec::new(),
                recovery_completion_timestamp: None,
            },
            privacy_settings: PrivacySettings {
                public_reputation: true,
                public_verification: true,
                data_sharing_consent: false,
                zero_knowledge_proof: false,
                selective_disclosure: vec![],
            },
            created_at: 1_000,
            last_activity: 4_321,
        }
    }

    fn metrics() -> ReputationMetrics {
        ReputationMetrics {
            total_transactions: 120,
            successful_transactions: 100,
            failed_transactions: 20,
            dispute_count: 4,
            dispute_resolved_count: 3,
            average_transaction_value: 1_500,
            total_value_transacted: 180_000,
            last_updated: 4_300,
            reputation_score: 742,
        }
    }

    fn verification(chain_id: ChainId, is_active: bool) -> CrossChainVerification {
        CrossChainVerification {
            chain_id,
            verified_at: 3_000,
            verification_hash: ink::primitives::Hash::from([0x42u8; 32]),
            reputation_score: 700,
            is_active,
        }
    }

    #[test]
    fn account_dashboard_copies_identity_state() {
        let dashboard = IdentityDashboard::build_account_dashboard(
            account(0x0a),
            identity(),
            metrics(),
            2,
            3,
            7,
        );

        assert_eq!(dashboard.account_id, account(0x0a));
        assert_eq!(dashboard.verification_level, VerificationLevel::Enhanced);
        assert!(dashboard.is_verified);
        assert_eq!(dashboard.reputation_score, 742);
        assert_eq!(dashboard.trust_score, 61);
        assert_eq!(dashboard.kyc_tier, KycTier::Tier2_Standard);
        assert_eq!(dashboard.verification_expires, Some(9_000));
        assert_eq!(dashboard.created_at, 1_000);
        assert_eq!(dashboard.last_activity, 4_321);
    }

    #[test]
    fn account_dashboard_copies_reputation_metrics() {
        let dashboard = IdentityDashboard::build_account_dashboard(
            account(0x0a),
            identity(),
            metrics(),
            2,
            3,
            7,
        );

        assert_eq!(dashboard.total_transactions, 120);
        assert_eq!(dashboard.successful_transactions, 100);
        assert_eq!(dashboard.failed_transactions, 20);
        assert_eq!(dashboard.dispute_count, 4);
        assert_eq!(dashboard.dispute_resolved_count, 3);
        assert_eq!(dashboard.average_transaction_value, 1_500);
        assert_eq!(dashboard.total_value_transacted, 180_000);
        assert_eq!(dashboard.last_updated, 4_300);
    }

    #[test]
    fn account_dashboard_copies_counts_and_recovery_state() {
        let dashboard = IdentityDashboard::build_account_dashboard(
            account(0x0a),
            identity(),
            metrics(),
            2,
            3,
            7,
        );

        assert_eq!(dashboard.supported_chains, 3);
        assert_eq!(dashboard.cross_chain_verifications, 2);
        assert_eq!(dashboard.audit_entries, 7);
        assert!(dashboard.social_recovery_enabled);
        assert_eq!(dashboard.guardian_count, 2);
        assert_eq!(dashboard.recovery_threshold, 2);
        assert!(!dashboard.is_recovery_active);
    }

    #[test]
    fn account_without_social_recovery_reports_it_disabled() {
        let mut identity = identity();
        identity.social_recovery.guardians = Vec::new();

        let dashboard =
            IdentityDashboard::build_account_dashboard(account(0x0a), identity, metrics(), 0, 1, 0);

        assert!(!dashboard.social_recovery_enabled);
        assert_eq!(dashboard.guardian_count, 0);
    }

    /// A cross-chain verification that has been deactivated must not be counted
    /// as an active one; the old placeholder reported a hardcoded `0` here.
    #[test]
    fn only_active_cross_chain_verifications_are_counted() {
        let summaries = IdentityDashboard::build_cross_chain_summary(vec![
            (1, Some(verification(1, true))),
            (2, Some(verification(2, true))),
            (3, Some(verification(3, false))),
            (4, None),
        ]);

        let active = summaries.iter().filter(|s| s.is_active).count() as u32;

        assert_eq!(summaries.len(), 3, "inactive and missing entries are still listed");
        assert_eq!(active, 2, "only the two active verifications are counted");
    }

    #[test]
    fn registry_dashboard_reports_real_totals() {
        let dashboard = IdentityDashboard::build_registry_dashboard(vec![1, 2, 3], 42);

        assert_eq!(dashboard.supported_chains, 3);
        assert_eq!(dashboard.audit_entries, 42);
    }

    /// An empty registry must read as zero, not as the fabricated `500`
    /// reputation and `5` supported chains the old statistics reported.
    #[test]
    fn empty_registry_reports_zeroes() {
        let dashboard = IdentityDashboard::build_registry_dashboard(Vec::new(), 0);

        assert_eq!(dashboard.supported_chains, 0);
        assert_eq!(dashboard.audit_entries, 0);
    }

    #[test]
    fn activity_history_without_metrics_reports_zeroes() {
        let history = IdentityDashboard::build_activity_history(account(0x0a), None);

        assert_eq!(history.total_transactions, 0);
        assert_eq!(history.successful_transactions, 0);
        assert_eq!(history.average_transaction_value, 0);
        assert!(history.recent_activities.is_empty());
    }

    #[test]
    fn activity_history_reflects_recorded_metrics() {
        let history = IdentityDashboard::build_activity_history(account(0x0a), Some(metrics()));

        assert_eq!(history.total_transactions, 120);
        assert_eq!(history.dispute_resolved_count, 3);
        assert_eq!(history.last_updated, 4_300);
    }

    #[test]
    fn identity_profile_success_rate_is_derived_from_metrics() {
        let profile = IdentityDashboard::build_identity_profile(
            account(0x0a),
            identity(),
            metrics(),
            Vec::new(),
        );

        assert_eq!(profile.reputation_metrics.success_rate, 83);
    }
}