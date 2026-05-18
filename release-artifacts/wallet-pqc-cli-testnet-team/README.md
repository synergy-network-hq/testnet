# Synergy Testnet wallet-pqc-cli team bundle

This bundle is self-contained for Synergy Testnet signing. It includes `wallet-pqc-cli` binaries and `synergy-testnet-tx.py`, which embeds the shared Faucet and Token Sales TESTNET keys.

Do not use this embedded-key pattern for mainnet or any wallet with real value.

## Included platforms

Included in this bundle:

- macOS Universal: `wallet-pqc-cli-macos-universal`
- macOS Apple Silicon: `wallet-pqc-cli-darwin-arm64`
- macOS Intel: `wallet-pqc-cli-darwin-x64`
- Linux x64: `wallet-pqc-cli-linux-x64`

Not included yet:

- Native Windows `.exe`
- Linux arm64

Windows users can run this bundle through WSL with the included Linux x64 binary. A native Windows build should be produced on a Windows or CI runner with the correct C++/RocksDB toolchain.

## Binary detection

The Python helper auto-detects the right binary when it is present in this folder:

- macOS Apple Silicon: `wallet-pqc-cli-darwin-arm64`
- macOS Intel: `wallet-pqc-cli-darwin-x64`
- macOS Universal: `wallet-pqc-cli-macos-universal`
- Linux x64: `wallet-pqc-cli-linux-x64`
- Linux arm64, if present: `wallet-pqc-cli-linux-arm64`
- Windows x64, if present: `wallet-pqc-cli-windows-x64.exe`

You can override detection with `SYNERGY_WALLET_CLI=/path/to/wallet-pqc-cli`.

## macOS/Linux setup

```bash
chmod +x ./synergy-testnet-tx.py ./wallet-pqc-cli-* 2>/dev/null || true
python3 ./synergy-testnet-tx.py list-wallets
python3 ./synergy-testnet-tx.py height
```

## Windows setup

Use WSL for the current bundle:

```bash
chmod +x ./synergy-testnet-tx.py ./wallet-pqc-cli-linux-x64
python3 ./synergy-testnet-tx.py list-wallets
python3 ./synergy-testnet-tx.py height
```

## Query commands

```bash
python3 ./synergy-testnet-tx.py status
python3 ./synergy-testnet-tx.py balance faucet
python3 ./synergy-testnet-tx.py balance token-sales
python3 ./synergy-testnet-tx.py nonce faucet
python3 ./synergy-testnet-tx.py nonce token-sales
```

## Send one signed transaction

```bash
python3 ./synergy-testnet-tx.py send \
  --from faucet \
  --to token-sales \
  --amount-snrg 1 \
  --wait \
  --yes
```

Send to any Synergy wallet address:

```bash
python3 ./synergy-testnet-tx.py send \
  --from token-sales \
  --to synw1exampleaddressreplacewithrealwallet \
  --amount-snrg 1 \
  --wait \
  --yes
```

## One-hour faucet/token-sales ping-pong

This sends 1 SNRG every 5 seconds for one hour, alternating directions.

```bash
python3 ./synergy-testnet-tx.py pingpong \
  --duration-seconds 1800 \
  --interval-seconds 0.25 \
  --amount-snrg 500 \
  --wait \
  --yes
```

Smoke test only two transactions:

```bash
python3 ./synergy-testnet-tx.py pingpong \
  --duration-seconds 10 \
  --interval-seconds 5 \
  --amount-nwei 1 \
  --max-transactions 2 \
  --wait \
  --yes
```

## Multi-machine DAG traffic

Run this from different machines at the same time. For clean nonce behavior, do not run the same sender alias concurrently from two machines unless you coordinate nonces.

```bash
python3 ./synergy-testnet-tx.py burst \
  --senders faucet token-sales \
  --tx-per-sender 5 \
  --amount-nwei 1 \
  --interval-seconds 1 \
  --wait \
  --yes
```

## RPC endpoint

Default RPC is `https://testnet-core-rpc.synergy-network.io`.

Override it per command:

```bash
SYNERGY_RPC_URL="https://testnet-core-rpc.synergy-network.io" python3 ./synergy-testnet-tx.py height
```
