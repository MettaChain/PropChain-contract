//! Integration tests for the PropChain Monitoring contract.
//!
//! These tests exercise the full on-chain lifecycle of the monitoring contract
//! against a live Substrate node via `ink_e2e`. They are gated behind the
//! `e2e-tests` feature flag and must be run with:
//!
//!   cargo test --features e2e-tests --package propchain-tests
//!
//! A locally running `substrate-contracts-node` is required.
//! See the project README and `scripts/local-node.sh` for setup instructions.

#![cfg(feature = "e2e-tests")]

use ink_e2e::build_message;
use propchain_monitoring::monitoring::MonitoringContract;
use propchain_traits::monitoring::{AlertType, HealthStatus, OperationType};

type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

async fn deploy_monitoring(
    client: &mut ink_e2e::Client<ink_e2e::PolkadotConfig, ink_e2e::DefaultEnvironment>,
) -> ink::primitives::AccountId {
    let constructor = MonitoringContract::new();
    client
        .instantiate("propchain-monitoring", &ink_e2e::alice(), constructor, 0, None)
        .await
        .expect("monitoring contract instantiation failed")
        .account_id
}

// =============================================================================
// Deployment
// =============================================================================

#[ink_e2e::test]
async fn e2e_monitoring_deployment() -> E2EResult<()> {
    let mut client =
        ink_e2e::Client::<ink_e2e::PolkadotConfig, ink_e2e::DefaultEnvironment>::new().await?;
    let contract_id = deploy_monitoring(&mut client).await;

    let get_admin_msg =
        build_message::<MonitoringContract>(contract_id.clone()).call(|c| c.get_admin());
    let admin = client
        .call_dry_run(&ink_e2e::alice(), &get_admin_msg, 0, None)
        .await
        .return_value();
    assert_eq!(admin, ink_e2e::alice().account_id());

    let status_msg =
        build_message::<MonitoringContract>(contract_id.clone()).call(|c| c.get_system_status());
    let status = client
        .call_dry_run(&ink_e2e::alice(), &status_msg, 0, None)
        .await
        .return_value();
    assert_eq!(status, HealthStatus::Healthy);

    let hc_msg =
        build_message::<MonitoringContract>(contract_id.clone()).call(|c| c.health_check());
    let hc = client
        .call_dry_run(&ink_e2e::alice(), &hc_msg, 0, None)
        .await
        .return_value();
    assert_eq!(hc.status, HealthStatus::Healthy);
    assert_eq!(hc.total_operations, 0);
    assert_eq!(hc.overall_error_rate_bips, 0);
    assert!(hc.is_accepting_calls);

    Ok(())
}

// =============================================================================
// Performance metrics — record_operation + get_performance_metrics
// =============================================================================

#[ink_e2e::test]
async fn e2e_record_operation_success() -> E2EResult<()> {
    let mut client =
        ink_e2e::Client::<ink_e2e::PolkadotConfig, ink_e2e::DefaultEnvironment>::new().await?;
    let contract_id = deploy_monitoring(&mut client).await;

    let record_msg = build_message::<MonitoringContract>(contract_id.clone())
        .call(|c| c.record_operation(OperationType::RegisterProperty, true));
    client
        .call(&ink_e2e::alice(), record_msg, 0, None)
        .await
        .expect("record_operation failed");

    let metrics_msg = build_message::<MonitoringContract>(contract_id.clone())
        .call(|c| c.get_performance_metrics(OperationType::RegisterProperty));
    let metrics = client
        .call_dry_run(&ink_e2e::alice(), &metrics_msg, 0, None)
        .await
        .return_value();

    assert_eq!(metrics.total_calls, 1);
    assert_eq!(metrics.success_count, 1);
    assert_eq!(metrics.error_count, 0);
    assert_eq!(metrics.error_rate_bips, 0);

    Ok(())
}

#[ink_e2e::test]
async fn e2e_record_operation_failure() -> E2EResult<()> {
    let mut client =
        ink_e2e::Client::<ink_e2e::PolkadotConfig, ink_e2e::DefaultEnvironment>::new().await?;
    let contract_id = deploy_monitoring(&mut client).await;

    let record_msg = build_message::<MonitoringContract>(contract_id.clone())
        .call(|c| c.record_operation(OperationType::TransferProperty, false));
    client
        .call(&ink_e2e::alice(), record_msg, 0, None)
        .await
        .expect("record_operation (failure) failed");

    let metrics_msg = build_message::<MonitoringContract>(contract_id.clone())
        .call(|c| c.get_performance_metrics(OperationType::TransferProperty));
    let metrics = client
        .call_dry_run(&ink_e2e::alice(), &metrics_msg, 0, None)
        .await
        .return_value();

    assert_eq!(metrics.total_calls, 1);
    assert_eq!(metrics.success_count, 0);
    assert_eq!(metrics.error_count, 1);
    // 1 error / 1 total = 100% = 10 000 bips
    assert_eq!(metrics.error_rate_bips, 10_000);

    Ok(())
}

