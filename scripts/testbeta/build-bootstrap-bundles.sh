#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="${BOOTSTRAP_OUT_DIR:-$ROOT_DIR/bootstrap-bundles}"
BINARIES_DIR="${BOOTSTRAP_BINARIES_DIR:-$ROOT_DIR/binaries}"
INSTALLERS_DIR="${BOOTSTRAP_INSTALLERS_DIR:-$ROOT_DIR/node-control-panel/testbeta/runtime/installers}"
GENESIS_FILE="${BOOTSTRAP_GENESIS_FILE:-$ROOT_DIR/config/genesis.json}"
BOOTNODE_DOMAIN="${BOOTSTRAP_BOOTNODE_DOMAIN:-synergynode.xyz}"
SEED_DOMAIN="${BOOTSTRAP_SEED_DOMAIN:-synergynode.xyz}"
DISCOVERY_DOMAIN="${BOOTSTRAP_DISCOVERY_DOMAIN:-synergynode.xyz}"
TESTBETA_ENV_DIR_DEFAULT="${TESTBETA_ENV_DIR_DEFAULT:-$ROOT_DIR/node-control-panel/testbeta/runtime/env-files}"
ENV_OVERRIDE_HELPER="${ENV_OVERRIDE_HELPER:-$ROOT_DIR/scripts/testbeta/testbeta-env-overrides.sh}"
P2P_PORT="${BOOTSTRAP_P2P_PORT:-5620}"
SEED_HTTP_PORT="${BOOTSTRAP_SEED_HTTP_PORT:-5621}"
GENESIS_VALIDATOR_COUNT="${BOOTSTRAP_GENESIS_VALIDATOR_COUNT:-5}"
MIN_GENESIS_VALIDATORS="${BOOTSTRAP_MIN_GENESIS_VALIDATORS:-4}"
VALIDATOR_VOTE_THRESHOLD="${BOOTSTRAP_VALIDATOR_VOTE_THRESHOLD:-3}"

BOOTNODE_RPC_PORT=5640
BOOTNODE_WS_PORT=5660
BOOTNODE_GRPC_PORT=50051
BOOTNODE_DISCOVERY_PORT=5680
RPC_GATEWAY_BUNDLE_NAME="${BOOTSTRAP_RPC_GATEWAY_BUNDLE_NAME:-genesisrpc}"
INDEXER_BUNDLE_NAME="${BOOTSTRAP_INDEXER_BUNDLE_NAME:-genesisindexer}"
RPC_INSTALLER_DIR="${BOOTSTRAP_RPC_INSTALLER_DIR:-$INSTALLERS_DIR/Node-RPC}"
INDEXER_INSTALLER_DIR="${BOOTSTRAP_INDEXER_INSTALLER_DIR:-$INSTALLERS_DIR/Node-EXP}"

BOOTNODES=(
  "bootnode1"
  "bootnode2"
  "bootnode3"
)

SEEDS=(
  "seed1"
  "seed2"
  "seed3"
)

if [[ -f "$ENV_OVERRIDE_HELPER" ]]; then
  # shellcheck disable=SC1090
  source "$ENV_OVERRIDE_HELPER"
fi

DARWIN_BINARY=""
LINUX_BINARY=""
WINDOWS_BINARY=""

bootnode_env_lookup() {
  local name="$1"
  local key="$2"
  local fallback="${3:-}"
  if declare -F testbeta_bootnode_env_value >/dev/null 2>&1; then
    testbeta_bootnode_env_value "$name" "$key" "$fallback"
    return 0
  fi
  printf '%s\n' "$fallback"
}

bootnode_fallback_host() {
  printf '%s.%s\n' "$1" "$BOOTNODE_DOMAIN"
}

bootnode_host() {
  bootnode_env_lookup "$1" "HOSTNAME" "$(bootnode_fallback_host "$1")"
}

bootnode_ip() {
  bootnode_env_lookup "$1" "PUBLIC_IP" ""
}

bootnode_p2p_port() {
  local name="$1"
  local port
  port="$(bootnode_env_lookup "$name" "P2P_PORT_EXTERNAL" "")"
  if [[ -z "$port" ]]; then
    port="$(bootnode_env_lookup "$name" "P2P_PORT" "$P2P_PORT")"
  fi
  printf '%s\n' "$port"
}

bootnode_discovery_port() {
  local name="$1"
  local port
  port="$(bootnode_env_lookup "$name" "DISCOVERY_PORT_EXTERNAL" "")"
  if [[ -z "$port" ]]; then
    port="$(bootnode_env_lookup "$name" "DISCOVERY_PORT" "$BOOTNODE_DISCOVERY_PORT")"
  fi
  printf '%s\n' "$port"
}

installer_env_value() {
  local installer_dir="$1"
  local key="$2"
  local env_file="$installer_dir/node.env"

  if [[ ! -f "$env_file" ]]; then
    return 1
  fi

  awk -F= -v lookup="$key" '$1 == lookup {print substr($0, index($0, "=") + 1); exit}' "$env_file"
}

seed_fallback_host() {
  printf '%s.%s\n' "$1" "$SEED_DOMAIN"
}

bootnode_name_for_seed() {
  case "$1" in
    seed1) echo "bootnode1" ;;
    seed2) echo "bootnode2" ;;
    seed3) echo "bootnode3" ;;
    *) return 1 ;;
  esac
}

seed_host() {
  local seed_name="$1"
  local seed_hostname
  seed_hostname=""
  if declare -F testbeta_bootnode_env_value >/dev/null 2>&1; then
    seed_hostname="$(testbeta_bootnode_env_value "$(bootnode_name_for_seed "$seed_name")" "SEED_HOSTNAME" "" || true)"
  fi
  if [[ -n "$seed_hostname" ]]; then
    printf '%s\n' "$seed_hostname"
    return 0
  fi
  seed_fallback_host "$seed_name"
}

seed_ip() {
  bootnode_ip "$(bootnode_name_for_seed "$1")"
}

resolve_binary() {
  local target="$1"
  shift
  local candidate
  for candidate in "$@"; do
    if [[ -f "$candidate" ]]; then
      printf -v "$target" '%s' "$candidate"
      return
    fi
  done
}

resolve_binaries() {
  resolve_binary DARWIN_BINARY \
    "$BINARIES_DIR/synergy-testbeta-darwin-arm64" \
    "$ROOT_DIR/target/aarch64-apple-darwin/release/synergy-testbeta" \
    "$ROOT_DIR/target/release/synergy-testbeta"

  resolve_binary LINUX_BINARY \
    "$BINARIES_DIR/synergy-testbeta-linux-amd64" \
    "$ROOT_DIR/target/x86_64-unknown-linux-gnu/release/synergy-testbeta"

  resolve_binary WINDOWS_BINARY \
    "$BINARIES_DIR/synergy-testbeta-windows-amd64.exe" \
    "$ROOT_DIR/target/x86_64-pc-windows-gnu/release/synergy-testbeta.exe" \
    "$ROOT_DIR/target/x86_64-pc-windows-msvc/release/synergy-testbeta.exe"
}

toml_bootnodes_for() {
  local current="$1"
  local entries=()
  local name
  for name in "${BOOTNODES[@]}"; do
    if [[ "$name" == "$current" ]]; then
      continue
    fi
    entries+=("\"snr://bootstrap@$(bootnode_host "$name"):$(bootnode_p2p_port "$name")\"")
  done

  local joined=""
  local entry
  for entry in "${entries[@]}"; do
    if [[ -n "$joined" ]]; then
      joined+=", "
    fi
    joined+="$entry"
  done
  printf '[%s]' "$joined"
}

csv_bootnodes_for() {
  local current="$1"
  local entries=()
  local name
  for name in "${BOOTNODES[@]}"; do
    if [[ "$name" == "$current" ]]; then
      continue
    fi
    entries+=("snr://bootstrap@$(bootnode_host "$name"):$(bootnode_p2p_port "$name")")
  done
  local joined=""
  local entry
  for entry in "${entries[@]}"; do
    if [[ -n "$joined" ]]; then
      joined+=","
    fi
    joined+="$entry"
  done
  printf '%s' "$joined"
}

