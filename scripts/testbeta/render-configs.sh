#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
INVENTORY_FILE="$ROOT_DIR/testbeta/runtime/node-inventory.csv"
HOSTS_FILE="${1:-$ROOT_DIR/testbeta/runtime/hosts.env}"
OUT_DIR="$ROOT_DIR/testbeta/runtime/configs"
NODE_ADDRESSES_FILE="$ROOT_DIR/testbeta/runtime/keys/node-addresses.csv"
MANIFEST_FILE="$ROOT_DIR/config/operational-manifest.json"
USE_HOST_OVERRIDES="false"
TESTBETA_CHAIN_ID="${TESTBETA_CHAIN_ID:-338639}"
TESTBETA_NETWORK_NAME="${TESTBETA_NETWORK_NAME:-synergy-testnet-beta}"
TESTBETA_BLOCK_TIME_SECS="${TESTBETA_BLOCK_TIME_SECS:-2}"
TESTBETA_EPOCH_LENGTH="${TESTBETA_EPOCH_LENGTH:-1000}"
TESTBETA_MIN_VALIDATORS="${TESTBETA_MIN_VALIDATORS:-2}"
TESTBETA_VALIDATOR_VOTE_THRESHOLD="${TESTBETA_VALIDATOR_VOTE_THRESHOLD:-2}"
TESTBETA_STATUS_READY_MIN_VALIDATORS="${TESTBETA_STATUS_READY_MIN_VALIDATORS:-2}"
TESTBETA_STATUS_READY_GENESIS_GRACE_SECS="${TESTBETA_STATUS_READY_GENESIS_GRACE_SECS:-60}"
TESTBETA_LEADER_TIMEOUT_SECS="${TESTBETA_LEADER_TIMEOUT_SECS:-120}"
TESTBETA_VOTE_TIMEOUT_SECS="${TESTBETA_VOTE_TIMEOUT_SECS:-12}"
TESTBETA_BLOCK_TIMEOUT_SECS="${TESTBETA_BLOCK_TIMEOUT_SECS:-30}"
TESTBETA_CONSENSUS_PENALIZATION_ENABLED="${TESTBETA_CONSENSUS_PENALIZATION_ENABLED:-false}"
TESTBETA_P2P_BOOTSTRAP_REFRESH_SECS="${TESTBETA_P2P_BOOTSTRAP_REFRESH_SECS:-60}"
ALLOW_WILDCARD_LISTEN="${ALLOW_WILDCARD_LISTEN:-false}"

normalize_bool() {
  local raw="${1:-}"
  raw="$(echo "$raw" | tr '[:upper:]' '[:lower:]' | xargs)"
  case "$raw" in
    1|true|yes|on)
      echo "true"
      ;;
    0|false|no|off|"")
      echo "false"
      ;;
    *)
      echo "false"
      ;;
  esac
}

if [[ ! -f "$INVENTORY_FILE" ]]; then
  echo "Missing inventory file: $INVENTORY_FILE" >&2
  exit 1
fi

if [[ ! -f "$NODE_ADDRESSES_FILE" ]]; then
  echo "Missing node address file: $NODE_ADDRESSES_FILE" >&2
  exit 1
fi

if [[ ! -f "$MANIFEST_FILE" ]]; then
  echo "Missing operational manifest: $MANIFEST_FILE" >&2
  exit 1
fi

if [[ -s "$HOSTS_FILE" ]]; then
  # shellcheck disable=SC1090
  source "$HOSTS_FILE"
  USE_HOST_OVERRIDES="true"
else
  echo "Hosts override file not found or empty at $HOSTS_FILE; using values from inventory." >&2
fi

mkdir -p "$OUT_DIR"

resolve_public_host() {
  local machine_id="$1"
  local default_host="$2"
  local machine_key
  if [[ "$USE_HOST_OVERRIDES" != "true" ]]; then
    echo "$default_host"
    return
  fi

  machine_key="$(echo "$machine_id" | tr '[:lower:]-' '[:upper:]_')"
  local var_name="${machine_key}_HOST"
  local value="${!var_name:-}"
  if [[ -n "$value" ]]; then
    echo "$value"
  else
    echo "$default_host"
  fi
}

