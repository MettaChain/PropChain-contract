//! Identity Management Dashboard Interface
//! 
//! This module provides a high-level interface for identity management operations
//! that can be used by frontend applications and dashboards.

use ink::prelude::string::String;
use ink::prelude::vec::Vec;
use ink::primitives::AccountId;
use super::*;

/// Dashboard interface for identity management operations
pub struct IdentityDashboard {
    registry: AccountId,
}

impl IdentityDashboard {
    /// Create new dashboard interface
    pub fn new(registry_address: AccountId) -> Self {
        Self {
            registry: registry_address,
        }
    }

    /// Get complete identity profile for dashboard display
    pub fn get_identity_profile(&self, account: AccountId) -> Option<IdentityProfile> {
        use ink::env::call::FromAccountId;
        let registry: ink::contract_ref!(IdentityRegistry) =
            FromAccountId::from_account_id(self.registry);

        let identity = registry.get_identity(account)?;
        let reputation_metrics = registry.get_reputation_metrics(account)?;

        Some(IdentityProfile {
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
                success_rate: if reputation_metrics.total_transactions > 0 {
                    (reputation_metrics.successful_transactions * 100) / reputation_metrics.total_transactions
                } else {
                    0
                },
            },
            privacy_settings: identity.privacy_settings,
            cross_chain_verifications: self.get_cross_chain_summary(account),
        })
    }

    /// Get trust assessment summary for counterparty evaluation
    pub fn get_trust_summary(&self, assessor: AccountId, target: AccountId) -> Option<TrustSummary> {
        use ink::env::call::FromAccountId;
        let registry: ink::contract_ref!(IdentityRegistry) =
            FromAccountId::from_account_id(self.registry);

        let trust_assessment = registry.get_trust_assessment(assessor, target)?;
        let target_identity = registry.get_identity(target)?;

        Some(TrustSummary {
            target_account: target,
            trust_score: trust_assessment.trust_score,
            risk_level: trust_assessment.risk_level,
            verification_level: target_identity.verification_level,
            reputation_score: target_identity.reputation_score,
            is_verified: target_identity.is_verified,
            assessment_expires: trust_assessment.expires_at,
            last_assessed: trust_assessment.assessment_date,
            recommended_actions: self.get_recommended_actions(&trust_assessment),
        })
    }

    /// Get identity verification status and requirements
    pub fn get_verification_status(&self, account: AccountId) -> Option<VerificationStatus> {
        use ink::env::call::FromAccountId;
        let registry: ink::contract_ref!(IdentityRegistry) =
            FromAccountId::from_account_id(self.registry);

        let identity = registry.get_identity(account)?;

        Some(VerificationStatus {
            account_id: account,
            current_level: identity.verification_level,
            is_verified: identity.is_verified,
            verified_at: identity.verified_at,
            expires_at: identity.verification_expires,
            next_required_level: self.get_next_verification_level(&identity.verification_level),
            verification_steps: self.get_verification_steps(&identity.verification_level),
        })
    }

    /// Get privacy and security settings
    pub fn get_privacy_security_settings(&self, account: AccountId) -> Option<PrivacySecuritySettings> {
        use ink::env::call::FromAccountId;
        let registry: ink::contract_ref!(IdentityRegistry) =
            FromAccountId::from_account_id(self.registry);

        let identity = registry.get_identity(account)?;

        Some(PrivacySecuritySettings {
            account_id: account,
            privacy_settings: identity.privacy_settings.clone(),
            social_recovery_enabled: !identity.social_recovery.guardians.is_empty(),
            guardian_count: identity.social_recovery.guardians.len() as u8,
            recovery_threshold: identity.social_recovery.threshold,
            is_recovery_active: identity.social_recovery.is_recovery_active,
            supported_chains: registry.get_supported_chains(),
            cross_chain_verifications: self.get_cross_chain_count(account),
        })
    }

    /// Get transaction and activity history
    pub fn get_activity_history(&self, account: AccountId, limit: u32) -> ActivityHistory {
        use ink::env::call::FromAccountId;
        let registry: ink::contract_ref!(IdentityRegistry) =
            FromAccountId::from_account_id(self.registry);

        let reputation_metrics = registry.get_reputation_metrics(account)
            .unwrap_or_else(|| ReputationMetrics {
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
        use ink::env::call::FromAccountId;
        let registry: ink::contract_ref!(IdentityRegistry) =
            FromAccountId::from_account_id(self.registry);

        let identity = match registry.get_identity(account) {
            Some(id) => id,
            None => return Vec::new(),
        };

        let supported_chains = registry.get_supported_chains();
        let mut summaries = Vec::new();

        for chain_id in supported_chains {
            if let Some(verification) = registry.get_cross_chain_verification(account, chain_id) {
                summaries.push(CrossChainSummary {
                    chain_id,
                    chain_name: self.get_chain_name(chain_id),
                    verified_at: verification.verified_at,
                    reputation_score: verification.reputation_score,
                    is_active: verification.is_active,
                });
            }
        }

        summaries
    }

    fn get_cross_chain_count(&self, account: AccountId) -> u32 {
        self.get_cross_chain_summary(account).len() as u32
    }

    fn get_chain_name(&self, chain_id: ChainId) -> String {
        match chain_id {
            1 => "Ethereum".to_string(),
            2 => "Polkadot".to_string(),
            3 => "Avalanche".to_string(),
            4 => "BSC".to_string(),
            5 => "Polygon".to_string(),
            _ => format!("Chain {}", chain_id),
        }
    }

    fn get_next_verification_level(&self, current: &VerificationLevel) -> VerificationLevel {
        match current {
            VerificationLevel::None => VerificationLevel::Basic,
            VerificationLevel::Basic => VerificationLevel::Standard,
            VerificationLevel::Standard => VerificationLevel::Enhanced,
            VerificationLevel::Enhanced => VerificationLevel::Premium,
            VerificationLevel::Premium => VerificationLevel::Premium, // Already at highest level
        }
    }

    fn get_verification_steps(&self, current: &VerificationLevel) -> Vec<String> {
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

    fn get_recommended_actions(&self, assessment: &TrustAssessment) -> Vec<String> {
        let mut actions = Vec::new();

        match assessment.risk_level {
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
