#!/usr/bin/env bash
set -euo pipefail
test "${SYNERGY_ARCHIVE_CHAIN_ID:-1264}" = "1264"
test "${SYNERGY_ARCHIVE_NETWORK_ID:-synergy-testnet-v2}" = "synergy-testnet-v2"
./scripts/verify-aegis-pqvm.sh
echo "Archive healthcheck passed"
