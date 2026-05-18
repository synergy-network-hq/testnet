#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

PATTERN='k256|secp256k1|ecdsa|Ed25519|SYNERGY_CONSENSUS_ALLOW_GENESIS_STATUS_BYPASS|allow_genesis_status_bypass[[:space:]]*=[[:space:]]*true|return true;[[:space:]]*//[[:space:]]*Placeholder|simulate VRF|placeholder QC|fallback_pub|skipping signature verification'

if rg -n "$PATTERN" src config scripts .github -g '!target' -g '!scripts/testnet/check-pqc-consensus-guard.sh'; then
  echo "PQC consensus guard failed: consensus-critical paths reference a prohibited classical, bypass, or placeholder crypto pattern." >&2
  exit 1
fi

echo "PQC consensus guard passed."
