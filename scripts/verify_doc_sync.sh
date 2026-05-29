#!/usr/bin/env bash

# CI Documentation Synchronization Checker
# Ensures that code snippets and signatures in docs/ match the actual implementation in contracts/

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}🔍 PropChain Documentation Synchronization Checker${NC}"
echo "=================================================="

FAILED=0

# Function to normalize code for comparison
# Removes comments, derives, attributes, and extra whitespace
normalize_code() {
    echo "$1" | \
        sed 's/\/\/.*//g' | \
        sed 's/\/\*.*\*\///g' | \
        sed 's/#!\[.*\]//g' | \
        sed 's/#\[.*\]//g' | \
        sed 's/&self,//g' | \
        sed 's/&mut self,//g' | \
        sed 's/&self//g' | \
        sed 's/&mut self//g' | \
        sed 's/pub //g' | \
        sed 's/fn //g' | \
        sed 's/Self:://g' | \
        sed 's/,[ ]*)/)/g' | \
        sed 's/,)/)/g' | \
        tr -d '\r\n' | \
        sed 's/  */ /g' | \
        sed 's/^ *//;s/ *$//' | \
        sed 's/{ /{/g;s/ }/}/g;s/( /(/g;s/ )/)/g;s/, /,/g;s/: /:/g;s/; /;/g;s/ -> /->/g;s/ ->/->/g;s/-> /->/g'
}

# Function to check a struct or enum definition
check_definition() {
    local name=$1
    local expected_content=$2
    local file_path=$3
    
    echo -n "Checking $name in $file_path... "
    
    # Normalize expected content
    local norm_expected=$(normalize_code "$expected_content")
    
    # Find the definition in the contracts directory
    local actual_file=$(grep -rl "pub struct $name" contracts/ --include="*.rs" | head -1)
    if [ -z "$actual_file" ]; then
        actual_file=$(grep -rl "pub enum $name" contracts/ --include="*.rs" | head -1)
    fi
    
    if [ -z "$actual_file" ]; then
        echo -e "${RED}FAILED${NC} (Definition not found in code)"
        FAILED=1
        return 1
    fi
    
    # Extract the definition block from the file
    local actual_content=$(awk "/pub (struct|enum) $name/ {found=1; print; if (/{/) {count++}} found && /{/ { if (!match(\$0, /pub (struct|enum) $name/)) {print; count++}} found && /}/ {count--; if (count == 0) {found=0; exit}}" "$actual_file")
    
    if [ -z "$actual_content" ]; then
        actual_content=$(grep -o "pub (struct|enum) $name[^;]*;" "$actual_file" | head -1 || true)
        if [ -z "$actual_content" ]; then
             echo -e "${RED}FAILED${NC} (Could not extract block from $actual_file)"
             FAILED=1
             return 1
        fi
    fi
    
    local norm_actual=$(normalize_code "$actual_content")
    
    # Allow partial match for structs (doc might have fewer fields)
    if [[ "$norm_actual" == *"$norm_expected"* ]]; then
        echo -e "${GREEN}PASSED${NC}"
    else
        echo -e "${RED}FAILED${NC}"
        echo -e "  ${YELLOW}Expected:${NC} $norm_expected"
        echo -e "  ${YELLOW}Actual:${NC}   $norm_actual"
        echo -e "  ${YELLOW}File:${NC}     $actual_file"
        FAILED=1
    fi
}

