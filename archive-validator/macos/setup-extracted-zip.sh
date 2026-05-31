#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_ROOT="/Library/Application Support/Synergy/archive-validator"
LOG_ROOT="/Library/Logs/Synergy/archive-validator"
BIN_ROOT="/usr/local/synergy/bin"
SHARE_ROOT="/usr/local/synergy/share/archive-validator"
ARCHIVE_BINARY="${ROOT_DIR}/bin/synergy-archive"
NODE_BINARY="${ROOT_DIR}/bin/synergy-node"
GENESIS_FILE=""
EXPECTED_GENESIS_HASH=""
WIREGUARD_CONFIG=""
WIREGUARD_TEMPLATE=""
AUTHORIZE_SNAPSHOT_SOURCE="false"

usage() {
  cat <<'USAGE'
Usage: sudo ./macos/setup-extracted-zip.sh \
  --archive-binary /trusted/path/synergy-archive \
  --node-binary /trusted/path/synergy-node \
  --genesis-file /trusted/path/genesis.testnet.json \
  --expected-genesis-hash <hash> \
  (--wireguard-config /secure/path/archive-validator.conf | \
   --wireguard-template /secure/path/rendered-archive-validator.conf) \
  [--source-node-majority-branch-proven]

The WireGuard input must be operator supplied and fully rendered. Private key
material is copied into the local protected config directory, never packaged.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --archive-binary) ARCHIVE_BINARY="$2"; shift 2 ;;
    --node-binary) NODE_BINARY="$2"; shift 2 ;;
    --genesis-file) GENESIS_FILE="$2"; shift 2 ;;
    --expected-genesis-hash) EXPECTED_GENESIS_HASH="$2"; shift 2 ;;
    --wireguard-config) WIREGUARD_CONFIG="$2"; shift 2 ;;
    --wireguard-template) WIREGUARD_TEMPLATE="$2"; shift 2 ;;
    --source-node-majority-branch-proven) AUTHORIZE_SNAPSHOT_SOURCE="true"; shift ;;
    --help|-h) usage; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; usage >&2; exit 1 ;;
  esac
done

[[ "$(uname -s)" == "Darwin" ]] || { echo "This installer supports macOS only." >&2; exit 1; }
[[ "$(id -u)" == "0" ]] || { echo "Run this installer with sudo." >&2; exit 1; }
[[ -x "${ARCHIVE_BINARY}" ]] || { echo "Trusted synergy-archive binary is missing or not executable." >&2; exit 1; }
[[ -x "${NODE_BINARY}" ]] || { echo "Trusted synergy-node binary is missing or not executable." >&2; exit 1; }
[[ -f "${GENESIS_FILE}" ]] || { echo "Canonical genesis file is required." >&2; exit 1; }
[[ -n "${EXPECTED_GENESIS_HASH}" ]] || { echo "Expected genesis hash is required." >&2; exit 1; }
if [[ -n "${WIREGUARD_CONFIG}" && -n "${WIREGUARD_TEMPLATE}" ]] || [[ -z "${WIREGUARD_CONFIG}${WIREGUARD_TEMPLATE}" ]]; then
  echo "Provide exactly one of --wireguard-config or --wireguard-template." >&2
  exit 1
fi
WIREGUARD_SOURCE="${WIREGUARD_CONFIG:-${WIREGUARD_TEMPLATE}}"
[[ -f "${WIREGUARD_SOURCE}" ]] || { echo "Operator WireGuard input is missing." >&2; exit 1; }

PATH="/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
for command_name in aegis-pqvm wg wg-quick launchctl plutil python3; do
  command -v "${command_name}" >/dev/null 2>&1 || { echo "${command_name} is required." >&2; exit 1; }
done

if grep -Eq '(<[^>]+>|\$\{[^}]+\})' "${WIREGUARD_SOURCE}"; then
  echo "WireGuard input still contains template placeholders." >&2
  exit 1
fi
grep -Eq '^[[:space:]]*PrivateKey[[:space:]]*=' "${WIREGUARD_SOURCE}" || { echo "WireGuard PrivateKey is missing." >&2; exit 1; }
grep -Eq '^[[:space:]]*Address[[:space:]]*=' "${WIREGUARD_SOURCE}" || { echo "WireGuard Address is missing." >&2; exit 1; }
grep -Eq '^[[:space:]]*Endpoint[[:space:]]*=' "${WIREGUARD_SOURCE}" || { echo "WireGuard relayer Endpoint is missing." >&2; exit 1; }

