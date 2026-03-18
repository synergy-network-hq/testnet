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
