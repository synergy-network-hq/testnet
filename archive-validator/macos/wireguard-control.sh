#!/usr/bin/env bash
set -euo pipefail

PATH="/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
CONFIG="${SYNERGY_ARCHIVE_WIREGUARD_CONFIG:-/Library/Application Support/Synergy/archive-validator/config/wireguard/archive-validator.conf}"
INTERFACE="${SYNERGY_ARCHIVE_WIREGUARD_INTERFACE:-$(basename "${CONFIG}" .conf)}"
ACTION="${1:-up}"

command -v wg >/dev/null 2>&1 || { echo "wg is required" >&2; exit 1; }
command -v wg-quick >/dev/null 2>&1 || { echo "wg-quick is required" >&2; exit 1; }
[[ -f "${CONFIG}" ]] || { echo "WireGuard config is missing: ${CONFIG}" >&2; exit 1; }

case "${ACTION}" in
  up)
    if wg show "${INTERFACE}" >/dev/null 2>&1; then
      exit 0
    fi
    exec wg-quick up "${CONFIG}"
    ;;
  down)
    if ! wg show "${INTERFACE}" >/dev/null 2>&1; then
      exit 0
    fi
    exec wg-quick down "${CONFIG}"
    ;;
  *)
    echo "Usage: $0 <up|down>" >&2
    exit 1
    ;;
esac
