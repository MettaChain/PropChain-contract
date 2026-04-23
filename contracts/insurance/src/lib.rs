#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::needless_borrows_for_generic_args,
    clippy::too_many_arguments
)]

use ink::storage::Mapping;

/// Decentralized Property Insurance Platform
#[ink::contract]
mod propchain_insurance {
    use super::*;
    use ink::prelude::{string::String, vec::Vec};

    // Error types extracted to errors.rs (Issue #101)
    include!("errors.rs");

    // Data types extracted to types.rs (Issue #101)
    include!("types.rs");

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

        // Platform settings
        platform_fee_rate: u32,     // Basis points (e.g. 200 = 2%)
        claim_cooldown_period: u64, // In seconds
        min_pool_capital: u128,
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
                platform_fee_rate: 200,            // 2%
                claim_cooldown_period: 2_592_000,  // 30 days in seconds
                min_pool_capital: 100_000_000_000, // Minimum pool capital
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

        /// Calculate premium for a policy
        #[ink(message)]
        pub fn calculate_premium(
            &self,
            property_id: u64,
            coverage_amount: u128,
            coverage_type: CoverageType,
        ) -> Result<PremiumCalculation, InsuranceError> {
            let assessment = self
                .risk_assessments
                .get(&property_id)
                .ok_or(InsuranceError::PropertyNotInsurable)?;

            // Base rate in basis points: 150 = 1.50%
            let base_rate: u32 = 150;

            // Risk multiplier based on score (100 = 1.0x, 200 = 2.0x)
            let risk_multiplier = self.risk_score_to_multiplier(assessment.overall_risk_score);

            // Coverage type multiplier
            let coverage_multiplier = Self::coverage_type_multiplier(&coverage_type);

            // Annual premium = coverage * base_rate * risk_mult * coverage_mult / 1_000_000
            let annual_premium = coverage_amount
                .saturating_mul(base_rate as u128)
                .saturating_mul(risk_multiplier as u128)
                .saturating_mul(coverage_multiplier as u128)
                / 1_000_000_000_000u128; // 3 basis point divisors × 10000 each

            let monthly_premium = annual_premium / 12;

            // Deductible: 5% of coverage_amount, scaled by risk
            let deductible = coverage_amount
                .saturating_mul(500u128)
                .saturating_mul(risk_multiplier as u128)
                / 10_000_000u128;

            Ok(PremiumCalculation {
                base_rate,
                risk_multiplier,
                coverage_multiplier,
                annual_premium,
                monthly_premium,
                deductible,
            })
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
            let caller = self.env().caller();

            if caller != self.admin && !self.authorized_assessors.get(&caller).unwrap_or(false) {
                return Err(InsuranceError::Unauthorized);
            }

            let mut claim = self
                .claims
                .get(&claim_id)
                .ok_or(InsuranceError::ClaimNotFound)?;
            if claim.status != ClaimStatus::Pending && claim.status != ClaimStatus::UnderReview {
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
            };

            self.reinsurance_agreements
                .insert(&agreement_id, &agreement);
            Ok(agreement_id)
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

                    let recovery = amount.saturating_sub(agreement.retention_limit);
                    let capped_recovery = recovery.min(agreement.coverage_limit);

                    if capped_recovery > 0 {
                        agreement.total_recoveries += capped_recovery;
                        self.reinsurance_agreements.insert(&i, &agreement);

                        self.env().emit_event(ReinsuranceActivated {
                            claim_id,
                            agreement_id: i,
                            recovery_amount: capped_recovery,
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

// Unit tests extracted to tests.rs (Issue #101)
