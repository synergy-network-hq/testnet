#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="${BOOTSTRAP_OUT_DIR:-$ROOT_DIR/bootstrap-bundles}"
BINARIES_DIR="${BOOTSTRAP_BINARIES_DIR:-$ROOT_DIR/binaries}"
DOMAIN="${BOOTSTRAP_DOMAIN:-synergynode.xyz}"
P2P_PORT="${BOOTSTRAP_P2P_PORT:-5620}"
SEED_HTTP_PORT="${BOOTSTRAP_SEED_HTTP_PORT:-5621}"
GENESIS_VALIDATOR_COUNT="${BOOTSTRAP_GENESIS_VALIDATOR_COUNT:-4}"

BOOTNODE_RPC_PORT=5730
BOOTNODE_WS_PORT=5830
BOOTNODE_GRPC_PORT=50051
BOOTNODE_DISCOVERY_PORT=5930

BOOTNODES=(
  "bootnode1|74.208.227.23"
  "bootnode2|73.79.66.255"
  "bootnode3|64.227.107.57"
)

SEEDS=(
  "seed1|74.208.227.23"
  "seed2|73.79.66.255"
  "seed3|64.227.107.57"
)

DARWIN_BINARY=""
LINUX_BINARY=""
WINDOWS_BINARY=""

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
  local record name
  for record in "${BOOTNODES[@]}"; do
    IFS='|' read -r name _ <<<"$record"
    if [[ "$name" == "$current" ]]; then
      continue
    fi
    entries+=("\"snr://bootstrap@${name}.${DOMAIN}:${P2P_PORT}\"")
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
  local record name
  for record in "${BOOTNODES[@]}"; do
    IFS='|' read -r name _ <<<"$record"
    if [[ "$name" == "$current" ]]; then
      continue
    fi
    entries+=("snr://bootstrap@${name}.${DOMAIN}:${P2P_PORT}")
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

  cat > "$node_dir/config/node.toml" <<EOF
[network]
id = 338639
name = "Synergy Testnet Beta"
p2p_port = ${P2P_PORT}
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
epoch_length = 30000
min_validators = ${GENESIS_VALIDATOR_COUNT}
validator_cluster_size = ${GENESIS_VALIDATOR_COUNT}
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
listen_address = "0.0.0.0:${P2P_PORT}"
public_address = "${name}.${DOMAIN}:${P2P_PORT}"
node_name = "${name}"
enable_discovery = true
discovery_port = ${BOOTNODE_DISCOVERY_PORT}
heartbeat_interval = 30

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

  cat > "$node_dir/node.env" <<EOF
MACHINE_ID=${name}
NODE_KIND=bootnode
NODE_NAME=${name}
NODE_HOSTNAME=${name}.${DOMAIN}
NODE_PUBLIC_IP=${ip}
P2P_PORT=${P2P_PORT}
RPC_PORT=${BOOTNODE_RPC_PORT}
WS_PORT=${BOOTNODE_WS_PORT}
GRPC_PORT=${BOOTNODE_GRPC_PORT}
DISCOVERY_PORT=${BOOTNODE_DISCOVERY_PORT}
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

if [[ -f "$PID_FILE" ]]; then
  pid="$(cat "$PID_FILE")"
  if kill -0 "$pid" 2>/dev/null; then
    echo "$MACHINE_ID already running with PID $pid"
    exit 0
  fi
fi

mkdir -p "$BASE_DIR/data/logs" "$BASE_DIR/data/chain"
BIN_SELECTED="$(select_binary)"
if [[ ! -f "$BIN_SELECTED" ]]; then
  echo "Missing binary: $BIN_SELECTED" >&2
  exit 1
fi

clear_quarantine_if_needed
chmod +x "$BIN_SELECTED"
nohup env \
  SYNERGY_BOOTSTRAP_ONLY=true \
  SYNERGY_AUTO_REGISTER_VALIDATOR=false \
  "$BIN_SELECTED" start --config "$BASE_DIR/config/node.toml" >"$OUT_FILE" 2>&1 &
echo $! > "$PID_FILE"
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
$DataDir = Join-Path $BaseDir "data"
$LogsDir = Join-Path $DataDir "logs"
$PidFile = Join-Path $DataDir "node.pid"
$OutFile = Join-Path $LogsDir "node.out"
$ErrFile = Join-Path $LogsDir "node.err"

New-Item -ItemType Directory -Force -Path $LogsDir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $DataDir "chain") | Out-Null

if (-not (Test-Path $BinPath)) { throw "Missing Windows binary: $BinPath" }

$proc = Start-Process -FilePath $BinPath -ArgumentList @("start", "--config", $ConfigPath) -WorkingDirectory $BaseDir -RedirectStandardOutput $OutFile -RedirectStandardError $ErrFile -PassThru -WindowStyle Hidden
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

