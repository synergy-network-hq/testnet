# How to query and exercise the Synergy Testnet from any machine

This guide uses the portable `wallet-pqc-cli-testnet-team` bundle. The bundle contains the command helper, available `wallet-pqc-cli` binaries, and embedded Synergy Testnet Faucet, Token Sales, and Validator Rewards wallet keys, so team members do not need separate `.json` key files.

Do not use this embedded-key pattern for mainnet or any wallet with real value.

## Prerequisites

- Python 3.
- The `wallet-pqc-cli-testnet-team` folder, `.zip`, or `.tar.gz` archive.
- Network access to `https://testnet-core-rpc.synergy-network.io`.

Network values:

- Network: Synergy Testnet
- Chain ID: `1264`
- RPC: `https://testnet-core-rpc.synergy-network.io`
- Atlas API: `https://testnet-atlas.synergy-network.io/api/v1`

Any `testnet` endpoint is wrong for this package and should be treated as a critical configuration fault.

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
python3 ./synergy-testnet-tx.py chain-id
python3 ./synergy-testnet-tx.py height
```

Expected wallet aliases:

- `faucet`
- `token-sales`
- `validator-rewards`

Override binary detection only when intentionally using another local build:

```bash
SYNERGY_WALLET_CLI="/path/to/wallet-pqc-cli" python3 ./synergy-testnet-tx.py chain-id
```

## Transaction nonce mode

The helper defaults to `--nonce-mode zero` because the current Testnet validator runtime verifies externally signed Faucet, Token Sales, and Validator Rewards wallet transactions against imported wallet metadata. The traffic and validator funding commands automatically append unique memo data, so repeated zero-nonce test transactions still produce unique transaction hashes and commit correctly.

Use `--nonce-mode rpc` only after the validator runtime nonce tracker is updated to advance imported external wallet metadata on committed transactions.

## Query the live network

Get the expected and RPC-reported chain ID:

```bash
python3 ./synergy-testnet-tx.py chain-id
```

Get the current block height:

```bash
python3 ./synergy-testnet-tx.py height
```

Get the latest block:

```bash
python3 ./synergy-testnet-tx.py latest-block
```

Get node status and identity details:

```bash
python3 ./synergy-testnet-tx.py status
python3 ./synergy-testnet-tx.py node-info
```

Get network statistics, validators, and peers:

```bash
python3 ./synergy-testnet-tx.py network-stats
python3 ./synergy-testnet-tx.py validators
python3 ./synergy-testnet-tx.py peers
```

Query Atlas DAG state:

```bash
python3 ./synergy-testnet-tx.py atlas-dag --view status
python3 ./synergy-testnet-tx.py atlas-dag --view frontier
python3 ./synergy-testnet-tx.py atlas-dag --view vertices --limit 25
python3 ./synergy-testnet-tx.py atlas-dag --view topology --limit 25
```

Check balances:

```bash
python3 ./synergy-testnet-tx.py balance faucet
python3 ./synergy-testnet-tx.py balance token-sales
python3 ./synergy-testnet-tx.py balance validator-rewards
python3 ./synergy-testnet-tx.py balance synw1replacewithanytestnetwallet
```

Check nonces:

```bash
python3 ./synergy-testnet-tx.py nonce faucet
python3 ./synergy-testnet-tx.py nonce token-sales
python3 ./synergy-testnet-tx.py nonce validator-rewards
python3 ./synergy-testnet-tx.py nonce synw1replacewithanytestnetwallet
```

Look up a transaction:

```bash
python3 ./synergy-testnet-tx.py tx syntxn-replace-with-real-hash
python3 ./synergy-testnet-tx.py receipt syntxn-replace-with-real-hash
```

## Fund the Faucet wallet

On the reset chain, the embedded Token Sales wallet is genesis-funded. Seed Faucet before running Faucet-origin stress traffic:

```bash
python3 ./synergy-testnet-tx.py seed-faucet \
  --amount-snrg 1000 \
  --wait \
  --yes
```

Verify the Faucet balance:

```bash
python3 ./synergy-testnet-tx.py balance faucet
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

## Fund a new validator stake wallet

Use the embedded Validator Rewards wallet to send the required 50,000 SNRG stake amount to a new validator address:

```bash
python3 ./synergy-testnet-tx.py fund-validator \
  --to synv1replacewithnewvalidatoraddress \
  --wait \
  --yes
```

Verify the destination balance:

