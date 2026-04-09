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
(
  cd "$BASE_DIR"
  nohup env \
    SYNERGY_PROJECT_ROOT="$BASE_DIR" \
    SYNERGY_CONFIG_PATH="$BASE_DIR/config/node.toml" \
    SYNERGY_GENESIS_FILE="$GENESIS_FILE" \
    SYNERGY_BOOTSTRAP_ONLY=true \
    SYNERGY_AUTO_REGISTER_VALIDATOR=false \
    SYNERGY_P2P_LISTEN_ADDRESS="${P2P_LISTEN_ADDRESS:-}" \
    SYNERGY_P2P_EXTERNAL_ADDRESS="${P2P_EXTERNAL_ADDRESS:-${P2P_PUBLIC_ADDRESS:-}}" \
    SYNERGY_P2P_PUBLIC_ADDRESS="${P2P_PUBLIC_ADDRESS:-${P2P_EXTERNAL_ADDRESS:-}}" \
    SYNERGY_DISCOVERY_PORT="${DISCOVERY_PORT:-}" \
    SYNERGY_DISCOVERY_LISTEN_ADDRESS="${DISCOVERY_LISTEN_ADDRESS:-}" \
    SYNERGY_DISCOVERY_EXTERNAL_ADDRESS="${DISCOVERY_EXTERNAL_ADDRESS:-${DISCOVERY_PUBLIC_ADDRESS:-}}" \
    SYNERGY_DISCOVERY_PUBLIC_ADDRESS="${DISCOVERY_PUBLIC_ADDRESS:-${DISCOVERY_EXTERNAL_ADDRESS:-}}" \
    "$BIN_SELECTED" start --config "$BASE_DIR/config/node.toml" >"$OUT_FILE" 2>&1 &
  echo $! > "$PID_FILE"
)
echo "Started $MACHINE_ID as bootstrap-only discovery node (PID $(cat "$PID_FILE"))"
echo "Logs: $OUT_FILE"
