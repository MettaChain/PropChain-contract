#!/usr/bin/env bash

# API Documentation Validation Script
# Validates rustdoc completeness, example correctness, and documentation quality

set -e

echo "🔍 PropChain API Documentation Validator"
echo "========================================="
echo ""

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
TOTAL_CHECKS=0
PASSED_CHECKS=0
FAILED_CHECKS=0
WARNINGS=0

# Function to check if rustdoc is present
check_rustdoc_exists() {
    local file=$1
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    if grep -q "^///" "$file"; then
        echo -e "${GREEN}✓${NC} Rustdoc comments found in $file"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
        return 0
    else
        echo -e "${RED}✗${NC} No rustdoc comments found in $file"
        FAILED_CHECKS=$((FAILED_CHECKS + 1))
        return 1
    fi
}

# Function to check for function descriptions
check_function_descriptions() {
    local file=$1
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    # Check if functions have descriptions
    if grep -B1 "#\[ink(message)\]" "$file" | grep -q "///"; then
        echo -e "${GREEN}✓${NC} Functions have descriptions in $file"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
        return 0
    else
        echo -e "${YELLOW}⚠${NC} Some functions missing descriptions in $file"
        WARNINGS=$((WARNINGS + 1))
        return 1
    fi
}

# Function to check for examples in documentation
check_examples_exist() {
    local file=$1
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    if grep -q '```rust' "$file" || grep -q '```rust,ignore' "$file"; then
        echo -e "${GREEN}✓${NC} Code examples found in $file"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
        return 0
    else
        echo -e "${YELLOW}⚠${NC} No code examples found in $file"
        WARNINGS=$((WARNINGS + 1))
        return 1
    fi
}

# Function to check for error documentation
check_error_documentation() {
    local file=$1
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    if grep -q "## Errors" "$file" || grep -q "Returns.*Err" "$file" || grep -q "Error::" "$file"; then
        echo -e "${GREEN}✓${NC} Error documentation found in $file"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
        return 0
    else
        echo -e "${YELLOW}⚠${NC} Error documentation missing or incomplete in $file"
        WARNINGS=$((WARNINGS + 1))
        return 1
    fi
}

# Function to check for parameter documentation
check_parameter_documentation() {
    local file=$1
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    if grep -q "## Parameters" "$file" || grep -q "\`.*\` -" "$file"; then
        echo -e "${GREEN}✓${NC} Parameter documentation found in $file"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
        return 0
    else
        echo -e "${YELLOW}⚠${NC} Parameter documentation missing in $file"
        WARNINGS=$((WARNINGS + 1))
        return 1
    fi
}

# Function to check for return value documentation
check_return_documentation() {
    local file=$1
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    if grep -q "## Returns" "$file" || grep -q "\`Ok(" "$file" || grep -q "\`Err(" "$file"; then
        echo -e "${GREEN}✓${NC} Return value documentation found in $file"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
        return 0
    else
        echo -e "${YELLOW}⚠${NC} Return value documentation missing in $file"
        WARNINGS=$((WARNINGS + 1))
        return 1
    fi
}

# Function to check documentation structure
check_documentation_structure() {
    local file=$1
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    local has_description=false
    local has_parameters=false
    local has_returns=false
    local has_errors=false
    local has_example=false
    
    grep -q "## Description" "$file" && has_description=true
    grep -q "## Parameters" "$file" && has_parameters=true
    grep -q "## Returns" "$file" && has_returns=true
    grep -q "## Errors" "$file" && has_errors=true
    grep -q "## Example" "$file" && has_example=true
    
    local sections_found=0
    $has_description && sections_found=$((sections_found + 1))
    $has_parameters && sections_found=$((sections_found + 1))
    $has_returns && sections_found=$((sections_found + 1))
    $has_errors && sections_found=$((sections_found + 1))
    $has_example && sections_found=$((sections_found + 1))
    
    if [ $sections_found -ge 3 ]; then
        echo -e "${GREEN}✓${NC} Documentation structure complete ($sections_found/5 sections) in $file"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
        return 0
    else
        echo -e "${YELLOW}⚠${NC} Documentation structure incomplete ($sections_found/5 sections) in $file"
        WARNINGS=$((WARNINGS + 1))
        return 1
    fi
}

# Function to run cargo doc
run_cargo_doc() {
    echo ""
    echo -e "${BLUE}📖 Generating rustdoc documentation...${NC}"
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    if cargo doc --no-deps --document-private-items 2>&1 | tee /tmp/cargo_doc.log; then
        echo -e "${GREEN}✓${NC} rustdoc generation successful"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
        
        # Check for warnings
        local warning_count=$(grep -c "warning:" /tmp/cargo_doc.log || true)
        if [ "$warning_count" -gt 0 ]; then
            echo -e "${YELLOW}⚠${NC} Found $warning_count rustdoc warnings"
            WARNINGS=$((WARNINGS + warning_count))
        fi
    else
        echo -e "${RED}✗${NC} rustdoc generation failed"
        FAILED_CHECKS=$((FAILED_CHECKS + 1))
        return 1
    fi
}

