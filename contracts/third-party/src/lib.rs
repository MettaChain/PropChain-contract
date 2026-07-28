#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unexpected_cfgs)]
#![allow(clippy::new_without_default)]

//! # PropChain Third-Party Service Integration
//!
//! Orchestrates interactions between PropChain contracts and external services:
//! - KYC/AML Providers (Identity verification, status checking)
//! - Fiat Payment Gateways (Bridging fiat payments to on-chain operations)
//! - Off-chain Monitoring and Alerting systems
//! - Service API endpoints and credential management
//!
//! Resolves: https://github.com/MettaChain/PropChain-contract/issues/113

use ink::prelude::string::String;
use ink::prelude::vec::Vec;
use ink::storage::Mapping;

#[ink::contract]
mod propchain_third_party {
    use super::*;

    // Data types extracted to types.rs (Issue #101)
    include!("types.rs");

    // Error types extracted to errors.rs (Issue #101)
    include!("errors.rs");

    // ========================================================================
    // EVENTS
    // ========================================================================

    #[ink(event)]
    pub struct ServiceRegistered {
        #[ink(topic)]
        service_id: ServiceId,
        service_type: ServiceType,
        name: String,
        provider_account: AccountId,
    }

    #[ink(event)]
    pub struct ServiceStatusChanged {
        #[ink(topic)]
        service_id: ServiceId,
        old_status: ServiceStatus,
        new_status: ServiceStatus,
    }

    #[ink(event)]
    pub struct KycRequestInitiated {
        #[ink(topic)]
        request_id: RequestId,
        #[ink(topic)]
        user: AccountId,
        service_id: ServiceId,
    }

    #[ink(event)]
    pub struct KycStatusUpdated {
        #[ink(topic)]
        request_id: RequestId,
        #[ink(topic)]
        user: AccountId,
        status: RequestStatus,
        verification_level: u8,
    }

    #[ink(event)]
    pub struct PaymentInitiated {
        #[ink(topic)]
        request_id: RequestId,
        #[ink(topic)]
        payer: AccountId,
        service_id: ServiceId,
        fiat_amount: u128,
        currency: String,
    }

    #[ink(event)]
    pub struct PaymentCompleted {
        #[ink(topic)]
        request_id: RequestId,
        status: RequestStatus,
        equivalent_tokens: u128,
    }

    #[ink(event)]
    pub struct MonitoringAlert {
        #[ink(topic)]
        service_id: ServiceId,
        #[ink(topic)]
        severity: u8,
        message: String,
        timestamp: u64,
    }

    // ========================================================================
    // CONTRACT STORAGE
    // ========================================================================

    #[ink(storage)]
    pub struct ThirdPartyIntegration {
        /// Contract admin
        admin: AccountId,
        /// Registered services
        services: Mapping<ServiceId, ServiceConfig>,
        /// Number of services
        service_counter: ServiceId,
        /// Provider account to service ID mapped
        provider_services: Mapping<AccountId, Vec<ServiceId>>,

        /// KYC records (User -> Record)
        kyc_records: Mapping<AccountId, KycRecord>,
        /// KYC requests
        kyc_requests: Mapping<RequestId, KycRequest>,

        /// Payment requests
        payment_requests: Mapping<RequestId, PaymentRequest>,

        /// Request counter
        request_counter: RequestId,
    }

    // ========================================================================
    // IMPLEMENTATION
    // ========================================================================

    impl ThirdPartyIntegration {
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            Self {
                admin: caller,
                services: Mapping::default(),
                service_counter: 0,
                provider_services: Mapping::default(),
                kyc_records: Mapping::default(),
                kyc_requests: Mapping::default(),
                payment_requests: Mapping::default(),
                request_counter: 0,
            }
        }

        // ====================================================================
        // SERVICE MANAGEMENT
        // ====================================================================