#[ink_e2e::test]
async fn e2e_get_all_metrics_covers_all_operation_types() -> E2EResult<()> {
    let mut client =
        ink_e2e::Client::<ink_e2e::PolkadotConfig, ink_e2e::DefaultEnvironment>::new().await?;
    let contract_id = deploy_monitoring(&mut client).await;

    for success in [true, false] {
        let msg = build_message::<MonitoringContract>(contract_id.clone())
            .call(move |c| c.record_operation(OperationType::Generic, success));
        client
            .call(&ink_e2e::alice(), msg, 0, None)
            .await
            .expect("record_operation failed");
    }

    let all_msg =
        build_message::<MonitoringContract>(contract_id.clone()).call(|c| c.get_all_metrics());
    let all = client
        .call_dry_run(&ink_e2e::alice(), &all_msg, 0, None)
        .await
        .return_value();

    assert_eq!(all.len(), 16);

    let generic_entry = all
        .iter()
        .find(|m| m.operation == OperationType::Generic)
        .expect("Generic entry missing");
    assert_eq!(generic_entry.total_calls, 2);
    assert_eq!(generic_entry.error_rate_bips, 5_000);

    Ok(())
}

// =============================================================================
// Health check endpoint
// =============================================================================

#[ink_e2e::test]
async fn e2e_health_check_reports_critical_on_high_error_rate() -> E2EResult<()> {
    let mut client =
        ink_e2e::Client::<ink_e2e::PolkadotConfig, ink_e2e::DefaultEnvironment>::new().await?;
    let contract_id = deploy_monitoring(&mut client).await;

    // 3 errors + 1 success = 75% error rate → Critical
    for success in [false, false, false, true] {
        let msg = build_message::<MonitoringContract>(contract_id.clone())
            .call(move |c| c.record_operation(OperationType::Generic, success));
        client
            .call(&ink_e2e::alice(), msg, 0, None)
            .await
            .expect("record_operation failed");
    }

    let hc_msg =
        build_message::<MonitoringContract>(contract_id.clone()).call(|c| c.health_check());
    let hc = client
        .call_dry_run(&ink_e2e::alice(), &hc_msg, 0, None)
        .await
        .return_value();

    assert_eq!(hc.status, HealthStatus::Critical);
    assert_eq!(hc.overall_error_rate_bips, 7_500);
    assert_eq!(hc.total_operations, 4);

    Ok(())
}

#[ink_e2e::test]
async fn e2e_health_check_healthy_with_no_errors() -> E2EResult<()> {
    let mut client =
        ink_e2e::Client::<ink_e2e::PolkadotConfig, ink_e2e::DefaultEnvironment>::new().await?;
    let contract_id = deploy_monitoring(&mut client).await;

    for _ in 0..5u32 {
        let msg = build_message::<MonitoringContract>(contract_id.clone())
            .call(|c| c.record_operation(OperationType::MintToken, true));
        client
            .call(&ink_e2e::alice(), msg, 0, None)
            .await
            .expect("record_operation failed");
    }

    let hc_msg =
        build_message::<MonitoringContract>(contract_id.clone()).call(|c| c.health_check());
    let hc = client
        .call_dry_run(&ink_e2e::alice(), &hc_msg, 0, None)
        .await
        .return_value();

    assert_eq!(hc.status, HealthStatus::Healthy);
    assert_eq!(hc.overall_error_rate_bips, 0);
    assert!(hc.is_accepting_calls);

    Ok(())
}

// =============================================================================
// Access control
// =============================================================================

#[ink_e2e::test]
async fn e2e_record_operation_rejects_unauthorized_caller() -> E2EResult<()> {
    let mut client =
        ink_e2e::Client::<ink_e2e::PolkadotConfig, ink_e2e::DefaultEnvironment>::new().await?;
    let contract_id = deploy_monitoring(&mut client).await;

    let msg = build_message::<MonitoringContract>(contract_id.clone())
        .call(|c| c.record_operation(OperationType::Generic, true));
    let result = client.call(&ink_e2e::bob(), msg, 0, None).await;
    assert!(result.is_err(), "Unauthorized call should be rejected");

    Ok(())
}