write_bootnode_config() {
  local node_dir="$1"
  local name="$2"
  local hostname p2p_port discovery_port
  hostname="$(bootnode_host "$name")"
  p2p_port="$(bootnode_p2p_port "$name")"
  discovery_port="$(bootnode_discovery_port "$name")"

  cat > "$node_dir/config/node.toml" <<EOF
[network]
id = 338639
name = "Synergy Testnet-Beta"
p2p_port = ${p2p_port}
rpc_port = ${BOOTNODE_RPC_PORT}
ws_port = ${BOOTNODE_WS_PORT}
max_peers = 128
bootnodes = $(toml_bootnodes_for "$name")

[blockchain]
block_time = 5
max_gas_limit = "0x2fefd8"
chain_id = 338639

[consensus]
algorithm = "Proof of Synergy"
block_time_secs = 5
epoch_length = 1000
min_validators = ${MIN_GENESIS_VALIDATORS}
validator_cluster_size = ${GENESIS_VALIDATOR_COUNT}
validator_vote_threshold = ${VALIDATOR_VOTE_THRESHOLD}
max_validators = ${GENESIS_VALIDATOR_COUNT}
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
log_file = "data/logs/${name}.log"
enable_console = true
max_file_size = 10485760
max_files = 5

[rpc]
bind_address = "127.0.0.1:${BOOTNODE_RPC_PORT}"
enable_http = false
http_port = ${BOOTNODE_RPC_PORT}
enable_ws = false
ws_port = ${BOOTNODE_WS_PORT}
enable_grpc = false
grpc_port = ${BOOTNODE_GRPC_PORT}
cors_enabled = false
cors_origins = []

[p2p]
listen_address = "0.0.0.0:${p2p_port}"
public_address = "${hostname}:${p2p_port}"
node_name = "${name}"
enable_discovery = true
discovery_port = ${discovery_port}
discovery_listen_address = "0.0.0.0:${discovery_port}"
discovery_public_address = "${hostname}:${discovery_port}"
heartbeat_interval = 10

[storage]
database = "rocksdb"
path = "data/chain"
enable_pruning = true
pruning_interval = 86400

[node]
bootstrap_only = true
auto_register_validator = false
validator_address = ""
strict_validator_allowlist = false
allowed_validator_addresses = []
EOF
}

write_bootnode_env() {
  local node_dir="$1"
  local name="$2"
  local ip="$3"
  local hostname p2p_port discovery_port
  hostname="$(bootnode_host "$name")"
  p2p_port="$(bootnode_p2p_port "$name")"
  discovery_port="$(bootnode_discovery_port "$name")"

  cat > "$node_dir/node.env" <<EOF
MACHINE_ID=${name}
NODE_KIND=bootnode
NODE_NAME=${name}
NODE_HOSTNAME=${hostname}
NODE_PUBLIC_IP=${ip}
P2P_PORT=${p2p_port}
P2P_LISTEN_ADDRESS=0.0.0.0:${p2p_port}
P2P_EXTERNAL_ADDRESS=${hostname}:${p2p_port}
P2P_PUBLIC_ADDRESS=${hostname}:${p2p_port}
RPC_PORT=${BOOTNODE_RPC_PORT}
WS_PORT=${BOOTNODE_WS_PORT}
GRPC_PORT=${BOOTNODE_GRPC_PORT}
DISCOVERY_PORT=${discovery_port}
DISCOVERY_LISTEN_ADDRESS=0.0.0.0:${discovery_port}
DISCOVERY_EXTERNAL_ADDRESS=${hostname}:${discovery_port}
DISCOVERY_PUBLIC_ADDRESS=${hostname}:${discovery_port}
BOOTSTRAP_ONLY=true
AUTO_REGISTER_VALIDATOR=false
BOOTNODE_LIST=$(csv_bootnodes_for "$name")
EOF
}

write_bootnode_scripts() {
  local node_dir="$1"

  cat > "$node_dir/install_and_start.sh" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail

BASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$BASE_DIR/node.env"

BIN_DARWIN="$BASE_DIR/bin/synergy-testbeta-darwin-arm64"
BIN_LINUX="$BASE_DIR/bin/synergy-testbeta-linux-amd64"
PID_FILE="$BASE_DIR/data/node.pid"
OUT_FILE="$BASE_DIR/data/logs/node.out"
GENESIS_FILE="$BASE_DIR/config/genesis.json"
CHAIN_DIR="$BASE_DIR/data/chain"
CHAIN_STATE_FILE="$BASE_DIR/data/chain.json"

select_binary() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  if [[ "$os" == "Linux" && "$arch" == "x86_64" ]]; then
    printf '%s' "$BIN_LINUX"
    return
  fi

  if [[ "$os" == "Darwin" && "$arch" == "arm64" ]]; then
    printf '%s' "$BIN_DARWIN"
    return
  fi

  echo "Unsupported platform ${os}/${arch}. Use install_and_start.ps1 on Windows." >&2
  exit 1
}

clear_quarantine_if_needed() {
  if [[ "$(uname -s)" != "Darwin" ]]; then
    return
  fi

  if command -v xattr >/dev/null 2>&1; then
    xattr -dr com.apple.quarantine "$BASE_DIR" 2>/dev/null || true
  fi
}

ensure_genesis_file() {
  if [[ ! -f "$GENESIS_FILE" ]]; then
    echo "Missing canonical genesis file: $GENESIS_FILE" >&2
    exit 1
  fi
}

reset_chain_state() {
  rm -rf "$CHAIN_DIR" "$CHAIN_STATE_FILE"
}

if [[ -f "$PID_FILE" ]]; then
  pid="$(cat "$PID_FILE")"
  if kill -0 "$pid" 2>/dev/null; then
    echo "$MACHINE_ID already running with PID $pid"
    exit 0
  fi
fi

ensure_genesis_file
reset_chain_state
mkdir -p "$BASE_DIR/data/logs" "$CHAIN_DIR"
BIN_SELECTED="$(select_binary)"
if [[ ! -f "$BIN_SELECTED" ]]; then
  echo "Missing binary: $BIN_SELECTED" >&2
  exit 1
fi

clear_quarantine_if_needed
chmod +x "$BIN_SELECTED"
bind_ip="${BIND_IP:-0.0.0.0}"
public_host="${NODE_PUBLIC_HOST:-${HOSTNAME:-${HOST:-}}}"
p2p_port_value="${P2P_PORT:-}"
public_p2p_port="${PUBLIC_P2P_PORT:-${P2P_PORT_EXTERNAL:-${p2p_port_value:-}}}"
discovery_port_value="${DISCOVERY_PORT:-}"
discovery_public_port="${DISCOVERY_PORT_EXTERNAL:-${discovery_port_value:-}}"
if [[ -n "$p2p_port_value" ]]; then
  p2p_listen_address="${bind_ip}:${p2p_port_value}"
else
  p2p_listen_address="${P2P_LISTEN_ADDRESS:-}"
fi
if [[ -n "$public_host" && -n "$public_p2p_port" ]]; then
  p2p_external_address="${public_host}:${public_p2p_port}"
else
  p2p_external_address="${P2P_EXTERNAL_ADDRESS:-${P2P_PUBLIC_ADDRESS:-}}"
fi
if [[ -n "$discovery_port_value" ]]; then
  discovery_listen_address="${bind_ip}:${discovery_port_value}"
else
  discovery_listen_address="${DISCOVERY_LISTEN_ADDRESS:-}"
fi
if [[ -n "$public_host" && -n "$discovery_public_port" ]]; then
  discovery_external_address="${public_host}:${discovery_public_port}"
else
  discovery_external_address="${DISCOVERY_EXTERNAL_ADDRESS:-${DISCOVERY_PUBLIC_ADDRESS:-}}"
