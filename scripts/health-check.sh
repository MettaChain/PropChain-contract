#!/usr/bin/env bash

# PropChain Health Check Script
# Checks contract build status, test status, and dependency versions

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONTRACTS_DIR="contracts"
HEALTH_STATUS=0

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $1"
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check toolchain health
check_toolchain() {
    log_info "Checking toolchain..."

    if command_exists rustc; then
        local rust_version
        rust_version=$(rustc --version)
        log_success "Rust installed: $rust_version"
    else
        log_error "Rust not installed"
        HEALTH_STATUS=1
    fi

    if command_exists cargo; then
        log_success "Cargo available"
    else
        log_error "Cargo not found"
        HEALTH_STATUS=1
    fi

    if command_exists cargo-contract; then
        local contract_version
        contract_version=$(cargo contract --version 2>/dev/null || echo "unknown")
        log_success "cargo-contract installed: $contract_version"
    else
        log_warning "cargo-contract not installed"
    fi

    if rustup target list --installed 2>/dev/null | grep -q wasm32-unknown-unknown; then
        log_success "WASM target available"
    else
        log_warning "WASM target (wasm32-unknown-unknown) not installed"
    fi
}

# Check workspace build health
check_build() {
    log_info "Checking workspace build..."

    cd "$WORKSPACE_ROOT"

    if cargo check --workspace 2>/dev/null; then
        log_success "Workspace compiles successfully"
    else
        log_error "Workspace build check failed"
        HEALTH_STATUS=1
    fi
}

# Check formatting
check_formatting() {
    log_info "Checking code formatting..."

    cd "$WORKSPACE_ROOT"

    if cargo fmt --all -- --check 2>/dev/null; then
        log_success "Code formatting is correct"
    else
        log_warning "Code formatting issues detected (run: cargo fmt --all)"
    fi
}

# Check tests
check_tests() {
    log_info "Running test health check..."

    cd "$WORKSPACE_ROOT"

    if cargo test --workspace --exclude ipfs-metadata --exclude oracle --exclude escrow --exclude proxy --exclude security-audit --exclude compliance_registry 2>/dev/null; then
        log_success "Workspace tests pass"
    else
        log_error "Some workspace tests failed"
        HEALTH_STATUS=1
    fi
}

# Check dependency versions
check_dependencies() {
    log_info "Checking dependency versions..."

    cd "$WORKSPACE_ROOT"

    # Check ink! version from workspace Cargo.toml
    local ink_version
    ink_version=$(grep 'ink.*version' Cargo.toml | head -1 | sed 's/.*version = "\([^"]*\)".*/\1/')
    if [ -n "$ink_version" ]; then
        log_success "ink! version: $ink_version"
    else
        log_warning "Could not determine ink! version"
    fi

    # Check scale-codec version
    local scale_version
    scale_version=$(grep 'parity-scale-codec.*version' Cargo.toml | head -1 | sed 's/.*version = "\([^"]*\)".*/\1/')
    if [ -n "$scale_version" ]; then
        log_success "parity-scale-codec version: $scale_version"
    else
        log_warning "Could not determine scale-codec version"
    fi

    # Check scale-info version
    local scale_info_version
    scale_info_version=$(grep 'scale-info.*version' Cargo.toml | head -1 | sed 's/.*version = "\([^"]*\)".*/\1/')
    if [ -n "$scale_info_version" ]; then
        log_success "scale-info version: $scale_info_version"
    else
        log_warning "Could not determine scale-info version"
    fi

    # Check for outdated dependencies if cargo-outdated is available
    if command_exists cargo-outdated; then
        log_info "Checking for outdated dependencies..."
        cargo outdated --workspace --root-deps-only
    fi
}

# Document on-chain health check endpoints
check_on_chain_health() {
    log_info "On-chain Health Check Endpoints"
    log_info "================================"
    log_info ""
    log_info "The following contracts expose health() message endpoints:"
    log_info ""
    log_info "  • property-token:  health() -> HealthReport"
    log_info "    - Returns: contract_name, status, total_operations, error_count, error_rate_bips"
    log_info ""
    log_info "  • insurance:       health() -> HealthReport"
    log_info "    - Returns: contract_name, status, total_operations, error_count, error_rate_bips"
    log_info ""
    log_info "  • lending:         health() -> HealthReport"
    log_info "    - Returns: contract_name, status, total_operations, error_count, error_rate_bips"
    log_info ""
    log_info "Aggregation (monitoring contract):"
    log_info "  • register_health_contract(contract: AccountId)   -> Register a contract for aggregation"
    log_info "  • unregister_health_contract(contract: AccountId) -> Unregister a contract"
    log_info "  • get_health_contracts()                          -> List registered contracts"
    log_info ""
    log_info "To check on-chain health after deployment, use cargo-contract call:"
    log_info "  cargo contract call --suri //Alice property_token health"
    log_info "  cargo contract call --suri //Alice insurance health"
    log_info "  cargo contract call --suri //Alice lending health"
    log_info ""
    log_info "Health status values: Healthy, Degraded, Critical, Paused"
    log_info ""
}

# Check contract artifacts
check_contracts() {
    log_info "Checking contract health..."

    cd "$WORKSPACE_ROOT/$CONTRACTS_DIR"

    local contract_count=0
    local healthy_count=0

    for contract_dir in */; do
        if [ -f "$contract_dir/Cargo.toml" ]; then
            contract_count=$((contract_count + 1))
            local name
            name=$(basename "$contract_dir")

            cd "$contract_dir"
            if cargo check 2>/dev/null; then
                log_success "Contract '$name' compiles"
                healthy_count=$((healthy_count + 1))
            else
                log_error "Contract '$name' has compilation errors"
                HEALTH_STATUS=1
            fi
            cd ..
        fi
    done

    cd "$WORKSPACE_ROOT"
    log_info "Contracts: $healthy_count/$contract_count healthy"
}

# Generate health report
generate_report() {
    local report_file="$WORKSPACE_ROOT/health-report-$(date +%Y%m%d-%H%M%S).txt"

    {
        echo "PropChain Health Check Report"
        echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
        echo "Workspace: $WORKSPACE_ROOT"
        echo ""
        if [ "$HEALTH_STATUS" -eq 0 ]; then
            echo "Overall Status: HEALTHY"
        else
            echo "Overall Status: UNHEALTHY"
        fi
    } > "$report_file"

    log_info "Report saved to: $report_file"
}

# Main function
main() {
    local skip_tests=false
    local skip_build=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            --skip-tests)
                skip_tests=true
                shift
                ;;
            --skip-build)
                skip_build=true
                shift
                ;;
            --help)
                echo "Usage: $0 [OPTIONS]"
                echo "Options:"
                echo "  --skip-tests   Skip test execution"
                echo "  --skip-build   Skip build verification"
                echo "  --help         Show this help message"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done

    log_info "Starting PropChain health check..."
    echo ""

    check_toolchain
    echo ""

    check_dependencies
    echo ""

    check_formatting
    echo ""

    if [ "$skip_build" = false ]; then
        check_build
        echo ""

        check_contracts
        echo ""
    fi

    check_on_chain_health
    echo ""

    if [ "$skip_tests" = false ]; then
        check_tests
        echo ""
    fi

    generate_report

    echo ""
    if [ "$HEALTH_STATUS" -eq 0 ]; then
        log_success "All health checks passed"
    else
        log_error "Some health checks failed"
        exit 1
    fi
}

main "$@"