resolve_p2p_host() {
  local machine_id="$1"
  local default_management_host="$2"
  local fallback_public_host="$3"
  local machine_key
  if [[ "$USE_HOST_OVERRIDES" != "true" ]]; then
    if [[ -n "${default_management_host}" ]]; then
      echo "${default_management_host}"
    else
      echo "${fallback_public_host}"
    fi
    return
  fi

  machine_key="$(echo "$machine_id" | tr '[:lower:]-' '[:upper:]_')"

  local management_host_var="${machine_key}_MANAGEMENT_HOST"
  local p2p_var="${machine_key}_P2P_HOST"
  local internal_var="${machine_key}_INTERNAL_HOST"

  if [[ -n "${!management_host_var:-}" ]]; then
    echo "${!management_host_var}"
    return
  fi

  if [[ -n "${!p2p_var:-}" ]]; then
    echo "${!p2p_var}"
    return
  fi

  if [[ -n "${!internal_var:-}" ]]; then
    echo "${!internal_var}"
    return
  fi

  if [[ -n "${default_management_host}" ]]; then
    echo "${default_management_host}"
    return
  fi

  echo "${fallback_public_host}"
}

compute_listen_address() {
  local p2p_host="$1"
  local p2p_port="$2"

  if [[ "$p2p_host" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    if [[ "$p2p_host" =~ ^10\. ]] || [[ "$p2p_host" =~ ^192\.168\. ]] || [[ "$p2p_host" =~ ^172\.([1][6-9]|2[0-9]|3[0-1])\. ]] || [[ "$p2p_host" =~ ^127\. ]]; then
      echo "${p2p_host}:${p2p_port}"
      return
    fi
    echo "0.0.0.0:${p2p_port}"
    return
  fi

  if [[ "$p2p_host" == "localhost" ]]; then
    echo "127.0.0.1:${p2p_port}"
    return
  fi

  echo "0.0.0.0:${p2p_port}"
}

compute_public_address() {
  local p2p_host="$1"
  local p2p_port="$2"

  if [[ "$p2p_host" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "${p2p_host}:${p2p_port}"
    return
  fi

  if [[ "$p2p_host" == "localhost" ]]; then
    echo "127.0.0.1:${p2p_port}"
    return
  fi

  echo "${p2p_host}:${p2p_port}"
}

lookup_node_address() {
  local machine_id="$1"
  awk -F, -v id="$machine_id" 'NR > 1 && $1 == id { print $6; exit }' "$NODE_ADDRESSES_FILE"
}

resolve_public_p2p_port() {
  local validator_address="$1"
  local default_port="$2"
  case "$validator_address" in
    synv114cvu472rkdgpmzvkj70zk9tu8cqqlu4x9ra) echo "5622" ;;
    synv11wrj74dnkc802jfl4e7j7jd2azj2zk2eqvgu) echo "5622" ;;
    synv11v2r4gnp5py3ae5ft6646lxpqphdv58k8tyu) echo "5622" ;;
    synv118u0v2gxn4zew5j886hwz32tkaujsvhykf49) echo "5622" ;;
    synv11mvlgy72uq7kuh200qnxv67zrqjugz267k46) echo "5622" ;;
    *) echo "$default_port" ;;
  esac
}

read_canonical_validators() {
  python3 - "$MANIFEST_FILE" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    manifest = json.load(handle)

for entry in manifest.get("validators", []):
    slot = entry.get("slot")
    address = str(entry.get("address") or "").strip()
    if slot is None or not address:
        continue
    print(f"{slot},{address}")
PY
}

collect_allowed_validator_addresses() {
  local addresses=()
  while IFS=, read -r _ validator_address || [[ -n "${validator_address:-}" ]]; do
    [[ -n "${validator_address:-}" ]] || continue
    addresses+=("\"$validator_address\"")
  done < <(read_canonical_validators)

  if [[ "${#addresses[@]}" -eq 0 ]]; then
    echo "[]"
    return
  fi

  local joined
  joined="$(IFS=,; echo "${addresses[*]}")"
  echo "[$joined]"
}