fi
(
  cd "$BASE_DIR"
  nohup env \
    SYNERGY_PROJECT_ROOT="$BASE_DIR" \
    SYNERGY_CONFIG_PATH="$BASE_DIR/config/node.toml" \
    SYNERGY_GENESIS_FILE="$GENESIS_FILE" \
    SYNERGY_BOOTSTRAP_ONLY=true \
    SYNERGY_AUTO_REGISTER_VALIDATOR=false \
    SYNERGY_P2P_PORT="${p2p_port_value:-}" \
    SYNERGY_P2P_LISTEN_ADDRESS="${p2p_listen_address:-}" \
    SYNERGY_P2P_EXTERNAL_ADDRESS="${p2p_external_address:-}" \
    SYNERGY_P2P_PUBLIC_ADDRESS="${p2p_external_address:-}" \
    SYNERGY_DISCOVERY_PORT="${discovery_port_value:-}" \
    SYNERGY_DISCOVERY_LISTEN_ADDRESS="${discovery_listen_address:-}" \
    SYNERGY_DISCOVERY_EXTERNAL_ADDRESS="${discovery_external_address:-}" \
    SYNERGY_DISCOVERY_PUBLIC_ADDRESS="${discovery_external_address:-}" \
    "$BIN_SELECTED" start --config "$BASE_DIR/config/node.toml" >"$OUT_FILE" 2>&1 &
  echo $! > "$PID_FILE"
)
echo "Started $MACHINE_ID as bootstrap-only discovery node (PID $(cat "$PID_FILE"))"
echo "Logs: $OUT_FILE"
SCRIPT

  cat > "$node_dir/nodectl.sh" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail

BASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$BASE_DIR/node.env"

PID_FILE="$BASE_DIR/data/node.pid"
OUT_FILE="$BASE_DIR/data/logs/node.out"

is_running() {
  if [[ -f "$PID_FILE" ]]; then
    local pid
    pid="$(cat "$PID_FILE")"
    if kill -0 "$pid" 2>/dev/null; then
      return 0
    fi
  fi
  return 1
}

stop_node() {
  if ! is_running; then
    echo "$MACHINE_ID is not running"
    rm -f "$PID_FILE"
    return
  fi

  local pid
  pid="$(cat "$PID_FILE")"
  kill "$pid" 2>/dev/null || true
  for _ in {1..10}; do
    if ! kill -0 "$pid" 2>/dev/null; then
      break
    fi
    sleep 1
  done
  if kill -0 "$pid" 2>/dev/null; then
    kill -9 "$pid" 2>/dev/null || true
  fi
  rm -f "$PID_FILE"
  echo "Stopped $MACHINE_ID"
}

case "${1:-}" in
  start)
    "$BASE_DIR/install_and_start.sh"
    ;;
  stop)
    stop_node
    ;;
  restart)
    stop_node
    "$BASE_DIR/install_and_start.sh"
    ;;
  status)
    if is_running; then
      echo "$MACHINE_ID is running (PID $(cat "$PID_FILE"))"
    else
      echo "$MACHINE_ID is stopped"
    fi
    ;;
  logs)
    if [[ "${2:-}" == "--follow" ]]; then
      tail -f "$OUT_FILE"
    else
      tail -n 120 "$OUT_FILE"
    fi
    ;;
  info)
    cat <<INFO
Machine ID: $MACHINE_ID
Role: $NODE_KIND
Hostname: $NODE_HOSTNAME
IP: $NODE_PUBLIC_IP
P2P Port: $P2P_PORT
Discovery Port: $DISCOVERY_PORT
Bootstrap Only: $BOOTSTRAP_ONLY
Bootnodes: $BOOTNODE_LIST
Config: $BASE_DIR/config/node.toml
INFO
    ;;
  *)
    echo "Usage: $0 <start|stop|restart|status|logs|info>" >&2
    exit 1
    ;;
esac
SCRIPT

  cat > "$node_dir/install_and_start.ps1" <<'SCRIPT'
$ErrorActionPreference = "Stop"

$BaseDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$EnvPath = Join-Path $BaseDir "node.env"
$NodeEnv = @{}
Get-Content $EnvPath | ForEach-Object {
  if ($_ -match '^\s*$' -or $_ -match '^\s*#') { return }
  $parts = $_ -split '=', 2
  if ($parts.Count -eq 2) { $NodeEnv[$parts[0].Trim()] = $parts[1].Trim() }
}

$BinPath = Join-Path $BaseDir "bin/synergy-testbeta-windows-amd64.exe"
$ConfigPath = Join-Path $BaseDir "config/node.toml"
$GenesisPath = Join-Path $BaseDir "config/genesis.json"
$DataDir = Join-Path $BaseDir "data"
$LogsDir = Join-Path $DataDir "logs"
$ChainDir = Join-Path $DataDir "chain"
$ChainStateFile = Join-Path $DataDir "chain.json"
$PidFile = Join-Path $DataDir "node.pid"
$OutFile = Join-Path $LogsDir "node.out"
$ErrFile = Join-Path $LogsDir "node.err"

New-Item -ItemType Directory -Force -Path $LogsDir | Out-Null
if (-not (Test-Path $GenesisPath)) { throw "Missing canonical genesis file: $GenesisPath" }
if (Test-Path $ChainDir) { Remove-Item -Recurse -Force $ChainDir }
if (Test-Path $ChainStateFile) { Remove-Item -Force $ChainStateFile }
New-Item -ItemType Directory -Force -Path $ChainDir | Out-Null

if (-not (Test-Path $BinPath)) { throw "Missing Windows binary: $BinPath" }

$previousProjectRoot = $env:SYNERGY_PROJECT_ROOT
$previousConfigPath = $env:SYNERGY_CONFIG_PATH
$previousGenesisPath = $env:SYNERGY_GENESIS_FILE
$previousBootstrapOnly = $env:SYNERGY_BOOTSTRAP_ONLY
$previousAutoRegister = $env:SYNERGY_AUTO_REGISTER_VALIDATOR
$previousP2PPort = $env:SYNERGY_P2P_PORT
$previousP2PListenAddress = $env:SYNERGY_P2P_LISTEN_ADDRESS
$previousP2PExternalAddress = $env:SYNERGY_P2P_EXTERNAL_ADDRESS
$previousP2PPublicAddress = $env:SYNERGY_P2P_PUBLIC_ADDRESS
$previousDiscoveryPort = $env:SYNERGY_DISCOVERY_PORT
$previousDiscoveryListenAddress = $env:SYNERGY_DISCOVERY_LISTEN_ADDRESS
$previousDiscoveryExternalAddress = $env:SYNERGY_DISCOVERY_EXTERNAL_ADDRESS
$previousDiscoveryPublicAddress = $env:SYNERGY_DISCOVERY_PUBLIC_ADDRESS

