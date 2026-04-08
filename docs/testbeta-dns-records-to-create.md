# Testbeta DNS Baseline and Remaining Actions

This file is the frozen DNS baseline for `synergy-testnet-beta`.

It reflects:

- the canonical beta hostnames referenced by the live repo
- the current DNS inventory for `synergy-network.io` and `synergynode.xyz`
- the frozen bootstrap topology of exactly 3 bootnodes and 3 seed services

## Summary

No additional launch-critical DNS records remain to be created.

The required beta records should now be treated as the frozen keep set. Remaining launch work is service deployment and endpoint verification, not creation of new names.

The required 3-host bootstrap set is:

- `bootnode1.synergynode.xyz`
- `bootnode2.synergynode.xyz`
- `bootnode3.synergynode.xyz`
- `seed1.synergynode.xyz`
- `seed2.synergynode.xyz`
- `seed3.synergynode.xyz`
- `_dnsaddr.bootstrap.synergynode.xyz` TXT records for bootnodes 1-3
- `_synergy-seed._tcp.synergynode.xyz` SRV records for seeds 1-3

The required beta surface on `synergy-network.io` is also part of the frozen keep set.

## Canonical Beta Keep Set

These are the records the canonical Testnet-Beta launch surfaces expect to exist and remain stable.

| Host | Type | Target | Status |
| --- | --- | --- | --- |
| `testbeta-core-rpc.synergy-network.io` | `A` | `74.208.227.23` | Keep as the canonical public beta RPC endpoint |
| `testbeta-core-ws.synergy-network.io` | `A` | `74.208.227.23` | Keep as the canonical public beta WebSocket endpoint |
| `testbeta-api.synergy-network.io` | `A` | `65.21.202.144` | Keep as the canonical beta API endpoint |
| `testbeta-wallet-api.synergy-network.io` | `A` | `65.21.202.144` | Keep as the wallet helper API endpoint |
| `testbeta-faucet.synergy-network.io` | `A` | `65.21.202.144` | Keep as the faucet endpoint |
| `testbeta-sxcp-api.synergy-network.io` | `A` | `65.21.202.144` | Keep as the SXCP API endpoint |
| `testbeta-sxcp-ws.synergy-network.io` | `A` | `65.21.202.144` | Keep as the SXCP WebSocket endpoint |
| `testbeta-synq-verify.synergy-network.io` | `A` | `65.21.202.144` | Keep as the SynQ verification endpoint |
| `testbeta-aegis-verify.synergy-network.io` | `A` | `65.21.202.144` | Keep as the Aegis verification endpoint |
| `testbeta-evm-rpc.synergy-network.io` | `A` | `65.21.202.144` | Keep as the compatibility EVM HTTP endpoint |
| `testbeta-evm-ws.synergy-network.io` | `A` | `65.21.202.144` | Keep as the compatibility EVM WebSocket endpoint |
| `testbeta.synergy-network.io` | `A` | `65.21.202.144` | Keep as the binary and update manifest host |
| `testbeta-explorer.synergy-network.io` | `A` | `74.208.227.23` | Keep as the explorer UI hostname |
| `testbeta-indexer.synergy-network.io` | `A` | `74.208.227.23` | Keep as the indexer host |
| `testbeta-atlas-api.synergy-network.io` | `A` | `74.208.227.23` | Keep as the Atlas backend API hostname |
| `testbeta-atlas.synergy-network.io` | `A` | `74.208.227.23` | Keep as the Atlas host alias |

## Compatibility Aliases

No compatibility aliases are part of the frozen Testnet-Beta keep set.

## Bootstrap Discovery Keep Set

These records remain the approved bootstrap discovery surface:

- `bootnode1.synergynode.xyz` -> `74.208.227.23`
- `bootnode2.synergynode.xyz` -> `73.79.66.255`
- `bootnode3.synergynode.xyz` -> `157.245.226.240`
- `seed1.synergynode.xyz` -> `74.208.227.23`
- `seed2.synergynode.xyz` -> `73.79.66.255`
- `seed3.synergynode.xyz` -> `157.245.226.240`
- `_dnsaddr.bootstrap.synergynode.xyz` TXT records pointing at `tcp/5620`
- `_synergy-seed._tcp.synergynode.xyz` SRV records pointing at `5621`

## Remaining DNS Work

DNS work is now limited to:

- verifying every keep-set record resolves exactly as frozen
- keeping retired and removed names out of certificates, docs, configs, and launch procedures
- verifying the services behind these names are actually deployed and healthy

## Assumptions

- `65.21.202.144` remains the host for the shared beta API, wallet, faucet, verification, SXCP, and compatibility EVM surfaces.
- `74.208.227.23` remains the host for the core RPC, core WS, explorer, indexer, and Atlas surfaces.
- `testbeta.synergy-network.io` should resolve to the machine serving `/var/www/synergy-portal/binaries`. If that is not `65.21.202.144` in your environment, change that one record to the correct binary host.
