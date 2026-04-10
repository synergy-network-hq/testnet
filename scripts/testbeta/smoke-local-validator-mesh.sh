#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BINARY="${ROOT_DIR}/target/release/synergy-testbeta"
TIMEOUT_SECS="${TIMEOUT_SECS:-120}"
POLL_INTERVAL_SECS="${POLL_INTERVAL_SECS:-2}"
KEEP_WORKDIR="false"
WORKDIR=""
CREATED_WORKDIR="false"

VALIDATOR_ADDRESSES=(
  "synv114cvu472rkdgpmzvkj70zk9tu8cqqlu4x9ra"
  "synv11wrj74dnkc802jfl4e7j7jd2azj2zk2eqvgu"
  "synv11v2r4gnp5py3ae5ft6646lxpqphdv58k8tyu"
  "synv118u0v2gxn4zew5j886hwz32tkaujsvhykf49"
  "synv11mvlgy72uq7kuh200qnxv67zrqjugz267k46"
)
P2P_PORTS=(5722 5723 5724 5725 5726)
RPC_PORTS=(5740 5741 5742 5743 5744)
WS_PORTS=(5760 5761 5762 5763 5764)
START_VALIDATOR_COUNT="${START_VALIDATOR_COUNT:-5}"
PIDS=()
WORKSPACES=()

usage() {
  cat <<USAGE
Usage: $0 [--binary PATH] [--timeout SECONDS] [--workdir PATH] [--keep-workdir]

Starts the first five local validators against the canonical five-validator
genesis and fails unless all active validators advance chain height from
genesis within the timeout.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --binary)
      BINARY="$2"
      shift 2
      ;;
    --timeout)
      TIMEOUT_SECS="$2"
      shift 2
      ;;
    --workdir)
      WORKDIR="$2"
      shift 2
      ;;
    --keep-workdir)
      KEEP_WORKDIR="true"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ ! -x "$BINARY" ]]; then
  echo "Binary not found at $BINARY; building release node binary..." >&2
  cargo build --manifest-path "$ROOT_DIR/src/Cargo.toml" --release --bin synergy-testbeta
fi

if [[ -z "$WORKDIR" ]]; then
  WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/synergy-testbeta-mesh-smoke.XXXXXX")"
  CREATED_WORKDIR="true"
else
  mkdir -p "$WORKDIR"
fi

cleanup() {
  for pid in "${PIDS[@]:-}"; do
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
    fi
  done

  for _ in {1..10}; do
    local running="false"
    for pid in "${PIDS[@]:-}"; do
      if kill -0 "$pid" 2>/dev/null; then
        running="true"
        break
      fi
    done
    [[ "$running" == "false" ]] && break
    sleep 1
  done

  for pid in "${PIDS[@]:-}"; do
    if kill -0 "$pid" 2>/dev/null; then
      kill -9 "$pid" 2>/dev/null || true
    fi
  done

  if [[ "$KEEP_WORKDIR" == "true" || "$CREATED_WORKDIR" != "true" ]]; then
    echo "Smoke-test workspace preserved at: $WORKDIR"
  else
    rm -rf "$WORKDIR"
  fi
}
trap cleanup EXIT

assert_port_available() {
  local port="$1"
  if lsof -nP -iTCP:"$port" -sTCP:LISTEN >/dev/null 2>&1; then
    echo "Smoke-test port already in use: $port" >&2
    lsof -nP -iTCP:"$port" -sTCP:LISTEN >&2 || true
    exit 1
  fi
}

assert_ports_available() {
  local count="$1"
  local i
  for ((i = 0; i < count; i++)); do
    assert_port_available "${P2P_PORTS[$i]}"
    assert_port_available "${RPC_PORTS[$i]}"
    assert_port_available "${WS_PORTS[$i]}"
  done
}

assert_ports_available "$START_VALIDATOR_COUNT"

json_array() {
  python3 - "$@" <<'PY'
import json
import sys
print(json.dumps(sys.argv[1:]))
PY
}

rpc_height() {
  local port="$1"
  python3 - "$port" <<'PY'
import json
import sys
import urllib.error
import urllib.request

port = int(sys.argv[1])
payload = json.dumps({
    "jsonrpc": "2.0",
    "method": "synergy_blockNumber",
    "params": [],
    "id": 1,
}).encode()
request = urllib.request.Request(
    f"http://127.0.0.1:{port}",
    data=payload,
    headers={"Content-Type": "application/json"},
)
try:
    with urllib.request.urlopen(request, timeout=2) as response:
        body = json.loads(response.read().decode())
except Exception:
    print(-1)
    raise SystemExit(0)

result = body.get("result", -1)
try:
    if isinstance(result, str):
        print(int(result, 0))
    else:
        print(int(result))
except Exception:
    print(-1)
PY
}