$env:SYNERGY_PROJECT_ROOT = $BaseDir
$env:SYNERGY_CONFIG_PATH = $ConfigPath
$env:SYNERGY_GENESIS_FILE = $GenesisPath
$env:SYNERGY_BOOTSTRAP_ONLY = "true"
$env:SYNERGY_AUTO_REGISTER_VALIDATOR = "false"
$bindIp = if ($NodeEnv.ContainsKey("BIND_IP")) { $NodeEnv["BIND_IP"] } else { "0.0.0.0" }
$publicHost = if ($NodeEnv.ContainsKey("NODE_PUBLIC_HOST")) { $NodeEnv["NODE_PUBLIC_HOST"] } elseif ($NodeEnv.ContainsKey("HOSTNAME")) { $NodeEnv["HOSTNAME"] } elseif ($NodeEnv.ContainsKey("HOST")) { $NodeEnv["HOST"] } else { "" }
$p2pPort = if ($NodeEnv.ContainsKey("P2P_PORT")) { $NodeEnv["P2P_PORT"] } else { "" }
$publicP2PPort = if ($NodeEnv.ContainsKey("PUBLIC_P2P_PORT")) { $NodeEnv["PUBLIC_P2P_PORT"] } elseif ($NodeEnv.ContainsKey("P2P_PORT_EXTERNAL")) { $NodeEnv["P2P_PORT_EXTERNAL"] } else { $p2pPort }
if (-not [string]::IsNullOrWhiteSpace($p2pPort)) { $env:SYNERGY_P2P_PORT = $p2pPort }
if (-not [string]::IsNullOrWhiteSpace($p2pPort)) {
  $env:SYNERGY_P2P_LISTEN_ADDRESS = "${bindIp}:$p2pPort"
} elseif ($NodeEnv.ContainsKey("P2P_LISTEN_ADDRESS")) {
  $env:SYNERGY_P2P_LISTEN_ADDRESS = $NodeEnv["P2P_LISTEN_ADDRESS"]
}
if (-not [string]::IsNullOrWhiteSpace($publicHost) -and -not [string]::IsNullOrWhiteSpace($publicP2PPort)) {
  $env:SYNERGY_P2P_EXTERNAL_ADDRESS = "${publicHost}:$publicP2PPort"
  $env:SYNERGY_P2P_PUBLIC_ADDRESS = "${publicHost}:$publicP2PPort"
} elseif ($NodeEnv.ContainsKey("P2P_EXTERNAL_ADDRESS")) {
  $env:SYNERGY_P2P_EXTERNAL_ADDRESS = $NodeEnv["P2P_EXTERNAL_ADDRESS"]
  $env:SYNERGY_P2P_PUBLIC_ADDRESS = $NodeEnv["P2P_EXTERNAL_ADDRESS"]
} elseif ($NodeEnv.ContainsKey("P2P_PUBLIC_ADDRESS")) {
  $env:SYNERGY_P2P_EXTERNAL_ADDRESS = $NodeEnv["P2P_PUBLIC_ADDRESS"]
  $env:SYNERGY_P2P_PUBLIC_ADDRESS = $NodeEnv["P2P_PUBLIC_ADDRESS"]
}
$discoveryPort = if ($NodeEnv.ContainsKey("DISCOVERY_PORT")) { $NodeEnv["DISCOVERY_PORT"] } else { "" }
$discoveryPublicPort = if ($NodeEnv.ContainsKey("DISCOVERY_PORT_EXTERNAL")) { $NodeEnv["DISCOVERY_PORT_EXTERNAL"] } else { $discoveryPort }
if (-not [string]::IsNullOrWhiteSpace($discoveryPort)) { $env:SYNERGY_DISCOVERY_PORT = $discoveryPort }
if (-not [string]::IsNullOrWhiteSpace($discoveryPort)) {
  $env:SYNERGY_DISCOVERY_LISTEN_ADDRESS = "${bindIp}:$discoveryPort"
} elseif ($NodeEnv.ContainsKey("DISCOVERY_LISTEN_ADDRESS")) {
  $env:SYNERGY_DISCOVERY_LISTEN_ADDRESS = $NodeEnv["DISCOVERY_LISTEN_ADDRESS"]
}
if (-not [string]::IsNullOrWhiteSpace($publicHost) -and -not [string]::IsNullOrWhiteSpace($discoveryPublicPort)) {
  $env:SYNERGY_DISCOVERY_EXTERNAL_ADDRESS = "${publicHost}:$discoveryPublicPort"
  $env:SYNERGY_DISCOVERY_PUBLIC_ADDRESS = "${publicHost}:$discoveryPublicPort"
} elseif ($NodeEnv.ContainsKey("DISCOVERY_EXTERNAL_ADDRESS")) {
  $env:SYNERGY_DISCOVERY_EXTERNAL_ADDRESS = $NodeEnv["DISCOVERY_EXTERNAL_ADDRESS"]
  $env:SYNERGY_DISCOVERY_PUBLIC_ADDRESS = $NodeEnv["DISCOVERY_EXTERNAL_ADDRESS"]
} elseif ($NodeEnv.ContainsKey("DISCOVERY_PUBLIC_ADDRESS")) {
  $env:SYNERGY_DISCOVERY_EXTERNAL_ADDRESS = $NodeEnv["DISCOVERY_PUBLIC_ADDRESS"]
  $env:SYNERGY_DISCOVERY_PUBLIC_ADDRESS = $NodeEnv["DISCOVERY_PUBLIC_ADDRESS"]
}

try {
  $proc = Start-Process -FilePath $BinPath -ArgumentList @("start", "--config", $ConfigPath) -WorkingDirectory $BaseDir -RedirectStandardOutput $OutFile -RedirectStandardError $ErrFile -PassThru -WindowStyle Hidden
}
finally {
  $env:SYNERGY_PROJECT_ROOT = $previousProjectRoot
  $env:SYNERGY_CONFIG_PATH = $previousConfigPath
  $env:SYNERGY_GENESIS_FILE = $previousGenesisPath
  $env:SYNERGY_BOOTSTRAP_ONLY = $previousBootstrapOnly
  $env:SYNERGY_AUTO_REGISTER_VALIDATOR = $previousAutoRegister
  $env:SYNERGY_P2P_PORT = $previousP2PPort
  $env:SYNERGY_P2P_LISTEN_ADDRESS = $previousP2PListenAddress
  $env:SYNERGY_P2P_EXTERNAL_ADDRESS = $previousP2PExternalAddress
  $env:SYNERGY_P2P_PUBLIC_ADDRESS = $previousP2PPublicAddress
  $env:SYNERGY_DISCOVERY_PORT = $previousDiscoveryPort
  $env:SYNERGY_DISCOVERY_LISTEN_ADDRESS = $previousDiscoveryListenAddress
  $env:SYNERGY_DISCOVERY_EXTERNAL_ADDRESS = $previousDiscoveryExternalAddress
  $env:SYNERGY_DISCOVERY_PUBLIC_ADDRESS = $previousDiscoveryPublicAddress
}

$proc.Id | Set-Content -Path $PidFile
"Started $($NodeEnv["MACHINE_ID"]) as bootstrap-only discovery node (PID $($proc.Id))"
SCRIPT

  cat > "$node_dir/nodectl.ps1" <<'SCRIPT'
$ErrorActionPreference = "Stop"

$BaseDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$EnvPath = Join-Path $BaseDir "node.env"
$NodeEnv = @{}
Get-Content $EnvPath | ForEach-Object {
  if ($_ -match '^\s*$' -or $_ -match '^\s*#') { return }
  $parts = $_ -split '=', 2
  if ($parts.Count -eq 2) { $NodeEnv[$parts[0].Trim()] = $parts[1].Trim() }
}

$PidFile = Join-Path $BaseDir "data/node.pid"
$OutFile = Join-Path $BaseDir "data/logs/node.out"

function Test-Running {
  if (-not (Test-Path $PidFile)) { return $false }
  $pid = Get-Content $PidFile
  return [bool](Get-Process -Id $pid -ErrorAction SilentlyContinue)
}

switch ($args[0]) {
  "start" { & (Join-Path $BaseDir "install_and_start.ps1") }
  "stop" {
    if (Test-Running) {
      $pid = Get-Content $PidFile
      Stop-Process -Id $pid -Force
      Remove-Item $PidFile -Force -ErrorAction SilentlyContinue
      "Stopped $($NodeEnv["MACHINE_ID"])"
    } else {
      "Node is not running"
    }
  }
  "status" {
    if (Test-Running) {
      "Running (PID $(Get-Content $PidFile))"
    } else {
      "Stopped"
    }
  }
  "logs" {
    if ($args.Count -gt 1 -and $args[1] -eq "--follow") {
      Get-Content $OutFile -Wait
    } else {
      Get-Content $OutFile -Tail 120
    }
  }
  default {
    "Usage: .\nodectl.ps1 <start|stop|status|logs>"
    exit 1
  }
}
SCRIPT

  chmod +x "$node_dir/install_and_start.sh" "$node_dir/nodectl.sh"
}

copy_bootnode_binaries() {
  local node_dir="$1"
  mkdir -p "$node_dir/bin"

  if [[ -n "$DARWIN_BINARY" ]]; then
    cp "$DARWIN_BINARY" "$node_dir/bin/synergy-testbeta-darwin-arm64"
    chmod +x "$node_dir/bin/synergy-testbeta-darwin-arm64"
  fi
  if [[ -n "$LINUX_BINARY" ]]; then
    cp "$LINUX_BINARY" "$node_dir/bin/synergy-testbeta-linux-amd64"
    chmod +x "$node_dir/bin/synergy-testbeta-linux-amd64"
  fi
  if [[ -n "$WINDOWS_BINARY" ]]; then
    cp "$WINDOWS_BINARY" "$node_dir/bin/synergy-testbeta-windows-amd64.exe"
  fi
}