# Function to check a method signature
check_signature() {
    local signature=$1
    local file_path=$2
    
    echo -n "Checking signature \"$signature\"... "
    
    local method_name=$(echo "$signature" | cut -d'(' -f1 | sed 's/.* //')
    
    # Skip generic 'new' methods as they exist in every contract and confuse the global grep
    if [ "$method_name" == "new" ]; then
        echo -e "${GREEN}PASSED${NC} (Skipped generic method)"
        return 0
    fi
    
    local norm_expected=$(normalize_code "$signature")
    local actual_file=$(grep -rl "pub fn $method_name(" contracts/ --include="*.rs" | head -1)
    
    if [ -z "$actual_file" ]; then
        echo -e "${RED}FAILED${NC} (Method not found in code)"
        FAILED=1
        return 1
    fi
    
    local actual_sig=$(awk "/pub fn $method_name\(/,/[{]/ {print}" "$actual_file" | tr -d '\r\n' | sed 's/{.*//')
    local norm_actual=$(normalize_code "$actual_sig")
    
    if [[ "$norm_actual" == *"$norm_expected"* ]]; then
        echo -e "${GREEN}PASSED${NC}"
    else
        echo -e "${RED}FAILED${NC}"
        echo -e "  ${YELLOW}Expected:${NC} $norm_expected"
        echo -e "  ${YELLOW}Actual:${NC}   $norm_actual"
        echo -e "  ${YELLOW}File:${NC}     $actual_file"
        FAILED=1
    fi
}

# Main process

# 1. Process SYSTEM_ARCHITECTURE_OVERVIEW.md for structs
echo -e "\n${BLUE}--- Checking SYSTEM_ARCHITECTURE_OVERVIEW.md ---${NC}"
DOC_FILE="docs/SYSTEM_ARCHITECTURE_OVERVIEW.md"
if [ -f "$DOC_FILE" ]; then
    echo "Processing $DOC_FILE..."
    TEMP_BLOCKS=$(mktemp)
    awk '/```rust/,/```/ {if ($0 !~ /```/) print}' "$DOC_FILE" | \
    awk '/pub struct [^ {]+/ {name=$3; content=$0; found=1; next} found {content=content "\n" $0; if ($0 ~ /}/) {print name; print "---"; print content; print "==="; found=0}}' > "$TEMP_BLOCKS"
    
    while IFS= read -r name; do
        [ -z "$name" ] && continue
        IFS= read -r separator
        content=""
        while IFS= read -r line && [ "$line" != "===" ]; do
            content="$content$line\n"
        done
        echo "Found struct: $name"
        content_clean=$(echo -e "$content" | sed 's/===//')
        check_definition "$name" "$content_clean" "$DOC_FILE" || true
    done < "$TEMP_BLOCKS"
    rm "$TEMP_BLOCKS"
else
    echo "File $DOC_FILE not found"
fi

# 2. Process contracts.md for signatures and structs
echo -e "\n${BLUE}--- Checking contracts.md ---${NC}"
DOC_FILE="docs/contracts.md"
if [ -f "$DOC_FILE" ]; then
    echo "Processing $DOC_FILE..."
    TEMP_SIGS=$(mktemp)
    grep "^##### \`" "$DOC_FILE" | sed 's/##### `//;s/`//' > "$TEMP_SIGS"
    while IFS= read -r sig; do
        [ -z "$sig" ] && continue
        echo "Found signature: $sig"
        check_signature "$sig" "$DOC_FILE" || true
    done < "$TEMP_SIGS"
    rm "$TEMP_SIGS"
    
    TEMP_BLOCKS=$(mktemp)
    awk '/```rust/,/```/ {if ($0 !~ /```/) print}' "$DOC_FILE" | \
    awk '/pub (struct|enum) [^ {]+/ {name=$3; content=$0; found=1; next} found {content=content "\n" $0; if ($0 ~ /}/) {print name; print "---"; print content; print "==="; found=0}}' > "$TEMP_BLOCKS"
    
    while IFS= read -r name; do
        [ -z "$name" ] && continue
        IFS= read -r separator
        content=""
        while IFS= read -r line && [ "$line" != "===" ]; do
            content="$content$line\n"
        done
        echo "Found block: $name"
        content_clean=$(echo -e "$content" | sed 's/===//')
        check_definition "$name" "$content_clean" "$DOC_FILE" || true
    done < "$TEMP_BLOCKS"
    rm "$TEMP_BLOCKS"
else
    echo "File $DOC_FILE not found"
fi

echo -e "\n=================================================="
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ All documentation is in sync!${NC}"
    exit 0
else
    echo -e "${RED}❌ Documentation synchronization errors found.${NC}"
    exit 1
fi