write_bootnode_readme() {
  local node_dir="$1"
  local name="$2"
  local ip="$3"

  cat > "$node_dir/README.txt" <<EOF
${name} bootstrap-only deployment bundle
======================================

Purpose
- Runs a Synergy Testnet Beta node in bootstrap-only mode.
- Discovery only: no validator self-registration, no consensus engine, no public RPC services.

Endpoint
- Hostname: ${name}.${DOMAIN}
- IP: ${ip}
- P2P Port: ${P2P_PORT}

Start
- Linux/macOS: ./install_and_start.sh
- Windows: powershell -ExecutionPolicy Bypass -File .\\install_and_start.ps1

Control
- Linux/macOS: ./nodectl.sh status | logs --follow | stop
- Windows: powershell -ExecutionPolicy Bypass -File .\\nodectl.ps1 status

Notes
- Open TCP ${P2P_PORT} on the target host firewall.
- Publish A record ${name}.${DOMAIN} -> ${ip}
- Publish _dnsaddr.bootstrap TXT records from the root DNS_RECORDS.txt file in ${OUT_DIR}
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
  local record name ip
  for record in "${BOOTNODES[@]}"; do
    IFS='|' read -r name ip <<<"$record"
    if [[ $first -eq 0 ]]; then
      printf ',\n'
    fi
    first=0
    cat <<EOF
    {
      "name": "${name}",
      "hostname": "${name}.${DOMAIN}",
      "ip": "${ip}",
      "port": ${P2P_PORT},
      "dial": "snr://bootstrap@${name}.${DOMAIN}:${P2P_PORT}"
    }
EOF
  done
}

seeds_json() {
  local first=1
  local record name ip
  for record in "${SEEDS[@]}"; do
    IFS='|' read -r name ip <<<"$record"
    if [[ $first -eq 0 ]]; then
      printf ',\n'
    fi
    first=0
    cat <<EOF
    {
      "name": "${name}",
      "hostname": "${name}.${DOMAIN}",
      "ip": "${ip}",
      "http_port": ${SEED_HTTP_PORT},
      "url": "http://${name}.${DOMAIN}:${SEED_HTTP_PORT}/peer-list.json"
    }
EOF
  done
}