copy_bootnode_genesis() {
  local node_dir="$1"

  if [[ ! -f "$GENESIS_FILE" ]]; then
    echo "Missing canonical genesis file: $GENESIS_FILE" >&2
    exit 1
  fi

  cp "$GENESIS_FILE" "$node_dir/config/genesis.json"
}

write_bootnode_readme() {
  local node_dir="$1"
  local name="$2"
  local ip="$3"
  local hostname p2p_port discovery_port
  hostname="$(bootnode_host "$name")"
  p2p_port="$(bootnode_p2p_port "$name")"
  discovery_port="$(bootnode_discovery_port "$name")"

  cat > "$node_dir/README.txt" <<EOF
${name} bootstrap-only deployment bundle
======================================

Purpose
- Runs a Synergy Testnet-Beta node in bootstrap-only mode.
- Discovery only: no validator self-registration, no consensus engine, no public RPC services.

Endpoint
- Hostname: ${hostname}
- IP: ${ip}
- P2P Port: ${p2p_port}
- Discovery Port: ${discovery_port}

Start
- Linux/macOS: ./install_and_start.sh
- Windows: powershell -ExecutionPolicy Bypass -File .\\install_and_start.ps1

Control
- Linux/macOS: ./nodectl.sh status | logs --follow | stop
- Windows: powershell -ExecutionPolicy Bypass -File .\\nodectl.ps1 status

Notes
- Open TCP ${p2p_port} and TCP/UDP ${discovery_port} on the target host firewall.
- Publish A record ${hostname} -> ${ip}
- Publish _dnsaddr.bootstrap TXT records from the root DNS_RECORDS.txt file in ${OUT_DIR}
- The bundle ships the canonical genesis file and resets stale local chain state on start.
EOF
}

write_bootnode_binary_status() {
  local node_dir="$1"

  cat > "$node_dir/BINARY_STATUS.txt" <<EOF
Darwin Binary:  ${DARWIN_BINARY:-missing}
Linux Binary:   ${LINUX_BINARY:-missing}
Windows Binary: ${WINDOWS_BINARY:-missing}
EOF
}

bootnodes_json() {
  local first=1
  local name ip hostname p2p_port
  for name in "${BOOTNODES[@]}"; do
    ip="$(bootnode_ip "$name")"
    hostname="$(bootnode_host "$name")"
    p2p_port="$(bootnode_p2p_port "$name")"
    if [[ $first -eq 0 ]]; then
      printf ',\n'
    fi
    first=0
    cat <<EOF
    {
      "name": "${name}",
      "hostname": "${hostname}",
      "ip": "${ip}",
      "port": ${p2p_port},
      "dial": "snr://bootstrap@${hostname}:${p2p_port}"
    }
EOF
  done
}

seeds_json() {
  local first=1
  local name ip hostname
  for name in "${SEEDS[@]}"; do
    ip="$(seed_ip "$name")"
    hostname="$(seed_host "$name")"
    if [[ $first -eq 0 ]]; then
      printf ',\n'
    fi
    first=0
    cat <<EOF
    {
      "name": "${name}",
      "hostname": "${hostname}",
      "ip": "${ip}",
      "http_port": ${SEED_HTTP_PORT},
      "url": "http://${hostname}:${SEED_HTTP_PORT}/peer-list.json"
    }
EOF
  done
}

write_seed_config() {
  local seed_dir="$1"
  local name="$2"
  local ip="$3"
  local hostname
  hostname="$(seed_host "$name")"

  cat > "$seed_dir/config/seed-service.json" <<EOF
{
  "service_name": "${name}",
  "domain": "${SEED_DOMAIN}",
  "listen_host": "0.0.0.0",
  "listen_port": ${SEED_HTTP_PORT},
  "public_url": "http://${hostname}:${SEED_HTTP_PORT}",
  "bind_ip_hint": "${ip}",
  "refresh_seconds": 30,
  "bootnodes": [
$(bootnodes_json)
  ],
  "seed_services": [
$(seeds_json)
  ]
}
EOF
}

