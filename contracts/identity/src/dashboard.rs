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

    /// Pure aggregation behind [`Self::get_activity_history`]. Falls back to
    /// zeroed defaults when the account has no recorded metrics.
    pub fn build_activity_history(
        account: AccountId,
        reputation_metrics: Option<ReputationMetrics>,
    ) -> ActivityHistory {
        let reputation_metrics = reputation_metrics.unwrap_or_else(|| ReputationMetrics {
            total_transactions: 0,
            successful_transactions: 0,
            failed_transactions: 0,
            dispute_count: 0,
            dispute_resolved_count: 0,
            average_transaction_value: 0,
            total_value_transacted: 0,
            last_updated: 0,
            reputation_score: 500,
        });

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

    /// Get dashboard statistics for admin view
    pub fn get_dashboard_statistics(&self) -> DashboardStatistics {
        // This would typically aggregate data from multiple sources
        // For now, return placeholder data
        DashboardStatistics {
            total_identities: 0,
            verified_identities: 0,
            average_reputation_score: 500,
            total_transactions: 0,
            active_verifications: 0,
            supported_chains: 5,
            cross_chain_verifications: 0,
            recovery_requests: 0,
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
pub struct DashboardStatistics {
    pub total_identities: u64,
    pub verified_identities: u64,
    pub average_reputation_score: u32,
    pub total_transactions: u64,
    pub active_verifications: u64,
    pub supported_chains: u32,
    pub cross_chain_verifications: u64,
    pub recovery_requests: u64,
}

pub mod cross_contract_helper {
    include!("cross_contract_helper.rs");
}