log_snippet() {
  local file="$1"
  [[ -f "$file" ]] || return 0
  python3 - "$file" <<'PY'
import pathlib
import re
import sys

path = pathlib.Path(sys.argv[1])
text = path.read_text(encoding="utf-8", errors="replace")
pattern = re.compile(
    r"(?ms)^.*?(Handshake received|Received status|Waiting for validator mesh status sync before block production|Vote request broadcast|Validator missed vote deadline|Peer disconnected|Failed to dial peer|Incoming peer connection|Sync complete).*$"
)
matches = pattern.findall(text)
lines = []
for line in text.splitlines():
    if any(token in line for token in [
        "Handshake received",
        "Received status",
        "Waiting for validator mesh status sync before block production",
        "Vote request broadcast",
        "Validator missed vote deadline",
        "Peer disconnected",
        "Failed to dial peer",
        "Incoming peer connection",
        "Sync complete",
    ]):
        lines.append(line)
    elif lines and line.startswith("  Metadata:"):
        lines.append(line)
trimmed = lines[-40:]
print("\n".join(trimmed))
PY
}

create_workspace() {
  local index="$1"
  local workspace="${WORKDIR}/validator-${index}"
  local validator_address="${VALIDATOR_ADDRESSES[$((index - 1))]}"
  local p2p_port="${P2P_PORTS[$((index - 1))]}"
  local rpc_port="${RPC_PORTS[$((index - 1))]}"
  local ws_port="${WS_PORTS[$((index - 1))]}"
  local node_name="smoke-validator-${index}"
  local additional_targets=()
  local i

  mkdir -p "$workspace/config" "$workspace/data" "$workspace/logs"
  cp "$ROOT_DIR/config/genesis.json" "$workspace/config/genesis.json"
  cp -R "$ROOT_DIR/config/genesis-validators" "$workspace/config/"

  for i in "${!P2P_PORTS[@]}"; do
    if [[ "$i" -ne $((index - 1)) ]]; then
      additional_targets+=("127.0.0.1:${P2P_PORTS[$i]}")
    fi
  done

  local allowed_json
  local targets_json
  allowed_json="$(json_array "${VALIDATOR_ADDRESSES[@]}")"
  targets_json="$(json_array "${additional_targets[@]}")"

  cat > "${workspace}/config/node.toml" <<CONFIG
[identity]
node_id = "${validator_address}"
role = "validator"
role_display = "Validator Node"
environment = "testbeta"
display_environment = "Testnet-Beta"
address = "${validator_address}"
label = "Smoke Validator ${index}"

[network]
id = 338639
name = "synergy-testnet-beta"
chain_name = "synergy-testnet-beta"
chain_id = 338639
p2p_port = ${p2p_port}
rpc_port = ${rpc_port}
ws_port = ${ws_port}
p2p_listen = "127.0.0.1:${p2p_port}"
bootnodes = []
seed_servers = []
bootstrap_dns_records = []
additional_dial_targets = ${targets_json}
max_peers = 32
public_host = "127.0.0.1"

[blockchain]
block_time = 2
max_gas_limit = "0x2fefd8"
chain_id = 338639

[consensus]
algorithm = "Proof of Synergy"
block_time_secs = 2
epoch_length = 1000
min_validators = 3
validator_cluster_size = 5
validator_vote_threshold = 3
max_validators = 5
synergy_score_decay_rate = 0.05
vrf_enabled = true
vrf_seed_epoch_interval = 1000
max_synergy_points_per_epoch = 100
max_tasks_per_validator = 10

[consensus.reward_weighting]
task_accuracy = 0.5
uptime = 0.3
collaboration = 0.2

[logging]
log_level = "info"
log_file = "${workspace}/logs/synergy-testbeta.log"
enable_console = true
max_file_size = 10485760
max_files = 5

[rpc]
bind_address = "127.0.0.1:${rpc_port}"
enable_http = true
http_port = ${rpc_port}
enable_ws = true
ws_port = ${ws_port}
enable_grpc = true
grpc_port = ${rpc_port}
cors_enabled = false
cors_origins = []

[p2p]
listen_address = "127.0.0.1:${p2p_port}"
public_address = "127.0.0.1:${p2p_port}"
node_name = "${node_name}"
enable_discovery = false
discovery_port = $((5800 + index))
heartbeat_interval = 5

[storage]
database = "rocksdb"
engine = "rocksdb"
path = "${workspace}/data"
mode = "role-bounded"
enable_pruning = false
pruning_interval = 86400

[node]
bootstrap_only = false
auto_register_validator = false
validator_address = "${validator_address}"
strict_validator_allowlist = true
allowed_validator_addresses = ${allowed_json}

[telemetry]
metrics_bind = "127.0.0.1:$((6000 + index))"
structured_logs = true
log_level = "info"

[policy]
allow_remote_admin = false
require_signed_updates = true
quarantine_on_policy_failure = true
quarantine_on_key_role_mismatch = true
connectivity_fail_mode = "warn-and-continue"

[wallet]
reward_address = "${validator_address}"
sponsored_stake_snrg = "5000.000000000"
sponsored_stake_nwei = "5000000000000"
treasury_wallet = "synw1rmj046xra4059csdc94pltfjsr5tkduhuc74ep"
stake_vault_wallet = "synl1ta7vczypesta385h64fw3n5sfsqg9sqv3upp5t"

[bootstrap]
status = "configured"
note = "Local smoke validator mesh"

[role]
compiled_profile = "validator_node"
services = ["p2p", "consensus", "mempool", "state", "aegis-verifier", "telemetry"]

[validator]
participation = "active"
verify_quorum_certificates = true
state_sync_before_join = true
CONFIG

  WORKSPACES+=("$workspace")
}

