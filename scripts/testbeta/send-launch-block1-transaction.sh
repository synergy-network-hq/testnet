#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PROFILE_PATH="${PROFILE_PATH:-$HOME/.synergy/testnet-beta/network/profile.json}"
RPC_URL="${RPC_URL:-http://127.0.0.1:5646}"
AMOUNT_SNRG="${AMOUNT_SNRG:-1}"
MEMO="${MEMO:-launch-block-1}"
RUNNER="${RUNNER:-$ROOT_DIR/scripts/testbeta/transaction-runner.sh}"

if [[ ! -x "$RUNNER" ]]; then
  echo "Transaction runner not found or not executable: $RUNNER" >&2
  exit 1
fi

if [[ ! -f "$PROFILE_PATH" ]]; then
  echo "Network profile not found: $PROFILE_PATH" >&2
  exit 1
fi

read_wallet() {
  local key="$1"
  python3 - "$PROFILE_PATH" "$key" <<'PY'
import json
import pathlib
import sys

profile = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
key = sys.argv[2]
record = profile.get(key) or {}
print((record.get("address") or "").strip())
PY
}

SOURCE_WALLET="${SOURCE_WALLET:-$(read_wallet treasury_wallet)}"
DESTINATION_WALLET="${DESTINATION_WALLET:-$(read_wallet faucet_wallet)}"

if [[ -z "$SOURCE_WALLET" || -z "$DESTINATION_WALLET" ]]; then
  echo "Could not resolve treasury/faucet wallets from $PROFILE_PATH" >&2
  exit 1
fi

PARAMS_JSON="$(python3 - <<PY
import json
print(json.dumps([
    ${SOURCE_WALLET@Q},
    ${DESTINATION_WALLET@Q},
    "SNRG",
    int(${AMOUNT_SNRG@Q}),
    ${MEMO@Q},
]))
PY
)"

echo "Submitting deterministic launch transaction from $SOURCE_WALLET to $DESTINATION_WALLET via $RPC_URL"
"$RUNNER" \
  --rpc-url "$RPC_URL" \
  --method synergy_sendTokens \
  --params-json "$PARAMS_JSON" \
  --duration-seconds 15 \
  --interval-ms 250 \
  --workers 1 \
  --request-timeout-seconds 5 \
  --verbose