write_seed_service() {
  local seed_dir="$1"

  cat > "$seed_dir/seed_service.py" <<'SCRIPT'
#!/usr/bin/env python3
import hmac
import ipaddress
import json
import os
import socket
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlparse


BASE_DIR = Path(__file__).resolve().parent
CONFIG_PATH = BASE_DIR / "config" / "seed-service.json"
PEER_PATH = BASE_DIR / "data" / "peers.json"
PEER_LOCK = threading.Lock()
STATE = {"generated_at": None, "bootnodes": [], "peers": [], "peer_updated_at": None}


def load_config():
    return json.loads(CONFIG_PATH.read_text())


def load_peers():
    if not PEER_PATH.exists():
        return []
    try:
        raw = json.loads(PEER_PATH.read_text())
    except (OSError, json.JSONDecodeError):
        return []
    if isinstance(raw, dict):
        raw = raw.get("peers", [])
    if not isinstance(raw, list):
        return []
    return dedupe_peer_records(raw)


def save_peers(peers):
    PEER_PATH.parent.mkdir(parents=True, exist_ok=True)
    PEER_PATH.write_text(json.dumps(dedupe_peer_records(peers), indent=2))


def parse_int(value):
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def is_plausible_dial_host(host):
    normalized = str(host or "").strip()
    if not normalized:
        return False
    if normalized.lower() == "localhost":
        return True
    try:
        ipaddress.ip_address(normalized)
        return True
    except ValueError:
        pass
    return "." in normalized and all(
        character.isalnum() or character in "-." for character in normalized
    )


def normalize_host_port(host, port):
    normalized_host = str(host or "").strip().strip("[]").rstrip(".")
    normalized_port = parse_int(port)
    if (
        normalized_port is None
        or normalized_port <= 0
        or normalized_port > 65535
        or not normalized_host
        or not is_plausible_dial_host(normalized_host)
    ):
        return None
    try:
        ip_value = ipaddress.ip_address(normalized_host)
    except ValueError:
        if ":" in normalized_host:
            return f"[{normalized_host}]:{normalized_port}"
        return f"{normalized_host}:{normalized_port}"

    if isinstance(ip_value, ipaddress.IPv6Address):
        return f"[{normalized_host}]:{normalized_port}"
    return f"{normalized_host}:{normalized_port}"


def normalize_dial_target(value):
    raw = str(value or "").strip()
    if not raw:
        return None

    if "://" in raw:
        raw = raw.split("://", 1)[1]
    if "@" in raw:
        raw = raw.rsplit("@", 1)[1]
    raw = raw.split("/", 1)[0]
    raw = raw.split("?", 1)[0]
    raw = raw.split("#", 1)[0]
    raw = raw.strip()
    if not raw:
        return None

    if raw.startswith("[") and "]:" in raw:
        host, port = raw[1:].rsplit("]:", 1)
        return normalize_host_port(host, port)
    if ":" not in raw:
        return None
    host, port = raw.rsplit(":", 1)
    return normalize_host_port(host, port)


def normalize_peer_payload(payload):
    if not isinstance(payload, dict):
        return None
    dial = payload.get("dial") or payload.get("peer") or payload.get("address")
    host = payload.get("public_host") or payload.get("host") or payload.get("hostname")
    port = payload.get("p2p_port") or payload.get("port")
    if not dial and host and port:
        dial = f"{host}:{port}"
    dial = normalize_dial_target(dial)
    if not dial:
        return None
    dial_host = dial[1:].split("]:", 1)[0] if dial.startswith("[") else dial.rsplit(":", 1)[0]
    dial_port = parse_int(dial.rsplit(":", 1)[1])
    updated_at = parse_int(payload.get("updated_at")) or int(time.time())
    return {
        "node_id": payload.get("node_id"),
        "role_id": payload.get("role_id"),
        "wallet_address": payload.get("wallet_address"),
        "public_host": str(host or dial_host).strip() or dial_host,
        "p2p_port": dial_port,
        "dial": dial,
        "updated_at": updated_at,
    }


def merge_peer(peers, incoming):
    key = incoming.get("node_id") or incoming.get("dial")
    for idx, peer in enumerate(peers):
        peer_key = peer.get("node_id") or peer.get("dial")
        if peer_key == key:
            merged = dict(peer)
            merged.update(incoming)
            peers[idx] = merged
            return peers
    peers.append(incoming)
    return peers


def dedupe_peer_records(records):
    peers = []
    for entry in records:
        normalized = normalize_peer_payload(entry)
        if not normalized:
            continue
        peers = merge_peer(peers, normalized)
    return peers


def peer_dials(peers):
    dials = {
        entry.get("dial")
        for entry in dedupe_peer_records(peers)
        if entry.get("dial")
    }
    return sorted(dials)


def expected_admin_token(config):
    token = str(os.environ.get("SEED_ADMIN_TOKEN") or config.get("admin_token") or "").strip()
    return token or None


def request_is_loopback(handler):
    try:
        return ipaddress.ip_address(handler.client_address[0]).is_loopback
    except ValueError:
        return handler.client_address[0] in {"localhost"}


def request_admin_token(handler):
    bearer = str(handler.headers.get("Authorization") or "").strip()
    if bearer.lower().startswith("bearer "):
        return bearer[7:].strip()
    return str(handler.headers.get("X-Seed-Admin-Token") or "").strip()


def is_admin_authorized(handler, config):
    if request_is_loopback(handler):
        return True
    expected = expected_admin_token(config)
    provided = request_admin_token(handler)
    if not expected or not provided:
        return False
    return hmac.compare_digest(provided, expected)


def check_bootnode(host, port, timeout=1.5):
    started = time.time()
    try:
        with socket.create_connection((host, port), timeout=timeout):
            latency_ms = int((time.time() - started) * 1000)
            return {"reachable": True, "latency_ms": latency_ms}
    except OSError as exc:
        return {"reachable": False, "error": str(exc)}


def rebuild_state(config):
    snapshot = []
    for entry in config["bootnodes"]:
        status = check_bootnode(entry["hostname"], entry["port"])
        merged = dict(entry)
        merged.update(status)
        snapshot.append(merged)

    STATE["generated_at"] = int(time.time())
    STATE["bootnodes"] = snapshot
    with PEER_LOCK:
        peers = load_peers()
        STATE["peers"] = peers
        STATE["peer_updated_at"] = int(time.time())


def refresh_loop(config):
    interval = max(int(config.get("refresh_seconds", 30)), 5)
    while True:
        rebuild_state(config)
        time.sleep(interval)


class Handler(BaseHTTPRequestHandler):
    def _send_json(self, payload, status=200):
        body = json.dumps(payload, indent=2).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _clear_registered_peers(self, config):
        if not is_admin_authorized(self, config):
            self._send_json({"error": "forbidden"}, status=403)
            return

        with PEER_LOCK:
            cleared = len(load_peers())
            peers = []
            save_peers(peers)
            STATE["peers"] = peers
            STATE["peer_updated_at"] = int(time.time())

        self._send_json(
            {
                "ok": True,
                "cleared": cleared,
                "remaining": 0,
            }
        )

    def log_message(self, fmt, *args):
        return

    def do_GET(self):
        config = load_config()
        path = urlparse(self.path).path

        if path == "/" or path == "/healthz":
            self._send_json(
                {
                    "ok": True,
                    "service": config["service_name"],
                    "generated_at": STATE["generated_at"],
                }
            )
            return

        if path == "/peer-list.json":
            self._send_json(
                {
                    "service": config["service_name"],
                    "public_url": config["public_url"],
                    "generated_at": STATE["generated_at"],
                    "bootnodes": STATE["bootnodes"],
                    "seed_services": config["seed_services"],
                    "peers": peer_dials(STATE["peers"]),
                    "dnsaddr_bootstrap": [
                        f"dnsaddr=/dns/{entry['hostname']}/tcp/{entry['port']}"
                        for entry in config["bootnodes"]
                    ],
                }
            )
            return

        if path == "/dns/bootstrap.txt":
            lines = [
                f"dnsaddr=/dns/{entry['hostname']}/tcp/{entry['port']}"
                for entry in config["bootnodes"]
            ]
            body = ("\n".join(lines) + "\n").encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "text/plain; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return

        if path == "/peers":
            self._send_json(
                {
                    "service": config["service_name"],
                    "generated_at": STATE["generated_at"],
                    "peer_updated_at": STATE["peer_updated_at"],
                    "peers": dedupe_peer_records(STATE["peers"]),
                }
            )
            return

        self._send_json({"error": "not_found"}, status=404)

    def do_POST(self):
        config = load_config()
        path = urlparse(self.path).path
        if path in {"/peers/clear", "/admin/peers/clear"}:
            self._clear_registered_peers(config)
            return

        if path != "/peers/register":
            self._send_json({"error": "not_found"}, status=404)
            return

        length = int(self.headers.get("Content-Length", 0) or 0)
        if length <= 0:
            self._send_json({"error": "missing_body"}, status=400)
            return

        try:
            payload = json.loads(self.rfile.read(length))
        except json.JSONDecodeError:
            self._send_json({"error": "invalid_json"}, status=400)
            return

        registrations = payload if isinstance(payload, list) else [payload]
        updated = 0
        with PEER_LOCK:
            peers = load_peers()
            for entry in registrations:
                normalized = normalize_peer_payload(entry)
                if not normalized:
                    continue
                peers = merge_peer(peers, normalized)
                updated += 1
            save_peers(peers)
            STATE["peers"] = dedupe_peer_records(peers)
            STATE["peer_updated_at"] = int(time.time())

        if updated == 0:
            self._send_json({"error": "invalid_payload"}, status=400)
            return

        self._send_json(
            {
                "ok": True,
                "registered": updated,
                "peers": dedupe_peer_records(STATE["peers"]),
            }
        )

    def do_DELETE(self):
        config = load_config()
        path = urlparse(self.path).path
        if path in {"/peers", "/peers/clear", "/admin/peers/clear"}:
            self._clear_registered_peers(config)
            return
        self._send_json({"error": "not_found"}, status=404)


def main():
    config = load_config()
    rebuild_state(config)
    thread = threading.Thread(target=refresh_loop, args=(config,), daemon=True)
    thread.start()

    server = ThreadingHTTPServer((config["listen_host"], config["listen_port"]), Handler)
    print(
        f"Seed service {config['service_name']} listening on "
        f"{config['listen_host']}:{config['listen_port']}"
    )
    server.serve_forever()


if __name__ == "__main__":
    main()
SCRIPT
  chmod +x "$seed_dir/seed_service.py"
}

write_seed_env() {
  local seed_dir="$1"
  local name="$2"
  local ip="$3"
  local hostname
  hostname="$(seed_host "$name")"

  cat > "$seed_dir/node.env" <<EOF
MACHINE_ID=${name}
NODE_KIND=seed-service
SERVICE_NAME=${name}
SERVICE_HOSTNAME=${hostname}
SERVICE_IP=${ip}
SERVICE_PORT=${SEED_HTTP_PORT}
# Optional remote admin token for clearing peer registrations.
# Uncomment and set before start if you need authenticated DELETE /peers access.
# export SEED_ADMIN_TOKEN=replace-with-a-long-random-token
EOF
}

