#!/usr/bin/env bash

# PropChain Contract Interaction Playground
#
# Interactive CLI for exercising the most common contract calls against a
# deployed PropChain stack (property registration, escrow, staking,
# governance, insurance) without having to remember every message
# signature and cargo-contract invocation by hand.
#
# Resolves contract addresses from the deployment records written by
# scripts/deploy.sh (deployments/<network>/<contract-dir>.json), so run
# deploy.sh first (or pass an address manually when prompted).
#
# Addresses issue #517.

set -euo pipefail

# ---------------------------------------------------------------------------
# Output helpers (kept consistent with scripts/deploy.sh)
# ---------------------------------------------------------------------------
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# NOTE: these write to stderr, not stdout. Several of these helpers are
# called from inside functions whose actual return value is captured via
# `var=$(some_function ...)` (e.g. get_contract_address, prompt_menu_choice).
# Anything written to stdout inside such a function gets silently appended
# to the captured value, so all human-facing/status output below is routed
# to stderr to keep stdout clean for "real" return values only.
log_info()    { echo -e "${BLUE}[INFO]${NC} $1" >&2; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1" >&2; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $1" >&2; }
log_error()   { echo -e "${RED}[ERROR]${NC} $1" >&2; }
section()     { echo -e "\n${BOLD}== $1 ==${NC}" >&2; }

command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# ---------------------------------------------------------------------------
# Configuration (mirrors scripts/deploy.sh so addresses resolve correctly)
# ---------------------------------------------------------------------------
WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NETWORK="${NETWORK:-local}"

declare -A NETWORKS=(
    ["local"]="ws://localhost:9944"
    ["westend"]="wss://westend-rpc.polkadot.io"
    ["rococo"]="wss://rococo-rpc.polkadot.io"
    ["polkadot"]="wss://rpc.polkadot.io"
)

declare -A DEFAULT_ACCOUNTS=(
    ["local"]="//Alice"
    ["westend"]=""
    ["rococo"]=""
    ["polkadot"]=""
)

SURI="${SURI:-${DEFAULT_ACCOUNTS[$NETWORK]:-}}"
DEPLOYMENTS_DIR="$WORKSPACE_ROOT/deployments/$NETWORK"

# Menu option -> contract directory under contracts/
declare -A CONTRACT_DIR=(
    [1]="property-token"
    [2]="escrow"
    [3]="staking"
    [4]="governance"
    [5]="insurance"
)

# ---------------------------------------------------------------------------
# Usage
# ---------------------------------------------------------------------------
show_help() {
    cat << 'EOF'
PropChain Contract Interaction Playground

Usage:
  ./scripts/playground.sh [--help]

Description:
  Interactive menu for trying out the core PropChain contract calls
  against a running node, without hand-writing cargo-contract commands.

  Menu options:
    1) Register Property        (property-token :: register_property_with_token)
    2) Create Escrow             (escrow :: create_escrow_advanced)
    3) Stake Tokens              (staking :: stake)
    4) Vote on Proposal          (governance :: vote)
    5) Create Insurance Policy   (insurance :: create_policy)
    6) Show resolved contract addresses
    0) Exit

Environment variables:
  NETWORK   Target network: local | westend | rococo | polkadot (default: local)
  SURI      Signing key URI, e.g. //Alice (default: //Alice on local, required elsewhere)

Prerequisites:
  - cargo-contract:  cargo install cargo-contract --locked
  - jq
  - Contracts deployed via ./scripts/deploy.sh for the target network, so that
    deployments/<network>/<contract-dir>.json exists and contains an address.
    If a deployment file is missing, the script will offer to let you type
    in an address manually instead.

Examples:
  ./scripts/playground.sh
  NETWORK=westend SURI="$(cat ~/.suri)" ./scripts/playground.sh
EOF
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
    show_help
    exit 0
fi

# ---------------------------------------------------------------------------
# Prerequisites
# ---------------------------------------------------------------------------
check_prerequisites() {
    local missing=0

    if ! command_exists cargo-contract; then
        log_error "cargo-contract not found. Install it with: cargo install cargo-contract --locked"
        missing=1
    fi

    if ! command_exists jq; then
        log_error "jq not found. It's required to read deployment files (apt install jq / brew install jq)."
        missing=1
    fi

    if [[ -z "${NETWORKS[$NETWORK]:-}" ]]; then
        log_error "Unknown network: $NETWORK (expected one of: ${!NETWORKS[*]})"
        missing=1
    fi

    if [[ "$NETWORK" != "local" && -z "$SURI" ]]; then
        log_error "SURI is not set for network '$NETWORK'. Export it with: export SURI='your mnemonic or //Account'"
        missing=1
    fi

    if [[ "$missing" -ne 0 ]]; then
        exit 1
    fi
}

# ---------------------------------------------------------------------------
# Input helpers
# ---------------------------------------------------------------------------
prompt_required() {
    local prompt_text="$1"
    local value=""
    while [[ -z "$value" ]]; do
        read -r -p "$prompt_text: " value
        if [[ -z "$value" ]]; then
            log_warning "This field is required."
        fi
    done
    echo "$value"
}

