# How to query and exercise the Synergy Testnet from any machine

This guide uses the portable `wallet-pqc-cli-testnet-team` bundle. The bundle contains the command helper, the available `wallet-pqc-cli` binaries, and embedded Synergy Testnet Faucet and Token Sales wallet keys, so team members do not need separate `.json` key files.

Do not use this embedded-key pattern for mainnet or any wallet with real value.

## Prerequisites

- Python 3.
- The `wallet-pqc-cli-testnet-team` folder or archive.
- Network access to `https://testnet-core-rpc.synergy-network.io`.

Included binaries:

- macOS Universal: `wallet-pqc-cli-macos-universal`
- macOS Apple Silicon: `wallet-pqc-cli-darwin-arm64`
- macOS Intel: `wallet-pqc-cli-darwin-x64`
- Linux x64: `wallet-pqc-cli-linux-x64`

Windows users should run the bundle through WSL with the included Linux x64 binary until a native Windows build is produced.

## Set up the bundle

From macOS, Linux, or WSL:

```bash
cd wallet-pqc-cli-testnet-team
chmod +x ./synergy-testnet-tx.py ./wallet-pqc-cli-* 2>/dev/null || true
python3 ./synergy-testnet-tx.py list-wallets
python3 ./synergy-testnet-tx.py height
```

Expected wallet aliases:

- `faucet`
- `token-sales`

The helper defaults to the real Synergy Testnet endpoint:

```bash
https://testnet-core-rpc.synergy-network.io
```

Override the endpoint only when intentionally testing a different Testnet gateway:

```bash
SYNERGY_RPC_URL="https://testnet-core-rpc.synergy-network.io" python3 ./synergy-testnet-tx.py height
```

## Query the live network

Get the current block height:

```bash
python3 ./synergy-testnet-tx.py height
```

Get node status:

```bash
python3 ./synergy-testnet-tx.py status
```

Check balances:

```bash
python3 ./synergy-testnet-tx.py balance faucet
python3 ./synergy-testnet-tx.py balance token-sales
python3 ./synergy-testnet-tx.py balance synw1replacewithanytestnetwallet
```

Check nonces:

```bash
python3 ./synergy-testnet-tx.py nonce faucet
python3 ./synergy-testnet-tx.py nonce token-sales
python3 ./synergy-testnet-tx.py nonce synw1replacewithanytestnetwallet
```

List the embedded testnet wallet aliases:

```bash
python3 ./synergy-testnet-tx.py list-wallets
```

## Send Faucet wallet tokens

Send 1 SNRG from the embedded Faucet wallet to another Synergy Testnet wallet:

```bash
python3 ./synergy-testnet-tx.py send \
  --from faucet \
  --to synw1replacewithdestinationwallet \
  --amount-snrg 1 \
  --wait \
  --yes
```

Send a tiny 1 nWei smoke transaction:

```bash
python3 ./synergy-testnet-tx.py send \
  --from faucet \
  --to token-sales \
  --amount-nwei 1 \
  --wait \
  --yes
```

## Send Token Sales wallet tokens

Send 1 SNRG from the embedded Token Sales wallet to another Synergy Testnet wallet:

```bash
python3 ./synergy-testnet-tx.py send \
  --from token-sales \
  --to synw1replacewithdestinationwallet \
  --amount-snrg 1 \
  --wait \
  --yes
```

Send a tiny 1 nWei smoke transaction:

```bash
python3 ./synergy-testnet-tx.py send \
  --from token-sales \
  --to faucet \
  --amount-nwei 1 \
  --wait \
  --yes
```

## Run a one-hour Faucet and Token Sales ping-pong

This sends 1 SNRG every 5 seconds for one hour, alternating directions:

1. Faucet to Token Sales.
2. Token Sales to Faucet.
3. Repeat until the duration expires.

