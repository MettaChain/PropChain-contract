generate_playground_scenario() {
    section "Compiling Playground Scenario Metadata File"

    # Nothing was logged this session (no contract calls made), so there is nothing to export.
    if [ ! -s "$INTERACTION_LOG_TMP" ]; then
        log_warning "No runtime interaction traces were captured during this session. Skipping scenario export."
        return 0
    fi

    log_info "Extracting and structuring execution matrix profiles..."

    local raw_calls
    local raw_events

    # Pull out the "--- Call Step: ... ---" marker lines written by run_call and strip the markers.
    # "|| echo ''" stops set -e/pipefail from killing the script when grep finds no matches.
    raw_calls=$(grep -E "^--- Call Step:" "$INTERACTION_LOG_TMP" | sed 's/--- Call Step: //g' | sed 's/ ---//g' || echo "")

    # Pull out the contract "Event" lines from the cargo-contract output and trim leading spaces.
    raw_events=$(grep -E "^\s*Event\s" "$INTERACTION_LOG_TMP" | sed -e 's/^[[:space:]]*//' || echo "")

    # Build the JSON with jq. --arg passes shell values in safely (jq handles quoting/escaping).
    # NOTE: the signing key (SURI) is deliberately NOT written to the file, since the output
    # is committed to docs/. It is always masked.
    local json_payload
    json_payload=$(jq -n \
        --arg network "$NETWORK" \
        --arg ts "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
        --arg calls_str "$raw_calls" \
        --arg events_str "$raw_events" \
        '{
            "meta": {
                "generator": "scripts/playground.sh",
                "compiledAt": $ts,
                "compatibilityMode": "IDE-Playground-Engine/v2"
            },
            "environment": {
                "targetNetwork": $network,
                "signerIdentity": "MASKED_SESSION_KEY"
            },
            "traceData": {
                # Split the multi-line strings into arrays, dropping empty lines.
                "executedCalls": ($calls_str  | split("\n") | map(select(length > 0))),
                "emittedEvents": ($events_str | split("\n") | map(select(length > 0)))
            }
        }')

    # Pretty-print the JSON into the scenario file that docs/playground.html loads.
    echo "$json_payload" | jq '.' > "$OUTPUT_SCENARIO"
    log_success "Scenario file emitted successfully -> $OUTPUT_SCENARIO"
    log_info "Compatible for playback rendering inside docs/playground.html."
}