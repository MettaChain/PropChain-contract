#!/usr/bin/env bash
set -euo pipefail

echo "Starting PropChain test environment..."
docker compose -f docker-compose.test.yml up -d

echo "Waiting for node to be ready..."
until curl -sf http://localhost:9933/health >/dev/null 2>&1; do
  printf '.'
  sleep 2
done
echo ""
echo "Node ready at ws://localhost:9944"
echo "IPFS API at http://localhost:5001"
