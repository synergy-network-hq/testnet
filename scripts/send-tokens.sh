#!/bin/bash
# Synergy Testnet-Beta - Send SNRG Tokens to Validator
# Usage: ./send-tokens.sh <recipient_address> <amount>

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Configuration
FAUCET_ADDRESS="synw1lfgerdqglc6p74p9u6k8ghfssl59q8jzhuwm07"
RPC_ENDPOINT="${SYNERGY_RPC_ENDPOINT:-https://testbeta-core-rpc.synergy-network.io/rpc}"

# Check arguments
if [ $# -lt 2 ]; then
    echo -e "${RED}Error: Missing arguments${NC}"
    echo "Usage: $0 <recipient_address> <amount>"
    echo ""
    echo "Examples:"
    echo "  $0 synv1abc123... 50000        # Send 50k SNRG"
    echo "  $0 synv1xyz789... 100000       # Send 100k SNRG"
    exit 1
fi

RECIPIENT="$1"
AMOUNT="$2"

# Validate recipient address format
if [[ ! "$RECIPIENT" =~ ^syn[vwaszbjkmnulfro][0-9a-z]{38,42}$ ]]; then
    echo -e "${RED}Error: Invalid Synergy address format${NC}"
    echo "Address must start with syn and be lowercase"
    exit 1
fi

echo -e "${GREEN}===================================${NC}"
echo -e "${GREEN}Synergy Testnet-Beta - Send Tokens${NC}"
echo -e "${GREEN}===================================${NC}"
echo ""
echo "From:      $FAUCET_ADDRESS"
echo "To:        $RECIPIENT"
echo "Amount:    $AMOUNT SNRG"
echo ""

# Get faucet balance
echo "Checking faucet balance..."
FAUCET_BALANCE=$(curl -s -X POST "$RPC_ENDPOINT" \
  -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"synergy_getTokenBalance\",\"params\":[\"$FAUCET_ADDRESS\",\"SNRG\"],\"id\":1}" \
  | jq -r '.result // "0"')

echo "Faucet Balance: $FAUCET_BALANCE SNRG"

# Check if sufficient balance
if [ "$FAUCET_BALANCE" -lt "$AMOUNT" ]; then
    echo -e "${RED}Error: Insufficient faucet balance${NC}"
    echo "Requested: $AMOUNT SNRG"
    echo "Available: $FAUCET_BALANCE SNRG"
    exit 1
fi

echo ""
echo -e "${YELLOW}Ready to send tokens.${NC}"
read -p "Proceed? (yes/no): " confirm

if [ "$confirm" != "yes" ]; then
    echo "Transaction cancelled."
    exit 0
fi

# Send transaction via JSON-RPC (node signs using imported faucet identity)
echo ""
echo "Submitting transaction..."

RESULT=$(curl -s -X POST "$RPC_ENDPOINT" \
  -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"synergy_sendTokens\",\"params\":[\"$FAUCET_ADDRESS\",\"$RECIPIENT\",\"SNRG\",$AMOUNT],\"id\":1}")

SUCCESS=$(echo "$RESULT" | jq -r '.result.success // false')
if [ "$SUCCESS" != "true" ]; then
    ERROR=$(echo "$RESULT" | jq -r '.result.error // .error.message // "Unknown error"')
    echo -e "${RED}❌ Transaction failed${NC}"
    echo "Error: $ERROR"
    exit 1
fi

TX_HASH=$(echo "$RESULT" | jq -r '.result.tx_hash // empty')
echo -e "${GREEN}✅ Transaction submitted${NC}"
if [ -n "$TX_HASH" ]; then
    echo "Transaction Hash: $TX_HASH"
fi

echo ""
echo "Waiting for inclusion in a block..."

CONFIRMED="false"
for i in {1..15}; do
    sleep 2
    if [ -z "$TX_HASH" ]; then
        break
    fi
    TX_LOOKUP=$(curl -s -X POST "$RPC_ENDPOINT" \
      -H "Content-Type: application/json" \
      -d "{\"jsonrpc\":\"2.0\",\"method\":\"synergy_getTransactionByHash\",\"params\":[\"$TX_HASH\"],\"id\":1}")
    FOUND=$(echo "$TX_LOOKUP" | jq -r '.result // empty')
    if [ "$FOUND" != "" ] && [ "$FOUND" != "null" ]; then
        CONFIRMED="true"
        break
    fi
done

if [ "$CONFIRMED" == "true" ]; then
    echo -e "${GREEN}✅ Transaction confirmed on-chain${NC}"
else
    echo -e "${YELLOW}⚠️  Transaction not yet confirmed (check explorer/RPC)${NC}"
fi

# Check recipient balance
NEW_BALANCE=$(curl -s -X POST "$RPC_ENDPOINT" \
  -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"synergy_getTokenBalance\",\"params\":[\"$RECIPIENT\",\"SNRG\"],\"id\":1}" \
  | jq -r '.result // "0"')

echo "Recipient balance: $NEW_BALANCE SNRG"