prompt_optional() {
    local prompt_text="$1"
    local value=""
    read -r -p "$prompt_text (press Enter to skip): " value
    echo "$value"
}

prompt_yes_no() {
    local prompt_text="$1"
    local value=""
    while true; do
        read -r -p "$prompt_text [y/n]: " value
        case "$value" in
            y|Y|yes|YES) echo "true"; return ;;
            n|N|no|NO) echo "false"; return ;;
            *) log_warning "Please answer y or n." ;;
        esac
    done
}

prompt_menu_choice() {
    local prompt_text="$1"
    shift
    local options=("$@")
    local i=1
    for opt in "${options[@]}"; do
        echo "    $i) $opt" >&2
        i=$((i + 1))
    done
    local choice=""
    while true; do
        read -r -p "$prompt_text: " choice
        if [[ "$choice" =~ ^[0-9]+$ ]] && (( choice >= 1 && choice <= ${#options[@]} )); then
            echo "${options[$((choice - 1))]}"
            return
        fi
        log_warning "Enter a number between 1 and ${#options[@]}."
    done
}

# Converts a 2-letter country code (e.g. "US") into a SCALE byte array
# literal (e.g. "[85,83]") for the Jurisdiction.country_code: [u8; 2] field.
country_code_to_bytes() {
    local code="$1"
    if [[ ${#code} -ne 2 ]]; then
        log_warning "Country code should be 2 letters (e.g. US, NG); padding/truncating."
    fi
    local b1 b2
    b1=$(printf '%d' "'${code:0:1}")
    b2=$(printf '%d' "'${code:1:1}")
    echo "[$b1,$b2]"
}

# ---------------------------------------------------------------------------
# Deployment / address resolution
# ---------------------------------------------------------------------------
get_contract_address() {
    local contract_dir="$1"
    local deployment_file="$DEPLOYMENTS_DIR/$contract_dir.json"

    if [[ -f "$deployment_file" ]]; then
        local address
        address=$(jq -r '.address' "$deployment_file")
        if [[ -n "$address" && "$address" != "null" ]]; then
            log_info "Using deployed $contract_dir address from $deployment_file"
            echo "$address"
            return
        fi
    fi

    log_warning "No deployment record found at $deployment_file"
    log_warning "Run './scripts/deploy.sh --network $NETWORK --contract $contract_dir' first, or enter an address now."
    prompt_required "Enter the $contract_dir contract address"
}

show_addresses() {
    section "Resolved contract addresses ($NETWORK)"
    for dir in "${CONTRACT_DIR[@]}"; do
        local f="$DEPLOYMENTS_DIR/$dir.json"
        if [[ -f "$f" ]]; then
            echo "  $dir: $(jq -r '.address' "$f")"
        else
            echo "  $dir: (not deployed on $NETWORK)"
        fi
    done
}

# ---------------------------------------------------------------------------
# Contract call runner
# ---------------------------------------------------------------------------
# run_call <contract_dir> <message> <args...>
run_call() {
    local contract_dir="$1"
    local message="$2"
    shift 2
    local args=("$@")

    local address
    address=$(get_contract_address "$contract_dir")

    log_info "Calling $message on $contract_dir ($address)..."

    local output
    set +e
    output=$(
        cd "$WORKSPACE_ROOT/contracts/$contract_dir" && \
        cargo contract call \
            --contract "$address" \
            --message "$message" \
            "${args[@]}" \
            --url "${NETWORKS[$NETWORK]}" \
            --suri "$SURI" \
            --execute \
            --skip-confirm 2>&1
    )
    local status=$?
    set -e

    echo "$output"

    if [[ $status -ne 0 ]]; then
        log_error "Call to $message failed (exit code $status). See output above for details."
        return 1
    fi

    section "Emitted events"
    if echo "$output" | grep -qi "^Event"; then
        echo "$output" | awk '/^Event/{flag=1} flag'
    else
        log_info "No 'Event' lines found in output — the call may not have emitted any events, or the node returned a different format."
    fi

    log_success "$message executed."
}

# ---------------------------------------------------------------------------
# Menu actions
# ---------------------------------------------------------------------------
action_register_property() {
    section "Register Property — property-token :: register_property_with_token"

    local location size legal_description valuation documents_url
    location=$(prompt_required "Property location (e.g. '123 Main St, Lagos')")
    size=$(prompt_required "Size in square units (u64, e.g. 2000)")
    legal_description=$(prompt_required "Legal description (e.g. 'Lot 1 Block 2')")
    valuation=$(prompt_required "Valuation in smallest token unit (u128, e.g. 500000000000)")
    documents_url=$(prompt_required "Documents URL (e.g. IPFS link)")

    local metadata_arg
    metadata_arg=$(printf '{"location":"%s","size":%s,"legal_description":"%s","valuation":%s,"documents_url":"%s"}' \
        "$location" "$size" "$legal_description" "$valuation" "$documents_url")

    run_call "property-token" "register_property_with_token" --args "$metadata_arg"
}

action_create_escrow() {
    section "Create Escrow — escrow :: create_escrow_advanced"

    local property_id amount buyer seller participants_raw required_signatures
    local release_time_lock_raw jurisdiction_code country_code region_code locality_code

    property_id=$(prompt_required "Property ID (u64)")
    amount=$(prompt_required "Escrow amount in smallest token unit (u128)")
    buyer=$(prompt_required "Buyer AccountId (e.g. 5GrwvaEF... or //Bob for dev accounts)")
    seller=$(prompt_required "Seller AccountId")
    participants_raw=$(prompt_required "Participant AccountIds, comma-separated (must include buyer & seller)")
    required_signatures=$(prompt_required "Required signatures (u8, <= number of participants)")
    release_time_lock_raw=$(prompt_optional "Release time lock as a unix timestamp (u64)")
    jurisdiction_code=$(prompt_required "Jurisdiction code (u32, e.g. 1)")
    country_code=$(prompt_required "Jurisdiction country code, 2 letters (e.g. NG, US)")
    region_code=$(prompt_required "Jurisdiction region code (u16, e.g. 0)")
    locality_code=$(prompt_required "Jurisdiction locality code (u16, e.g. 0)")

    local participants_arg
    participants_arg="[$(echo "$participants_raw" | tr -d ' ')]"

    local release_time_lock_arg
    if [[ -z "$release_time_lock_raw" ]]; then
        release_time_lock_arg="None"
    else
        release_time_lock_arg="Some($release_time_lock_raw)"
    fi

    local country_bytes
    country_bytes=$(country_code_to_bytes "$country_code")

    local jurisdiction_arg
    jurisdiction_arg=$(printf '{"code":%s,"country_code":%s,"region_code":%s,"locality_code":%s}' \
        "$jurisdiction_code" "$country_bytes" "$region_code" "$locality_code")

    run_call "escrow" "create_escrow_advanced" \
        --args "$property_id" "$amount" "$buyer" "$seller" "$participants_arg" \
               "$required_signatures" "$release_time_lock_arg" "$jurisdiction_arg"
}

action_stake_tokens() {
    section "Stake Tokens — staking :: stake"

    local amount lock_choice lock_period_arg

    amount=$(prompt_required "Amount to stake in smallest token unit (u128)")
    lock_choice=$(prompt_menu_choice "Lock period" "Flexible" "ThirtyDays" "NinetyDays" "OneYear" "Custom (enter blocks)")

    if [[ "$lock_choice" == "Custom (enter blocks)" ]]; then
        local custom_blocks
        custom_blocks=$(prompt_required "Custom lock duration in blocks (u64)")
        lock_period_arg="Custom($custom_blocks)"
    else
        lock_period_arg="$lock_choice"
    fi

    run_call "staking" "stake" --args "$amount" "$lock_period_arg"
}

action_vote_on_proposal() {
    section "Vote on Proposal — governance :: vote"

    local proposal_id support
    proposal_id=$(prompt_required "Proposal ID (u64)")
    support=$(prompt_yes_no "Vote in support of this proposal?")

    run_call "governance" "vote" --args "$proposal_id" "$support"
}

action_create_insurance_policy() {
    section "Create Insurance Policy — insurance :: create_policy"

    local property_id coverage_type coverage_amount pool_id duration_seconds metadata_url

    property_id=$(prompt_required "Property ID (u64)")
    coverage_type=$(prompt_menu_choice "Coverage type" \
        "Fire" "Flood" "Earthquake" "Theft" "LiabilityDamage" "NaturalDisaster" "Comprehensive")
    coverage_amount=$(prompt_required "Coverage amount in smallest token unit (u128)")
    pool_id=$(prompt_required "Risk pool ID (u64)")
    duration_seconds=$(prompt_required "Policy duration in seconds (u64, e.g. 31536000 for 1 year)")
    metadata_url=$(prompt_required "Policy metadata URL (e.g. IPFS link)")

    run_call "insurance" "create_policy" \
        --args "$property_id" "$coverage_type" "$coverage_amount" "$pool_id" "$duration_seconds" "$metadata_url"
}

# ---------------------------------------------------------------------------
# Main menu
# ---------------------------------------------------------------------------
main() {
    check_prerequisites

    echo -e "${BOLD}PropChain Contract Playground${NC}  (network: $NETWORK)"
    echo "Run with --help for usage details."

    while true; do
        echo
        echo "  1) Register Property"
        echo "  2) Create Escrow"
        echo "  3) Stake Tokens"
        echo "  4) Vote on Proposal"
        echo "  5) Create Insurance Policy"
        echo "  6) Show resolved contract addresses"
        echo "  0) Exit"
        echo
        local choice
        read -r -p "Select an option: " choice

        case "$choice" in
            1) action_register_property ;;
            2) action_create_escrow ;;
            3) action_stake_tokens ;;
            4) action_vote_on_proposal ;;
            5) action_create_insurance_policy ;;
            6) show_addresses ;;
            0) log_info "Bye!"; exit 0 ;;
            *) log_warning "Unknown option: $choice" ;;
        esac
    done
}

main "$@"