#[ink_e2e::test]
async fn e2e_admin_message_rejects_non_admin() -> E2EResult<()> {
    let mut client =
        ink_e2e::Client::<ink_e2e::PolkadotConfig, ink_e2e::DefaultEnvironment>::new().await?;
    let contract_id = deploy_monitoring(&mut client).await;

    let pause_msg =
        build_message::<MonitoringContract>(contract_id.clone()).call(|c| c.pause());
    let result = client.call(&ink_e2e::bob(), pause_msg, 0, None).await;
    assert!(result.is_err(), "Non-admin pause should be rejected");

    Ok(())
}

#[ink_e2e::test]
async fn e2e_authorized_reporter_can_record_then_revoke() -> E2EResult<()> {
    let mut client =
        ink_e2e::Client::<ink_e2e::PolkadotConfig, ink_e2e::DefaultEnvironment>::new().await?;
    let contract_id = deploy_monitoring(&mut client).await;

    let add_msg = build_message::<MonitoringContract>(contract_id.clone())
        .call(|c| c.add_reporter(ink_e2e::bob().account_id()));
    client
        .call(&ink_e2e::alice(), add_msg, 0, None)
        .await
        .expect("add_reporter failed");

    let is_auth_msg = build_message::<MonitoringContract>(contract_id.clone())
        .call(|c| c.is_authorized_reporter(ink_e2e::bob().account_id()));
    let is_auth = client
        .call_dry_run(&ink_e2e::alice(), &is_auth_msg, 0, None)
        .await
        .return_value();
    assert!(is_auth);

    let record_msg = build_message::<MonitoringContract>(contract_id.clone())
        .call(|c| c.record_operation(OperationType::Generic, true));
    client
        .call(&ink_e2e::bob(), record_msg, 0, None)
        .await
        .expect("authorized reporter should be able to record");

    let remove_msg = build_message::<MonitoringContract>(contract_id.clone())
        .call(|c| c.remove_reporter(ink_e2e::bob().account_id()));
    client
        .call(&ink_e2e::alice(), remove_msg, 0, None)
        .await
        .expect("remove_reporter failed");

    let record_after_revoke = build_message::<MonitoringContract>(contract_id.clone())
        .call(|c| c.record_operation(OperationType::Generic, true));
    let result = client
        .call(&ink_e2e::bob(), record_after_revoke, 0, None)
        .await;
    assert!(result.is_err(), "Revoked reporter should be rejected");

    Ok(())
}

// =============================================================================
// Alerting system
// =============================================================================

#[ink_e2e::test]
async fn e2e_high_error_rate_alert_triggers() -> E2EResult<()> {
    let mut client =
        ink_e2e::Client::<ink_e2e::PolkadotConfig, ink_e2e::DefaultEnvironment>::new().await?;
    let contract_id = deploy_monitoring(&mut client).await;

    // Activate HighErrorRate alert at 5% threshold
    let set_alert_msg = build_message::<MonitoringContract>(contract_id.clone())
        .call(|c| c.set_alert_config(AlertType::HighErrorRate, 500, true));
    client
        .call(&ink_e2e::alice(), set_alert_msg, 0, None)
        .await
        .expect("set_alert_config failed");

    let get_alert_msg = build_message::<MonitoringContract>(contract_id.clone())
        .call(|c| c.get_alert_config(AlertType::HighErrorRate));
    let cfg = client
        .call_dry_run(&ink_e2e::alice(), &get_alert_msg, 0, None)
        .await
        .return_value();
    assert!(cfg.is_active);
    assert_eq!(cfg.threshold_bips, 500);

    // 1 success + 2 errors = 66% error rate > 5% threshold
    for success in [true, false, false] {
        let msg = build_message::<MonitoringContract>(contract_id.clone())
            .call(move |c| c.record_operation(OperationType::Generic, success));
        client
            .call(&ink_e2e::alice(), msg, 0, None)
            .await
            .expect("record_operation failed");
    }

    // Alert should have fired: last_triggered_at is set
    let cfg_after = client
        .call_dry_run(&ink_e2e::alice(), &get_alert_msg, 0, None)
        .await
        .return_value();
    assert!(
        cfg_after.last_triggered_at > 0,
        "HighErrorRate alert should have been triggered"
    );

    Ok(())
}

