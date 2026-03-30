# Synergy Network — Master Network, Ports, and Nginx Configuration

**Version:** FINAL v1.0 (Consolidated)  
**Status:** Authoritative / Implementation‑Binding  
**Scope:** Global + Devnet  
**Excluded:** Testnet, Mainnet‑Beta, Mainnet  

> This document is the single source of truth binding DNS, nginx, service ports,
> SDK configuration, and operational security rules for Synergy Network.

---

## 1. Canonical DNS & Endpoint URLs

### 1.1 Global (Environment‑Agnostic)

| Purpose | URL |
|------|-----|
| Global API Router | <https://api.synergy-network.io> |
| Global RPC Router | <https://rpc.synergy-network.io> |
| Global WS Router | wss://ws.synergy-network.io |
| Explorer Hub | <https://explorer.synergy-network.io> |
| Indexer Hub | <https://indexer.synergy-network.io> |
| Validator Portal | <https://validators.synergy-network.io> |
| Status Page | <https://status.synergy-network.io> |
| Static CDN | <https://assets.synergy-network.io> |
| Developer Docs | <https://docs.synergy-network.io> |
| Bootnode 1 | bootnode1.synergy-network.io |

---

### 1.2 Devnet (Authoritative)

| Surface | URL |
|------|-----|
| Core RPC | <https://devnet-core-rpc.synergy-network.io> |
| Core WS | wss://devnet-core-ws.synergy-network.io |
| EVM RPC | <https://devnet-evm-rpc.synergy-network.io> |
| EVM WS | wss://devnet-evm-ws.synergy-network.io |
| REST API | <https://devnet-api.synergy-network.io> |
| Explorer UI | <https://devnet-explorer.synergy-network.io> |
| Explorer API | <https://devnet-explorer-api.synergy-network.io> |
| Indexer API | <https://devnet-indexer.synergy-network.io> |
| Faucet | <https://devnet-faucet.synergy-network.io> |
| Wallet API | <https://devnet-wallet-api.synergy-network.io> |
| SXCP API | <https://devnet-sxcp-api.synergy-network.io> |
| SXCP WS | wss://devnet-sxcp-ws.synergy-network.io |
| Aegis Verify | <https://devnet-aegis-verify.synergy-network.io> |
| Aegis KMS (PRIVATE) | <https://devnet-aegis-kms.synergy-network.io> |
| SynQ Verify | <https://devnet-synq-verify.synergy-network.io> |

---

## 2. Canonical Port Allocation

### 2.1 L1 Node (Testnet-Beta)

| Purpose | Port |
|-----|----:|
| P2P | 5622 + assignment |
| Core RPC | 5640 + assignment |
| Core WS | 5660 + assignment |
| Metrics | 9090 (localhost only) |

---

### 2.1.1 SXCP Relayer Node (Testnet-Beta)

Relayer nodes must use **separate ports** from L1 nodes to avoid conflicts and to allow operators
to apply firewall / ACL policies specifically for SXCP traffic.

| Purpose | Port |
|-----|----:|
| Relayer P2P (SXCP) | 5622 + assignment |
| Relayer RPC (SXCP) | 5640 + assignment |
| Relayer WS (SXCP) | 5660 + assignment |

**Hard rule**: Relayer P2P is **DNS‑only, never proxied** (same as L1 P2P).

---

### 2.2 EVM RPC (Testnet-Beta)

| Purpose | Port |
|-----|----:|
| HTTP | 8545 |
| WS | 8546 |

---

### 2.3 Microservices (Testnet-Beta)

| Service | Port |
|------|----:|
| Portal API | 3001 |
| Faucet | 3002 |
| Wallet API | 3003 |
| Indexer Ingest | 3010 |
| Indexer API | 3011 |
| Explorer API | 3020 |
| SynQ Verify | 3030 |
| SXCP API | 3040 |
| SXCP WS | 3041 |
| Aegis Verify | 3050 |
| Aegis KMS | 3051 (PRIVATE, mTLS ONLY) |

---

## 3. Nginx — Testnet-Beta Upstream Map

```nginx
upstream testbeta_core_rpc     { server 127.0.0.1:5640; }
upstream testbeta_core_ws      { server 127.0.0.1:5660; }

upstream testbeta_evm_rpc      { server 127.0.0.1:8545; }
upstream testbeta_evm_ws       { server 127.0.0.1:8546; }

upstream testbeta_api          { server 127.0.0.1:3001; }
upstream testbeta_explorer_ui  { server 127.0.0.1:80; }
upstream testbeta_explorer_api { server 127.0.0.1:3020; }
upstream testbeta_indexer_api  { server 127.0.0.1:3011; }
upstream testbeta_wallet_api   { server 127.0.0.1:3003; }
upstream testbeta_faucet       { server 127.0.0.1:3002; }

upstream testbeta_sxcp_api     { server 127.0.0.1:3040; }
upstream testbeta_sxcp_ws      { server 127.0.0.1:3041; }

upstream testbeta_aegis_verify { server 127.0.0.1:3050; }

upstream testbeta_synq_verify  { server 127.0.0.1:3030; }
```

---

