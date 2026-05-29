# Load Test Runner Script for PropChain (PowerShell)
# 
# This script provides convenient commands for running various load tests
# against the PropChain smart contracts.
#
# Usage:
#   .\scripts\load_test.ps1 [command] [options]
#
# Commands:
#   quick       - Run quick validation test (2-3 minutes)
#   standard    - Run standard test suite (10-15 minutes)
#   stress      - Run stress tests (15-20 minutes)
#   endurance   - Run endurance tests (5-10 minutes)
#   scalability - Run scalability tests (10-15 minutes)
#   full        - Run complete load test suite (30+ minutes)
#   help        - Show this help message

param(
    [Parameter(Position=0)]
    [string]$Command = "help",
    
    [Parameter(Position=1)]
    [string]$TestPattern = "",
    
    [switch]$Debug,
    [switch]$Quiet,
    [switch]$Verbose
)

# Configuration
$Package = "propchain-tests"
$ReleaseFlag = if ($Debug) { "" } else { "--release" }
$OutputFlag = if ($Quiet) { "" } else { "--nocapture" }

# Helper functions
function Print-Header {
    param([string]$Text)
    Write-Host "========================================" -ForegroundColor Blue
    Write-Host $Text -ForegroundColor Blue
    Write-Host "========================================" -ForegroundColor Blue
}

function Print-Success {
    param([string]$Text)
    Write-Host "✓ $Text" -ForegroundColor Green
}

function Print-Warning {
    param([string]$Text)
    Write-Host "⚠ $Text" -ForegroundColor Yellow
}

function Print-Error {
    param([string]$Text)
    Write-Host "✗ $Text" -ForegroundColor Red
}

function Check-Prerequisites {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Print-Error "Cargo is not installed. Please install Rust first."
        exit 1
    }
    
    Print-Success "Prerequisites check passed"
}

function Run-LoadTest {
    param(
        [string]$TestPattern,
        [string]$Description
    )
    
    Print-Header "Running: $Description"
    Write-Host ""
    
    $cargoArgs = @("test", "--package", $Package, $ReleaseFlag, $OutputFlag)

    if ($TestPattern) {
        $cargoArgs += $TestPattern
    } else {
        $cargoArgs = @("test", "--package", $Package, $ReleaseFlag, $OutputFlag)
    }
    
    & cargo @cargoArgs
    
    if ($LASTEXITCODE -eq 0) {
        Print-Success "Load test completed: $Description"
    } else {
        Print-Error "Load test failed: $Description"
        exit 1
    }
    
    Write-Host ""
}

function Show-Help {
    Write-Host @"
PropChain Load Test Runner (PowerShell)
========================================

Usage: .\scripts\load_test.ps1 [command] [options]

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
              Usage: .\scripts\load_test.ps1 custom <test_pattern>
              Example: .\scripts\load_test.ps1 custom "load_test_concurrent.*light"
  
  help        Show this help message

Options:
  -Debug      Run without --release flag (faster compilation, slower execution)
  -Quiet      Suppress detailed output
  -Verbose    Show additional debugging information

Examples:
  # Quick validation after code changes
  .\scripts\load_test.ps1 quick
  
  # Full performance validation before release
  .\scripts\load_test.ps1 full
  
  # Run specific test
  .\scripts\load_test.ps1 custom "stress_test_mass_registration"
  
  # Run with debug mode (faster compilation)
  .\scripts\load_test.ps1 -Debug quick

Performance Thresholds:
  Light Load:   >95% success, <500ms response, >20 ops/sec
  Medium Load:  >92% success, <750ms response, >50 ops/sec
  Heavy Load:   >90% success, <1000ms response, >100 ops/sec
  Stress:       >85% success, <2000ms response, >200 ops/sec

For more information, see docs\LOAD_TESTING_GUIDE.md

"@
}

# Main command handler
switch ($Command.ToLower()) {
    "quick" {
        Check-Prerequisites
        Run-LoadTest -TestPattern "load_test_concurrent_registration_light" -Description "Quick Validation Test"
    }
    
    "standard" {
        Check-Prerequisites
        Run-LoadTest -TestPattern "load_test_concurrent_registration" -Description "Standard Test Suite"
    }
    
    "stress" {
        Check-Prerequisites
        Run-LoadTest -TestPattern "stress_test_" -Description "Stress Test Suite"
    }
    
    "endurance" {
        Check-Prerequisites
        Run-LoadTest -TestPattern "endurance_test_" -Description "Endurance Test Suite"
    }
    
    "scalability" {
        Check-Prerequisites
        Run-LoadTest -TestPattern "scalability_test_memory_usage" -Description "Scalability Memory Usage Test"
    }
    
    "mixed" {
        Check-Prerequisites
        Run-LoadTest -TestPattern "load_test_mixed_operations" -Description "Mixed Workload Test"
    }
    
    "full" {
        Check-Prerequisites
        Print-Header "Complete Load Test Suite"
        Write-Host ""
        Print-Warning "This will run all load tests and may take 30+ minutes"
        Write-Host ""
        
        $response = Read-Host "Continue? [y/N]"
        if ($response -match '^[Yy]$') {
            Run-LoadTest -TestPattern "" -Description "Complete Load Test Suite"
        } else {
            Write-Host "Aborted"
            exit 0
        }
    }
    
    "custom" {
        Check-Prerequisites
        if (-not $TestPattern) {
            Print-Error "Please specify a test pattern"
            Write-Host "Usage: .\scripts\load_test.ps1 custom <test_pattern>"
            Write-Host "Example: .\scripts\load_test.ps1 custom `"load_test_concurrent_registration_light`""
            exit 1
        }
        Run-LoadTest -TestPattern $TestPattern -Description "Custom Test: $TestPattern"
    }
    
    "help" {
        Show-Help
    }
    
    default {
        Print-Error "Unknown command: $Command"
        Write-Host ""
        Show-Help
        exit 1
    }
}

Write-Host ""
Print-Success "Load test execution completed successfully!"
Write-Host ""
Write-Host "Next steps:"
Write-Host "  - Review test output for performance metrics"
Write-Host "  - Check for any threshold violations"
Write-Host "  - Compare results with baseline metrics"
Write-Host "  - See docs\LOAD_TEST_MONITORING.md for analysis guidance"
Write-Host ""
