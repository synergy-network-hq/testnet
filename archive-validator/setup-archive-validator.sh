#!/usr/bin/env bash
set -euo pipefail

CHAIN_ID="1264"
NETWORK_ID="synergy-testnet-v2"
GENESIS_FILE=""
EXPECTED_GENESIS_HASH=""
ARCHIVE_DATA_DIR="/var/lib/synergy/archive-validator"
SNAPSHOT_API_BIND="0.0.0.0:48640"
SNAPSHOT_PUBLIC_URL=""
P2P_BIND="0.0.0.0:38639"
METRICS_BIND="127.0.0.1:9091"
ENABLE_NGINX="false"
YES="false"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --chain-id) CHAIN_ID="$2"; shift 2 ;;
    --network-id) NETWORK_ID="$2"; shift 2 ;;
    --genesis-file) GENESIS_FILE="$2"; shift 2 ;;
    --expected-genesis-hash) EXPECTED_GENESIS_HASH="$2"; shift 2 ;;
    --archive-data-dir) ARCHIVE_DATA_DIR="$2"; shift 2 ;;
    --snapshot-api-bind) SNAPSHOT_API_BIND="$2"; shift 2 ;;
    --snapshot-public-url) SNAPSHOT_PUBLIC_URL="$2"; shift 2 ;;
    --p2p-bind) P2P_BIND="$2"; shift 2 ;;
    --metrics-bind) METRICS_BIND="$2"; shift 2 ;;
    --enable-nginx) ENABLE_NGINX="$2"; shift 2 ;;
    --bootnodes|--snapshot-interval-blocks) shift 2 ;;
    --yes) YES="true"; shift ;;
    *) echo "Unknown argument: $1" >&2; exit 1 ;;
  esac
done

[[ "${CHAIN_ID}" == "1264" ]] || { echo "chain_id must be 1264" >&2; exit 1; }
[[ "${NETWORK_ID}" == "synergy-testnet-v2" ]] || { echo "network_id must be synergy-testnet-v2" >&2; exit 1; }
[[ -n "${GENESIS_FILE}" && -f "${GENESIS_FILE}" ]] || { echo "genesis file is missing" >&2; exit 1; }
[[ -n "${EXPECTED_GENESIS_HASH}" ]] || { echo "expected genesis hash is required" >&2; exit 1; }
command -v aegis-pqvm >/dev/null 2>&1 || { echo "aegis-pqvm is required and unavailable" >&2; exit 1; }

if [[ "${YES}" != "true" ]]; then
  read -r -p "Install Synergy Archive Validator Node for Testnet chain 1264? [y/N] " answer
  [[ "${answer}" == "y" || "${answer}" == "Y" ]] || exit 1
fi

GENESIS_HASH="$(python3 - "$GENESIS_FILE" <<'PY'
import hashlib, json, sys
with open(sys.argv[1], 'rb') as fh:
    data = json.loads(fh.read())
payload = json.dumps(data, sort_keys=True, separators=(',', ':')).encode()
print(hashlib.blake2s(payload).hexdigest())
PY
)"
[[ "${GENESIS_HASH}" == "${EXPECTED_GENESIS_HASH}" ]] || { echo "computed genesis hash ${GENESIS_HASH} does not match expected ${EXPECTED_GENESIS_HASH}" >&2; exit 1; }

install -d -m 0750 "${ARCHIVE_DATA_DIR}/"{config,keys,data,logs,tmp,backups,run,snapshots}
install -d -m 0750 "${ARCHIVE_DATA_DIR}/data/"{blocks,qcs,state,epochs,validators,evidence,indexes}
install -m 0640 "${GENESIS_FILE}" "${ARCHIVE_DATA_DIR}/config/genesis.json"
install -m 0640 ./config/archive-validator.testnet.toml "${ARCHIVE_DATA_DIR}/config/archive-validator.toml"

./scripts/verify-aegis-pqvm.sh
./scripts/init-aegis-archive-identity.sh

if command -v systemctl >/dev/null 2>&1; then
  install -m 0644 ./systemd/*.service /etc/systemd/system/
  systemctl daemon-reload
  systemctl enable synergy-archive-validator.service synergy-archive-snapshot-api.service synergy-archive-snapshot-worker.service
  systemctl start synergy-archive-validator.service synergy-archive-snapshot-api.service synergy-archive-snapshot-worker.service
fi

if [[ "${ENABLE_NGINX}" == "true" ]]; then
  install -m 0644 ./nginx/synergy-archive-snapshot-api.conf /etc/nginx/conf.d/synergy-archive-snapshot-api.conf
fi

SNAPSHOT_PUBLIC_URL="${SNAPSHOT_PUBLIC_URL}" SNAPSHOT_API_BIND="${SNAPSHOT_API_BIND}" P2P_BIND="${P2P_BIND}" METRICS_BIND="${METRICS_BIND}" ./scripts/healthcheck.sh
echo "Archive validator ready. snapshot_api_bind=${SNAPSHOT_API_BIND} p2p_bind=${P2P_BIND} metrics_bind=${METRICS_BIND}"
