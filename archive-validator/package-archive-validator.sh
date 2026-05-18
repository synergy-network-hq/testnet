#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ARTIFACT="${ROOT_DIR}/synergy-archive-validator-testnet-v2.zip"

if find "${ROOT_DIR}" -name '*.key' -o -name '*.pem' -o -name '.env' | grep -q .; then
  echo "Refusing to package private keys, PEM files, or .env secrets." >&2
  exit 1
fi

rm -f "${ARTIFACT}"
cd "${ROOT_DIR}/.."
zip -r "${ARTIFACT}" archive-validator \
  -x 'archive-validator/synergy-archive-validator-testnet-v2.zip' \
  -x 'archive-validator/**/.DS_Store'
echo "Created ${ARTIFACT}"