        /// Register a new third-party service (Admin only)
        #[ink(message)]
        pub fn register_service(
            &mut self,
            service_type: ServiceType,
            name: String,
            provider_account: AccountId,
            endpoint_url: String,
            api_version: String,
            fee_percentage: u16,
        ) -> Result<ServiceId, Error> {
            self.ensure_admin()?;

            if fee_percentage > 10000 {
                return Err(Error::InvalidFeePercentage);
            }

            self.service_counter += 1;
            let service_id = self.service_counter;

            let config = ServiceConfig {
                service_id,
                service_type: service_type.clone(),
                name: name.clone(),
                provider_account,
                endpoint_url,
                api_version,
                status: ServiceStatus::Active,
                registered_at: self.env().block_timestamp(),
                fees_collected: 0,
                fee_percentage,
            };

            self.services.insert(service_id, &config);

            let mut provider_list = self
                .provider_services
                .get(provider_account)
                .unwrap_or_default();
            provider_list.push(service_id);
            self.provider_services
                .insert(provider_account, &provider_list);

            self.env().emit_event(ServiceRegistered {
                service_id,
                service_type,
                name,
                provider_account,
            });

            Ok(service_id)
        }

        /// Update service status (Admin or Provider)
        #[ink(message)]
        pub fn update_service_status(
            &mut self,
            service_id: ServiceId,
            new_status: ServiceStatus,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut service = self.get_service_mut(service_id)?;

            if caller != self.admin && caller != service.provider_account {
                return Err(Error::Unauthorized);
            }

            let old_status = service.status.clone();
            service.status = new_status.clone();
            self.services.insert(service_id, &service);

            self.env().emit_event(ServiceStatusChanged {
                service_id,
                old_status,
                new_status,
            });

            Ok(())
        }

        // ====================================================================
        // KYC INTEGRATION
        // ====================================================================

        /// Initiate KYC request (User or Admin)
        #[ink(message)]
        pub fn initiate_kyc_request(
            &mut self,
            service_id: ServiceId,
            user: AccountId,
            reference_id: String,
        ) -> Result<RequestId, Error> {
            let caller = self.env().caller();
            if caller != user && caller != self.admin {
                return Err(Error::Unauthorized);
            }

            self.ensure_service_active(service_id, ServiceType::KycProvider)?;

            self.request_counter += 1;
            let request_id = self.request_counter;

            let req = KycRequest {
                request_id,
                user,
                service_id,
                reference_id,
                status: RequestStatus::Pending,
                initiated_at: self.env().block_timestamp(),
                updated_at: self.env().block_timestamp(),
                expiry_date: None,
            };

            self.kyc_requests.insert(request_id, &req);

            self.env().emit_event(KycRequestInitiated {
                request_id,
                user,
                service_id,
            });

            Ok(request_id)
        }

        /// Update KYC status (Provider only)
        #[ink(message)]
        pub fn update_kyc_status(
            &mut self,
            request_id: RequestId,
            status: RequestStatus,
            verification_level: u8,
            valid_for_days: u64,
        ) -> Result<(), Error> {
            let caller = self.env().caller();

            let mut req = self
                .kyc_requests
                .get(request_id)
                .ok_or(Error::RequestNotFound)?;
            let service = self.get_service(req.service_id)?;

            if caller != service.provider_account {
                return Err(Error::Unauthorized);
            }

            // Only update active statuses
            if req.status == RequestStatus::Approved || req.status == RequestStatus::Rejected {
                return Err(Error::InvalidStatusTransition);
            }

            let timestamp = self.env().block_timestamp();
            req.status = status.clone();
            req.updated_at = timestamp;

            if status == RequestStatus::Approved {
                let expires_at = timestamp + (valid_for_days * 86_400_000);
                req.expiry_date = Some(expires_at);

                let record = KycRecord {
                    user: req.user,
                    provider_id: req.service_id,
                    verification_level,
                    verified_at: timestamp,
                    expires_at,
                    is_active: true,
                };
                self.kyc_records.insert(req.user, &record);
            }

            self.kyc_requests.insert(request_id, &req);

            self.env().emit_event(KycStatusUpdated {
                request_id,
                user: req.user,
                status,
                verification_level,
            });

            Ok(())
        }

        /// Check if a user is KYC verified (view function for other contracts)
        #[ink(message)]
        pub fn is_kyc_verified(&self, user: AccountId, required_level: u8) -> bool {
            if let Some(record) = self.kyc_records.get(user) {
                if record.is_active
                    && record.verification_level >= required_level
                    && record.expires_at > self.env().block_timestamp()
                {
                    return true;
                }
            }
            false
        }

