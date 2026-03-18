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
