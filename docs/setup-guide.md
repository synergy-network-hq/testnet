# Synergy Network Testnet Beta Setup Guide

This guide walks you through setting up and running a node for the Synergy Network Testnet Beta.

---

## 🛠️ Prerequisites

Make sure your system has the following installed:

- Ubuntu 20.04+ (native or WSL2)
- Git
- Rust (via rustup)
- Build tools:
  ```bash
  sudo apt install build-essential libssl-dev pkg-config
  ```

---

## 🚀 Setup Steps

### 1. Clone the Repository

```bash
git clone https://github.com/synergy-network-hq/testnet-beta.git synergy-testbeta
cd synergy-testbeta
```

### 2. Install Rust (if not already installed)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 3. Build the Project

```bash
cargo build --release --bin synergy-testbeta
```

### 4. Start the Testnet Beta Node

```bash
bash scripts/start-testbeta.sh
```

This will:
- Load `config/genesis.json`
- Load `config/network-config.toml`
- Start the node in background
- Save logs to `data/logs/testbeta.out`

### 5. Stop the Testnet Beta Node

```bash
bash scripts/stop-testbeta.sh
```

---

## 🧪 Running Tests

Run all Rust unit/integration tests:

```bash
cargo test
```

---

## 📁 File Overview

- `src/` — Blockchain, consensus, RPC
- `config/` — Genesis, network, token metadata
- `scripts/` — Start/stop/testbeta helper scripts
- `docs/` — Developer documentation
- `tests/` — Core integration/unit tests
- `dependencies/` — Optional dependency manifests

---

## 🌐 Running Additional Nodes

To run a second node, copy the repo to another machine, update `config/network-config.toml` to:

- Use a unique `listen.p2p` port
- Include a valid `bootnodes` entry pointing to your first node

Use the same `genesis.json` for all nodes.

---

## 💬 Need Help?

- Check open GitHub issues
- Open a new ticket for bugs
- Or reach out to the Synergy Network dev team