start_node() {
  local workspace="$1"
  local config_path="${workspace}/config/node.toml"
  local stdout_path="${workspace}/logs/control-start.stdout.log"
  local stderr_path="${workspace}/logs/control-start.stderr.log"

  (
    cd "$workspace"
    SYNERGY_PROJECT_ROOT="$workspace" \
    SYNERGY_CONFIG_PATH="$config_path" \
    "$BINARY" start --config "$config_path" >"$stdout_path" 2>"$stderr_path"
  ) &
  PIDS+=("$!")
}

print_diagnostics() {
  local i
  echo
  echo "Smoke test diagnostics:"
  for i in "${!WORKSPACES[@]}"; do
    local workspace="${WORKSPACES[$i]}"
    local rpc_port="${RPC_PORTS[$i]}"
    local height
    height="$(rpc_height "$rpc_port")"
    echo
    echo "validator-$((i + 1)) rpc_port=${rpc_port} height=${height} workspace=${workspace}"
    echo "--- ${workspace}/logs/synergy-testbeta.log ---"
    log_snippet "${workspace}/logs/synergy-testbeta.log"
    echo "--- ${workspace}/logs/control-start.stderr.log ---"
    tail -n 20 "${workspace}/logs/control-start.stderr.log" 2>/dev/null || true
  done
}

for index in $(seq 1 "$START_VALIDATOR_COUNT"); do
  create_workspace "$index"
done

for workspace in "${WORKSPACES[@]}"; do
  start_node "$workspace"
done

echo "Started local ${START_VALIDATOR_COUNT}-validator smoke mesh against canonical 5-validator genesis in: $WORKDIR"
deadline=$(( $(date +%s) + TIMEOUT_SECS ))
while [[ "$(date +%s)" -lt "$deadline" ]]; do
  ready="true"
  min_height=999999999
  max_height=-1
  for i in "${!WORKSPACES[@]}"; do
    height="$(rpc_height "${RPC_PORTS[$i]}")"
    if [[ "$height" -lt 1 ]]; then
      ready="false"
    fi
    if [[ "$height" -ge 0 && "$height" -lt "$min_height" ]]; then
      min_height="$height"
    fi
    if [[ "$height" -gt "$max_height" ]]; then
      max_height="$height"
    fi
  done

  if [[ "$ready" == "true" && "$max_height" -ge 2 ]]; then
    echo "Smoke test passed: all validators advanced beyond genesis (min_height=${min_height}, max_height=${max_height})."
    exit 0
  fi

  sleep "$POLL_INTERVAL_SECS"
done

echo "Smoke test failed: validators did not all advance height within ${TIMEOUT_SECS}s." >&2
print_diagnostics >&2
exit 1