GENESIS_HASH="$(python3 - "${GENESIS_FILE}" <<'PY'
import json
import sys
with open(sys.argv[1], "r", encoding="utf-8") as handle:
    document = json.load(handle)
value = document.get("integrity", {}).get("genesis_hash")
if not value:
    raise SystemExit("integrity.genesis_hash missing from genesis file")
print(value)
PY
)"
[[ "${GENESIS_HASH}" == "${EXPECTED_GENESIS_HASH}" ]] || { echo "Genesis hash ${GENESIS_HASH} does not match expected ${EXPECTED_GENESIS_HASH}." >&2; exit 1; }

stop_services() {
  for label in \
    io.synergynetwork.archive-snapshot-worker \
    io.synergynetwork.archive-snapshot-api \
    io.synergynetwork.archive-validator \
    io.synergynetwork.archive-wireguard
  do
    launchctl bootout "system/${label}" >/dev/null 2>&1 || true
  done
}

stop_services
install -d -m 0755 "${BIN_ROOT}" "${SHARE_ROOT}" "${LOG_ROOT}"
install -d -m 0750 "${INSTALL_ROOT}/config/wireguard" "${INSTALL_ROOT}/data/snapshots" "${INSTALL_ROOT}/tmp"
install -m 0755 "${ARCHIVE_BINARY}" "${BIN_ROOT}/synergy-archive"
install -m 0755 "${NODE_BINARY}" "${BIN_ROOT}/synergy-node"
install -m 0755 "${ROOT_DIR}/macos/create-initial-snapshot.sh" "${SHARE_ROOT}/create-initial-snapshot.sh"
install -m 0755 "${ROOT_DIR}/macos/run-snapshot-worker.sh" "${SHARE_ROOT}/run-snapshot-worker.sh"
install -m 0755 "${ROOT_DIR}/macos/wireguard-control.sh" "${SHARE_ROOT}/wireguard-control.sh"
install -m 0755 "${ROOT_DIR}/macos/uninstall-macos.sh" "${SHARE_ROOT}/uninstall-macos.sh"
install -m 0644 "${ROOT_DIR}/config/archive-validator.macos.testnet.toml" "${INSTALL_ROOT}/config/archive-validator.toml"
install -m 0644 "${ROOT_DIR}/config/snapshot-policy.testnet.toml" "${INSTALL_ROOT}/config/snapshot-policy.toml"
install -m 0644 "${ROOT_DIR}/config/archive-api.testnet.toml" "${INSTALL_ROOT}/config/archive-api.toml"
install -m 0644 "${GENESIS_FILE}" "${INSTALL_ROOT}/config/genesis.json"
install -m 0600 "${WIREGUARD_SOURCE}" "${INSTALL_ROOT}/config/wireguard/archive-validator.conf"
install -m 0644 "${ROOT_DIR}/launchd/"*.plist /Library/LaunchDaemons/

for plist in "${ROOT_DIR}/launchd/"*.plist; do
  plutil -lint "${plist}" >/dev/null
done

"${BIN_ROOT}/synergy-archive" init
"${SHARE_ROOT}/wireguard-control.sh" up

for plist in \
  io.synergynetwork.archive-wireguard.plist \
  io.synergynetwork.archive-validator.plist \
  io.synergynetwork.archive-snapshot-api.plist \
  io.synergynetwork.archive-snapshot-worker.plist
do
  launchctl bootstrap system "/Library/LaunchDaemons/${plist}"
  launchctl enable "system/${plist%.plist}"
done

"${BIN_ROOT}/synergy-archive" status >/dev/null
if [[ "${AUTHORIZE_SNAPSHOT_SOURCE}" == "true" ]]; then
  if ! "${SHARE_ROOT}/create-initial-snapshot.sh" --source-node-majority-branch-proven; then
    echo "Initial snapshot is deferred until verified finalized archive state is available; launchd will retry." >&2
  fi
fi

echo "Archive validator launchd services installed."
echo "WireGuard config installed from operator input at ${INSTALL_ROOT}/config/wireguard/archive-validator.conf."
echo "Authorize initial snapshot creation only after majority-branch verification:"
echo "  sudo ${SHARE_ROOT}/create-initial-snapshot.sh --source-node-majority-branch-proven"
