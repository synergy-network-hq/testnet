# Synergy Network Testnet Setup Guide

This guide walks you through setting up and running a node for the Synergy Network Testnet.

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
git clone https://github.com/synergy-network-hq/testnet.git synergy-testnet
cd synergy-testnet
```

### 2. Install Rust (if not already installed)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 3. Build the Project

```bash
cargo build --release --bin synergy-testnet
```

### 4. Start the Testnet Node

```bash
bash scripts/start-testnet.sh
```

This will:
- Load `config/genesis.json`
- Load `config/network-config.toml`
- Start the node in background
- Save logs to `data/logs/testnet.out`

### 5. Stop the Testnet Node

```bash
bash scripts/stop-testnet.sh
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
- `scripts/` — Start/stop/testnet helper scripts
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