write_seed_scripts() {
  local seed_dir="$1"

  cat > "$seed_dir/install_and_start.sh" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail

BASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$BASE_DIR/node.env"

PID_FILE="$BASE_DIR/data/seed.pid"
OUT_FILE="$BASE_DIR/data/logs/seed.out"
PYTHON_BIN="${PYTHON_BIN:-python3}"

if ! command -v "$PYTHON_BIN" >/dev/null 2>&1; then
  if command -v python >/dev/null 2>&1; then
    PYTHON_BIN="python"
  else
    echo "Python is required to run the seed service." >&2
    exit 1
  fi
fi

if [[ "$(uname -s)" == "Darwin" ]] && command -v xattr >/dev/null 2>&1; then
  xattr -dr com.apple.quarantine "$BASE_DIR" 2>/dev/null || true
fi

if [[ -f "$PID_FILE" ]]; then
  pid="$(cat "$PID_FILE")"
  if kill -0 "$pid" 2>/dev/null; then
    echo "$MACHINE_ID already running with PID $pid"
    exit 0
  fi
fi

mkdir -p "$BASE_DIR/data/logs"
nohup "$PYTHON_BIN" "$BASE_DIR/seed_service.py" >"$OUT_FILE" 2>&1 &
echo $! > "$PID_FILE"
echo "Started $MACHINE_ID peer-list publisher on port $SERVICE_PORT (PID $(cat "$PID_FILE"))"
SCRIPT

  cat > "$seed_dir/nodectl.sh" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail

BASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$BASE_DIR/node.env"

PID_FILE="$BASE_DIR/data/seed.pid"
OUT_FILE="$BASE_DIR/data/logs/seed.out"

is_running() {
  if [[ -f "$PID_FILE" ]]; then
    local pid
    pid="$(cat "$PID_FILE")"
    if kill -0 "$pid" 2>/dev/null; then
      return 0
    fi
  fi
  return 1
}

case "${1:-}" in
  start)
    "$BASE_DIR/install_and_start.sh"
    ;;
  stop)
    if is_running; then
      pid="$(cat "$PID_FILE")"
      kill "$pid" 2>/dev/null || true
      rm -f "$PID_FILE"
      echo "Stopped $MACHINE_ID"
    else
      echo "$MACHINE_ID is not running"
    fi
    ;;
  status)
    if is_running; then
      echo "$MACHINE_ID is running (PID $(cat "$PID_FILE"))"
    else
      echo "$MACHINE_ID is stopped"
    fi
    ;;
  logs)
    if [[ "${2:-}" == "--follow" ]]; then
      tail -f "$OUT_FILE"
    else
      tail -n 120 "$OUT_FILE"
    fi
    ;;
  info)
    cat <<INFO
Machine ID: $MACHINE_ID
Role: $NODE_KIND
Hostname: $SERVICE_HOSTNAME
IP: $SERVICE_IP
HTTP Port: $SERVICE_PORT
Peer List: http://$SERVICE_HOSTNAME:$SERVICE_PORT/peer-list.json
INFO
    ;;
  *)
    echo "Usage: $0 <start|stop|status|logs|info>" >&2
    exit 1
    ;;
esac
SCRIPT

  cat > "$seed_dir/install_and_start.ps1" <<'SCRIPT'
$ErrorActionPreference = "Stop"
$BaseDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Py = Get-Command python3 -ErrorAction SilentlyContinue
if (-not $Py) { $Py = Get-Command python -ErrorAction SilentlyContinue }
if (-not $Py) { throw "Python is required to run the seed service." }

$DataDir = Join-Path $BaseDir "data"
$LogsDir = Join-Path $DataDir "logs"
$PidFile = Join-Path $DataDir "seed.pid"
$OutFile = Join-Path $LogsDir "seed.out"
$ErrFile = Join-Path $LogsDir "seed.err"

New-Item -ItemType Directory -Force -Path $LogsDir | Out-Null
$proc = Start-Process -FilePath $Py.Source -ArgumentList @((Join-Path $BaseDir "seed_service.py")) -WorkingDirectory $BaseDir -RedirectStandardOutput $OutFile -RedirectStandardError $ErrFile -PassThru -WindowStyle Hidden
$proc.Id | Set-Content -Path $PidFile
"Started seed service PID $($proc.Id)"
SCRIPT

  cat > "$seed_dir/nodectl.ps1" <<'SCRIPT'
$ErrorActionPreference = "Stop"

$BaseDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$PidFile = Join-Path $BaseDir "data/seed.pid"
$OutFile = Join-Path $BaseDir "data/logs/seed.out"

function Test-Running {
  if (-not (Test-Path $PidFile)) { return $false }
  $pid = Get-Content $PidFile
  return [bool](Get-Process -Id $pid -ErrorAction SilentlyContinue)
}

switch ($args[0]) {
  "start" { & (Join-Path $BaseDir "install_and_start.ps1") }
  "stop" {
    if (Test-Running) {
      $pid = Get-Content $PidFile
      Stop-Process -Id $pid -Force
      Remove-Item $PidFile -Force -ErrorAction SilentlyContinue
      "Stopped seed service"
    } else {
      "Seed service is not running"
    }
  }
  "status" {
    if (Test-Running) {
      "Running (PID $(Get-Content $PidFile))"
    } else {
      "Stopped"
    }
  }
  "logs" {
    if ($args.Count -gt 1 -and $args[1] -eq "--follow") {
      Get-Content $OutFile -Wait
    } else {
      Get-Content $OutFile -Tail 120
    }
  }
  default {
    "Usage: .\nodectl.ps1 <start|stop|status|logs>"
    exit 1
  }
}
SCRIPT

  chmod +x "$seed_dir/install_and_start.sh" "$seed_dir/nodectl.sh"
}

write_seed_readme() {
  local seed_dir="$1"
  local name="$2"
  local ip="$3"
  local hostname
  hostname="$(seed_host "$name")"

  cat > "$seed_dir/README.txt" <<EOF
${name} seed-service deployment bundle
====================================

Purpose
- Runs a lightweight HTTP publisher for bootstrap metadata.
- This is not a validator, relayer, or P2P node.

Endpoint
- Hostname: ${hostname}
- IP: ${ip}
- HTTP Port: ${SEED_HTTP_PORT}

Published endpoints
- /healthz
- /peer-list.json
- /dns/bootstrap.txt
- /peers
- /peers/register
- /peers/clear (admin)

Start
- Linux/macOS: ./install_and_start.sh
- Windows: powershell -ExecutionPolicy Bypass -File .\\install_and_start.ps1

Clear registered peers
- Local only without a token: curl -X DELETE http://127.0.0.1:${SEED_HTTP_PORT}/peers
- Remote with token: curl -X DELETE -H "X-Seed-Admin-Token: <token>" http://${hostname}:${SEED_HTTP_PORT}/peers

DNS
- Publish A record ${hostname} -> ${ip}
- Optional SRV record: _synergy-seed._tcp.${SEED_DOMAIN} -> ${hostname}:${SEED_HTTP_PORT}
EOF
}

