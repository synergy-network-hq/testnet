#!/usr/bin/env bash
set -euo pipefail
out="/var/lib/synergy/archive-validator/logs/diagnostics-$(date +%Y%m%d%H%M%S).txt"
{
  date -u
  synergy-archive status || true
  systemctl status synergy-archive-validator.service --no-pager || true
  systemctl status synergy-archive-snapshot-api.service --no-pager || true
  systemctl status synergy-archive-snapshot-worker.service --no-pager || true
} > "${out}"
echo "${out}"
