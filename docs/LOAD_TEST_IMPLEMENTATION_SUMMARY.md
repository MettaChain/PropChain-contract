# Load Test Implementation Summary

This document summarizes the latest load testing updates for PropChain.

## New capabilities

- Added NetworkLatencyConfig to the load testing framework.
- Added Westend and Polkadot network latency profiles.
- Simulated packet loss, jitter, and congestion in E2E load tests.
- Added new focused load tests:
  - load_test_concurrent_registration_light
  - load_test_concurrent_registration_medium
  - load_test_concurrent_registration_heavy
  - load_test_concurrent_registration_extreme
  - load_test_endurance_sustained_load
  - load_test_spike_under_latency

## Run instructions

`ash
cargo test --package propchain-tests --test load_tests load_test_concurrent_registration_medium --release -- --nocapture
`