        // ====================================================================
        // FIAT PAYMENT GATEWAY INTEGRATION
        // ====================================================================

        /// Initiate fiat payment bridging
        #[ink(message)]
        pub fn initiate_fiat_payment(
            &mut self,
            service_id: ServiceId,
            target_contract: AccountId,
            operation_type: u8,
            fiat_amount: u128,
            fiat_currency: String,
            payment_reference: String,
        ) -> Result<RequestId, Error> {
            let payer = self.env().caller();
            self.ensure_service_active(service_id, ServiceType::PaymentGateway)?;

            self.request_counter += 1;
            let request_id = self.request_counter;

            let req = PaymentRequest {
                request_id,
                payer,
                service_id,
                target_contract,
                operation_type,
                fiat_amount,
                fiat_currency: fiat_currency.clone(),
                equivalent_tokens: 0,
                payment_reference,
                status: RequestStatus::Pending,
                init_time: self.env().block_timestamp(),
                complete_time: None,
            };

            self.payment_requests.insert(request_id, &req);

            self.env().emit_event(PaymentInitiated {
                request_id,
                payer,
                service_id,
                fiat_amount,
                currency: fiat_currency,
            });

            Ok(request_id)
        }

        /// Complete fiat payment (Provider only)
        #[ink(message)]
        pub fn complete_payment(
            &mut self,
            request_id: RequestId,
            success: bool,
            equivalent_tokens: u128,
        ) -> Result<(), Error> {
            let caller = self.env().caller();

            let mut req = self
                .payment_requests
                .get(request_id)
                .ok_or(Error::RequestNotFound)?;
            let service = self.get_service(req.service_id)?;

            if caller != service.provider_account {
                return Err(Error::Unauthorized);
            }

            if req.status != RequestStatus::Pending && req.status != RequestStatus::Processing {
                return Err(Error::InvalidStatusTransition);
            }

            req.status = if success {
                RequestStatus::Approved
            } else {
                RequestStatus::Failed
            };
            req.equivalent_tokens = equivalent_tokens;
            req.complete_time = Some(self.env().block_timestamp());

            self.payment_requests.insert(request_id, &req);

            self.env().emit_event(PaymentCompleted {
                request_id,
                status: req.status,
                equivalent_tokens,
            });

            Ok(())
        }

        // ====================================================================
        // MONITORING & ALERTING
        // ====================================================================

        /// Log an alert from an external monitoring system
        #[ink(message)]
        pub fn log_alert(
            &mut self,
            service_id: ServiceId,
            severity: u8,
            message: String,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let service = self.get_service(service_id)?;

            if caller != service.provider_account && service.service_type == ServiceType::Monitoring
            {
                return Err(Error::Unauthorized);
            }

            self.env().emit_event(MonitoringAlert {
                service_id,
                severity,
                message,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        // ====================================================================
        // QUERIES
        // ====================================================================

        #[ink(message)]
        pub fn get_service_config(&self, service_id: ServiceId) -> Option<ServiceConfig> {
            self.services.get(service_id)
        }

        #[ink(message)]
        pub fn get_kyc_record(&self, user: AccountId) -> Option<KycRecord> {
            self.kyc_records.get(user)
        }

        #[ink(message)]
        pub fn get_payment_request(&self, request_id: RequestId) -> Option<PaymentRequest> {
            self.payment_requests.get(request_id)
        }

        // ====================================================================
        // INTERNAL
        // ====================================================================

        fn ensure_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        fn get_service(&self, service_id: ServiceId) -> Result<ServiceConfig, Error> {
            self.services.get(service_id).ok_or(Error::ServiceNotFound)
        }

        fn get_service_mut(&self, service_id: ServiceId) -> Result<ServiceConfig, Error> {
            self.services.get(service_id).ok_or(Error::ServiceNotFound)
        }

        fn ensure_service_active(
            &self,
            service_id: ServiceId,
            expected_type: ServiceType,
        ) -> Result<(), Error> {
            let service = self.get_service(service_id)?;
            if service.status != ServiceStatus::Active {
                return Err(Error::ServiceInactive);
            }
            if service.service_type != expected_type {
                return Err(Error::ServiceNotFound);
            }
            Ok(())
        }
    }

    impl Default for ThirdPartyIntegration {
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