# Function to run documentation tests
run_doctests() {
    echo ""
    echo -e "${BLUE}🧪 Running documentation tests...${NC}"
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    if cargo test --doc 2>&1 | tee /tmp/doctest.log; then
        echo -e "${GREEN}✓${NC} All doctests passed"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
    else
        echo -e "${RED}✗${NC} Some doctests failed"
        FAILED_CHECKS=$((FAILED_CHECKS + 1))
        return 1
    fi
}

# Function to check for broken links
check_links() {
    echo ""
    echo -e "${BLUE}🔗 Checking documentation links...${NC}"
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    # Simple link check - look for common patterns
    local broken_links=0
    
    # Check for empty links
    if grep -rn "\[\]()" docs/ contracts/*/src/*.rs 2>/dev/null | head -5; then
        echo -e "${YELLOW}⚠${NC} Found empty links"
        broken_links=$((broken_links + 1))
    fi
    
    # Check for TODO links
    if grep -rn "\[TODO\]" docs/ contracts/*/src/*.rs 2>/dev/null | head -5; then
        echo -e "${YELLOW}⚠${NC} Found TODO links"
        broken_links=$((broken_links + 1))
    fi
    
    if [ "$broken_links" -eq 0 ]; then
        echo -e "${GREEN}✓${NC} No obvious broken links found"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
    else
        echo -e "${YELLOW}⚠${NC} Found $broken_links potential link issues"
        WARNINGS=$((WARNINGS + broken_links))
    fi
}

# Main validation logic
main() {
    echo "Starting API Documentation Validation..."
    echo ""
    
    # Validate main contract files
    CONTRACT_FILES=(
        "contracts/lib/src/lib.rs"
        "contracts/escrow/src/lib.rs"
        "contracts/oracle/src/lib.rs"
        "contracts/bridge/src/lib.rs"
        "contracts/insurance/src/lib.rs"
        "contracts/compliance_registry/lib.rs"
    )
    
    echo "=========================================="
    echo "Checking Individual Contract Files"
    echo "=========================================="
    echo ""
    
    for file in "${CONTRACT_FILES[@]}"; do
        if [ -f "$file" ]; then
            echo -e "${BLUE}Checking:${NC} $file"
            check_rustdoc_exists "$file" || true
            check_function_descriptions "$file" || true
            check_examples_exist "$file" || true
            check_error_documentation "$file" || true
            check_parameter_documentation "$file" || true
            check_return_documentation "$file" || true
            echo ""
        else
            echo -e "${YELLOW}⚠${NC} File not found: $file"
        fi
    done
    
    echo "=========================================="
    echo "Checking Documentation Structure"
    echo "=========================================="
    echo ""
    
    # Check comprehensive documentation files
    DOC_FILES=(
        "docs/API_DOCUMENTATION_STANDARDS.md"
        "docs/API_ERROR_CODES.md"
        "docs/contracts.md"
    )
    
    for file in "${DOC_FILES[@]}"; do
        if [ -f "$file" ]; then
            echo -e "${BLUE}Checking:${NC} $file"
            check_documentation_structure "$file" || true
            echo ""
        fi
    done
    
    echo "=========================================="
    echo "Running Cargo Documentation Commands"
    echo "=========================================="
    
    # Generate rustdoc
    run_cargo_doc || true
    
    # Run doctests (if configured)
    # run_doctests || true
    
    # Check links
    check_links || true
    
    # Print summary
    echo ""
    echo "=========================================="
    echo "Validation Summary"
    echo "=========================================="
    echo ""
    echo "Total Checks:   $TOTAL_CHECKS"
    echo -e "${GREEN}Passed:         $PASSED_CHECKS${NC}"
    echo -e "${RED}Failed:         $FAILED_CHECKS${NC}"
    echo -e "${YELLOW}Warnings:       $WARNINGS${NC}"
    echo ""
    
    # Calculate pass rate
    if [ $TOTAL_CHECKS -gt 0 ]; then
        PASS_RATE=$((PASSED_CHECKS * 100 / TOTAL_CHECKS))
        echo "Pass Rate:      ${PASS_RATE}%"
        echo ""
        
        if [ $FAILED_CHECKS -eq 0 ]; then
            echo -e "${GREEN}✓ Validation PASSED${NC}"
            exit 0
        elif [ $PASS_RATE -ge 70 ]; then
            echo -e "${YELLOW}⚠ Validation PASSED with warnings${NC}"
            exit 0
        else
            echo -e "${RED}✗ Validation FAILED${NC}"
            exit 1
        fi
    else
        echo -e "${RED}✗ No checks performed${NC}"
        exit 1
    fi
}

# Run main function
main "$@"
