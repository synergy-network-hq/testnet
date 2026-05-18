#!/usr/bin/env bash
set -euo pipefail
command -v aegis-pqvm >/dev/null 2>&1
test -f /var/lib/synergy/archive-validator/config/archive-validator.toml
test -f /var/lib/synergy/archive-validator/config/genesis.json
if command -v systemctl >/dev/null 2>&1; then
  systemctl is-enabled synergy-archive-validator.service >/dev/null
  systemctl is-enabled synergy-archive-snapshot-api.service >/dev/null
  systemctl is-enabled synergy-archive-snapshot-worker.service >/dev/null
fi
echo "Archive validator install readiness checks passed."
