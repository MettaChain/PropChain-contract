#!/bin/bash
# Load Test Runner Script for PropChain
# 
# This script provides convenient commands for running various load tests
# against the PropChain smart contracts.
#
# Usage:
#   ./scripts/load_test.sh [command] [options]
#
# Commands:
#   quick       - Run quick validation test (2-3 minutes)
#   standard    - Run standard test suite (10-15 minutes)
#   stress      - Run stress tests (15-20 minutes)
#   endurance   - Run endurance tests (5-10 minutes)
#   scalability - Run scalability tests (10-15 minutes)
#   full        - Run complete load test suite (30+ minutes)
#   help        - Show this help message

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PACKAGE="propchain-tests"
RELEASE_FLAG="--release"
OUTPUT_FLAG="--nocapture"

# Helper functions
print_header() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

check_prerequisites() {
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo is not installed. Please install Rust first."
        exit 1
    fi
    
    print_success "Prerequisites check passed"
}

run_load_test() {
    local test_pattern=$1
    local description=$2
    
    print_header "Running: $description"
    echo ""
    
    if [ -n "$test_pattern" ]; then
        cargo test --package "$PACKAGE" $test_pattern $RELEASE_FLAG -- $OUTPUT_FLAG
    else
        cargo test --package "$PACKAGE" $RELEASE_FLAG -- $OUTPUT_FLAG
    fi
    
    print_success "Load test completed: $description"
    echo ""
}

show_help() {
    cat << EOF
PropChain Load Test Runner
==========================

Usage: ./scripts/load_test.sh [command] [options]

Commands:
  quick       Run quick validation test (2-3 minutes)
              Test: Light load concurrent registration
              Use Case: Quick sanity check after code changes
  
  standard    Run standard test suite (10-15 minutes)
              Tests: All concurrent registration tests
              Use Case: Regular development testing
  
  stress      Run stress tests (15-20 minutes)
              Tests: Mass registration and transfer stress tests
              Use Case: Finding breaking points and bottlenecks
  
  endurance   Run endurance tests (5-10 minutes)
              Tests: Sustained load and short endurance tests
              Use Case: Detecting memory leaks and degradation
  
  scalability Run scalability tests (10-15 minutes)
              Tests: Database, user, and memory scalability
              Use Case: Capacity planning and growth analysis
  
  mixed       Run mixed workload tests (10-12 minutes)
              Tests: Mixed read/write operations
              Use Case: Simulating real-world usage patterns
  
  full        Run complete load test suite (30+ minutes)
              Tests: All load tests including stress and endurance
              Use Case: Comprehensive performance validation
  
  custom      Run custom test pattern
              Usage: ./scripts/load_test.sh custom <test_pattern>
              Example: ./scripts/load_test.sh custom "load_test_concurrent.*light"
  
  help        Show this help message

Options:
  --debug     Run without --release flag (faster compilation, slower execution)
  --quiet     Suppress detailed output
  --verbose   Show additional debugging information

Examples:
  # Quick validation after code changes
  ./scripts/load_test.sh quick
  
  # Full performance validation before release
  ./scripts/load_test.sh full
  
  # Run specific test
  ./scripts/load_test.sh custom "stress_test_mass_registration"
  
  # Run with debug mode (faster compilation)
  ./scripts/load_test.sh --debug quick

Performance Thresholds:
  Light Load:   >95% success, <500ms response, >20 ops/sec
  Medium Load:  >92% success, <750ms response, >50 ops/sec
  Heavy Load:   >90% success, <1000ms response, >100 ops/sec
  Stress:       >85% success, <2000ms response, >200 ops/sec

For more information, see docs/LOAD_TESTING_GUIDE.md

EOF
}

# Main command handler
case "${1:-help}" in
    quick)
        check_prerequisites
        run_load_test "load_test_concurrent_registration_light" "Quick Validation Test"
        ;;
    
    standard)
        check_prerequisites
        run_load_test "load_test_concurrent_registration" "Standard Test Suite"
        ;;
    
    stress)
        check_prerequisites
        run_load_test "stress_test_" "Stress Test Suite"
        ;;
    
    endurance)
        check_prerequisites
        run_load_test "endurance_test_" "Endurance Test Suite"
        ;;
    
    scalability)
        check_prerequisites
        run_load_test "scalability_test_memory_usage" "Scalability Memory Usage Test"
        ;;
    
    mixed)
        check_prerequisites
        run_load_test "load_test_mixed_operations" "Mixed Workload Test"
        ;;
    
    full)
        check_prerequisites
        print_header "Complete Load Test Suite"
        echo ""
        print_warning "This will run all load tests and may take 30+ minutes"
        echo ""
        read -p "Continue? [y/N] " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            run_load_test "" "Complete Load Test Suite"
        else
            echo "Aborted"
            exit 0
        fi
        ;;
    
    rate)
        check_prerequisites
        run_load_test "api_rate_limit" "API Rate Limit Tests (Issue #162)"
        ;;

    custom)
        check_prerequisites
        if [ -z "$2" ]; then
            print_error "Please specify a test pattern"
            echo "Usage: ./scripts/load_test.sh custom <test_pattern>"
            echo "Example: ./scripts/load_test.sh custom \"load_test_concurrent_registration_light\""
            exit 1
        fi
        run_load_test "$2" "Custom Test: $2"
        ;;
    
    help|--help|-h)
        show_help
        ;;
    
    *)
        print_error "Unknown command: $1"
        echo ""
        show_help
        exit 1
        ;;
esac

echo ""
print_success "Load test execution completed successfully!"
echo ""
echo "Next steps:"
echo "  - Review test output for performance metrics"
echo "  - Check for any threshold violations"
echo "  - Compare results with baseline metrics"
echo "  - See docs/LOAD_TEST_MONITORING.md for analysis guidance"
echo ""
