#!/usr/bin/env bash
set -euo pipefail

# Interactive Synergy Testnet-Beta faucet transfer helper.
#
# Sends SNRG from the faucet wallet to a recipient via the PUBLIC RPC
# gateway. The public gateway only exposes the signed-tx path
# (synergy_sendTransaction); state-mutating helpers like
# synergy_sendTokens are blocked off-host by exposure policy. This
# script therefore signs locally with wallet-pqc-cli and submits.
#
# Defaults can be overridden without editing this file:
#   SYNERGY_RPC_ENDPOINT     default: https://testbeta-core-rpc.synergy-network.io
#   SYNERGY_FAUCET_KEYFILE   default: /Users/devpup/Desktop/testbeta-keyfiles/faucet.dec.json
#   SYNERGY_WALLET_CLI       default: <repo>/target/debug/wallet-pqc-cli
#   SYNERGY_SIGN_ALGO        default: fndsa
#   SYNERGY_GAS_PRICE        default: 1000   (nWei per gas)
#   SYNERGY_GAS_LIMIT        default: 21000

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

RPC_ENDPOINT="${SYNERGY_RPC_ENDPOINT:-https://testbeta-core-rpc.synergy-network.io}"
KEYFILE="${SYNERGY_FAUCET_KEYFILE:-/Users/devpup/Desktop/testbeta-keyfiles/faucet.dec.json}"
WALLET_CLI="${SYNERGY_WALLET_CLI:-$REPO_ROOT/target/debug/wallet-pqc-cli}"
SIGN_ALGO="${SYNERGY_SIGN_ALGO:-fndsa}"
TOKEN_SYMBOL="${SYNERGY_TOKEN_SYMBOL:-SNRG}"
GAS_PRICE="${SYNERGY_GAS_PRICE:-1000}"
GAS_LIMIT="${SYNERGY_GAS_LIMIT:-21000}"
NWEI_PER_SNRG=1000000000

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || { echo "Missing required command: $1" >&2; exit 1; }
}
require_cmd curl
require_cmd jq
require_cmd python3

if [[ ! -f "$KEYFILE" ]]; then
  echo "Faucet keyfile not found: $KEYFILE" >&2
  exit 1
fi
if [[ ! -x "$WALLET_CLI" ]]; then
  echo "wallet-pqc-cli not found or not executable at: $WALLET_CLI" >&2
  echo "Build it with:" >&2
  echo "  (cd \"$REPO_ROOT\" && cargo build --bin wallet-pqc-cli)" >&2
  exit 1
fi

FAUCET_ADDRESS="$(jq -r '.address // empty' "$KEYFILE")"
FAUCET_PK_B64="$(jq -r '.private_key // empty' "$KEYFILE")"
if [[ -z "$FAUCET_ADDRESS" ]]; then
  echo "Could not read .address from $KEYFILE" >&2
  exit 1
fi
if [[ -z "$FAUCET_PK_B64" ]]; then
  echo "Could not read .private_key from $KEYFILE" >&2
  exit 1
fi

# wallet-pqc-cli sign-tx expects --private-key as hex; the keyfile stores it as base64.
FAUCET_PK_HEX="$(python3 - "$FAUCET_PK_B64" <<'PY'
import base64, sys
sys.stdout.write(base64.b64decode(sys.argv[1]).hex())
PY
)"

rpc() {
  local method="$1" params_json="${2:-[]}" payload response
  payload="$(jq -cn --arg method "$method" --argjson params "$params_json" \
    '{jsonrpc:"2.0",id:1,method:$method,params:$params}')"
  response="$(curl --fail --silent --show-error --max-time 25 \
    -H "Content-Type: application/json" \
    --data "$payload" \
    "$RPC_ENDPOINT")"
  if jq -e '.error != null' >/dev/null 2>&1 <<<"$response"; then
    jq -r '.error.message // (.error|tostring) // "Unknown RPC error"' <<<"$response" >&2
    return 1
  fi
  printf '%s\n' "$response"
}

format_nwei_as_snrg() {
  python3 - "$1" "$NWEI_PER_SNRG" <<'PY'
from decimal import Decimal, getcontext
import sys
getcontext().prec = 80
nwei = Decimal(sys.argv[1])
unit = Decimal(sys.argv[2])
value = nwei / unit
text = f"{value:.9f}"
text = text.rstrip("0").rstrip(".") if "." in text else text
print(text or "0")
PY
}

echo "Synergy Testnet-Beta Faucet Transfer (signed via synergy_sendTransaction)"
echo "RPC:     $RPC_ENDPOINT"
echo "Faucet:  $FAUCET_ADDRESS"
echo "Algo:    $SIGN_ALGO"
echo "Keyfile: $KEYFILE"
echo

