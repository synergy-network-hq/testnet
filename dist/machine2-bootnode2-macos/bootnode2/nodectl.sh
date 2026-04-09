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
