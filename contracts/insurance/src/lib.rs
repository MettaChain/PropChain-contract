#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::needless_borrows_for_generic_args,
    clippy::too_many_arguments
)]

use ink::storage::Mapping;

// Risk Assessment Model (Task #254)
mod risk_assessment;

// Fraud Detection System (Task #258)
mod fraud_detection;

/// Decentralized Property Insurance Platform
#[ink::contract]
mod propchain_insurance {
    use super::*;
    use ink::prelude::{string::String, vec::Vec};
    use propchain_traits::{non_reentrant, ReentrancyError, ReentrancyGuard};

    // Error types extracted to errors.rs (Issue #101)
    include!("errors.rs");

    impl From<ReentrancyError> for InsuranceError {
        fn from(_: ReentrancyError) -> Self {
            InsuranceError::ReentrantCall
        }
    }

    // Data types extracted to types.rs (Issue #101)
    include!("types.rs");

    // Risk Assessment Model (Task #254)
    use crate::risk_assessment::risk_model;

    // Fraud Detection System (Task #258)
    use crate::fraud_detection::fraud_detection;
    // Premium calculation engine
    mod premium_engine;

    #[ink(storage)]
    pub struct PropertyInsurance {
        admin: AccountId,

        // Policies
        policies: Mapping<u64, InsurancePolicy>,
        policy_count: u64,
        policyholder_policies: Mapping<AccountId, Vec<u64>>,
        property_policies: Mapping<u64, Vec<u64>>,

        // Claims
        claims: Mapping<u64, InsuranceClaim>,
        claim_count: u64,
        policy_claims: Mapping<u64, Vec<u64>>,

        // Risk Pools
        pools: Mapping<u64, RiskPool>,
        pool_count: u64,

        // Risk Assessments
        risk_assessments: Mapping<u64, RiskAssessment>,

        // Reinsurance
        reinsurance_agreements: Mapping<u64, ReinsuranceAgreement>,
        reinsurance_count: u64,

        // Reinsurance distribution ledger
        premium_cessions: Mapping<u64, PremiumCession>,
        cession_count: u64,
        loss_recoveries: Mapping<u64, LossRecovery>,
        loss_recovery_count: u64,
        // agreement_id -> list of cession IDs
        agreement_cessions: Mapping<u64, Vec<u64>>,
        // agreement_id -> list of recovery IDs
        agreement_recoveries: Mapping<u64, Vec<u64>>,

        // Insurance Tokens (secondary market)
        insurance_tokens: Mapping<u64, InsuranceToken>,
        token_count: u64,
        token_listings: Vec<u64>, // Tokens listed for sale

        // Actuarial Models
        actuarial_models: Mapping<u64, ActuarialModel>,
        model_count: u64,

        // Underwriting
        underwriting_criteria: Mapping<u64, UnderwritingCriteria>, // pool_id -> criteria

        // Liquidity providers
        liquidity_providers: Mapping<(u64, AccountId), PoolLiquidityProvider>,
        pool_providers: Mapping<u64, Vec<AccountId>>,

        // Oracle addresses
        authorized_oracles: Mapping<AccountId, bool>,

        // Assessors
        authorized_assessors: Mapping<AccountId, bool>,

        // Claim cooldown: property_id -> last_claim_timestamp
        claim_cooldowns: Mapping<u64, u64>,

        // Claim automation: oracle-triggered parametric claims
        claim_triggers: Mapping<u64, ClaimTrigger>,
        trigger_count: u64,
        policy_triggers: Mapping<u64, Vec<u64>>, // policy_id -> trigger_ids

        // Platform settings
        platform_fee_rate: u32,     // Basis points (e.g. 200 = 2%)
        claim_cooldown_period: u64, // In seconds
        min_pool_capital: u128,

        // =========================================================================
        // RISK ASSESSMENT MODEL (Task #254)
        // =========================================================================
        property_risk_models: Mapping<u64, PropertyRiskModel>,
        risk_model_count: u64,

        // =========================================================================
        // FRAUD DETECTION SYSTEM (Task #258)
        // =========================================================================
        fraud_assessments: Mapping<u64, FraudRiskAssessment>,
        fraud_assessment_count: u64,
        fraud_patterns: Mapping<u64, FraudPattern>,
        fraud_pattern_count: u64,
        fraud_detection_stats: Option<FraudDetectionStats>,

        // Reentrancy protection
        reentrancy_guard: ReentrancyGuard,

        // ── Parametric insurance (Issue #249) ────────────────────────────────
        parametric_policies: Mapping<u64, ParametricPolicy>,
        parametric_policy_count: u64,
        /// property_id → list of parametric policy IDs
        property_parametric_policies: Mapping<u64, Vec<u64>>,
        /// holder → list of parametric policy IDs
        holder_parametric_policies: Mapping<AccountId, Vec<u64>>,
        /// oracle data points submitted
        oracle_data: Mapping<u64, OracleDataPoint>,
        oracle_data_count: u64,
    }

    // =========================================================================
    // EVENTS
    // =========================================================================

    #[ink(event)]
    pub struct PolicyCreated {
        #[ink(topic)]
        policy_id: u64,
        #[ink(topic)]
        policyholder: AccountId,
        #[ink(topic)]
        property_id: u64,
        coverage_type: CoverageType,
        coverage_amount: u128,
        premium_amount: u128,
        start_time: u64,
        end_time: u64,
    }

    #[ink(event)]
    pub struct PolicyCancelled {
        #[ink(topic)]
        policy_id: u64,
        #[ink(topic)]
        policyholder: AccountId,
        cancelled_at: u64,
    }

    #[ink(event)]
    pub struct ClaimSubmitted {
        #[ink(topic)]
        claim_id: u64,
        #[ink(topic)]
        policy_id: u64,
        #[ink(topic)]
        claimant: AccountId,
        claim_amount: u128,
        submitted_at: u64,
    }

