# Synergy Public Testnet 1263 Deployment Guide

This directory contains bootstrap and service bundles for the public Synergy Testnet reset to chain ID `1263`.

Before starting any node on this chain, stop the old process and remove old chain `1262` database state from the node runtime. Do not reuse a `data/chain` directory from chain `1262`.

Canonical identity:

- Network: `synergy-testnet`
- Chain ID: `1263`
- CAIP-2: `synergy:testnet`
- Reserved EIP-155: `eip155:1263`
- Genesis hash: `dd9ad8cfc74be1ab17a0a0fce9db65281df1b325fe5a2530130dce8935e450b8`
- Network magic bytes: `d5d5bb99`

Port model:

- Bootnodes: P2P `5620`, discovery `5680`
- Seed services: P2P `5621`, discovery `5681`
- Validators, relayers, observer, indexer: P2P `5622`, qRPC `5640`, WS `5660`, discovery `5680`, metrics `6030`
- RPC Gateway: P2P `5623`, qRPC `5641`, WS `5661`, discovery `5681`, metrics `6031`

Deployment order:

1. Install updated control panel/runtime packages.
2. Stop all old chain `1262` services.
3. Remove old chain databases and old genesis artifacts from each runtime data directory.
4. Install the chain `1263` bundle with the canonical `config/genesis.json`.
5. Start bootnodes and seed services.
6. Start the five genesis validators.
7. Verify a single genesis hash and increasing block height across all validators.
8. Start relayers, observer, RPC gateway, and Atlas indexer.
9. Verify Grafana/Atlas report chain `1263` and the canonical genesis hash.
