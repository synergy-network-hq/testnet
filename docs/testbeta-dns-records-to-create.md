# Testbeta DNS Records To Create

This file lists the DNS records that still need to be created based on:

- the current `testbeta` hostnames referenced in this repository
- the DNS inventory you provided on March 17, 2026
- the decision to use exactly 3 bootnodes and 3 seed servers

## Summary

No additional `synergynode.xyz` records are required for bootstrap discovery.

You already have the needed 3-host bootstrap set:

- `bootnode1.synergynode.xyz`
- `bootnode2.synergynode.xyz`
- `bootnode3.synergynode.xyz`
- `seed1.synergynode.xyz`
- `seed2.synergynode.xyz`
- `seed3.synergynode.xyz`
- `_dnsaddr.bootstrap.synergynode.xyz` TXT records for bootnodes 1-3
- `_synergy-seed._tcp.synergynode.xyz` SRV records for seeds 1-3

The missing work is on `synergy-network.io`.

## Records To Create Now

These are the records the canonical Testnet-Beta launch surfaces expect.

| Host | Type | Target | Why |
| --- | --- | --- | --- |
| `testbeta-core-rpc.synergy-network.io` | `A` | `74.208.227.23` | Canonical public Testbeta RPC endpoint used by the control panel, Atlas backend, Atlas indexer, and node configs |
| `testbeta-core-ws.synergy-network.io` | `A` | `74.208.227.23` | Canonical public Testbeta WebSocket endpoint used by node configs and generated peer metadata |
| `testbeta-api.synergy-network.io` | `A` | `65.21.202.144` | Testbeta REST/API endpoint used by the control panel `.env.example` and cert scripts |
| `testbeta-wallet-api.synergy-network.io` | `A` | `65.21.202.144` | Wallet API endpoint used by generated Testbeta peer metadata and control panel defaults |
| `testbeta-faucet.synergy-network.io` | `A` | `65.21.202.144` | Faucet endpoint used in Testbeta environment defaults and SSL scripts |
| `testbeta-sxcp-api.synergy-network.io` | `A` | `65.21.202.144` | SXCP API endpoint used by control panel defaults and generated Testbeta peer metadata |
| `testbeta-sxcp-ws.synergy-network.io` | `A` | `65.21.202.144` | SXCP WebSocket endpoint used by control panel defaults |
| `testbeta-synq-verify.synergy-network.io` | `A` | `65.21.202.144` | Verification endpoint used by control panel defaults and SSL scripts |
| `testbeta-aegis-verify.synergy-network.io` | `A` | `65.21.202.144` | Default Aegis verify fallback used by the control panel runtime |
| `testbeta-evm-rpc.synergy-network.io` | `A` | `65.21.202.144` | Included in Testbeta certificate-generation scripts |
| `testbeta-evm-ws.synergy-network.io` | `A` | `65.21.202.144` | Included in Testbeta certificate-generation scripts |
| `testbeta.synergy-network.io` | `A` | `65.21.202.144` | Binary/update manifest host used by `https://testbeta.synergy-network.io/binaries/latest.json` |

## Compatibility Aliases To Create

These are not the canonical names anymore, but parts of the repo and operator scripts still reference them.

| Host | Type | Target | Why |
| --- | --- | --- | --- |
| `testbeta-rpc.synergy-network.io` | `CNAME` | `testbeta-core-rpc.synergy-network.io` | Compatibility alias only; canonical launch traffic stays on `testbeta-core-rpc` |
| `testbeta-explorer-api.synergy-network.io` | `CNAME` | `testbeta-atlas-api.synergy-network.io` | SSL/cert scripts still reference this older explorer API alias |

If your DNS provider does not support `CNAME` the way you want for these aliases, use `A` records pointing at the same IP as the canonical target instead:

- `testbeta-rpc.synergy-network.io` -> `74.208.227.23`
- `testbeta-explorer-api.synergy-network.io` -> `74.208.227.23`

## Records Already Present

Based on the DNS inventory you provided, these Testbeta records already exist and do not need to be created again:

- `testbeta-explorer.synergy-network.io` -> `74.208.227.23`
- `testbeta-indexer.synergy-network.io` -> `74.208.227.23`
- `testbeta-atlas-api.synergy-network.io` -> `74.208.227.23`
- `testbeta-atlas.synergy-network.io` -> `74.208.227.23`

## Optional Cleanup

These are not required for the current 3-node bootstrap plan:

- `bootnode4.synergynode.xyz`
- `seed4.synergynode.xyz`

They can stay in DNS unused, but they should not be advertised in:

- `_dnsaddr.bootstrap.synergynode.xyz`
- `_synergy-seed._tcp.synergynode.xyz`

## Assumptions

- `65.21.202.144` remains the host for the shared Testbeta RPC/API-style services because your existing `rpc`, `ws`, `api`, `devnet-core-rpc`, `devnet-core-ws`, `devnet-api`, `devnet-wallet-api`, `devnet-sxcp-api`, and related records already point there.
- `74.208.227.23` remains the host for the explorer/indexer/Atlas services because your existing `explorer`, `devnet-explorer`, `devnet-indexer`, `devnet-atlas-api`, `testbeta-explorer`, `testbeta-indexer`, and `testbeta-atlas-api` records already point there.
- `testbeta.synergy-network.io` should resolve to the machine serving `/var/www/synergy-portal/binaries`. If that is not `65.21.202.144` in your environment, change that one record to the correct binary host.