    #[ink(event)]
    pub struct ClaimApproved {
        #[ink(topic)]
        claim_id: u64,
        #[ink(topic)]
        policy_id: u64,
        payout_amount: u128,
        approved_by: AccountId,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct ClaimRejected {
        #[ink(topic)]
        claim_id: u64,
        #[ink(topic)]
        policy_id: u64,
        reason: String,
        rejected_by: AccountId,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct PayoutExecuted {
        #[ink(topic)]
        claim_id: u64,
        #[ink(topic)]
        recipient: AccountId,
        amount: u128,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct PoolCapitalized {
        #[ink(topic)]
        pool_id: u64,
        #[ink(topic)]
        provider: AccountId,
        amount: u128,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct ReinsuranceActivated {
        #[ink(topic)]
        claim_id: u64,
        agreement_id: u64,
        recovery_amount: u128,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct PremiumCeded {
        #[ink(topic)]
        agreement_id: u64,
        #[ink(topic)]
        policy_id: u64,
        cession_id: u64,
        ceded_amount: u128,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct LossRecovered {
        #[ink(topic)]
        agreement_id: u64,
        #[ink(topic)]
        claim_id: u64,
        recovery_id: u64,
        recovered_amount: u128,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct ReinsuranceAgreementDeactivated {
        #[ink(topic)]
        agreement_id: u64,
        deactivated_by: AccountId,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct InsuranceTokenMinted {
        #[ink(topic)]
        token_id: u64,
        #[ink(topic)]
        policy_id: u64,
        #[ink(topic)]
        owner: AccountId,
        face_value: u128,
    }

    #[ink(event)]
    pub struct InsuranceTokenTransferred {
        #[ink(topic)]
        token_id: u64,
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        price: u128,
    }

    #[ink(event)]
    pub struct RiskAssessmentUpdated {
        #[ink(topic)]
        property_id: u64,
        overall_score: u32,
        risk_level: RiskLevel,
        timestamp: u64,
    }

    // ── Parametric insurance events (Issue #249) ──────────────────────────────

    #[ink(event)]
    pub struct ParametricPolicyCreated {
        #[ink(topic)]
        policy_id: u64,
        #[ink(topic)]
        policyholder: AccountId,
        #[ink(topic)]
        property_id: u64,
        metric: String,
        trigger_threshold: i128,
        coverage_amount: u128,
    }

    #[ink(event)]
    pub struct ParametricPolicyTriggered {
        #[ink(topic)]
        policy_id: u64,
        #[ink(topic)]
        policyholder: AccountId,
        oracle_value: i128,
        payout_amount: u128,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct OracleDataSubmitted {
        #[ink(topic)]
        data_id: u64,
        #[ink(topic)]
        property_id: u64,
        metric: String,
        value: i128,
        timestamp: u64,
    }

    // =========================================================================
    // RISK ASSESSMENT MODEL EVENTS (Task #254)
    // =========================================================================

    #[ink(event)]
    pub struct PropertyRiskModelCreated {
        #[ink(topic)]
        risk_id: u64,
        #[ink(topic)]
        property_id: u64,
        overall_risk_score: u32,
        final_risk_level: RiskLevel,
        premium_multiplier: u32,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct PropertyRiskModelUpdated {
        #[ink(topic)]
        risk_id: u64,
        #[ink(topic)]
        property_id: u64,
        new_risk_score: u32,
        new_risk_level: RiskLevel,
        timestamp: u64,
    }

    // =========================================================================
    // FRAUD DETECTION EVENTS (Task #258)
    // =========================================================================

    #[ink(event)]
    pub struct FraudRiskAssessmentCreated {
        #[ink(topic)]
        assessment_id: u64,
        #[ink(topic)]
        claim_id: u64,
        #[ink(topic)]
        policyholder: AccountId,
        fraud_score: u32,
        fraud_level: RiskLevel,
        requires_manual_review: bool,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct HighFraudRiskDetected {
        #[ink(topic)]
        claim_id: u64,
        #[ink(topic)]
        policyholder: AccountId,
        fraud_score: u32,
        indicator_count: u32,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct FraudPatternDetected {
        #[ink(topic)]
        claim_id: u64,
        indicator_type: String,
        risk_increase: u32,
        timestamp: u64,
    }

    // =========================================================================
    // IMPLEMENTATION
    // =========================================================================

    impl PropertyInsurance {
        #[ink(constructor)]
        pub fn new(admin: AccountId) -> Self {
            Self {
                admin,
                policies: Mapping::default(),
                policy_count: 0,
                policyholder_policies: Mapping::default(),
                property_policies: Mapping::default(),
                claims: Mapping::default(),
                claim_count: 0,
                policy_claims: Mapping::default(),
                pools: Mapping::default(),
                pool_count: 0,
                risk_assessments: Mapping::default(),
                reinsurance_agreements: Mapping::default(),
                reinsurance_count: 0,
                premium_cessions: Mapping::default(),
                cession_count: 0,
                loss_recoveries: Mapping::default(),
                loss_recovery_count: 0,
                agreement_cessions: Mapping::default(),
                agreement_recoveries: Mapping::default(),
                insurance_tokens: Mapping::default(),
                token_count: 0,
                token_listings: Vec::new(),
                actuarial_models: Mapping::default(),
                model_count: 0,
                underwriting_criteria: Mapping::default(),
                liquidity_providers: Mapping::default(),
                pool_providers: Mapping::default(),
                authorized_oracles: Mapping::default(),
                authorized_assessors: Mapping::default(),
                claim_cooldowns: Mapping::default(),
                claim_triggers: Mapping::default(),
                trigger_count: 0,
                policy_triggers: Mapping::default(),
                platform_fee_rate: 200,            // 2%
                claim_cooldown_period: 2_592_000,  // 30 days in seconds
                min_pool_capital: 100_000_000_000, // Minimum pool capital
                // Risk Assessment Model (Task #254)
                property_risk_models: Mapping::default(),
                risk_model_count: 0,
                // Fraud Detection System (Task #258)
                fraud_assessments: Mapping::default(),
                fraud_assessment_count: 0,
                fraud_patterns: Mapping::default(),
                fraud_pattern_count: 0,
                fraud_detection_stats: Some(FraudDetectionStats {
                    total_assessments: 0,
                    high_risk_claims: 0,
                    rejected_fraud_claims: 0,
                    patterns_detected: 0,
                    false_positive_count: 0,
                    average_fraud_score: 0,
                    last_update: 0,
                }),
                reentrancy_guard: ReentrancyGuard::new(),
                // Parametric insurance (Issue #249)
                parametric_policies: Mapping::default(),
                parametric_policy_count: 0,
                property_parametric_policies: Mapping::default(),
                holder_parametric_policies: Mapping::default(),
                oracle_data: Mapping::default(),
                oracle_data_count: 0,
            }
        }

        // =====================================================================
        // POOL MANAGEMENT
        // =====================================================================

        /// Create a new risk pool (admin only)
        #[ink(message)]
        pub fn create_risk_pool(
            &mut self,
            name: String,
            coverage_type: CoverageType,
            max_coverage_ratio: u32,
            reinsurance_threshold: u128,
        ) -> Result<u64, InsuranceError> {
            self.ensure_admin()?;

            let pool_id = self.pool_count + 1;
            self.pool_count = pool_id;

            let pool = RiskPool {
                pool_id,
                name,
                coverage_type,
                total_capital: 0,
                available_capital: 0,
                total_premiums_collected: 0,
                total_claims_paid: 0,
                active_policies: 0,
                max_coverage_ratio,
                reinsurance_threshold,
                created_at: self.env().block_timestamp(),
                is_active: true,
            };

            self.pools.insert(&pool_id, &pool);
            Ok(pool_id)
        }

        /// Provide liquidity to a pool
        #[ink(message, payable)]
        pub fn provide_pool_liquidity(&mut self, pool_id: u64) -> Result<(), InsuranceError> {
            let caller = self.env().caller();
            let amount = self.env().transferred_value();

            let mut pool = self
                .pools
                .get(&pool_id)
                .ok_or(InsuranceError::PoolNotFound)?;
            if !pool.is_active {
                return Err(InsuranceError::PoolNotFound);
            }

            pool.total_capital += amount;
            pool.available_capital += amount;
            self.pools.insert(&pool_id, &pool);

            // Update liquidity provider record
            let key = (pool_id, caller);
            let mut provider =
                self.liquidity_providers
                    .get(&key)
                    .unwrap_or(PoolLiquidityProvider {
                        provider: caller,
                        pool_id,
                        deposited_amount: 0,
                        share_percentage: 0,
                        deposited_at: self.env().block_timestamp(),
                        last_reward_claim: self.env().block_timestamp(),
                        accumulated_rewards: 0,
                    });
            provider.deposited_amount += amount;
            self.liquidity_providers.insert(&key, &provider);

            // Track providers per pool
            let mut providers = self.pool_providers.get(&pool_id).unwrap_or_default();
            if !providers.contains(&caller) {
                providers.push(caller);
                self.pool_providers.insert(&pool_id, &providers);
            }

            self.env().emit_event(PoolCapitalized {
                pool_id,
                provider: caller,
                amount,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        // =====================================================================
        // RISK ASSESSMENT
        // =====================================================================

        /// Submit or update risk assessment for a property (oracle/admin)
        #[ink(message)]
        pub fn update_risk_assessment(
            &mut self,
            property_id: u64,
            location_score: u32,
            construction_score: u32,
            age_score: u32,
            claims_history_score: u32,
            valid_for_seconds: u64,
        ) -> Result<(), InsuranceError> {
            let caller = self.env().caller();
            if caller != self.admin && !self.authorized_oracles.get(&caller).unwrap_or(false) {
                return Err(InsuranceError::Unauthorized);
            }

            let overall = (location_score
                .saturating_add(construction_score)
                .saturating_add(age_score)
                .saturating_add(claims_history_score))
                / 4;

            let risk_level = Self::score_to_risk_level(overall);

            let now = self.env().block_timestamp();
            let assessment = RiskAssessment {
                property_id,
                location_risk_score: location_score,
                construction_risk_score: construction_score,
                age_risk_score: age_score,
                claims_history_score,
                overall_risk_score: overall,
                risk_level: risk_level.clone(),
                assessed_at: now,
                valid_until: now.saturating_add(valid_for_seconds),
            };

            self.risk_assessments.insert(&property_id, &assessment);

            self.env().emit_event(RiskAssessmentUpdated {
                property_id,
                overall_score: overall,
                risk_level,
                timestamp: now,
            });

            Ok(())
        }

        /// Calculate premium for a policy (basic version)
        #[ink(message)]
        pub fn calculate_premium(
            &self,
            property_id: u64,
            coverage_amount: u128,
            coverage_type: CoverageType,
        ) -> Result<PremiumCalculation, InsuranceError> {
            // Default 1 year duration
            self.calculate_premium_with_modifiers(
                property_id,
                coverage_amount,
                coverage_type,
                31_536_000, // 1 year in seconds
                PremiumModifiers {
                    has_multiple_policies: false,
                    claim_free_years: 0,
                    has_safety_features: false,
                    loyalty_years: 0,
                },
            )
        }

        /// Calculate premium with dynamic modifiers and pool utilization
        #[ink(message)]
        pub fn calculate_premium_with_modifiers(
            &self,
            property_id: u64,
            coverage_amount: u128,
            coverage_type: CoverageType,
            duration_seconds: u64,
            modifiers: PremiumModifiers,
        ) -> Result<PremiumCalculation, InsuranceError> {
            let assessment = self
                .risk_assessments
                .get(&property_id)
                .ok_or(InsuranceError::PropertyNotInsurable)?;

            // Find a suitable pool for this coverage type
            let pool = self.find_pool_for_coverage(&coverage_type)?;

            // Try to get actuarial model for this coverage type
            let actuarial_model = self.get_actuarial_model_for_coverage(&coverage_type);

            // Use the premium engine for dynamic calculation
            let calculation = premium_engine::calculate_dynamic_premium(
                &assessment,
                coverage_amount,
                &coverage_type,
                &pool,
                actuarial_model.as_ref(),
                &modifiers,
                duration_seconds,
            );

            Ok(calculation)
        }

        // =====================================================================
        // POLICY MANAGEMENT
        // =====================================================================

        /// Create an insurance policy (policyholder pays premium)
        #[ink(message, payable)]
        pub fn create_policy(
            &mut self,
            property_id: u64,
            coverage_type: CoverageType,
            coverage_amount: u128,
            pool_id: u64,
            duration_seconds: u64,
            metadata_url: String,
        ) -> Result<u64, InsuranceError> {
            let caller = self.env().caller();
            let paid = self.env().transferred_value();
            let now = self.env().block_timestamp();

            // Validate pool
            let mut pool = self
                .pools
                .get(&pool_id)
                .ok_or(InsuranceError::PoolNotFound)?;
            if !pool.is_active {
                return Err(InsuranceError::PoolNotFound);
            }

            // Check pool has enough capital for coverage
            let max_exposure = pool
                .available_capital
                .saturating_mul(pool.max_coverage_ratio as u128)
                / 10_000;
            if coverage_amount > max_exposure {
                return Err(InsuranceError::InsufficientPoolFunds);
            }

            // Get risk assessment
            let assessment = self
                .risk_assessments
                .get(&property_id)
                .ok_or(InsuranceError::PropertyNotInsurable)?;

            // Check assessment is still valid
            if now > assessment.valid_until {
                return Err(InsuranceError::PropertyNotInsurable);
            }

            // Calculate required premium
            let calc =
                self.calculate_premium(property_id, coverage_amount, coverage_type.clone())?;
            if paid < calc.annual_premium {
                return Err(InsuranceError::InsufficientPremium);
            }

            // Platform fee
            let fee = paid.saturating_mul(self.platform_fee_rate as u128) / 10_000;
            let pool_share = paid.saturating_sub(fee);

            // Update pool
            pool.total_premiums_collected += pool_share;
            pool.available_capital += pool_share;
            pool.active_policies += 1;
            self.pools.insert(&pool_id, &pool);

            // Create policy
            let policy_id = self.policy_count + 1;
            self.policy_count = policy_id;

            let policy = InsurancePolicy {
                policy_id,
                property_id,
                policyholder: caller,
                coverage_type: coverage_type.clone(),
                coverage_amount,
                premium_amount: paid,
                deductible: calc.deductible,
                start_time: now,
                end_time: now.saturating_add(duration_seconds),
                status: PolicyStatus::Active,
                risk_level: assessment.risk_level,
                pool_id,
                claims_count: 0,
                total_claimed: 0,
                metadata_url,
            };

            self.policies.insert(&policy_id, &policy);

            let mut ph_policies = self.policyholder_policies.get(&caller).unwrap_or_default();
            ph_policies.push(policy_id);
            self.policyholder_policies.insert(&caller, &ph_policies);

            let mut prop_policies = self.property_policies.get(&property_id).unwrap_or_default();
            prop_policies.push(policy_id);
            self.property_policies.insert(&property_id, &prop_policies);

            // Mint insurance token
            self.internal_mint_token(policy_id, caller, coverage_amount)?;

            self.env().emit_event(PolicyCreated {
                policy_id,
                policyholder: caller,
                property_id,
                coverage_type,
                coverage_amount,
                premium_amount: paid,
                start_time: now,
                end_time: now.saturating_add(duration_seconds),
            });

            Ok(policy_id)
        }

        /// Cancel an active policy (policyholder or admin)
        #[ink(message)]
        pub fn cancel_policy(&mut self, policy_id: u64) -> Result<(), InsuranceError> {
            let caller = self.env().caller();
            let mut policy = self
                .policies
                .get(&policy_id)
                .ok_or(InsuranceError::PolicyNotFound)?;

            if caller != policy.policyholder && caller != self.admin {
                return Err(InsuranceError::Unauthorized);
            }

            if policy.status != PolicyStatus::Active {
                return Err(InsuranceError::PolicyInactive);
            }

            policy.status = PolicyStatus::Cancelled;
            self.policies.insert(&policy_id, &policy);

            // Reduce pool active count
            if let Some(mut pool) = self.pools.get(&policy.pool_id) {
                if pool.active_policies > 0 {
                    pool.active_policies -= 1;
                }
                self.pools.insert(&policy.pool_id, &pool);
            }

            self.env().emit_event(PolicyCancelled {
                policy_id,
                policyholder: policy.policyholder,
                cancelled_at: self.env().block_timestamp(),
            });

            Ok(())
        }

        // =====================================================================
        // CLAIMS PROCESSING
        // =====================================================================

        /// Submit an insurance claim
        #[ink(message)]
        pub fn submit_claim(
            &mut self,
            policy_id: u64,
            claim_amount: u128,
            description: String,
            evidence_url: String,
        ) -> Result<u64, InsuranceError> {
            let caller = self.env().caller();
            let now = self.env().block_timestamp();

            let mut policy = self
                .policies
                .get(&policy_id)
                .ok_or(InsuranceError::PolicyNotFound)?;

            if policy.policyholder != caller {
                return Err(InsuranceError::Unauthorized);
            }
            if policy.status != PolicyStatus::Active {
                return Err(InsuranceError::PolicyInactive);
            }
            if now > policy.end_time {
                return Err(InsuranceError::PolicyExpired);
            }

            // Check claim amount doesn't exceed remaining coverage
            let remaining = policy.coverage_amount.saturating_sub(policy.total_claimed);
            if claim_amount > remaining {
                return Err(InsuranceError::ClaimExceedsCoverage);
            }

            // Cooldown check
            let last_claim = self.claim_cooldowns.get(&policy.property_id).unwrap_or(0);
            if now.saturating_sub(last_claim) < self.claim_cooldown_period {
                return Err(InsuranceError::CooldownPeriodActive);
            }

            let claim_id = self.claim_count + 1;
            self.claim_count = claim_id;

            let claim = InsuranceClaim {
                claim_id,
                policy_id,
                claimant: caller,
                claim_amount,
                description,
                evidence_url,
                oracle_report_url: String::new(),
                status: ClaimStatus::Pending,
                submitted_at: now,
                processed_at: None,
                payout_amount: 0,
                assessor: None,
                rejection_reason: String::new(),
            };

            self.claims.insert(&claim_id, &claim);

            let mut policy_claims = self.policy_claims.get(&policy_id).unwrap_or_default();
            policy_claims.push(claim_id);
            self.policy_claims.insert(&policy_id, &policy_claims);

            policy.claims_count += 1;
            self.policies.insert(&policy_id, &policy);

            self.env().emit_event(ClaimSubmitted {
                claim_id,
                policy_id,
                claimant: caller,
                claim_amount,
                submitted_at: now,
            });

            Ok(claim_id)
        }

        /// Assessor reviews a claim and either approves or rejects it
        #[ink(message)]
        pub fn process_claim(
            &mut self,
            claim_id: u64,
            approved: bool,
            oracle_report_url: String,
            rejection_reason: String,
        ) -> Result<(), InsuranceError> {
            non_reentrant!(self, {
                let caller = self.env().caller();

                if caller != self.admin && !self.authorized_assessors.get(&caller).unwrap_or(false)
                {
                    return Err(InsuranceError::Unauthorized);
                }

                let mut claim = self
                    .claims
                    .get(&claim_id)
                    .ok_or(InsuranceError::ClaimNotFound)?;
                if claim.status != ClaimStatus::Pending && claim.status != ClaimStatus::UnderReview
                {
                    return Err(InsuranceError::ClaimAlreadyProcessed);
                }

                let now = self.env().block_timestamp();
                claim.assessor = Some(caller);
                claim.oracle_report_url = oracle_report_url;
                claim.processed_at = Some(now);

                if approved {
                    let policy = self
                        .policies
                        .get(&claim.policy_id)
                        .ok_or(InsuranceError::PolicyNotFound)?;

                    // Apply deductible
                    let payout = if claim.claim_amount > policy.deductible {
                        claim.claim_amount.saturating_sub(policy.deductible)
                    } else {
                        0
                    };

                    claim.payout_amount = payout;
                    claim.status = ClaimStatus::Approved;
                    self.claims.insert(&claim_id, &claim);

                    // Execute payout
                    self.execute_payout(claim_id, claim.policy_id, claim.claimant, payout)?;

                    self.env().emit_event(ClaimApproved {
                        claim_id,
                        policy_id: claim.policy_id,
                        payout_amount: payout,
                        approved_by: caller,
                        timestamp: now,
                    });
                } else {
                    claim.status = ClaimStatus::Rejected;
                    claim.rejection_reason = rejection_reason.clone();
                    self.claims.insert(&claim_id, &claim);

                    self.env().emit_event(ClaimRejected {
                        claim_id,
                        policy_id: claim.policy_id,
                        reason: rejection_reason,
                        rejected_by: caller,
                        timestamp: now,
                    });
                }

                Ok(())
            })
        }

        // =====================================================================
        // CLAIM AUTOMATION (oracle-triggered parametric claims)
        // =====================================================================

        /// Register an oracle-driven claim trigger against a policy. The
        /// policyholder or admin may register; once an authorized oracle reports
        /// a value satisfying the comparator/threshold via `report_oracle_event`,
        /// the contract creates, approves, and pays a claim automatically.
        #[ink(message)]
        pub fn register_claim_trigger(
            &mut self,
            policy_id: u64,
            metric: TriggerMetric,
            comparator: TriggerComparator,
            threshold: u128,
            payout_mode: PayoutMode,
        ) -> Result<u64, InsuranceError> {
            let caller = self.env().caller();

            let policy = self
                .policies
                .get(&policy_id)
                .ok_or(InsuranceError::PolicyNotFound)?;
            if caller != policy.policyholder && caller != self.admin {
                return Err(InsuranceError::Unauthorized);
            }
            if policy.status != PolicyStatus::Active {
                return Err(InsuranceError::PolicyInactive);
            }
            Self::ensure_payout_mode_valid(&payout_mode)?;

            let trigger_id = self.trigger_count + 1;
            self.trigger_count = trigger_id;
            let now = self.env().block_timestamp();

            let trigger = ClaimTrigger {
                trigger_id,
                policy_id,
                metric,
                comparator,
                threshold,
                payout_mode,
                is_active: true,
                triggered: false,
                last_observed_value: None,
                last_report_url: String::new(),
                created_at: now,
                triggered_at: None,
                triggering_claim_id: None,
            };
            self.claim_triggers.insert(&trigger_id, &trigger);

            let mut list = self.policy_triggers.get(&policy_id).unwrap_or_default();
            list.push(trigger_id);
            self.policy_triggers.insert(&policy_id, &list);

            self.env().emit_event(ClaimTriggerRegistered {
                trigger_id,
                policy_id,
                metric,
                threshold,
            });

            Ok(trigger_id)
        }

        /// Deactivate an active trigger. Only the policyholder or admin may
        /// deactivate. Already-fired triggers cannot be re-deactivated.
        #[ink(message)]
        pub fn deactivate_claim_trigger(
            &mut self,
            trigger_id: u64,
        ) -> Result<(), InsuranceError> {
            let caller = self.env().caller();
            let mut trigger = self
                .claim_triggers
                .get(&trigger_id)
                .ok_or(InsuranceError::TriggerNotFound)?;
            if !trigger.is_active {
                return Err(InsuranceError::TriggerInactive);
            }

            let policy = self
                .policies
                .get(&trigger.policy_id)
                .ok_or(InsuranceError::PolicyNotFound)?;
            if caller != policy.policyholder && caller != self.admin {
                return Err(InsuranceError::Unauthorized);
            }

            trigger.is_active = false;
            self.claim_triggers.insert(&trigger_id, &trigger);

            self.env().emit_event(ClaimTriggerDeactivated {
                trigger_id,
                policy_id: trigger.policy_id,
            });
            Ok(())
        }

        /// Oracle entry point: report an observed value for a trigger. If the
        /// value meets the trigger condition and the underlying policy is
        /// still payable, this auto-creates an approved claim and runs the
        /// payout in one transaction.
        ///
        /// Callers must be admin or an authorized oracle. The trigger fires
        /// at most once. If the condition is not met, the report is recorded
        /// but no claim is created.
        #[ink(message)]
        pub fn report_oracle_event(
            &mut self,
            trigger_id: u64,
            observed_value: u128,
            oracle_report_url: String,
        ) -> Result<Option<u64>, InsuranceError> {
            non_reentrant!(self, {
                let caller = self.env().caller();
                if caller != self.admin && !self.authorized_oracles.get(&caller).unwrap_or(false) {
                    return Err(InsuranceError::Unauthorized);
                }

                let mut trigger = self
                    .claim_triggers
                    .get(&trigger_id)
                    .ok_or(InsuranceError::TriggerNotFound)?;
                if !trigger.is_active {
                    return Err(InsuranceError::TriggerInactive);
                }
                if trigger.triggered {
                    return Err(InsuranceError::TriggerAlreadyFired);
                }

                let now = self.env().block_timestamp();
                let condition_met = Self::evaluate_condition(
                    &trigger.comparator,
                    observed_value,
                    trigger.threshold,
                );

                trigger.last_observed_value = Some(observed_value);
                trigger.last_report_url = oracle_report_url.clone();

                self.env().emit_event(OracleEventReceived {
                    trigger_id,
                    oracle: caller,
                    observed_value,
                    threshold_met: condition_met,
                    timestamp: now,
                });

                if !condition_met {
                    self.claim_triggers.insert(&trigger_id, &trigger);
                    return Ok(None);
                }

                let mut policy = self
                    .policies
                    .get(&trigger.policy_id)
                    .ok_or(InsuranceError::PolicyNotFound)?;
                if policy.status != PolicyStatus::Active {
                    return Err(InsuranceError::PolicyInactive);
                }
                if now > policy.end_time {
                    return Err(InsuranceError::PolicyExpired);
                }

                let last_claim = self.claim_cooldowns.get(&policy.property_id).unwrap_or(0);
                if now.saturating_sub(last_claim) < self.claim_cooldown_period {
                    return Err(InsuranceError::CooldownPeriodActive);
                }

                let remaining = policy.coverage_amount.saturating_sub(policy.total_claimed);
                if remaining == 0 {
                    return Err(InsuranceError::ClaimExceedsCoverage);
                }
                let claim_amount =
                    Self::compute_claim_amount(&trigger.payout_mode, remaining)?;
                if claim_amount == 0 {
                    return Err(InsuranceError::TriggerConditionNotMet);
                }
                let payout = claim_amount.saturating_sub(policy.deductible);

                // Create the auto-claim record.
                let claim_id = self.claim_count + 1;
                self.claim_count = claim_id;

                let mut claim = InsuranceClaim {
                    claim_id,
                    policy_id: trigger.policy_id,
                    claimant: policy.policyholder,
                    claim_amount,
                    description: String::from("Oracle-triggered parametric claim"),
                    evidence_url: String::new(),
                    oracle_report_url: oracle_report_url.clone(),
                    status: ClaimStatus::OracleVerifying,
                    submitted_at: now,
                    processed_at: Some(now),
                    payout_amount: payout,
                    assessor: Some(caller),
                    rejection_reason: String::new(),
                };

                policy.claims_count += 1;
                self.policies.insert(&trigger.policy_id, &policy);

                let mut policy_claims =
                    self.policy_claims.get(&trigger.policy_id).unwrap_or_default();
                policy_claims.push(claim_id);
                self.policy_claims
                    .insert(&trigger.policy_id, &policy_claims);

                claim.status = ClaimStatus::Approved;
                self.claims.insert(&claim_id, &claim);

                self.env().emit_event(ClaimApproved {
                    claim_id,
                    policy_id: trigger.policy_id,
                    payout_amount: payout,
                    approved_by: caller,
                    timestamp: now,
                });

                // Run payout (debits pool, marks claim Paid, updates cooldown).
                self.execute_payout(claim_id, trigger.policy_id, policy.policyholder, payout)?;

                trigger.triggered = true;
                trigger.triggered_at = Some(now);
                trigger.triggering_claim_id = Some(claim_id);
                self.claim_triggers.insert(&trigger_id, &trigger);

                self.env().emit_event(ClaimAutoPaid {
                    trigger_id,
                    claim_id,
                    policy_id: trigger.policy_id,
                    payout_amount: payout,
                    timestamp: now,
                });

                Ok(Some(claim_id))
            })
        }

        /// Get claim trigger by id.
        #[ink(message)]
        pub fn get_claim_trigger(&self, trigger_id: u64) -> Option<ClaimTrigger> {
            self.claim_triggers.get(&trigger_id)
        }

        /// Get all trigger ids registered against a policy.
        #[ink(message)]
        pub fn get_policy_triggers(&self, policy_id: u64) -> Vec<u64> {
            self.policy_triggers.get(&policy_id).unwrap_or_default()
        }

        /// Total number of triggers ever registered.
        #[ink(message)]
        pub fn get_trigger_count(&self) -> u64 {
            self.trigger_count
        }

        // =====================================================================
        // REINSURANCE
        // =====================================================================

        /// Register a reinsurance agreement (admin only)
        #[ink(message)]
        pub fn register_reinsurance(
            &mut self,
            reinsurer: AccountId,
            coverage_limit: u128,
            retention_limit: u128,
            premium_ceded_rate: u32,
            coverage_types: Vec<CoverageType>,
            duration_seconds: u64,
        ) -> Result<u64, InsuranceError> {
            self.ensure_admin()?;

            let now = self.env().block_timestamp();
            let agreement_id = self.reinsurance_count + 1;
            self.reinsurance_count = agreement_id;

            let agreement = ReinsuranceAgreement {
                agreement_id,
                reinsurer,
                coverage_limit,
                retention_limit,
                premium_ceded_rate,
                coverage_types,
                start_time: now,
                end_time: now.saturating_add(duration_seconds),
                is_active: true,
                total_ceded_premiums: 0,
                total_recoveries: 0,
                treaty_type: ReinsuranceTreatyType::ExcessOfLoss,
                cession_count: 0,
                recovery_count: 0,
            };

            self.reinsurance_agreements
                .insert(&agreement_id, &agreement);
            Ok(agreement_id)
        }

        /// Register a reinsurance agreement with an explicit treaty type (admin only)
        #[ink(message)]
        pub fn register_reinsurance_with_type(
            &mut self,
            reinsurer: AccountId,
            treaty_type: ReinsuranceTreatyType,
            coverage_limit: u128,
            retention_limit: u128,
            premium_ceded_rate: u32,
            coverage_types: Vec<CoverageType>,
            duration_seconds: u64,
        ) -> Result<u64, InsuranceError> {
            self.ensure_admin()?;

            let now = self.env().block_timestamp();
            let agreement_id = self.reinsurance_count + 1;
            self.reinsurance_count = agreement_id;

            let agreement = ReinsuranceAgreement {
                agreement_id,
                reinsurer,
                coverage_limit,
                retention_limit,
                premium_ceded_rate,
                coverage_types,
                start_time: now,
                end_time: now.saturating_add(duration_seconds),
                is_active: true,
                total_ceded_premiums: 0,
                total_recoveries: 0,
                treaty_type,
                cession_count: 0,
                recovery_count: 0,
            };

            self.reinsurance_agreements
                .insert(&agreement_id, &agreement);
            Ok(agreement_id)
        }

        /// Deactivate a reinsurance agreement (admin only)
        #[ink(message)]
        pub fn deactivate_reinsurance(&mut self, agreement_id: u64) -> Result<(), InsuranceError> {
            self.ensure_admin()?;
            let mut agreement = self
                .reinsurance_agreements
                .get(&agreement_id)
                .ok_or(InsuranceError::ReinsuranceAgreementNotFound)?;

            agreement.is_active = false;
            self.reinsurance_agreements
                .insert(&agreement_id, &agreement);

            self.env().emit_event(ReinsuranceAgreementDeactivated {
                agreement_id,
                deactivated_by: self.env().caller(),
                timestamp: self.env().block_timestamp(),
            });
            Ok(())
        }

        /// Manually cede a premium amount to a specific reinsurance agreement (admin only).
        /// Useful for proportional (quota share / surplus) treaties where cession is
        /// tracked separately from automatic excess-of-loss recovery.
        #[ink(message)]
        pub fn cede_premium(
            &mut self,
            agreement_id: u64,
            policy_id: u64,
            gross_premium: u128,
        ) -> Result<u64, InsuranceError> {
            self.ensure_admin()?;

            let mut agreement = self
                .reinsurance_agreements
                .get(&agreement_id)
                .ok_or(InsuranceError::ReinsuranceAgreementNotFound)?;

            if !agreement.is_active {
                return Err(InsuranceError::ReinsuranceAgreementInactive);
            }
            let now = self.env().block_timestamp();
            if now > agreement.end_time {
                return Err(InsuranceError::ReinsuranceAgreementExpired);
            }

            // Ceded amount depends on treaty type
            let ceded_amount = match agreement.treaty_type {
                ReinsuranceTreatyType::QuotaShare => {
                    // Fixed % of every premium
                    gross_premium.saturating_mul(agreement.premium_ceded_rate as u128) / 10_000
                }
                ReinsuranceTreatyType::Surplus => {
                    // Cede the portion above the retention limit
                    if gross_premium > agreement.retention_limit {
                        gross_premium
                            .saturating_sub(agreement.retention_limit)
                            .min(agreement.coverage_limit)
                    } else {
                        0
                    }
                }
                ReinsuranceTreatyType::ExcessOfLoss => {
                    // Premium cession for XL is typically a flat rate
                    gross_premium.saturating_mul(agreement.premium_ceded_rate as u128) / 10_000
                }
            };

            if ceded_amount == 0 {
                return Err(InsuranceError::InvalidParameters);
            }

            let cession_id = self.cession_count + 1;
            self.cession_count = cession_id;

            let cession = PremiumCession {
                cession_id,
                agreement_id,
                policy_id,
                gross_premium,
                ceded_premium: ceded_amount,
                ceded_at: now,
            };
            self.premium_cessions.insert(&cession_id, &cession);

            // Update agreement totals
            agreement.total_ceded_premiums =
                agreement.total_ceded_premiums.saturating_add(ceded_amount);
            agreement.cession_count += 1;
            self.reinsurance_agreements
                .insert(&agreement_id, &agreement);

            // Index cession under agreement
            let mut cessions = self
                .agreement_cessions
                .get(&agreement_id)
                .unwrap_or_default();
            cessions.push(cession_id);
            self.agreement_cessions.insert(&agreement_id, &cessions);

            self.env().emit_event(PremiumCeded {
                agreement_id,
                policy_id,
                cession_id,
                ceded_amount,
                timestamp: now,
            });

            Ok(cession_id)
        }

        /// Record a loss recovery from a reinsurer for an approved claim (admin only).
        /// Call this after the reinsurer has settled their share of a large claim.
        #[ink(message)]
        pub fn record_loss_recovery(
            &mut self,
            agreement_id: u64,
            claim_id: u64,
            gross_loss: u128,
        ) -> Result<u64, InsuranceError> {
            self.ensure_admin()?;

            let mut agreement = self
                .reinsurance_agreements
                .get(&agreement_id)
                .ok_or(InsuranceError::ReinsuranceAgreementNotFound)?;

            if !agreement.is_active {
                return Err(InsuranceError::ReinsuranceAgreementInactive);
            }

            // Verify the claim exists
            self.claims
                .get(&claim_id)
                .ok_or(InsuranceError::ClaimNotFound)?;

            let now = self.env().block_timestamp();

            let recovered_amount = match agreement.treaty_type {
                ReinsuranceTreatyType::ExcessOfLoss => {
                    // Recover the portion above retention, capped at coverage limit
                    if gross_loss > agreement.retention_limit {
                        gross_loss
                            .saturating_sub(agreement.retention_limit)
                            .min(agreement.coverage_limit)
                    } else {
                        0
                    }
                }
                ReinsuranceTreatyType::QuotaShare => {
                    // Reinsurer pays their quota share of the loss
                    gross_loss.saturating_mul(agreement.premium_ceded_rate as u128) / 10_000
                }
                ReinsuranceTreatyType::Surplus => {
                    // Recover the surplus portion
                    if gross_loss > agreement.retention_limit {
                        gross_loss
                            .saturating_sub(agreement.retention_limit)
                            .min(agreement.coverage_limit)
                    } else {
                        0
                    }
                }
            };

            if recovered_amount == 0 {
                return Err(InsuranceError::InvalidParameters);
            }

            let recovery_id = self.loss_recovery_count + 1;
            self.loss_recovery_count = recovery_id;

            let recovery = LossRecovery {
                recovery_id,
                agreement_id,
                claim_id,
                gross_loss,
                recovered_amount,
                recovered_at: now,
            };
            self.loss_recoveries.insert(&recovery_id, &recovery);

            // Update agreement totals
            agreement.total_recoveries =
                agreement.total_recoveries.saturating_add(recovered_amount);
            agreement.recovery_count += 1;
            self.reinsurance_agreements
                .insert(&agreement_id, &agreement);

            // Index recovery under agreement
            let mut recoveries = self
                .agreement_recoveries
                .get(&agreement_id)
                .unwrap_or_default();
            recoveries.push(recovery_id);
            self.agreement_recoveries.insert(&agreement_id, &recoveries);

            self.env().emit_event(LossRecovered {
                agreement_id,
                claim_id,
                recovery_id,
                recovered_amount,
                timestamp: now,
            });

            Ok(recovery_id)
        }

        // =====================================================================
        // INSURANCE TOKENIZATION & SECONDARY MARKET
        // =====================================================================

        /// List an insurance token for sale on the secondary market
        #[ink(message)]
        pub fn list_token_for_sale(
            &mut self,
            token_id: u64,
            price: u128,
        ) -> Result<(), InsuranceError> {
            let caller = self.env().caller();
            let mut token = self
                .insurance_tokens
                .get(&token_id)
                .ok_or(InsuranceError::TokenNotFound)?;

            if token.owner != caller {
                return Err(InsuranceError::Unauthorized);
            }
            if !token.is_tradeable {
                return Err(InsuranceError::InvalidParameters);
            }

            token.listed_price = Some(price);
            self.insurance_tokens.insert(&token_id, &token);

            if !self.token_listings.contains(&token_id) {
                self.token_listings.push(token_id);
            }

            Ok(())
        }

        /// Purchase an insurance token from the secondary market
        #[ink(message, payable)]
        pub fn purchase_token(&mut self, token_id: u64) -> Result<(), InsuranceError> {
            let caller = self.env().caller();
            let paid = self.env().transferred_value();

            let mut token = self
                .insurance_tokens
                .get(&token_id)
                .ok_or(InsuranceError::TokenNotFound)?;
            let price = token
                .listed_price
                .ok_or(InsuranceError::InvalidParameters)?;

            if paid < price {
                return Err(InsuranceError::InsufficientPremium);
            }

            let seller = token.owner;
            let old_owner = seller;

            // Transfer the policy to the buyer
            let policy = self
                .policies
                .get(&token.policy_id)
                .ok_or(InsuranceError::PolicyNotFound)?;
            if policy.status != PolicyStatus::Active {
                return Err(InsuranceError::PolicyInactive);
            }

            // Update policy policyholder
            let mut updated_policy = policy;
            updated_policy.policyholder = caller;
            self.policies.insert(&token.policy_id, &updated_policy);

            // Update ownership tracking
            let mut seller_policies = self.policyholder_policies.get(&seller).unwrap_or_default();
            seller_policies.retain(|&p| p != token.policy_id);
            self.policyholder_policies.insert(&seller, &seller_policies);

            let mut buyer_policies = self.policyholder_policies.get(&caller).unwrap_or_default();
            buyer_policies.push(token.policy_id);
            self.policyholder_policies.insert(&caller, &buyer_policies);

            // Update token
            token.owner = caller;
            token.listed_price = None;
            self.insurance_tokens.insert(&token_id, &token);

            // Remove from listings
            self.token_listings.retain(|&t| t != token_id);

            self.env().emit_event(InsuranceTokenTransferred {
                token_id,
                from: old_owner,
                to: caller,
                price: paid,
            });

            Ok(())
        }

        // =====================================================================
        // ACTUARIAL MODELING
        // =====================================================================

        /// Update actuarial model (admin/authorized oracle)
        #[ink(message)]
        pub fn update_actuarial_model(
            &mut self,
            coverage_type: CoverageType,
            loss_frequency: u32,
            average_loss_severity: u128,
            expected_loss_ratio: u32,
            confidence_level: u32,
            data_points: u32,
        ) -> Result<u64, InsuranceError> {
            let caller = self.env().caller();
            if caller != self.admin && !self.authorized_oracles.get(&caller).unwrap_or(false) {
                return Err(InsuranceError::Unauthorized);
            }

            let model_id = self.model_count + 1;
            self.model_count = model_id;

            let model = ActuarialModel {
                model_id,
                coverage_type,
                loss_frequency,
                average_loss_severity,
                expected_loss_ratio,
                confidence_level,
                last_updated: self.env().block_timestamp(),
                data_points,
            };

            self.actuarial_models.insert(&model_id, &model);
            Ok(model_id)
        }

        // =====================================================================
        // UNDERWRITING
        // =====================================================================

        /// Set underwriting criteria for a pool (admin only)
        #[ink(message)]
        pub fn set_underwriting_criteria(
            &mut self,
            pool_id: u64,
            max_property_age_years: u32,
            min_property_value: u128,
            max_property_value: u128,
            required_safety_features: bool,
            max_previous_claims: u32,
            min_risk_score: u32,
        ) -> Result<(), InsuranceError> {
            self.ensure_admin()?;
            self.pools
                .get(&pool_id)
                .ok_or(InsuranceError::PoolNotFound)?;

            let criteria = UnderwritingCriteria {
                max_property_age_years,
                min_property_value,
                max_property_value,
                excluded_locations: Vec::new(),
                required_safety_features,
                max_previous_claims,
                min_risk_score,
            };

            self.underwriting_criteria.insert(&pool_id, &criteria);
            Ok(())
        }

        // =====================================================================
        // PARAMETRIC INSURANCE (Issue #249)
        // =====================================================================

        /// Create a parametric insurance policy.
        ///
        /// The caller pays the premium upfront. When an authorized oracle later
        /// submits a data point for `property_id` / `metric` that satisfies the
        /// trigger condition, the full `coverage_amount` is paid out automatically
        /// from the backing risk pool — no manual claims assessment required.
        #[ink(message, payable)]
        pub fn create_parametric_policy(
            &mut self,
            property_id: u64,
            metric: String,
            trigger_threshold: i128,
            comparison: TriggerComparison,
            coverage_amount: u128,
            pool_id: u64,
            duration_seconds: u64,
        ) -> Result<u64, InsuranceError> {
            let caller = self.env().caller();
            let paid = self.env().transferred_value();
            let now = self.env().block_timestamp();

            if paid == 0 {
                return Err(InsuranceError::InsufficientPremium);
            }

            // Validate pool has enough capital
            let mut pool = self
                .pools
                .get(&pool_id)
                .ok_or(InsuranceError::PoolNotFound)?;
            if !pool.is_active {
                return Err(InsuranceError::PoolNotFound);
            }
            let max_exposure = pool
                .available_capital
                .saturating_mul(pool.max_coverage_ratio as u128)
                / 10_000;
            if coverage_amount > max_exposure {
                return Err(InsuranceError::InsufficientPoolFunds);
            }

            // Credit premium to pool
            let fee = paid.saturating_mul(self.platform_fee_rate as u128) / 10_000;
            let pool_share = paid.saturating_sub(fee);
            pool.total_premiums_collected += pool_share;
            pool.available_capital += pool_share;
            pool.active_policies += 1;
            self.pools.insert(&pool_id, &pool);

            let policy_id = self.parametric_policy_count + 1;
            self.parametric_policy_count = policy_id;

            let policy = ParametricPolicy {
                policy_id,
                property_id,
                policyholder: caller,
                metric: metric.clone(),
                trigger_threshold,
                comparison,
                coverage_amount,
                premium_amount: paid,
                pool_id,
                start_time: now,
                end_time: now.saturating_add(duration_seconds),
                status: ParametricPolicyStatus::Active,
            };

            self.parametric_policies.insert(&policy_id, &policy);

            let mut prop_list = self
                .property_parametric_policies
                .get(&property_id)
                .unwrap_or_default();
            prop_list.push(policy_id);
            self.property_parametric_policies
                .insert(&property_id, &prop_list);

            let mut holder_list = self
                .holder_parametric_policies
                .get(&caller)
                .unwrap_or_default();
            holder_list.push(policy_id);
            self.holder_parametric_policies.insert(&caller, &holder_list);

            self.env().emit_event(ParametricPolicyCreated {
                policy_id,
                policyholder: caller,
                property_id,
                metric,
                trigger_threshold,
                coverage_amount,
            });

            Ok(policy_id)
        }

        /// Submit an oracle data point for a property metric.
        ///
        /// Only authorized oracles (or the admin) may call this. After recording
        /// the data point, all active parametric policies for the property whose
        /// trigger condition is satisfied are paid out automatically.
        #[ink(message)]
        pub fn submit_oracle_data(
            &mut self,
            property_id: u64,
            metric: String,
            value: i128,
        ) -> Result<u64, InsuranceError> {
            let caller = self.env().caller();
            if caller != self.admin && !self.authorized_oracles.get(&caller).unwrap_or(false) {
                return Err(InsuranceError::Unauthorized);
            }

            let now = self.env().block_timestamp();
            let data_id = self.oracle_data_count + 1;
            self.oracle_data_count = data_id;

            let data_point = OracleDataPoint {
                data_id,
                property_id,
                metric: metric.clone(),
                value,
                submitted_by: caller,
                submitted_at: now,
            };
            self.oracle_data.insert(&data_id, &data_point);

            self.env().emit_event(OracleDataSubmitted {
                data_id,
                property_id,
                metric: metric.clone(),
                value,
                timestamp: now,
            });

            // Evaluate all active parametric policies for this property + metric
            let policy_ids = self
                .property_parametric_policies
                .get(&property_id)
                .unwrap_or_default();

            for pid in policy_ids {
                if let Some(policy) = self.parametric_policies.get(&pid) {
                    if policy.status != ParametricPolicyStatus::Active {
                        continue;
                    }
                    if now > policy.end_time {
                        continue;
                    }
                    if policy.metric != metric {
                        continue;
                    }
                    let triggered = match policy.comparison {
                        TriggerComparison::GreaterThanOrEqual => value >= policy.trigger_threshold,
                        TriggerComparison::LessThanOrEqual => value <= policy.trigger_threshold,
                    };
                    if triggered {
                        // Ignore individual payout errors so other policies still process
                        let _ = self.execute_parametric_payout(pid, value, now);
                    }
                }
            }

            Ok(data_id)
        }

        /// Cancel an active parametric policy (policyholder or admin).
        #[ink(message)]
        pub fn cancel_parametric_policy(&mut self, policy_id: u64) -> Result<(), InsuranceError> {
            let caller = self.env().caller();
            let mut policy = self
                .parametric_policies
                .get(&policy_id)
                .ok_or(InsuranceError::ParametricPolicyNotFound)?;

            if caller != policy.policyholder && caller != self.admin {
                return Err(InsuranceError::Unauthorized);
            }
            if policy.status != ParametricPolicyStatus::Active {
                return Err(InsuranceError::ParametricPolicyInactive);
            }

            policy.status = ParametricPolicyStatus::Cancelled;
            self.parametric_policies.insert(&policy_id, &policy);

            if let Some(mut pool) = self.pools.get(&policy.pool_id) {
                if pool.active_policies > 0 {
                    pool.active_policies -= 1;
                }
                self.pools.insert(&policy.pool_id, &pool);
            }

            Ok(())
        }

        // ── Parametric queries ────────────────────────────────────────────────

        /// Get a parametric policy by ID.
        #[ink(message)]
        pub fn get_parametric_policy(&self, policy_id: u64) -> Option<ParametricPolicy> {
            self.parametric_policies.get(&policy_id)
        }

        /// Get all parametric policy IDs for a property.
        #[ink(message)]
        pub fn get_property_parametric_policies(&self, property_id: u64) -> Vec<u64> {
            self.property_parametric_policies
                .get(&property_id)
                .unwrap_or_default()
        }

        /// Get all parametric policy IDs for a policyholder.
        #[ink(message)]
        pub fn get_holder_parametric_policies(&self, holder: AccountId) -> Vec<u64> {
            self.holder_parametric_policies
                .get(&holder)
                .unwrap_or_default()
        }

        /// Get an oracle data point by ID.
        #[ink(message)]
        pub fn get_oracle_data(&self, data_id: u64) -> Option<OracleDataPoint> {
            self.oracle_data.get(&data_id)
        }

        /// Get total parametric policy count.
        #[ink(message)]
        pub fn get_parametric_policy_count(&self) -> u64 {
            self.parametric_policy_count
        }

        // ── Internal parametric helper ────────────────────────────────────────

        fn execute_parametric_payout(
            &mut self,
            policy_id: u64,
            oracle_value: i128,
            now: u64,
        ) -> Result<(), InsuranceError> {
            let mut policy = self
                .parametric_policies
                .get(&policy_id)
                .ok_or(InsuranceError::ParametricPolicyNotFound)?;

            if policy.status != ParametricPolicyStatus::Active {
                return Err(InsuranceError::ParametricPolicyAlreadyTriggered);
            }

            let mut pool = self
                .pools
                .get(&policy.pool_id)
                .ok_or(InsuranceError::PoolNotFound)?;

            if pool.available_capital < policy.coverage_amount {
                return Err(InsuranceError::InsufficientPoolFunds);
            }

            pool.available_capital = pool
                .available_capital
                .saturating_sub(policy.coverage_amount);
            pool.total_claims_paid += policy.coverage_amount;
            if pool.active_policies > 0 {
                pool.active_policies -= 1;
            }
            self.pools.insert(&policy.pool_id, &pool);

            policy.status = ParametricPolicyStatus::Triggered;
            self.parametric_policies.insert(&policy_id, &policy);

            self.env().emit_event(ParametricPolicyTriggered {
                policy_id,
                policyholder: policy.policyholder,
                oracle_value,
                payout_amount: policy.coverage_amount,
                timestamp: now,
            });

            Ok(())
        }

        // =====================================================================
        // ADMIN / AUTHORITY MANAGEMENT
        // =====================================================================

        /// Authorize an oracle address
        #[ink(message)]
        pub fn authorize_oracle(&mut self, oracle: AccountId) -> Result<(), InsuranceError> {
            self.ensure_admin()?;
            self.authorized_oracles.insert(&oracle, &true);
            Ok(())
        }

        /// Authorize a claims assessor
        #[ink(message)]
        pub fn authorize_assessor(&mut self, assessor: AccountId) -> Result<(), InsuranceError> {
            self.ensure_admin()?;
            self.authorized_assessors.insert(&assessor, &true);
            Ok(())
        }

        /// Update platform fee rate (admin only)
        #[ink(message)]
        pub fn set_platform_fee_rate(&mut self, rate: u32) -> Result<(), InsuranceError> {
            self.ensure_admin()?;
            if rate > 1000 {
                return Err(InsuranceError::InvalidParameters); // Max 10%
            }
            self.platform_fee_rate = rate;
            Ok(())
        }

        /// Update claim cooldown period (admin only)
        #[ink(message)]
        pub fn set_claim_cooldown(&mut self, period_seconds: u64) -> Result<(), InsuranceError> {
            self.ensure_admin()?;
            self.claim_cooldown_period = period_seconds;
            Ok(())
        }

        // =====================================================================
        // QUERIES
        // =====================================================================

        /// Get policy details
        #[ink(message)]
        pub fn get_policy(&self, policy_id: u64) -> Option<InsurancePolicy> {
            self.policies.get(&policy_id)
        }

        /// Get claim details
        #[ink(message)]
        pub fn get_claim(&self, claim_id: u64) -> Option<InsuranceClaim> {
            self.claims.get(&claim_id)
        }

        /// Get pool details
        #[ink(message)]
        pub fn get_pool(&self, pool_id: u64) -> Option<RiskPool> {
            self.pools.get(&pool_id)
        }

        /// Get risk assessment for a property
        #[ink(message)]
        pub fn get_risk_assessment(&self, property_id: u64) -> Option<RiskAssessment> {
            self.risk_assessments.get(&property_id)
        }

        /// Get all policies for a policyholder
        #[ink(message)]
        pub fn get_policyholder_policies(&self, holder: AccountId) -> Vec<u64> {
            self.policyholder_policies.get(&holder).unwrap_or_default()
        }

        /// Get all policy IDs for a property
        #[ink(message)]
        pub fn get_property_policies(&self, property_id: u64) -> Vec<u64> {
            self.property_policies.get(&property_id).unwrap_or_default()
        }

        /// Get all claims for a policy
        #[ink(message)]
        pub fn get_policy_claims(&self, policy_id: u64) -> Vec<u64> {
            self.policy_claims.get(&policy_id).unwrap_or_default()
        }

        /// Get insurance token details
        #[ink(message)]
        pub fn get_token(&self, token_id: u64) -> Option<InsuranceToken> {
            self.insurance_tokens.get(&token_id)
        }

        /// Get all token listings on the secondary market
        #[ink(message)]
        pub fn get_token_listings(&self) -> Vec<u64> {
            self.token_listings.clone()
        }

        /// Get actuarial model
        #[ink(message)]
        pub fn get_actuarial_model(&self, model_id: u64) -> Option<ActuarialModel> {
            self.actuarial_models.get(&model_id)
        }

        /// Get reinsurance agreement
        #[ink(message)]
        pub fn get_reinsurance_agreement(&self, agreement_id: u64) -> Option<ReinsuranceAgreement> {
            self.reinsurance_agreements.get(&agreement_id)
        }

        /// Get a premium cession record
        #[ink(message)]
        pub fn get_premium_cession(&self, cession_id: u64) -> Option<PremiumCession> {
            self.premium_cessions.get(&cession_id)
        }

        /// Get a loss recovery record
        #[ink(message)]
        pub fn get_loss_recovery(&self, recovery_id: u64) -> Option<LossRecovery> {
            self.loss_recoveries.get(&recovery_id)
        }

        /// Get all cession IDs for a reinsurance agreement
        #[ink(message)]
        pub fn get_agreement_cessions(&self, agreement_id: u64) -> Vec<u64> {
            self.agreement_cessions
                .get(&agreement_id)
                .unwrap_or_default()
        }

        /// Get all recovery IDs for a reinsurance agreement
        #[ink(message)]
        pub fn get_agreement_recoveries(&self, agreement_id: u64) -> Vec<u64> {
            self.agreement_recoveries
                .get(&agreement_id)
                .unwrap_or_default()
        }

        /// Get aggregated reinsurance statistics for an agreement
        #[ink(message)]
        pub fn get_reinsurance_stats(&self, agreement_id: u64) -> Option<ReinsuranceStats> {
            let agreement = self.reinsurance_agreements.get(&agreement_id)?;
            let net_recovery = (agreement.total_recoveries as i128)
                .saturating_sub(agreement.total_ceded_premiums as i128);
            Some(ReinsuranceStats {
                agreement_id,
                treaty_type: agreement.treaty_type,
                total_ceded_premiums: agreement.total_ceded_premiums,
                total_recoveries: agreement.total_recoveries,
                cession_count: agreement.cession_count,
                recovery_count: agreement.recovery_count,
                net_recovery,
            })
        }

        /// Get total number of reinsurance agreements
        #[ink(message)]
        pub fn get_reinsurance_count(&self) -> u64 {
            self.reinsurance_count
        }

        /// Get underwriting criteria for a pool
        #[ink(message)]
        pub fn get_underwriting_criteria(&self, pool_id: u64) -> Option<UnderwritingCriteria> {
            self.underwriting_criteria.get(&pool_id)
        }

        /// Get liquidity provider info
        #[ink(message)]
        pub fn get_liquidity_provider(
            &self,
            pool_id: u64,
            provider: AccountId,
        ) -> Option<PoolLiquidityProvider> {
            self.liquidity_providers.get(&(pool_id, provider))
        }

        /// Get total policies count
        #[ink(message)]
        pub fn get_policy_count(&self) -> u64 {
            self.policy_count
        }

        /// Get total claims count
        #[ink(message)]
        pub fn get_claim_count(&self) -> u64 {
            self.claim_count
        }

        /// Get admin address
        #[ink(message)]
        pub fn get_admin(&self) -> AccountId {
            self.admin
        }

        // =====================================================================
        // RISK ASSESSMENT MODEL (Task #254)
        // =====================================================================

        /// Create a comprehensive property risk model for accurate pricing
        /// Returns the risk_id and premium multiplier
        #[ink(message)]
        pub fn assess_property_risk_comprehensive(
            &mut self,
            property_id: u64,
            property_age_years: u32,
            property_value: u128,
            location_code: String,
            construction_type: String,
            has_security_system: bool,
            has_fire_extinguisher: bool,
            has_alarm_system: bool,
            owner_age_years: u32,
            years_as_owner: u32,
        ) -> Result<(u64, u32), InsuranceError> {
            self.ensure_admin()?;

            let now = self.env().block_timestamp();

            // Create property risk factors
            let property_factors = PropertyRiskFactors {
                property_id,
                property_age_years,
                property_value,
                location_code: location_code.clone(),
                construction_type: construction_type.clone(),
                has_security_system,
                has_fire_extinguisher,
                has_alarm_system,
                owner_age_years,
                years_as_owner,
                assessed_at: now,
            };

            // Get existing claims history for this property
            let claims = self.property_policies.get(&property_id).unwrap_or_default();
            let mut historical_claims_count = 0u32;
            let mut historical_claims_amount = 0u128;

            for policy_id in claims.iter() {
                if let Some(policy) = self.policies.get(policy_id) {
                    historical_claims_count =
                        historical_claims_count.saturating_add(policy.claims_count);
                    historical_claims_amount =
                        historical_claims_amount.saturating_add(policy.total_claimed);
                }
            }

            // Calculate individual risk scores
            let location_risk_score = risk_model::calculate_location_risk_score(&location_code);
            let construction_risk_score =
                risk_model::calculate_construction_risk_score(&construction_type);
            let age_risk_score = risk_model::calculate_age_risk_score(property_age_years);
            let ownership_risk_score =
                risk_model::calculate_ownership_risk_score(owner_age_years, years_as_owner);
            let claims_history_score = risk_model::calculate_claims_history_score(
                historical_claims_count,
                historical_claims_amount,
            );
            let safety_features_score = risk_model::calculate_safety_features_score(
                has_security_system,
                has_fire_extinguisher,
                has_alarm_system,
            );

            // Calculate overall risk score
            let overall_risk_score = risk_model::calculate_overall_risk_score(
                location_risk_score,
                construction_risk_score,
                age_risk_score,
                ownership_risk_score,
                claims_history_score,
                safety_features_score,
            );

            // Map to risk level
            let final_risk_level = risk_model::score_to_risk_level(overall_risk_score);

            // Calculate premium multiplier
            let premium_multiplier = risk_model::calculate_premium_multiplier(overall_risk_score);

            // Create risk model
            let risk_id = self.risk_model_count + 1;
            self.risk_model_count = risk_id;

            let validity_duration = risk_model::get_assessment_validity_seconds();
            let risk_model_data = PropertyRiskModel {
                risk_id,
                property_id,
                property_factors,
                historical_claims_count,
                historical_claims_amount,
                location_risk_score,
                construction_risk_score,
                age_risk_score,
                ownership_risk_score,
                claims_history_score,
                safety_features_score,
                overall_risk_score,
                final_risk_level: final_risk_level.clone(),
                premium_multiplier,
                assessed_at: now,
                valid_until: now + validity_duration,
                model_version: risk_model::get_model_version(),
            };

            self.property_risk_models.insert(&risk_id, &risk_model_data);

            self.env().emit_event(PropertyRiskModelCreated {
                risk_id,
                property_id,
                overall_risk_score,
                final_risk_level: final_risk_level.clone(),
                premium_multiplier,
                timestamp: now,
            });

            Ok((risk_id, premium_multiplier))
        }

        /// Get property risk model details
        #[ink(message)]
        pub fn get_property_risk_model(
            &self,
            risk_id: u64,
        ) -> Result<PropertyRiskModel, InsuranceError> {
            self.property_risk_models
                .get(&risk_id)
                .ok_or(InsuranceError::RiskModelGenerationFailed)
        }

        /// Update risk assessment for a property (reassess based on changes)
        #[ink(message)]
        pub fn update_property_risk_assessment(
            &mut self,
            risk_id: u64,
            property_age_years: u32,
            has_security_system: bool,
            has_fire_extinguisher: bool,
            has_alarm_system: bool,
        ) -> Result<(u32, u32), InsuranceError> {
            self.ensure_admin()?;

            let mut risk_model = self
                .property_risk_models
                .get(&risk_id)
                .ok_or(InsuranceError::RiskModelGenerationFailed)?;

            let now = self.env().block_timestamp();

            // Recalculate affected scores
            let age_risk_score = risk_model::calculate_age_risk_score(property_age_years);
            let safety_features_score = risk_model::calculate_safety_features_score(
                has_security_system,
                has_fire_extinguisher,
                has_alarm_system,
            );

            // Recalculate overall score with updated values
            let new_overall_score = risk_model::calculate_overall_risk_score(
                risk_model.location_risk_score,
                risk_model.construction_risk_score,
                age_risk_score,
                risk_model.ownership_risk_score,
                risk_model.claims_history_score,
                safety_features_score,
            );

            let new_risk_level = risk_model::score_to_risk_level(new_overall_score);
            let new_premium_multiplier =
                risk_model::calculate_premium_multiplier(new_overall_score);

            // Update model
            risk_model.age_risk_score = age_risk_score;
            risk_model.safety_features_score = safety_features_score;
            risk_model.overall_risk_score = new_overall_score;
            risk_model.final_risk_level = new_risk_level.clone();
            risk_model.premium_multiplier = new_premium_multiplier;
            risk_model.assessed_at = now;

            self.property_risk_models.insert(&risk_id, &risk_model);

            self.env().emit_event(PropertyRiskModelUpdated {
                risk_id,
                property_id: risk_model.property_id,
                new_risk_score: new_overall_score,
                new_risk_level: new_risk_level.clone(),
                timestamp: now,
            });

            Ok((new_overall_score, new_premium_multiplier))
        }

        // =====================================================================
        // FRAUD DETECTION SYSTEM (Task #258)
        // =====================================================================

        /// Perform comprehensive fraud risk assessment on a claim
        #[ink(message)]
        pub fn assess_claim_fraud_risk(
            &mut self,
            claim_id: u64,
            policy_id: u64,
        ) -> Result<(u64, u32, bool), InsuranceError> {
            let caller = self.env().caller();
            if caller != self.admin && !self.authorized_assessors.get(&caller).unwrap_or(false) {
                return Err(InsuranceError::Unauthorized);
            }

            let claim = self
                .claims
                .get(&claim_id)
                .ok_or(InsuranceError::ClaimNotFound)?;

            let policy = self
                .policies
                .get(&policy_id)
                .ok_or(InsuranceError::PolicyNotFound)?;

            let now = self.env().block_timestamp();

            // Collect fraud indicators and scores
            let mut fraud_scores = Vec::new();
            let mut detected_indicators = Vec::new();

            // 1. Check for multiple claims in short period
            let policyholder_claims = self
                .policyholder_policies
                .get(&claim.claimant)
                .unwrap_or_default();
            let time_since_last = if let Some(&last_claim_id) = policyholder_claims.last() {
                if let Some(last_claim) = self.claims.get(&last_claim_id) {
                    Some(now.saturating_sub(last_claim.submitted_at))
                } else {
                    None
                }
            } else {
                None
            };

            let (detected, score) = fraud_detection::detect_multiple_claims_short_period(
                policyholder_claims.len() as u32,
                time_since_last,
            );
            if detected {
                fraud_scores.push(score);
                detected_indicators.push(FraudIndicator::MultipleClaimsShortPeriod);
            }

            // 2. Check for anomalous claim amounts
            let avg_claim_amount = if policy.claims_count > 0 {
                policy.total_claimed / (policy.claims_count as u128)
            } else {
                claim.claim_amount
            };

            let (detected, score) = fraud_detection::detect_anomalous_claim_amount(
                claim.claim_amount,
                avg_claim_amount,
                policy.coverage_amount,
            );
            if detected {
                fraud_scores.push(score);
                detected_indicators.push(FraudIndicator::AnomalousClaimAmount);
            }

            // 3. Check for suspicious timing
            let (detected, score) = fraud_detection::detect_suspicious_timing(claim.submitted_at);
            if detected {
                fraud_scores.push(score);
                detected_indicators.push(FraudIndicator::SuspiciousTimingPattern);
            }

            // 4. Check for excessive coverage ratio
            let (detected, score) = fraud_detection::detect_excessive_coverage_ratio(
                claim.claim_amount,
                policy.coverage_amount,
            );
            if detected {
                fraud_scores.push(score);
                detected_indicators.push(FraudIndicator::ExcessiveCoverageRatio);
            }

            // 5. Check for historical fraud patterns
            let policyholder_rejection_rate = 0u32; // Would be calculated from history
            let (detected, score) = fraud_detection::detect_historical_fraud_pattern(
                policyholder_claims.len() as u32,
                policyholder_rejection_rate,
            );
            if detected {
                fraud_scores.push(score);
                detected_indicators.push(FraudIndicator::HistoricalFraudPattern);
            }

            // 6. Check for misrepresentation
            let (detected, score) = fraud_detection::detect_misrepresentation(
                claim.description.len() as u32,
                !claim.evidence_url.is_empty(),
            );
            if detected {
                fraud_scores.push(score);
                detected_indicators.push(FraudIndicator::Misrepresentation);
            }

            // Calculate total fraud risk score
            let fraud_score = fraud_detection::calculate_fraud_risk_score(&fraud_scores);
            let fraud_level = fraud_detection::score_to_fraud_risk_level(fraud_score);
            let requires_manual_review = fraud_detection::requires_manual_review(
                fraud_score,
                detected_indicators.len() as u32,
            );

            // Create assessment
            let assessment_id = self.fraud_assessment_count + 1;
            self.fraud_assessment_count = assessment_id;

            let assessment = FraudRiskAssessment {
                assessment_id,
                claim_id,
                policy_id,
                policyholder: claim.claimant,
                fraud_score,
                fraud_level: fraud_level.clone(),
                detected_indicators: detected_indicators.clone(),
                claim_amount: claim.claim_amount,
                expected_amount_range: (avg_claim_amount / 2, avg_claim_amount * 2),
                time_since_last_claim: time_since_last,
                similar_claims_count: policyholder_claims.len() as u32,
                policyholder_claims_count: policyholder_claims.len() as u32,
                assessor_notes: String::new(),
                assessment_timestamp: now,
                requires_manual_review,
            };

            self.fraud_assessments.insert(&assessment_id, &assessment);

            // Emit events based on fraud level
            self.env().emit_event(FraudRiskAssessmentCreated {
                assessment_id,
                claim_id,
                policyholder: claim.claimant,
                fraud_score,
                fraud_level: fraud_level.clone(),
                requires_manual_review,
                timestamp: now,
            });

            if fraud_score > fraud_detection::get_high_fraud_risk_threshold() {
                self.env().emit_event(HighFraudRiskDetected {
                    claim_id,
                    policyholder: claim.claimant,
                    fraud_score,
                    indicator_count: detected_indicators.len() as u32,
                    timestamp: now,
                });
            }

            // Update fraud detection stats
            if let Some(mut stats) = self.fraud_detection_stats.take() {
                stats.total_assessments = stats.total_assessments.saturating_add(1);
                if fraud_score > fraud_detection::get_high_fraud_risk_threshold() {
                    stats.high_risk_claims = stats.high_risk_claims.saturating_add(1);
                }
                stats.average_fraud_score = (stats.average_fraud_score
                    * (stats.total_assessments - 1) as u32
                    + fraud_score)
                    / stats.total_assessments as u32;
                stats.last_update = now;
                self.fraud_detection_stats = Some(stats);
            }

            Ok((assessment_id, fraud_score, requires_manual_review))
        }

        /// Get fraud risk assessment details
        #[ink(message)]
        pub fn get_fraud_assessment(
            &self,
            assessment_id: u64,
        ) -> Result<FraudRiskAssessment, InsuranceError> {
            self.fraud_assessments
                .get(&assessment_id)
                .ok_or(InsuranceError::FraudAssessmentNotFound)
        }

        /// Get fraud detection statistics
        #[ink(message)]
        pub fn get_fraud_detection_stats(&self) -> Option<FraudDetectionStats> {
            self.fraud_detection_stats.clone()
        }

        // =====================================================================
        // INTERNAL HELPERS
        // =====================================================================

        fn ensure_admin(&self) -> Result<(), InsuranceError> {
            if self.env().caller() != self.admin {
                return Err(InsuranceError::Unauthorized);
            }
            Ok(())
        }

        fn score_to_risk_level(score: u32) -> RiskLevel {
            match score {
                0..=20 => RiskLevel::VeryHigh,
                21..=40 => RiskLevel::High,
                41..=60 => RiskLevel::Medium,
                61..=80 => RiskLevel::Low,
                _ => RiskLevel::VeryLow,
            }
        }

        fn risk_score_to_multiplier(&self, score: u32) -> u32 {
            // score 0-100: higher score = lower risk = lower multiplier
            // Range: 400 (very high risk) to 80 (very low risk)
            match score {
                0..=20 => 400,
                21..=40 => 250,
                41..=60 => 150,
                61..=80 => 110,
                _ => 80,
            }
        }

        fn coverage_type_multiplier(coverage_type: &CoverageType) -> u32 {
            match coverage_type {
                CoverageType::Fire => 100,
                CoverageType::Theft => 80,
                CoverageType::Flood => 150,
                CoverageType::Earthquake => 200,
                CoverageType::LiabilityDamage => 120,
                CoverageType::NaturalDisaster => 180,
                CoverageType::Comprehensive => 250,
            }
        }

        /// Find a suitable pool for the given coverage type
        fn find_pool_for_coverage(&self, coverage_type: &CoverageType) -> Result<RiskPool, InsuranceError> {
            // Iterate through pools to find an active one matching the coverage type
            // For now, return the first active pool or a default
            let pool_count = self.pool_count;
            
            for pool_id in 1..=pool_count {
                if let Some(pool) = self.pools.get(&pool_id) {
                    if pool.is_active && pool.coverage_type == *coverage_type {
                        return Ok(pool);
                    }
                }
            }

            // If no specific pool found, return the first active pool
            for pool_id in 1..=pool_count {
                if let Some(pool) = self.pools.get(&pool_id) {
                    if pool.is_active {
                        return Ok(pool);
                    }
                }
            }

            Err(InsuranceError::PoolNotFound)
        }

        /// Get actuarial model for coverage type
        fn get_actuarial_model_for_coverage(&self, coverage_type: &CoverageType) -> Option<ActuarialModel> {
            // Search through models to find one matching the coverage type
            let model_count = self.model_count;
            
            for model_id in 1..=model_count {
                if let Some(model) = self.actuarial_models.get(&model_id) {
                    if model.coverage_type == *coverage_type {
                        return Some(model);
                    }
                }
            }
            
            None
        }

        fn internal_mint_token(
            &mut self,
            policy_id: u64,
            owner: AccountId,
            face_value: u128,
        ) -> Result<u64, InsuranceError> {
            let token_id = self.token_count + 1;
            self.token_count = token_id;

            let token = InsuranceToken {
                token_id,
                policy_id,
                owner,
                face_value,
                is_tradeable: true,
                created_at: self.env().block_timestamp(),
                listed_price: None,
            };

            self.insurance_tokens.insert(&token_id, &token);

            self.env().emit_event(InsuranceTokenMinted {
                token_id,
                policy_id,
                owner,
                face_value,
            });

            Ok(token_id)
        }

        fn execute_payout(
            &mut self,
            claim_id: u64,
            policy_id: u64,
            recipient: AccountId,
            amount: u128,
        ) -> Result<(), InsuranceError> {
            if amount == 0 {
                return Ok(());
            }

            let mut policy = self
                .policies
                .get(&policy_id)
                .ok_or(InsuranceError::PolicyNotFound)?;
            let mut pool = self
                .pools
                .get(&policy.pool_id)
                .ok_or(InsuranceError::PoolNotFound)?;

            // Check if reinsurance is needed
            let use_reinsurance = amount > pool.reinsurance_threshold;

            if use_reinsurance {
                // Try to recover excess from reinsurance
                self.try_reinsurance_recovery(claim_id, policy_id, amount)?;
            }

            if pool.available_capital < amount {
                return Err(InsuranceError::InsufficientPoolFunds);
            }

            pool.available_capital = pool.available_capital.saturating_sub(amount);
            pool.total_claims_paid += amount;
            self.pools.insert(&policy.pool_id, &pool);

            // Update policy
            policy.total_claimed += amount;
            if policy.total_claimed >= policy.coverage_amount {
                policy.status = PolicyStatus::Claimed;
            }
            self.policies.insert(&policy_id, &policy);

            // Update cooldown
            self.claim_cooldowns
                .insert(&policy.property_id, &self.env().block_timestamp());

            // Update claim status
            if let Some(mut claim) = self.claims.get(&claim_id) {
                claim.status = ClaimStatus::Paid;
                self.claims.insert(&claim_id, &claim);
            }

            self.env().emit_event(PayoutExecuted {
                claim_id,
                recipient,
                amount,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        fn evaluate_condition(
            comparator: &TriggerComparator,
            observed: u128,
            threshold: u128,
        ) -> bool {
            match comparator {
                TriggerComparator::GreaterOrEqual => observed >= threshold,
                TriggerComparator::LessOrEqual => observed <= threshold,
            }
        }

        fn compute_claim_amount(
            mode: &PayoutMode,
            remaining_coverage: u128,
        ) -> Result<u128, InsuranceError> {
            let amount = match mode {
                PayoutMode::Fixed(v) => (*v).min(remaining_coverage),
                PayoutMode::PercentBps(bps) => {
                    if *bps == 0 || *bps > 10_000 {
                        return Err(InsuranceError::InvalidPayoutMode);
                    }
                    remaining_coverage.saturating_mul(*bps as u128) / 10_000
                }
                PayoutMode::FullCoverage => remaining_coverage,
            };
            Ok(amount)
        }

        fn ensure_payout_mode_valid(mode: &PayoutMode) -> Result<(), InsuranceError> {
            match mode {
                PayoutMode::Fixed(v) => {
                    if *v == 0 {
                        return Err(InsuranceError::InvalidPayoutMode);
                    }
                }
                PayoutMode::PercentBps(bps) => {
                    if *bps == 0 || *bps > 10_000 {
                        return Err(InsuranceError::InvalidPayoutMode);
                    }
                }
                PayoutMode::FullCoverage => {}
            }
            Ok(())
        }

        fn try_reinsurance_recovery(
            &mut self,
            claim_id: u64,
            _policy_id: u64,
            amount: u128,
        ) -> Result<(), InsuranceError> {
            // Look for an active reinsurance agreement
            for i in 1..=self.reinsurance_count {
                if let Some(mut agreement) = self.reinsurance_agreements.get(&i) {
                    if !agreement.is_active {
                        continue;
                    }
                    let now = self.env().block_timestamp();
                    if now > agreement.end_time {
                        continue;
                    }

                    let recovery = match agreement.treaty_type {
                        ReinsuranceTreatyType::ExcessOfLoss => {
                            if amount > agreement.retention_limit {
                                amount
                                    .saturating_sub(agreement.retention_limit)
                                    .min(agreement.coverage_limit)
                            } else {
                                0
                            }
                        }
                        ReinsuranceTreatyType::QuotaShare => {
                            amount.saturating_mul(agreement.premium_ceded_rate as u128) / 10_000
                        }
                        ReinsuranceTreatyType::Surplus => {
                            if amount > agreement.retention_limit {
                                amount
                                    .saturating_sub(agreement.retention_limit)
                                    .min(agreement.coverage_limit)
                            } else {
                                0
                            }
                        }
                    };

                    if recovery > 0 {
                        // Record the loss recovery
                        let recovery_id = self.loss_recovery_count + 1;
                        self.loss_recovery_count = recovery_id;

                        let loss_recovery = LossRecovery {
                            recovery_id,
                            agreement_id: i,
                            claim_id,
                            gross_loss: amount,
                            recovered_amount: recovery,
                            recovered_at: now,
                        };
                        self.loss_recoveries.insert(&recovery_id, &loss_recovery);

                        let mut recoveries = self.agreement_recoveries.get(&i).unwrap_or_default();
                        recoveries.push(recovery_id);
                        self.agreement_recoveries.insert(&i, &recoveries);

                        agreement.total_recoveries += recovery;
                        agreement.recovery_count += 1;
                        self.reinsurance_agreements.insert(&i, &agreement);

                        self.env().emit_event(ReinsuranceActivated {
                            claim_id,
                            agreement_id: i,
                            recovery_amount: recovery,
                            timestamp: now,
                        });

                        self.env().emit_event(LossRecovered {
                            agreement_id: i,
                            claim_id,
                            recovery_id,
                            recovered_amount: recovery,
                            timestamp: now,
                        });

                        return Ok(());
                    }
                }
            }
            Ok(())
        }
    }

    impl Default for PropertyInsurance {
        fn default() -> Self {
            Self::new(AccountId::from([0x0; 32]))
        }
    }
}

pub use crate::propchain_insurance::{InsuranceError, PropertyInsurance};

#[cfg(test)]
mod tests;

// Unit tests extracted to tests.rs (Issue #101)
#[path = "tests.rs"]
mod insurance_tests_module;