read -r -p "Recipient Synergy wallet address: " RECIPIENT
RECIPIENT="$(printf '%s' "$RECIPIENT" | tr -d '[:space:]')"
if [[ ! "$RECIPIENT" =~ ^syn[0-9a-z]{10,70}$ ]]; then
  echo "Invalid Synergy wallet address: $RECIPIENT" >&2
  echo "Expected a lowercase address beginning with syn." >&2
  exit 1
fi

echo
echo "Checking chain and faucet balance..."
BLOCK_RESPONSE="$(rpc synergy_getBlockNumber '[]')"
CURRENT_BLOCK="$(jq -r '.result // empty' <<<"$BLOCK_RESPONSE")"

FAUCET_BAL_PARAMS="$(jq -cn --arg a "$FAUCET_ADDRESS" --arg t "$TOKEN_SYMBOL" '[$a,$t]')"
BALANCE_RESPONSE="$(rpc synergy_getTokenBalance "$FAUCET_BAL_PARAMS")"
FAUCET_BALANCE_NWEI="$(jq -r '.result // "0"' <<<"$BALANCE_RESPONSE")"
FAUCET_BALANCE_SNRG="$(format_nwei_as_snrg "$FAUCET_BALANCE_NWEI")"

echo "Current block:  $CURRENT_BLOCK"
echo "Faucet balance: $FAUCET_BALANCE_SNRG $TOKEN_SYMBOL"
echo

read -r -p "Amount to send, in whole SNRG: " AMOUNT_SNRG
AMOUNT_SNRG="${AMOUNT_SNRG//,/}"
AMOUNT_SNRG="$(printf '%s' "$AMOUNT_SNRG" | tr -d '[:space:]')"
if [[ ! "$AMOUNT_SNRG" =~ ^[0-9]+$ ]] || [[ "$AMOUNT_SNRG" == "0" ]]; then
  echo "Amount must be a positive whole-SNRG integer." >&2
  exit 1
fi

REQUESTED_NWEI="$(python3 - "$AMOUNT_SNRG" "$NWEI_PER_SNRG" <<'PY'
import sys
print(int(sys.argv[1]) * int(sys.argv[2]))
PY
)"

MAX_FEE_NWEI=$(( GAS_PRICE * GAS_LIMIT ))

if ! python3 - "$FAUCET_BALANCE_NWEI" "$REQUESTED_NWEI" "$MAX_FEE_NWEI" <<'PY'
import sys
sys.exit(0 if int(sys.argv[1]) >= int(sys.argv[2]) + int(sys.argv[3]) else 1)
PY
then
  echo "Insufficient faucet balance for amount + max gas fee." >&2
  echo "Requested: $AMOUNT_SNRG $TOKEN_SYMBOL  +  max gas fee: $MAX_FEE_NWEI nWei" >&2
  echo "Available: $FAUCET_BALANCE_SNRG $TOKEN_SYMBOL" >&2
  exit 1
fi

# Pull next nonce. PublicRead, allowed on the gateway.
NONCE_PARAMS="$(jq -cn --arg a "$FAUCET_ADDRESS" '[$a]')"
NONCE_RESPONSE="$(rpc synergy_getAccountNonce "$NONCE_PARAMS")"
NONCE="$(jq -r '.result // 0' <<<"$NONCE_RESPONSE")"
TIMESTAMP="$(date -u +%s)"
MEMO="manual faucet transfer $(date -u +%Y-%m-%dT%H:%M:%SZ)"

echo
echo "Transfer preview"
echo "From:     $FAUCET_ADDRESS"
echo "To:       $RECIPIENT"
echo "Amount:   $AMOUNT_SNRG $TOKEN_SYMBOL  ($REQUESTED_NWEI nWei)"
echo "Nonce:    $NONCE"
echo "Gas:      gas_price=$GAS_PRICE  gas_limit=$GAS_LIMIT  (max fee $MAX_FEE_NWEI nWei)"
echo "Memo:     $MEMO"
echo
read -r -p "Type yes to sign and submit this on-chain transaction: " CONFIRM
if [[ "$CONFIRM" != "yes" ]]; then
  echo "Cancelled. No transaction was submitted."
  exit 0
fi

# Build canonical unsigned transaction. Field set must match Transaction
# in src/transaction.rs; signing canonicalisation is internal to wallet-pqc-cli.
#
# Public-path native SNRG transfers must NOT carry a token_transfer:{} data
# envelope. The chain treats `amount` + `receiver` as canonical; the envelope
# is a legacy local-faucet shape that confuses downstream indexers (Atlas
# shows '--' for body fields when it can't parse the data field).
DATA_FIELD=""

