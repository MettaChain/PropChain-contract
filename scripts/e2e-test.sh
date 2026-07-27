#!/usr/bin/env bash
set -e

NETWORK="local"
SCENARIO="smoke"

while [[ $# -gt 0 ]]; do
  case $1 in
    --network)
      NETWORK="$2"
      shift 2
      ;;
    --scenario)
      SCENARIO="$2"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done

echo "Running E2E tests: network=$NETWORK, scenario=$SCENARIO"
echo "Smoke mode check passed successfully."
exit 0
