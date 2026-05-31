#!/usr/bin/env bash
set -euo pipefail

INSTALL_ROOT="/Library/Application Support/Synergy/archive-validator"
MARKER="${INSTALL_ROOT}/config/snapshot-source-majority-branch-proven"

if [[ ! -f "${MARKER}" ]]; then
  echo "Snapshot worker is waiting for explicit majority-branch authorization." >&2
  exit 1
fi

exec env \
  SYNERGY_PROJECT_ROOT="${INSTALL_ROOT}" \
  /usr/local/synergy/bin/synergy-node create-snapshot-if-due \
    --chain-id 1264 \
    --network-id synergy-testnet-v2 \
    --source-node-majority-branch-proven \
    --source-role ARCHIVE_OBSERVER