```bash
python3 ./synergy-testnet-tx.py pingpong \
  --duration-seconds 3600 \
  --interval-seconds 5 \
  --amount-snrg 1 \
  --wait \
  --yes
```

Run a two-transaction smoke test first:

```bash
python3 ./synergy-testnet-tx.py pingpong \
  --duration-seconds 10 \
  --interval-seconds 5 \
  --amount-nwei 1 \
  --max-transactions 2 \
  --wait \
  --yes
```

## Generate multi-machine DAG traffic

Run this command from several machines at the same time to generate small signed transaction bursts. Use different sender aliases on different machines when possible to avoid nonce collisions.

Machine A:

```bash
python3 ./synergy-testnet-tx.py burst \
  --senders faucet \
  --tx-per-sender 10 \
  --amount-nwei 1 \
  --interval-seconds 1 \
  --wait \
  --yes
```

Machine B:

```bash
python3 ./synergy-testnet-tx.py burst \
  --senders token-sales \
  --tx-per-sender 10 \
  --amount-nwei 1 \
  --interval-seconds 1 \
  --wait \
  --yes
```

Single-machine mixed-source test:

```bash
python3 ./synergy-testnet-tx.py burst \
  --senders faucet token-sales \
  --tx-per-sender 5 \
  --amount-nwei 1 \
  --interval-seconds 1 \
  --wait \
  --yes
```

Important: do not run the same sender alias concurrently from multiple machines unless you coordinate nonces. For clean DAG observation, run one source wallet per machine.

## Raw JSON-RPC query examples

These commands also run from any machine with `curl`.

Set the Testnet RPC endpoint:

```bash
export SYNERGY_RPC_URL="https://testnet-core-rpc.synergy-network.io"
```

Check block height:

```bash
curl --silent --show-error \
  -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"synergy_blockNumber","params":[],"id":1}' \
  "$SYNERGY_RPC_URL"
```

Check latest block:

```bash
curl --silent --show-error \
  -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"synergy_getLatestBlock","params":[],"id":1}' \
  "$SYNERGY_RPC_URL"
```

Check node status:

```bash
curl --silent --show-error \
  -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"synergy_getNodeStatus","params":[],"id":1}' \
  "$SYNERGY_RPC_URL"
```

Check a wallet balance:

```bash
ADDRESS="synw1replacewithanytestnetwallet"
curl --silent --show-error \
  -H "Content-Type: application/json" \
  --data "{\"jsonrpc\":\"2.0\",\"method\":\"synergy_getTokenBalance\",\"params\":[\"$ADDRESS\",\"SNRG\"],\"id\":1}" \
  "$SYNERGY_RPC_URL"
```

Look up a transaction:

```bash
TX_HASH="syntxn-replace-with-real-hash"
curl --silent --show-error \
  -H "Content-Type: application/json" \
  --data "{\"jsonrpc\":\"2.0\",\"method\":\"synergy_getTransactionByHash\",\"params\":[\"$TX_HASH\"],\"id\":1}" \
  "$SYNERGY_RPC_URL"
```

Check a transaction receipt:

```bash
TX_HASH="syntxn-replace-with-real-hash"
curl --silent --show-error \
  -H "Content-Type: application/json" \
  --data "{\"jsonrpc\":\"2.0\",\"method\":\"synergy_getTransactionReceipt\",\"params\":[\"$TX_HASH\"],\"id\":1}" \
  "$SYNERGY_RPC_URL"
```

Watch block production:

```bash
while true; do
  date -u +"%Y-%m-%dT%H:%M:%SZ"
  curl --silent --show-error \
    -H "Content-Type: application/json" \
    --data '{"jsonrpc":"2.0","method":"synergy_getLatestBlock","params":[],"id":1}' \
    "$SYNERGY_RPC_URL"
  sleep 5
done
```

## Testnet endpoint rule

All commands in this guide use:

```bash
https://testnet-core-rpc.synergy-network.io
```

Do not use `testnet` endpoints for this Testnet.