collect_static_validator_mesh_peers() {
  local current_validator_address="${1:-}"
  local peers=()
  while IFS=, read -r slot peer_id || [[ -n "${peer_id:-}" ]]; do
    [[ -n "${peer_id:-}" ]] || continue
    if [[ -n "$current_validator_address" && "$peer_id" == "$current_validator_address" ]]; then
      continue
    fi
    local public_p2p_port
    public_p2p_port="$(resolve_public_p2p_port "$peer_id" "5622")"
    peers+=("\"snr://${peer_id}@genesisval${slot}.synergynode.xyz:${public_p2p_port}\"")
  done < <(read_canonical_validators)

  local joined
  joined="$(IFS=,; echo "${peers[*]}")"
  echo "[${joined}]"
}

render_bootnode_list() {
  local joined
  joined="$(IFS=,; echo "$*")"
  echo "[${joined}]"
}

BOOTNODE1_HOST=""
BOOTNODE1_PORT=""
BOOTNODE2_HOST=""
BOOTNODE2_PORT=""

while IFS=, read -r machine_id _ _ _ _ _ p2p_port _ _ _ _ host management_host _ _ _ || [[ -n "${machine_id:-}" ]]; do
  [[ "$machine_id" == "machine_id" ]] && continue
  resolved_host="$(resolve_public_host "$machine_id" "$host")"
  resolved_p2p_host="$(resolve_p2p_host "$machine_id" "$management_host" "$resolved_host")"
  if [[ "$machine_id" == "machine-01" ]]; then
    BOOTNODE1_HOST="$resolved_p2p_host"
    BOOTNODE1_PORT="$p2p_port"
  elif [[ "$machine_id" == "machine-02" ]]; then
    BOOTNODE2_HOST="$resolved_p2p_host"
    BOOTNODE2_PORT="$p2p_port"
  fi
done < "$INVENTORY_FILE"

if [[ -z "$BOOTNODE1_HOST" || -z "$BOOTNODE1_PORT" ]]; then
  echo "Inventory is missing machine-01 bootstrap data." >&2
  exit 1
fi

if [[ -z "$BOOTNODE2_HOST" || -z "$BOOTNODE2_PORT" ]]; then
  echo "Inventory is missing machine-02 bootstrap data." >&2
  exit 1
fi

BOOTNODE1="snr://bootstrap@${BOOTNODE1_HOST}:${BOOTNODE1_PORT}"
BOOTNODE2="snr://bootstrap@${BOOTNODE2_HOST}:${BOOTNODE2_PORT}"
BOOTNODE3="snr://bootstrap@bootnode3.synergynode.xyz:5620"
CANONICAL_BOOTNODES="$(render_bootnode_list "\"$BOOTNODE1\"" "\"$BOOTNODE2\"" "\"$BOOTNODE3\"")"
ALLOWED_VALIDATOR_ADDRESSES="$(collect_allowed_validator_addresses)"

while IFS=, read -r machine_id node_id role_group role node_type _ p2p_port rpc_port ws_port grpc_port discovery_port host management_host auto_register enable_pruning vrf_enabled || [[ -n "${machine_id:-}" ]]; do
  [[ "$machine_id" == "machine_id" ]] && continue

  resolved_public_host="$(resolve_public_host "$machine_id" "$host")"
  resolved_p2p_host="$(resolve_p2p_host "$machine_id" "$management_host" "$resolved_public_host")"
  listen_address="$(compute_listen_address "$resolved_p2p_host" "$p2p_port")"
  public_address="$(compute_public_address "$resolved_p2p_host" "$p2p_port")"
  validator_address="$(lookup_node_address "$machine_id")"
  if [[ -z "$validator_address" ]]; then
    echo "Missing validator address mapping for ${machine_id} in ${NODE_ADDRESSES_FILE}" >&2
    exit 1
  fi

  bootnodes='[]'
  additional_dial_targets='[]'
  if [[ "$role" == "validator" || "$node_type" == "validator" ]]; then
    bootnodes="$CANONICAL_BOOTNODES"
    additional_dial_targets="$(collect_static_validator_mesh_peers "$validator_address")"
  elif [[ "$machine_id" == "machine-02" ]]; then
    bootnodes="[\"$BOOTNODE1\"]"
  elif [[ "$machine_id" != "machine-01" ]]; then
    bootnodes="[\"$BOOTNODE1\", \"$BOOTNODE2\"]"
  fi

  auto_register="$(normalize_bool "$auto_register")"
  enable_pruning="$(normalize_bool "$enable_pruning")"
  vrf_enabled="$(normalize_bool "$vrf_enabled")"

  cat > "$OUT_DIR/${machine_id}.toml" <<CONFIG