write_root_files() {
  local bootstrap_rows=""
  local bootnode_checks=""
  local service_rows=""
  local service_checks=""
  local dns_a_records=""
  local dns_txt_records=""
  local name host ip p2p_port discovery_port
  local rpc_host rpc_ip rpc_p2p rpc_rpc rpc_ws rpc_discovery
  local indexer_host indexer_ip indexer_p2p indexer_rpc indexer_ws indexer_discovery

  for name in "${BOOTNODES[@]}"; do
    host="$(bootnode_host "$name")"
    ip="$(bootnode_ip "$name")"
    p2p_port="$(bootnode_p2p_port "$name")"
    discovery_port="$(bootnode_discovery_port "$name")"
    bootstrap_rows+="| ${name} | ${host} | ${ip} | ${p2p_port}/tcp | ${discovery_port}/tcp,udp |"$'\n'
    bootnode_checks+="nc -zv ${host} ${p2p_port}"$'\n'
    dns_a_records+="${host} -> ${ip}"$'\n'
    dns_txt_records+="_dnsaddr.bootstrap.${DISCOVERY_DOMAIN} -> \"dnsaddr=/dns/${host}/tcp/${p2p_port}\""$'\n'
  done

  rpc_host="$(installer_env_value "$RPC_INSTALLER_DIR" "HOSTNAME")"
  rpc_ip="$(installer_env_value "$RPC_INSTALLER_DIR" "PUBLIC_IP")"
  rpc_p2p="$(installer_env_value "$RPC_INSTALLER_DIR" "P2P_PORT")"
  rpc_rpc="$(installer_env_value "$RPC_INSTALLER_DIR" "RPC_PORT")"
  rpc_ws="$(installer_env_value "$RPC_INSTALLER_DIR" "WS_PORT")"
  rpc_discovery="$(installer_env_value "$RPC_INSTALLER_DIR" "DISCOVERY_PORT")"
  indexer_host="$(installer_env_value "$INDEXER_INSTALLER_DIR" "HOSTNAME")"
  indexer_ip="$(installer_env_value "$INDEXER_INSTALLER_DIR" "PUBLIC_IP")"
  indexer_p2p="$(installer_env_value "$INDEXER_INSTALLER_DIR" "P2P_PORT")"
  indexer_rpc="$(installer_env_value "$INDEXER_INSTALLER_DIR" "RPC_PORT")"
  indexer_ws="$(installer_env_value "$INDEXER_INSTALLER_DIR" "WS_PORT")"
  indexer_discovery="$(installer_env_value "$INDEXER_INSTALLER_DIR" "DISCOVERY_PORT")"

  service_rows+="| ${RPC_GATEWAY_BUNDLE_NAME} | ${rpc_host} | ${rpc_ip} | ${rpc_p2p}/tcp, ${rpc_rpc}/tcp, ${rpc_ws}/tcp, ${rpc_discovery}/tcp,udp | rpc-gateway installer bundle |"$'\n'
  service_rows+="| ${INDEXER_BUNDLE_NAME} | ${indexer_host} | ${indexer_ip} | ${indexer_p2p}/tcp, ${indexer_rpc}/tcp, ${indexer_ws}/tcp, ${indexer_discovery}/tcp,udp | explorer/indexer installer bundle |"$'\n'
  service_checks+="nc -zv ${rpc_host} ${rpc_p2p}"$'\n'
  service_checks+="nc -zv ${rpc_host} ${rpc_rpc}"$'\n'
  service_checks+="nc -zv ${indexer_host} ${indexer_p2p}"$'\n'
  service_checks+="nc -zv ${indexer_host} ${indexer_rpc}"$'\n'
  dns_a_records+="${rpc_host} -> ${rpc_ip}"$'\n'
  dns_a_records+="${indexer_host} -> ${indexer_ip}"$'\n'

  cat > "$OUT_DIR/DEPLOYMENT_GUIDE.md" <<EOF
# Synergy Testnet-Beta Bootstrap Deployment Guide

## Launch Baseline

- Chain ID: 338639
- Network ID: synergy-testnet-beta
- Token symbol: SNRG
- Genesis validators: ${GENESIS_VALIDATOR_COUNT}
- Minimum active validators to start consensus: ${MIN_GENESIS_VALIDATORS}
- Validator vote threshold: ${VALIDATOR_VOTE_THRESHOLD}
- Bootnodes: 3
- Service bundles: 2

## Assigned Deployment Bundles

| Role | Hostname | IP | Port | Notes |
| --- | --- | --- | --- | --- |
${bootstrap_rows}${service_rows}

## Port Freeze

| Purpose | Value |
| --- | --- |
| Bootnode listener | Per bootnode \`P2P_PORT\` /tcp |
| Bootnode discovery | Per bootnode \`DISCOVERY_PORT\` /tcp,udp |
| Validator P2P listener | 5622 |
| Validator RPC | 5640 |
| Validator WS | 5660 |
| Validator discovery | 5680 |
| Validator metrics | 6030 |

## Bootnode Deployment

1. Download the assigned bootnode bundle from the Genesis Dashboard.
2. Transfer the bundle to the target host.
3. Extract the archive on the target host.
4. Open the assigned bootnode P2P TCP port and discovery TCP/UDP port on the host firewall.
5. Confirm the A record for the assigned hostname points to the target IP.
6. Start the bundle with \`./install_and_start.sh\` on Linux or macOS, or \`install_and_start.ps1\` on Windows.
7. Confirm the process is running with \`./nodectl.sh status\` or \`nodectl.ps1 status\`.

## Service Bundle Deployment

1. Download the assigned service bundle from the Genesis Dashboard.
2. Transfer the bundle to the target host.
3. Extract the archive on the target host.
4. Open the listed P2P, RPC, WS, and discovery ports on the host firewall.
5. Confirm the A record for the assigned hostname points to the target IP.
6. Start the installer with \`./install_and_start.sh\` on Linux or macOS, or \`install_and_start.ps1\` on Windows.
7. Confirm the process is running with \`./nodectl.sh status\` or \`nodectl.ps1 status\`.

## Verification

Run these checks after the assigned bundle is started.

\`\`\`bash
# Bootnode reachability
${bootnode_checks}

# Service reachability
${service_checks}
\`\`\`

## DNS

Use the exact records in \`DNS_RECORDS.txt\`.
EOF

  cat > "$OUT_DIR/DNS_RECORDS.txt" <<EOF
Required A records
${dns_a_records}

Required TXT records for bootnode discovery
${dns_txt_records}
EOF

  cat > "$OUT_DIR/README.txt" <<EOF
Bootstrap bundles
=================

Contents
- bootnode1, bootnode2, bootnode3: bootstrap-only Synergy node bundles
- ${RPC_GATEWAY_BUNDLE_NAME}: rpc-gateway installer bundle with canonical genesis.json
- ${INDEXER_BUNDLE_NAME}: explorer/indexer installer bundle with canonical genesis.json
- DNS_RECORDS.txt: DNS records to create in Cloudflare or another DNS provider

How to rebuild
1. Populate ${BINARIES_DIR} with synergy-testbeta binaries for darwin-arm64, linux-amd64, and windows-amd64.
2. Run: ./scripts/testbeta/build-bootstrap-bundles.sh

Output directory
- ${OUT_DIR}
EOF
}

build_bootnode_bundle() {
  local name="$1"
  local ip="$2"
  local node_dir="$OUT_DIR/$name"

  rm -rf "$node_dir"
  mkdir -p "$node_dir/config" "$node_dir/data/logs"
  write_bootnode_config "$node_dir" "$name"
  copy_bootnode_genesis "$node_dir"
  write_bootnode_env "$node_dir" "$name" "$ip"
  write_bootnode_scripts "$node_dir"
  copy_bootnode_binaries "$node_dir"
  write_bootnode_readme "$node_dir" "$name" "$ip"
  write_bootnode_binary_status "$node_dir"
}

copy_installer_bundle() {
  local bundle_name="$1"
  local source_dir="$2"
  local dest_dir="$OUT_DIR/$bundle_name"

  if [[ ! -d "$source_dir" ]]; then
    echo "Missing installer bundle source: $source_dir" >&2
    exit 1
  fi

  rm -rf "$dest_dir"
  cp -R "$source_dir" "$dest_dir"
}

main() {
  mkdir -p "$OUT_DIR"
  resolve_binaries
  if [[ ! -f "$GENESIS_FILE" ]]; then
    echo "Missing canonical genesis file: $GENESIS_FILE" >&2
    exit 1
  fi

  rm -rf \
    "$OUT_DIR/seed1" \
    "$OUT_DIR/seed2" \
    "$OUT_DIR/seed3" \
    "$OUT_DIR/bootseed2" \
    "$OUT_DIR/rpc-gateway" \
    "$OUT_DIR/indexer-explorer" \
    "$OUT_DIR/Bootstrap2" \
    "$OUT_DIR/Bootstrap3"

  local name ip
  for name in "${BOOTNODES[@]}"; do
    ip="$(bootnode_ip "$name")"
    build_bootnode_bundle "$name" "$ip"
  done

  copy_installer_bundle "$RPC_GATEWAY_BUNDLE_NAME" "$RPC_INSTALLER_DIR"
  copy_installer_bundle "$INDEXER_BUNDLE_NAME" "$INDEXER_INSTALLER_DIR"

  write_root_files

  echo "Bootstrap bundles generated in $OUT_DIR"
}

main "$@"