#[ink_e2e::test]
async fn e2e_set_alert_config_rejects_invalid_threshold() -> E2EResult<()> {
    let mut client =
        ink_e2e::Client::<ink_e2e::PolkadotConfig, ink_e2e::DefaultEnvironment>::new().await?;
    let contract_id = deploy_monitoring(&mut client).await;

    let set_msg = build_message::<MonitoringContract>(contract_id.clone())
        .call(|c| c.set_alert_config(AlertType::HighErrorRate, 10_001, true));
    let result = client.call(&ink_e2e::alice(), set_msg, 0, None).await;
    assert!(result.is_err(), "Threshold > 10 000 must be rejected");

    Ok(())
}

// =============================================================================
// Snapshot (circular buffer)
// =============================================================================

#[ink_e2e::test]
async fn e2e_take_and_retrieve_metrics_snapshot() -> E2EResult<()> {
    let mut client =
        ink_e2e::Client::<ink_e2e::PolkadotConfig, ink_e2e::DefaultEnvironment>::new().await?;
    let contract_id = deploy_monitoring(&mut client).await;

    // 3 successes + 1 error
    for success in [true, true, true, false] {
        let msg = build_message::<MonitoringContract>(contract_id.clone())
            .call(move |c| c.record_operation(OperationType::BridgeTransfer, success));
        client
            .call(&ink_e2e::alice(), msg, 0, None)
            .await
            .expect("record_operation failed");
    }

    let snap_msg =
        build_message::<MonitoringContract>(contract_id.clone()).call(|c| c.take_metrics_snapshot());
    client
        .call(&ink_e2e::alice(), snap_msg, 0, None)
        .await
        .expect("take_metrics_snapshot failed");

    let get_snap_msg = build_message::<MonitoringContract>(contract_id.clone())
        .call(|c| c.get_metrics_snapshot(0));
    let snap = client
        .call_dry_run(&ink_e2e::alice(), &get_snap_msg, 0, None)
        .await
        .return_value()
        .expect("snapshot at slot 0 should exist");

    assert_eq!(snap.snapshot_id, 0);
    assert_eq!(snap.total_calls, 4);
    assert_eq!(snap.total_errors, 1);
    // 1/4 = 25% = 2500 bips
    assert_eq!(snap.error_rate_bips, 2_500);

    Ok(())
}

#[ink_e2e::test]
async fn e2e_snapshot_circular_buffer_wraps() -> E2EResult<()> {
    let mut client =
        ink_e2e::Client::<ink_e2e::PolkadotConfig, ink_e2e::DefaultEnvironment>::new().await?;
    let contract_id = deploy_monitoring(&mut client).await;

    // 101 writes forces a wrap: snapshot 100 lands at slot 100 % 100 == 0
    for _ in 0..=100u64 {
        let msg = build_message::<MonitoringContract>(contract_id.clone())
            .call(|c| c.take_metrics_snapshot());
        client
            .call(&ink_e2e::alice(), msg, 0, None)
            .await
            .expect("take_metrics_snapshot failed");
    }

    let get_msg = build_message::<MonitoringContract>(contract_id.clone())
        .call(|c| c.get_metrics_snapshot(0));
    let snap = client
        .call_dry_run(&ink_e2e::alice(), &get_msg, 0, None)
        .await
        .return_value()
        .expect("slot 0 should exist after wrap");

    assert_eq!(snap.snapshot_id, 100);

    Ok(())
}

#[ink_e2e::test]
async fn e2e_snapshot_rejects_unauthorized() -> E2EResult<()> {
    let mut client =
        ink_e2e::Client::<ink_e2e::PolkadotConfig, ink_e2e::DefaultEnvironment>::new().await?;
    let contract_id = deploy_monitoring(&mut client).await;

    let msg =
        build_message::<MonitoringContract>(contract_id.clone()).call(|c| c.take_metrics_snapshot());
    let result = client.call(&ink_e2e::bob(), msg, 0, None).await;
    assert!(result.is_err(), "Unauthorized snapshot must be rejected");

    Ok(())
}

// =============================================================================
// Pause / resume lifecycle
// =============================================================================

