#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GENESIS_FILE="${TESTBETA_GENESIS_FILE:-$ROOT_DIR/config/genesis.json}"
MANIFEST_FILE="${TESTBETA_MANIFEST_FILE:-$ROOT_DIR/config/operational-manifest.json}"
BUNDLE_DIR="${TESTBETA_BUNDLE_DIR:-$ROOT_DIR/bootstrap-bundles}"

for required in "$GENESIS_FILE" "$MANIFEST_FILE" "$BUNDLE_DIR"; do
  if [[ ! -e "$required" ]]; then
    echo "Missing required beta launch asset: $required" >&2
    exit 1
  fi
done

python3 - "$GENESIS_FILE" "$MANIFEST_FILE" <<'PY'
import json
import sys

genesis_path, manifest_path = sys.argv[1:3]
errors = []

with open(genesis_path, encoding="utf-8") as handle:
    genesis = json.load(handle)
with open(manifest_path, encoding="utf-8") as handle:
    manifest = json.load(handle)

expected_ports = {
    "bootnode": 5620,
    "seed": 5621,
    "reserved": 5622,
    "p2p_base": 5630,
    "rpc_base": 5730,
    "ws_base": 5830,
    "discovery_base": 5930,
    "metrics_base": 6030,
}

if genesis["metadata"].get("network_id") != "synergy-testnet-beta":
    errors.append("genesis metadata.network_id must be synergy-testnet-beta")
if str(genesis["metadata"].get("chain_id")) != "338639":
    errors.append("genesis metadata.chain_id must be 338639")
if genesis["network"].get("chain_id") != 338639:
    errors.append("genesis network.chain_id must be 338639")
if genesis["supply"].get("token_symbol") != "SNRG":
    errors.append("genesis supply.token_symbol must be SNRG")
if genesis["consensus"]["parameters"].get("min_validators") != 4:
    errors.append("genesis consensus min_validators must be 4")
if genesis["consensus"]["parameters"].get("max_validators") != 4:
    errors.append("genesis consensus max_validators must be 4")
if genesis["consensus"]["parameters"].get("dynamic_validator_registration") is not False:
    errors.append("genesis dynamic_validator_registration must be false")
if len(genesis.get("validators", [])) != 4:
    errors.append("genesis must contain exactly 4 validators")
if len(genesis["network"].get("bootnodes", [])) != 3:
    errors.append("genesis network.bootnodes must contain exactly 3 entries")

treasury_allocations = [
    entry["address"]
    for entry in genesis.get("genesis_allocations", [])
    if entry.get("type") == "treasury"
]
treasury_address = treasury_allocations[0] if treasury_allocations else None
if genesis["governance"].get("treasury_address") != treasury_address:
    errors.append("genesis governance.treasury_address must match the treasury allocation address")

if manifest.get("network_id") != "synergy-testnet-beta":
    errors.append("operational manifest network_id must be synergy-testnet-beta")
if manifest.get("chain_id") != 338639:
    errors.append("operational manifest chain_id must be 338639")
if manifest.get("token", {}).get("symbol") != "SNRG":
    errors.append("operational manifest token.symbol must be SNRG")
if len(manifest.get("bootstrap", {}).get("bootnodes", [])) != 3:
    errors.append("operational manifest must contain exactly 3 bootnodes")
if len(manifest.get("bootstrap", {}).get("seed_servers", [])) != 3:
    errors.append("operational manifest must contain exactly 3 seed servers")
if len(manifest.get("validators", [])) != 4:
    errors.append("operational manifest must contain exactly 4 validators")
if manifest.get("ports") != expected_ports:
    errors.append("operational manifest ports must match the frozen beta port model")

public_endpoints = manifest.get("public_endpoints", {})
expected_urls = {
    "core_rpc": "https://testbeta-core-rpc.synergy-network.io",
    "core_ws": "wss://testbeta-core-ws.synergy-network.io",
    "api": "https://testbeta-api.synergy-network.io",
}
for key, expected in expected_urls.items():
    actual = public_endpoints.get(key, {}).get("url")
    if actual != expected:
        errors.append(f"operational manifest {key}.url must be {expected}")

compat_alias = public_endpoints.get("compatibility", {}).get("rpc_alias")
if compat_alias and compat_alias != "https://testbeta-rpc.synergy-network.io":
    errors.append("operational manifest compatibility.rpc_alias must be https://testbeta-rpc.synergy-network.io when present")

if errors:
    for error in errors:
        print(error)
    sys.exit(1)
PY

failures=0

for bundle in bootnode1 bootnode2 bootnode3; do
  node_config="$BUNDLE_DIR/$bundle/config/node.toml"
  if [[ ! -f "$node_config" ]]; then
    echo "Missing bootnode bundle config: $node_config" >&2
    failures=$((failures + 1))
    continue
  fi

  if ! rg -q '^p2p_port = 5620$' "$node_config"; then
    echo "[$bundle] p2p_port must be 5620" >&2
    failures=$((failures + 1))
  fi
  if ! rg -q '^validator_cluster_size = 4$' "$node_config"; then
    echo "[$bundle] validator_cluster_size must be 4" >&2
    failures=$((failures + 1))
  fi
  if ! rg -q '^max_validators = 4$' "$node_config"; then
    echo "[$bundle] max_validators must be 4" >&2
    failures=$((failures + 1))
  fi
  if rg -q '38638|48638|58638|18080' "$node_config"; then
    echo "[$bundle] contains stale closed-testnet ports" >&2
    failures=$((failures + 1))
  fi
done

for bundle in seed1 seed2 seed3; do
  seed_config="$BUNDLE_DIR/$bundle/config/seed-service.json"
  if [[ ! -f "$seed_config" ]]; then
    echo "Missing seed bundle config: $seed_config" >&2
    failures=$((failures + 1))
    continue
  fi

  if ! rg -q '"listen_port": 5621' "$seed_config"; then
    echo "[$bundle] listen_port must be 5621" >&2
    failures=$((failures + 1))
  fi
  if rg -q '38638|48638|58638|18080' "$seed_config"; then
    echo "[$bundle] contains stale closed-testnet ports" >&2
    failures=$((failures + 1))
  fi
done

if [[ -d "$BUNDLE_DIR/Bootstrap2" || -d "$BUNDLE_DIR/Bootstrap3" ]]; then
  echo "bootstrap-bundles must not retain stale Bootstrap2 or Bootstrap3 directories" >&2
  failures=$((failures + 1))
fi

for doc in "$BUNDLE_DIR/DEPLOYMENT_GUIDE.md" "$BUNDLE_DIR/DNS_RECORDS.txt" "$BUNDLE_DIR/README.txt"; do
  if [[ ! -f "$doc" ]]; then
    echo "Missing support document: $doc" >&2
    failures=$((failures + 1))
    continue
  fi
  if rg -q '38638|48638|58638|18080|synergy-testbeta-closed' "$doc"; then
    echo "Support document contains stale beta data: $doc" >&2
    failures=$((failures + 1))
  fi
done

if (( failures > 0 )); then
  echo "Testnet-Beta validation failed (${failures} issue(s))." >&2
  exit 2
fi

echo "Testnet-Beta validation passed."