```bash
python3 ./synergy-testnet-tx.py balance synv1replacewithnewvalidatoraddress
```

Override the amount only when intentionally sending a different testnet validator stake grant:

```bash
python3 ./synergy-testnet-tx.py fund-validator \
  --to synv1replacewithnewvalidatoraddress \
  --amount-snrg 50000 \
  --wait \
  --yes
```

## Run a one-hour Faucet and Token Sales ping-pong

This sends 1 SNRG every 5 seconds for one hour, alternating directions.

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

## Generate rapid-fire DAG stress traffic

Use `stress` for duration-based transaction generation. The default stress amount is `1` nWei.

Short local smoke:

```bash
python3 ./synergy-testnet-tx.py stress \
  --senders token-sales \
  --receiver faucet \
  --duration-seconds 10 \
  --interval-seconds 0.5 \
  --amount-nwei 1 \
  --max-transactions 20 \
  --continue-on-error \
  --yes
```

Single-machine mixed-source run:

```bash
python3 ./synergy-testnet-tx.py stress \
  --senders faucet token-sales \
  --duration-seconds 300 \
  --interval-seconds 0.25 \
  --amount-nwei 1 \
  --continue-on-error \
  --yes
```

One-hour mixed-source run:

```bash
python3 ./synergy-testnet-tx.py stress \
  --senders faucet token-sales \
  --duration-seconds 3600 \
  --interval-seconds 0.25 \
  --amount-nwei 1 \
  --continue-on-error \
  --yes
```

Maximum-rate bounded run:

```bash
python3 ./synergy-testnet-tx.py stress \
  --senders token-sales \
  --receiver faucet \
  --interval-seconds 0 \
  --max-transactions 100 \
  --amount-nwei 1 \
  --continue-on-error \
  --yes
```

## Generate multi-machine traffic

The bundle is safe to run from multiple machines with the default `--nonce-mode zero`; each generated traffic transaction includes unique memo data to avoid duplicate hashes. Use one sender per machine when you want cleaner source attribution in Atlas and logs.

Machine A:

```bash
python3 ./synergy-testnet-tx.py stress \
  --senders token-sales \
  --receiver faucet \
  --duration-seconds 600 \
  --interval-seconds 0.25 \
  --amount-nwei 1 \
  --continue-on-error \
  --yes
```

Machine B:

```bash
python3 ./synergy-testnet-tx.py stress \
  --senders faucet \
  --receiver token-sales \
  --duration-seconds 600 \
  --interval-seconds 0.25 \
  --amount-nwei 1 \
  --continue-on-error \
  --yes
```

Single-machine finite burst:

```bash
python3 ./synergy-testnet-tx.py burst \
  --senders faucet token-sales \
  --tx-per-sender 5 \
  --amount-nwei 1 \
  --interval-seconds 1 \
  --wait \
  --yes
```

## Raw JSON-RPC query examples

These commands run from any machine with `curl`.

Set the Testnet RPC endpoint:

```bash
export SYNERGY_RPC_URL="https://testnet-core-rpc.synergy-network.io"
```

Check node info and chain ID:

```bash
curl --silent --show-error \
  -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"synergy_nodeInfo","params":[],"id":1}' \
  "$SYNERGY_RPC_URL"
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

Check network stats:

```bash
curl --silent --show-error \
  -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"synergy_getNetworkStats","params":[],"id":1}' \
  "$SYNERGY_RPC_URL"
```

Check validators:

```bash
curl --silent --show-error \
  -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"synergy_getValidatorActivity","params":[],"id":1}' \
  "$SYNERGY_RPC_URL"
```

Check peers:

```bash
curl --silent --show-error \
  -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"synergy_getPeerInfo","params":[],"id":1}' \
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

## Raw Atlas DAG query examples

These commands run from any machine with `curl`.

```bash
export SYNERGY_ATLAS_API_URL="https://testnet-atlas.synergy-network.io/api/v1"
```

Check DAG status:

```bash
curl --silent --show-error "$SYNERGY_ATLAS_API_URL/dag/status"
```

Check DAG frontier:

```bash
curl --silent --show-error "$SYNERGY_ATLAS_API_URL/dag/frontier"
```

Check recent DAG vertices:

```bash
curl --silent --show-error "$SYNERGY_ATLAS_API_URL/dag/vertices?limit=25"
```

Check DAG topology:

```bash
curl --silent --show-error "$SYNERGY_ATLAS_API_URL/dag/topology?limit=25"
```