write_seed_config() {
  local seed_dir="$1"
  local name="$2"
  local ip="$3"

  cat > "$seed_dir/config/seed-service.json" <<EOF
{
  "service_name": "${name}",
  "domain": "${DOMAIN}",
  "listen_host": "0.0.0.0",
  "listen_port": ${SEED_HTTP_PORT},
  "public_url": "http://${name}.${DOMAIN}:${SEED_HTTP_PORT}",
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

  cat > "$seed_dir/node.env" <<EOF
MACHINE_ID=${name}
NODE_KIND=seed-service
SERVICE_NAME=${name}
SERVICE_HOSTNAME=${name}.${DOMAIN}
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

  cat > "$seed_dir/README.txt" <<EOF
${name} seed-service deployment bundle
====================================

Purpose
- Runs a lightweight HTTP publisher for bootstrap metadata.
- This is not a validator, relayer, or P2P node.

Endpoint
- Hostname: ${name}.${DOMAIN}
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
- Remote with token: curl -X DELETE -H "X-Seed-Admin-Token: <token>" http://${name}.${DOMAIN}:${SEED_HTTP_PORT}/peers

DNS
- Publish A record ${name}.${DOMAIN} -> ${ip}
- Optional SRV record: _synergy-seed._tcp.${DOMAIN} -> ${name}.${DOMAIN}:${SEED_HTTP_PORT}
EOF
}

write_root_files() {
  cat > "$OUT_DIR/DEPLOYMENT_GUIDE.md" <<EOF
# Synergy Testnet-Beta Bootstrap Deployment Guide

## Launch Baseline

- Chain ID: 338639
- Network ID: synergy-testnet-beta
- Token symbol: SNRG
- Genesis validators: ${GENESIS_VALIDATOR_COUNT}
- Bootnodes: 3
- Seed services: 3

## Assigned Bootstrap Hosts

| Role | Hostname | IP | Port |
| --- | --- | --- | --- |
| bootnode1 | bootnode1.${DOMAIN} | 74.208.227.23 | ${P2P_PORT}/tcp |
| bootnode2 | bootnode2.${DOMAIN} | 73.79.66.255 | ${P2P_PORT}/tcp |
| bootnode3 | bootnode3.${DOMAIN} | 64.227.107.57 | ${P2P_PORT}/tcp |
| seed1 | seed1.${DOMAIN} | 74.208.227.23 | ${SEED_HTTP_PORT}/tcp |
| seed2 | seed2.${DOMAIN} | 73.79.66.255 | ${SEED_HTTP_PORT}/tcp |
| seed3 | seed3.${DOMAIN} | 64.227.107.57 | ${SEED_HTTP_PORT}/tcp |

## Port Freeze

| Purpose | Value |
| --- | --- |
| Bootnode listener | ${P2P_PORT}/tcp |
| Seed-service listener | ${SEED_HTTP_PORT}/tcp |
| Reserved conflict port | 5622 |
| Slotted node P2P base | 5630 + port_slot |
| Slotted node RPC base | 5730 + port_slot |
| Slotted node WS base | 5830 + port_slot |
| Slotted node discovery base | 5930 + port_slot |
| Slotted node metrics base | 6030 + port_slot |

## Bootnode Deployment

1. Download the assigned bootnode bundle from the Genesis Dashboard.
2. Transfer the bundle to the target host.
3. Extract the archive on the target host.
4. Open inbound TCP ${P2P_PORT} on the host firewall.
5. Confirm the A record for the assigned hostname points to the target IP.
6. Start the bundle with \`./install_and_start.sh\` on Linux or macOS, or \`install_and_start.ps1\` on Windows.
7. Confirm the process is running with \`./nodectl.sh status\` or \`nodectl.ps1 status\`.

## Seed-Service Deployment

1. Download the assigned seed bundle from the Genesis Dashboard.
2. Transfer the bundle to the target host.
3. Extract the archive on the target host.
4. Open inbound TCP ${SEED_HTTP_PORT} on the host firewall.
5. Confirm the A record for the assigned hostname points to the target IP.
6. Start the service with \`./install_and_start.sh\` on Linux or macOS, or \`install_and_start.ps1\` on Windows.
7. Confirm the process is running with \`./nodectl.sh status\` or \`nodectl.ps1 status\`.

## Verification

Run these checks after the assigned bundle is started.

\`\`\`bash
# Bootnode reachability
nc -zv bootnode1.${DOMAIN} ${P2P_PORT}
nc -zv bootnode2.${DOMAIN} ${P2P_PORT}
nc -zv bootnode3.${DOMAIN} ${P2P_PORT}

# Seed-service health
curl -s http://seed1.${DOMAIN}:${SEED_HTTP_PORT}/healthz
curl -s http://seed2.${DOMAIN}:${SEED_HTTP_PORT}/healthz
curl -s http://seed3.${DOMAIN}:${SEED_HTTP_PORT}/healthz

# Seed-service discovery payload
curl -s http://seed1.${DOMAIN}:${SEED_HTTP_PORT}/peer-list.json
curl -s http://seed2.${DOMAIN}:${SEED_HTTP_PORT}/peer-list.json
curl -s http://seed3.${DOMAIN}:${SEED_HTTP_PORT}/peer-list.json
\`\`\`

## DNS

Use the exact records in \`DNS_RECORDS.txt\`.
EOF

  cat > "$OUT_DIR/DNS_RECORDS.txt" <<EOF
Required A records
bootnode1.${DOMAIN} -> 74.208.227.23
bootnode2.${DOMAIN} -> 73.79.66.255
bootnode3.${DOMAIN} -> 64.227.107.57
seed1.${DOMAIN} -> 74.208.227.23
seed2.${DOMAIN} -> 73.79.66.255
seed3.${DOMAIN} -> 64.227.107.57

Required TXT records for bootnode discovery
_dnsaddr.bootstrap.${DOMAIN} -> "dnsaddr=/dns/bootnode1.${DOMAIN}/tcp/${P2P_PORT}"
_dnsaddr.bootstrap.${DOMAIN} -> "dnsaddr=/dns/bootnode2.${DOMAIN}/tcp/${P2P_PORT}"
_dnsaddr.bootstrap.${DOMAIN} -> "dnsaddr=/dns/bootnode3.${DOMAIN}/tcp/${P2P_PORT}"

Optional SRV records for seed-service discovery
_synergy-seed._tcp.${DOMAIN} -> 0 0 ${SEED_HTTP_PORT} seed1.${DOMAIN}
_synergy-seed._tcp.${DOMAIN} -> 0 0 ${SEED_HTTP_PORT} seed2.${DOMAIN}
_synergy-seed._tcp.${DOMAIN} -> 0 0 ${SEED_HTTP_PORT} seed3.${DOMAIN}
EOF

  cat > "$OUT_DIR/README.txt" <<EOF
Bootstrap bundles
=================

Contents
- bootnode1, bootnode2, bootnode3: bootstrap-only Synergy node bundles
- seed1, seed2, seed3: lightweight peer-list publisher services
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
  write_bootnode_env "$node_dir" "$name" "$ip"
  write_bootnode_scripts "$node_dir"
  copy_bootnode_binaries "$node_dir"
  write_bootnode_readme "$node_dir" "$name" "$ip"
  write_bootnode_binary_status "$node_dir"
}

build_seed_bundle() {
  local name="$1"
  local ip="$2"
  local seed_dir="$OUT_DIR/$name"

  rm -rf "$seed_dir"
  mkdir -p "$seed_dir/config" "$seed_dir/data/logs"
  write_seed_config "$seed_dir" "$name" "$ip"
  write_seed_service "$seed_dir"
  write_seed_env "$seed_dir" "$name" "$ip"
  write_seed_scripts "$seed_dir"
  write_seed_readme "$seed_dir" "$name" "$ip"
}

main() {
  mkdir -p "$OUT_DIR"
  resolve_binaries

  local record name ip
  for record in "${BOOTNODES[@]}"; do
    IFS='|' read -r name ip <<<"$record"
    build_bootnode_bundle "$name" "$ip"
  done

  for record in "${SEEDS[@]}"; do
    IFS='|' read -r name ip <<<"$record"
    build_seed_bundle "$name" "$ip"
  done

  write_root_files

  echo "Bootstrap bundles generated in $OUT_DIR"
}

main "$@"