UNSIGNED_TX="$(jq -cn \
  --arg sender "$FAUCET_ADDRESS" \
  --arg receiver "$RECIPIENT" \
  --argjson amount "$REQUESTED_NWEI" \
  --argjson nonce "$NONCE" \
  --argjson timestamp "$TIMESTAMP" \
  --argjson gas_price "$GAS_PRICE" \
  --argjson gas_limit "$GAS_LIMIT" \
  --arg data "$DATA_FIELD" \
  --arg algo "$SIGN_ALGO" \
  '{
    sender:$sender,
    receiver:$receiver,
    amount:$amount,
    nonce:$nonce,
    signature:[],
    timestamp:$timestamp,
    gas_price:$gas_price,
    gas_limit:$gas_limit,
    data:$data,
    signature_algorithm:$algo
  }')"

echo
echo "Signing transaction with $SIGN_ALGO..."
SIGNED_OUT="$("$WALLET_CLI" sign-tx \
  --private-key "$FAUCET_PK_HEX" \
  --tx "$UNSIGNED_TX" \
  --algo "$SIGN_ALGO")"

# Drop the in-memory copy of the key as soon as signing is done.
unset FAUCET_PK_HEX FAUCET_PK_B64

SIGNED_TX="$(jq -c '.transaction // empty' <<<"$SIGNED_OUT")"
if [[ -z "$SIGNED_TX" ]]; then
  echo "Signing failed. CLI output:" >&2
  echo "$SIGNED_OUT" >&2
  exit 1
fi

echo "Submitting signed transaction via synergy_sendTransaction..."
SEND_PARAMS="$(jq -cn --argjson tx "$SIGNED_TX" '[$tx]')"
SEND_RESPONSE="$(rpc synergy_sendTransaction "$SEND_PARAMS")"

# Result may be a bare hash string OR an object with tx_hash/success.
TX_HASH="$(jq -r '
  if (.result | type) == "string" then .result
  elif (.result | type) == "object" then (.result.tx_hash // .result.hash // empty)
  else empty end
' <<<"$SEND_RESPONSE")"

SUCCESS_FIELD="$(jq -r '
  if (.result | type) == "object" then (.result.success // empty | tostring)
  else "" end
' <<<"$SEND_RESPONSE")"

if [[ "$SUCCESS_FIELD" == "false" ]]; then
  echo "Transaction rejected:" >&2
  jq -r '.result.error // .error.message // "unknown error"' <<<"$SEND_RESPONSE" >&2
  exit 1
fi

echo "Transaction submitted."
[[ -n "$TX_HASH" ]] && echo "Transaction hash: $TX_HASH"

echo
echo "Waiting for on-chain inclusion..."
CONFIRMED="false"
if [[ -n "$TX_HASH" ]]; then
  LOOKUP_PARAMS="$(jq -cn --arg h "$TX_HASH" '[$h]')"
  for _ in {1..30}; do
    sleep 2
    if RECEIPT_RESPONSE="$(rpc synergy_getTransactionReceipt "$LOOKUP_PARAMS" 2>/dev/null)"; then
      if jq -e '.result != null and .result != ""' >/dev/null 2>&1 <<<"$RECEIPT_RESPONSE"; then
        CONFIRMED="true"; break
      fi
    fi
    if LOOKUP_RESPONSE="$(rpc synergy_getTransactionByHash "$LOOKUP_PARAMS" 2>/dev/null)"; then
      if jq -e '.result != null and .result != ""' >/dev/null 2>&1 <<<"$LOOKUP_RESPONSE"; then
        CONFIRMED="true"; break
      fi
    fi
  done
fi

if [[ "$CONFIRMED" == "true" ]]; then
  echo "Transaction confirmed on-chain."
else
  echo "Transaction was submitted, but confirmation was not observed within 60 seconds."
fi

echo
echo "Refreshing balances..."
RECIP_BAL_PARAMS="$(jq -cn --arg a "$RECIPIENT" --arg t "$TOKEN_SYMBOL" '[$a,$t]')"
NEW_FAUCET_NWEI="$(jq -r '.result // "0"' <<<"$(rpc synergy_getTokenBalance "$FAUCET_BAL_PARAMS")")"
RECIPIENT_NWEI="$(jq -r '.result // "0"' <<<"$(rpc synergy_getTokenBalance "$RECIP_BAL_PARAMS")")"

echo "Faucet balance:    $(format_nwei_as_snrg "$NEW_FAUCET_NWEI") $TOKEN_SYMBOL"
echo "Recipient balance: $(format_nwei_as_snrg "$RECIPIENT_NWEI") $TOKEN_SYMBOL"