# Auto-generated by scripts/testbeta/render-configs.sh
# Machine: ${machine_id}
# Role Group: ${role_group}
# Role: ${role}
# Node Type: ${node_type}

[network]
id = ${TESTBETA_CHAIN_ID}
name = "${TESTBETA_NETWORK_NAME}"
p2p_port = ${p2p_port}
rpc_port = ${rpc_port}
ws_port = ${ws_port}
max_peers = 100
bootnodes = ${bootnodes}
seed_servers = []
bootstrap_dns_records = []
additional_dial_targets = ${additional_dial_targets}

[blockchain]
block_time = ${TESTBETA_BLOCK_TIME_SECS}
max_gas_limit = "0x2fefd8"
chain_id = ${TESTBETA_CHAIN_ID}

[consensus]
algorithm = "Proof of Synergy"
block_time_secs = ${TESTBETA_BLOCK_TIME_SECS}
epoch_length = ${TESTBETA_EPOCH_LENGTH}
min_validators = ${TESTBETA_MIN_VALIDATORS}
validator_cluster_size = 5
validator_vote_threshold = ${TESTBETA_VALIDATOR_VOTE_THRESHOLD}
max_validators = 5
status_ready_gate_enabled = true
status_ready_min_validators = ${TESTBETA_STATUS_READY_MIN_VALIDATORS}
status_ready_genesis_grace_secs = ${TESTBETA_STATUS_READY_GENESIS_GRACE_SECS}
allow_genesis_status_bypass = true
mesh_settle_secs = 3
leader_timeout_secs = ${TESTBETA_LEADER_TIMEOUT_SECS}
vote_timeout_secs = ${TESTBETA_VOTE_TIMEOUT_SECS}
block_timeout_secs = ${TESTBETA_BLOCK_TIMEOUT_SECS}
penalization_enabled = ${TESTBETA_CONSENSUS_PENALIZATION_ENABLED}
synergy_score_decay_rate = 0.05
vrf_enabled = ${vrf_enabled}
vrf_seed_epoch_interval = 1000
max_synergy_points_per_epoch = 100
max_tasks_per_validator = 10

[consensus.reward_weighting]
task_accuracy = 0.5
uptime = 0.3
collaboration = 0.2

[logging]
log_level = "debug"
log_file = "data/testbeta15/${machine_id}/logs/${node_id}.log"
enable_console = true
max_file_size = 10485760
max_files = 5

[rpc]
bind_address = "${resolved_p2p_host}:${rpc_port}"
enable_http = true
http_port = ${rpc_port}
enable_ws = true
ws_port = ${ws_port}
enable_grpc = true
grpc_port = ${grpc_port}
cors_enabled = false
cors_origins = []

[p2p]
listen_address = "${listen_address}"
public_address = "${public_address}"
node_name = "${node_id}"
enable_discovery = false
discovery_port = ${discovery_port}
heartbeat_interval = 10
bootstrap_refresh_secs = ${TESTBETA_P2P_BOOTSTRAP_REFRESH_SECS}

[storage]
database = "rocksdb"
path = "data/testbeta15/${machine_id}/chain"
enable_pruning = ${enable_pruning}
pruning_interval = 86400

[node]
auto_register_validator = ${auto_register}
validator_address = "${validator_address}"
strict_validator_allowlist = true
allowed_validator_addresses = ${ALLOWED_VALIDATOR_ADDRESSES}
CONFIG

  echo "Generated ${OUT_DIR}/${machine_id}.toml"
done < "$INVENTORY_FILE"

echo "Rendered 15-node configs into: $OUT_DIR"
