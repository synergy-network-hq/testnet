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
import blake3

genesis_path, manifest_path = sys.argv[1:3]
errors = []

with open(genesis_path, encoding="utf-8") as handle:
    genesis = json.load(handle)
with open(manifest_path, encoding="utf-8") as handle:
    manifest = json.load(handle)

expected_ports = {
    "bootnode": 5620,
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

def canonical_json(value):
    if value is None:
        return "null"
    if value is True:
        return "true"
    if value is False:
        return "false"
    if isinstance(value, (int, float)):
        return json.dumps(value, ensure_ascii=False, separators=(",", ":"))
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False, separators=(",", ":"))
    if isinstance(value, list):
        return "[" + ",".join(canonical_json(item) for item in value) + "]"
    if isinstance(value, dict):
        return "{" + ",".join(
            f'{json.dumps(key, ensure_ascii=False, separators=(",", ":"))}:{canonical_json(value[key])}'
            for key in sorted(value.keys())
        ) + "}"
    raise TypeError(f"unsupported value type: {type(value)!r}")

def hash_json(value):
    return blake3.blake3(canonical_json(value).encode("utf-8")).hexdigest()

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
if len(genesis.get("validators", [])) != 5:
    errors.append("genesis must contain exactly 5 validators")
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

expected_state_root = hash_json({
    "accounts": genesis.get("accounts"),
    "balances": genesis.get("balances"),
    "allocations": genesis.get("allocations"),
    "validators": genesis.get("validators"),
    "contracts": genesis.get("contracts"),
    "modules": genesis.get("modules"),
})
expected_data_root = hash_json({
    "contracts": genesis.get("contracts"),
    "modules": genesis.get("modules"),
    "precompiles": genesis.get("precompiles"),
})
expected_allocation_hash = hash_json(genesis.get("allocations"))
expected_validator_hash = hash_json(genesis.get("validators"))
expected_contract_hash = hash_json(genesis.get("contracts"))
genesis_for_hash = json.loads(json.dumps(genesis))
genesis_for_hash.setdefault("integrity", {})["genesis_hash"] = ""
expected_genesis_hash = hash_json(genesis_for_hash)

if genesis.get("header", {}).get("state_root") != expected_state_root:
    errors.append("genesis header.state_root does not match canonical genesis contents")
if genesis.get("header", {}).get("data_root") != expected_data_root:
    errors.append("genesis header.data_root does not match canonical genesis contents")
if integrity.get("state_root") != expected_state_root:
    errors.append("genesis integrity.state_root does not match canonical genesis contents")
if integrity.get("allocation_hash") != expected_allocation_hash:
    errors.append("genesis integrity.allocation_hash does not match canonical genesis contents")
if integrity.get("validator_hash") != expected_validator_hash:
    errors.append("genesis integrity.validator_hash does not match canonical genesis contents")
if integrity.get("contract_hash") != expected_contract_hash:
    errors.append("genesis integrity.contract_hash does not match canonical genesis contents")
if integrity.get("genesis_hash") != expected_genesis_hash:
    errors.append("genesis integrity.genesis_hash does not match canonical genesis contents")

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
if manifest.get("bootstrap", {}).get("routing", {}).get("bootnodes") != ["sentry1"]:
    errors.append("operational manifest bootnode routing must pin to sentry1")
if manifest.get("bootstrap", {}).get("routing", {}).get("seed_servers") != ["sentry2"]:
    errors.append("operational manifest seed server routing must pin to sentry2")
if len(manifest.get("validators", [])) != 5:
    errors.append("operational manifest must contain exactly 5 validators")
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
  bundle_genesis="$BUNDLE_DIR/$bundle/config/genesis.json"
  if [[ ! -f "$node_config" ]]; then
    echo "Missing bootnode bundle config: $node_config" >&2
    failures=$((failures + 1))
    continue
  fi
  if [[ ! -f "$bundle_genesis" ]]; then
    echo "Missing bootnode bundle genesis: $bundle_genesis" >&2
    failures=$((failures + 1))
  elif ! cmp -s "$GENESIS_FILE" "$bundle_genesis"; then
    echo "[$bundle] genesis.json does not match canonical config/genesis.json" >&2
    failures=$((failures + 1))
  fi

  if ! rg -q '^p2p_port = 5620$' "$node_config"; then
    echo "[$bundle] p2p_port must be 5620" >&2
    failures=$((failures + 1))
  fi
  if ! rg -q '^validator_cluster_size = 5$' "$node_config"; then
    echo "[$bundle] validator_cluster_size must be 5" >&2
    failures=$((failures + 1))
  fi
  if ! rg -q '^max_validators = 5$' "$node_config"; then
    echo "[$bundle] max_validators must be 5" >&2
    failures=$((failures + 1))
  fi
  if rg -q '38638|48638|58638|18080|5730|5830|5930' "$node_config"; then
    echo "[$bundle] contains stale closed-testnet ports" >&2
    failures=$((failures + 1))
  fi
done

for bundle in genesisrpc genesisindexer; do
  node_config="$BUNDLE_DIR/$bundle/config/node.toml"
  bundle_genesis="$BUNDLE_DIR/$bundle/config/genesis.json"
  if [[ ! -f "$node_config" ]]; then
    echo "Missing service bundle config: $node_config" >&2
    failures=$((failures + 1))
    continue
  fi

  if [[ ! -f "$bundle_genesis" ]]; then
    echo "Missing service bundle genesis: $bundle_genesis" >&2
    failures=$((failures + 1))
  elif ! cmp -s "$GENESIS_FILE" "$bundle_genesis"; then
    echo "[$bundle] genesis.json does not match canonical config/genesis.json" >&2
    failures=$((failures + 1))
  fi

  if ! rg -q '^seed_servers = \[\]$' "$node_config"; then
    echo "[$bundle] seed_servers must be empty" >&2
    failures=$((failures + 1))
  fi
  if rg -q '38638|48638|58638|18080|5730|5830|5930' "$node_config"; then
    echo "[$bundle] contains stale closed-testnet ports" >&2
    failures=$((failures + 1))
  fi
done

for stale_dir in seed1 seed2 seed3 bootseed2 rpc-gateway indexer-explorer; do
  if [[ -d "$BUNDLE_DIR/$stale_dir" ]]; then
    echo "bootstrap-bundles must not retain stale $stale_dir directory" >&2
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
