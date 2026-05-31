#!/usr/bin/env bash
set -euo pipefail

INSTALL_ROOT="/Library/Application Support/Synergy/archive-validator"
MARKER="${INSTALL_ROOT}/config/snapshot-source-majority-branch-proven"

if [[ "${1:-}" != "--source-node-majority-branch-proven" ]]; then
  echo "Refusing snapshot creation without --source-node-majority-branch-proven." >&2
  exit 1
fi

install -d -m 0750 "${INSTALL_ROOT}/config"
umask 027
printf 'authorized_at_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "${MARKER}"

exec env \
  SYNERGY_PROJECT_ROOT="${INSTALL_ROOT}" \
  /usr/local/synergy/bin/synergy-node create-snapshot \
    --chain-id 1264 \
    --network-id synergy-testnet-v2 \
    --source-node-majority-branch-proven \
    --source-role ARCHIVE_OBSERVER