## 4. Nginx — Shared Proxy & WebSocket Headers

```nginx
map $http_upgrade $connection_upgrade {
    default upgrade;
    ''      close;
}

proxy_set_header Host              $host;
proxy_set_header X-Real-IP         $remote_addr;
proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
proxy_set_header X-Forwarded-Proto $scheme;
proxy_http_version 1.1;
```

---

## 5. Nginx — Devnet VHost Map (Exact DNS Match)

### 5.1 Core RPC / WS

```nginx
server {
    listen 443 ssl http2;
    server_name devnet-core-rpc.synergy-network.io;
    include snippets/ssl.conf;
    location / { proxy_pass http://devnet_core_rpc; }
}

server {
    listen 443 ssl http2;
    server_name devnet-core-ws.synergy-network.io;
    include snippets/ssl.conf;
    location / {
        proxy_pass http://devnet_core_ws;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $connection_upgrade;
    }
}
```

### 5.2 EVM RPC / WS

```nginx
server {
    listen 443 ssl http2;
    server_name devnet-evm-rpc.synergy-network.io;
    include snippets/ssl.conf;
    location / { proxy_pass http://devnet_evm_rpc; }
}

server {
    listen 443 ssl http2;
    server_name devnet-evm-ws.synergy-network.io;
    include snippets/ssl.conf;
    location / {
        proxy_pass http://devnet_evm_ws;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $connection_upgrade;
    }
}
```

### 5.3 APIs, Explorer, Indexer

```nginx
server {
    listen 443 ssl http2;
    server_name devnet-api.synergy-network.io;
    include snippets/ssl.conf;
    location / { proxy_pass http://devnet_api; }
}

server {
    listen 443 ssl http2;
    server_name devnet-explorer.synergy-network.io;
    include snippets/ssl.conf;
    location / { proxy_pass http://devnet_explorer_ui; }
}

server {
    listen 443 ssl http2;
    server_name devnet-explorer-api.synergy-network.io;
    include snippets/ssl.conf;
    location / { proxy_pass http://devnet_explorer_api; }
}

server {
    listen 443 ssl http2;
    server_name devnet-indexer.synergy-network.io;
    include snippets/ssl.conf;
    location / { proxy_pass http://devnet_indexer_api; }
}
```

### 5.4 Wallet, Faucet, SXCP, SynQ

```nginx
server {
    listen 443 ssl http2;
    server_name devnet-wallet-api.synergy-network.io;
    include snippets/ssl.conf;
    location / { proxy_pass http://devnet_wallet_api; }
}

server {
    listen 443 ssl http2;
    server_name devnet-faucet.synergy-network.io;
    include snippets/ssl.conf;
    location / { proxy_pass http://devnet_faucet; }
}

server {
    listen 443 ssl http2;
    server_name devnet-sxcp-api.synergy-network.io;
    include snippets/ssl.conf;
    location / { proxy_pass http://devnet_sxcp_api; }
}

server {
    listen 443 ssl http2;
    server_name devnet-sxcp-ws.synergy-network.io;
    include snippets/ssl.conf;
    location / {
        proxy_pass http://devnet_sxcp_ws;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $connection_upgrade;
    }
}

server {
    listen 443 ssl http2;
    server_name devnet-aegis-verify.synergy-network.io;
    include snippets/ssl.conf;
    location / { proxy_pass http://devnet_aegis_verify; }
}

server {
    listen 443 ssl http2;
    server_name devnet-synq-verify.synergy-network.io;
    include snippets/ssl.conf;
    location / { proxy_pass http://devnet_synq_verify; }
}
```

---

## 6. Global Router VHosts (Path‑Based Routing)

### 6.1 Global RPC Router

```nginx
server {
    listen 443 ssl http2;
    server_name rpc.synergy-network.io;
    include snippets/ssl.conf;

    location /devnet/ {
        proxy_pass http://devnet_core_rpc/;
    }

    location /healthz { return 200 "ok\n"; }
    location /readyz  { return 200 "ready\n"; }
    location /version { return 200 "rpc-router-v1\n"; }
}
```

### 6.2 Global API Router

```nginx
server {
    listen 443 ssl http2;
    server_name api.synergy-network.io;
    include snippets/ssl.conf;

    location /devnet/ {
        proxy_pass http://devnet_api/;
    }

    location /healthz { return 200 "ok\n"; }
    location /readyz  { return 200 "ready\n"; }
    location /version { return 200 "api-router-v1\n"; }
}
```

### 6.3 Global WebSocket Router

```nginx
server {
    listen 443 ssl http2;
    server_name ws.synergy-network.io;
    include snippets/ssl.conf;

    location /devnet/ {
        proxy_pass http://devnet_core_ws/;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $connection_upgrade;
    }
}
```

---

## 7. Security & Hard Rules

- All public services: **443 only**
- P2P ports: **DNS‑only, never proxied**
- Aegis KMS: **NEVER public**
- Metrics: **localhost only**
- Mainnet writes: **explicit routing required**
- If it’s not in this document, **it is not official**

---

## 8. Change Control

- This file is the **binding contract** between DNS, nginx, and services
- All future environments MUST extend this document
- No silent overrides permitted