#[ink_e2e::test]
async fn e2e_pause_blocks_operations_resume_restores() -> E2EResult<()> {
    let mut client =
        ink_e2e::Client::<ink_e2e::PolkadotConfig, ink_e2e::DefaultEnvironment>::new().await?;
    let contract_id = deploy_monitoring(&mut client).await;

    let pause_msg =
        build_message::<MonitoringContract>(contract_id.clone()).call(|c| c.pause());
    client
        .call(&ink_e2e::alice(), pause_msg, 0, None)
        .await
        .expect("pause failed");

    let status_msg =
        build_message::<MonitoringContract>(contract_id.clone()).call(|c| c.get_system_status());
    let status = client
        .call_dry_run(&ink_e2e::alice(), &status_msg, 0, None)
        .await
        .return_value();
    assert_eq!(status, HealthStatus::Paused);

    let hc_msg =
        build_message::<MonitoringContract>(contract_id.clone()).call(|c| c.health_check());
    let hc = client
        .call_dry_run(&ink_e2e::alice(), &hc_msg, 0, None)
        .await
        .return_value();
    assert_eq!(hc.status, HealthStatus::Paused);
    assert!(!hc.is_accepting_calls);

    let record_msg = build_message::<MonitoringContract>(contract_id.clone())
        .call(|c| c.record_operation(OperationType::Generic, true));
    assert!(
        client.call(&ink_e2e::alice(), record_msg, 0, None).await.is_err(),
        "record_operation must fail when paused"
    );

    let snap_msg =
        build_message::<MonitoringContract>(contract_id.clone()).call(|c| c.take_metrics_snapshot());
    assert!(
        client.call(&ink_e2e::alice(), snap_msg, 0, None).await.is_err(),
        "snapshot must fail when paused"
    );

    let resume_msg =
        build_message::<MonitoringContract>(contract_id.clone()).call(|c| c.resume());
    client
        .call(&ink_e2e::alice(), resume_msg, 0, None)
        .await
        .expect("resume failed");

    let status_after = client
        .call_dry_run(&ink_e2e::alice(), &status_msg, 0, None)
        .await
        .return_value();
    assert_eq!(status_after, HealthStatus::Healthy);

    let record_after_msg = build_message::<MonitoringContract>(contract_id.clone())
        .call(|c| c.record_operation(OperationType::Generic, true));
    client
        .call(&ink_e2e::alice(), record_after_msg, 0, None)
        .await
        .expect("record_operation after resume failed");

    Ok(())
}

#[ink_e2e::test]
async fn e2e_double_pause_is_idempotent() -> E2EResult<()> {
    let mut client =
        ink_e2e::Client::<ink_e2e::PolkadotConfig, ink_e2e::DefaultEnvironment>::new().await?;
    let contract_id = deploy_monitoring(&mut client).await;

    for _ in 0..2u8 {
        let msg =
            build_message::<MonitoringContract>(contract_id.clone()).call(|c| c.pause());
        client
            .call(&ink_e2e::alice(), msg, 0, None)
            .await
            .expect("pause should be idempotent");
    }

    let status_msg =
        build_message::<MonitoringContract>(contract_id.clone()).call(|c| c.get_system_status());
    let status = client
        .call_dry_run(&ink_e2e::alice(), &status_msg, 0, None)
        .await
        .return_value();
    assert_eq!(status, HealthStatus::Paused);

    Ok(())
}

// =============================================================================
// Admin transfer
// =============================================================================

#[ink_e2e::test]
async fn e2e_transfer_admin() -> E2EResult<()> {
    let mut client =
        ink_e2e::Client::<ink_e2e::PolkadotConfig, ink_e2e::DefaultEnvironment>::new().await?;
    let contract_id = deploy_monitoring(&mut client).await;

    let transfer_msg = build_message::<MonitoringContract>(contract_id.clone())
        .call(|c| c.transfer_admin(ink_e2e::bob().account_id()));
    client
        .call(&ink_e2e::alice(), transfer_msg, 0, None)
        .await
        .expect("transfer_admin failed");

    let get_admin_msg =
        build_message::<MonitoringContract>(contract_id.clone()).call(|c| c.get_admin());
    let admin = client
        .call_dry_run(&ink_e2e::alice(), &get_admin_msg, 0, None)
        .await
        .return_value();
    assert_eq!(admin, ink_e2e::bob().account_id());

    let pause_msg =
        build_message::<MonitoringContract>(contract_id.clone()).call(|c| c.pause());
    let result = client.call(&ink_e2e::alice(), pause_msg, 0, None).await;
    assert!(result.is_err(), "Former admin should be rejected after transfer");

    Ok(())
}
