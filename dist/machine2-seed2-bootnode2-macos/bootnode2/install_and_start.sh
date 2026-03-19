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
