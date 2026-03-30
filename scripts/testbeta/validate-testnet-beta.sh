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
    "node_listener_base": 5622,
    "rpc_base": 5640,
    "ws_base": 5660,
    "discovery_base": 5680,
    "metrics_base": 6030,
}

required_contracts = {
    "validator_registry",
    "synergy_oracle",
    "staking",
    "governance",
    "treasury",
    "reward_distributor",
    "slashing",
    "identity",
}

zero_hash = "0" * 64

def has_placeholder(value):
    if isinstance(value, str):
        return "<" in value and ">" in value
    if isinstance(value, list):
        return any(has_placeholder(item) for item in value)
    if isinstance(value, dict):
        return any(has_placeholder(item) for item in value.values())
    return False

def is_hex_hash(value):
    return isinstance(value, str) and len(value) == 64 and all(ch in "0123456789abcdef" for ch in value.lower())

if genesis.get("schema_version") != "v1":
    errors.append("genesis schema_version must be v1")
if genesis.get("env") != "testbeta":
    errors.append("genesis env must be testbeta")
if genesis.get("network", {}).get("chain_id") != 338639:
    errors.append("genesis network.chain_id must be 338639")
if genesis.get("network", {}).get("network_id") != 338639:
    errors.append("genesis network.network_id must be 338639")
if genesis.get("header", {}).get("block_height") != 0:
    errors.append("genesis header.block_height must be 0")
if genesis.get("header", {}).get("parent_hash") != zero_hash:
    errors.append("genesis header.parent_hash must be the zero hash")
if not isinstance(genesis.get("header", {}).get("timestamp"), int):
    errors.append("genesis header.timestamp must be an integer unix timestamp")
if genesis.get("token", {}).get("symbol") != "SNRG":
    errors.append("genesis token.symbol must be SNRG")
if genesis.get("token", {}).get("minting_policy") != "fixed_cap":
    errors.append("genesis token.minting_policy must be fixed_cap")
if genesis.get("consensus", {}).get("min_validator_count") != 4:
    errors.append("genesis consensus.min_validator_count must be 4")
if genesis.get("consensus", {}).get("min_quorum_threshold") != 3:
    errors.append("genesis consensus.min_quorum_threshold must be 3")
if len(genesis.get("validators", [])) != 4:
    errors.append("genesis must contain exactly 4 validators")
if not isinstance(genesis.get("contracts"), dict):
    errors.append("genesis contracts must be an object")
else:
    contract_keys = set(genesis["contracts"].keys())
    missing_contracts = sorted(required_contracts - contract_keys)
    if missing_contracts:
        errors.append(f"genesis contracts missing required entries: {', '.join(missing_contracts)}")

if has_placeholder(genesis):
    errors.append("genesis must not contain any <PLACEHOLDER> values")

balances_total = 0
for entry in genesis.get("balances", []):
    try:
        balances_total += int(entry.get("balance_nwei", "0"))
    except (TypeError, ValueError):
        errors.append(f"invalid balance_nwei for {entry.get('address')}")

allocations_total = 0
for entry in genesis.get("allocations", []):
    try:
        allocations_total += int(entry.get("amount_nwei", "0"))
    except (TypeError, ValueError):
        errors.append(f"invalid amount_nwei for allocation {entry.get('name')}")

try:
    total_supply_cap = int(genesis.get("token", {}).get("total_supply_cap_nwei", "0"))
except (TypeError, ValueError):
    total_supply_cap = -1
    errors.append("genesis token.total_supply_cap_nwei must be a decimal string")

if balances_total != total_supply_cap:
    errors.append("genesis balances must sum to token.total_supply_cap_nwei")
if allocations_total != total_supply_cap:
    errors.append("genesis allocations must sum to token.total_supply_cap_nwei")

integrity = genesis.get("integrity", {})
for field in ("genesis_hash", "state_root", "allocation_hash", "validator_hash", "contract_hash"):
    if not is_hex_hash(integrity.get(field)):
        errors.append(f"genesis integrity.{field} must be a 64-character lowercase hex hash")

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
  if rg -q '38638|48638|58638|18080|5730|5830|5930' "$node_config"; then
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
  if rg -q '38638|48638|58638|18080|5730|5830|5930' "$seed_config"; then
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
  if rg -q '38638|48638|58638|18080|5730|5830|5930|synergy-testbeta-closed' "$doc"; then
    echo "Support document contains stale beta data: $doc" >&2
    failures=$((failures + 1))
  fi
done

if (( failures > 0 )); then
  echo "Testnet-Beta validation failed (${failures} issue(s))." >&2
  exit 2
fi

echo "Testnet-Beta validation passed."
