# Synergy Testnet Chain 1264 Live Rollout

Date: 2026-05-19

This runbook is for Synergy Testnet chain 1264 only.

Canonical identity:
- Product/network name: Synergy Testnet
- chain_id: `1264`
- chain_id_hex: `0x4f0`
- network_id: `synergy-testnet-v2`
- genesis validators: 5
- active validator quorum: 4-of-5
- cluster_count: 1
- cluster_id: 0
- target block cadence: 2 seconds

## Release Source

Trusted runtime artifacts must come from the public canonical repositories:
- Node source and binaries: `synergy-network-hq/testnet`
- Node Control Panel: `synergy-network-hq/synergy-node-control-panel`
- Atlas, when applicable: `synergy-network-hq/synergy-atlas`

Do not deploy ad hoc local binaries while the public GitHub Actions artifact path is available.

Current deployed release:
- Node runtime tag: `v12.2.18`
- Node runtime commit: `ad96256133af76ad670ea7f5f532ada454d06f4b`
- Node GitHub Actions run: `26103018856`
- Control Panel tag: `v12.2.18`
- Control Panel commit: `6d2ff941facc2d07a310dd2a3b8321223b04f276`
- Control Panel GitHub Actions run: `26104388942`

Trusted Linux runtime checksum:
- `828ebdf05cb52ae31f2694b19d0b2c2663c3c43e1bae31e99ded91ccd5592689`

Trusted Control Panel Linux package:
- `synergy-node-control-panel_12.2.18_amd64.deb`
- digest: `sha256:29f33189fdf5ebc6cc30258196ed33d82b98e4280e4b9bedd254f8ccb94f1c40`

## Final Reset / Start Sequence Used

The final v12.2.18 reset preserved validator keys, peer identity keys, configs, WireGuard state, logs, evidence, and service credentials.

Cleared on validator and relayer runtime workspaces:
- `chain.json`
- `committed_qcs.json`
- `canonical_locks.json`
- `consensus_vote_locks.json`
- DAG and token state
- stale proposal/vote/local-finalized caches
- stale per-network state directories moved into timestamped backups

Start sequence:
1. Stopped relayers.
2. Installed Control Panel `12.2.18` on all five validators.
3. Staged validator runtimes from installed package resources.
4. Reset validator runtime state.
5. Started all five genesis validators.
6. Verified QCs and canonical locks from height 1 onward.
7. Updated and restarted relayers with the trusted `v12.2.18` binary.
8. Repointed Atlas to relayer-only local tunnels.
9. Backed up and reset Atlas DB tables.
10. Restarted public RPC and Atlas services.

## Live Service Routing

Relayer local tunnels on the Atlas/RPC host:
- Relay 1 qRPC: `http://127.0.0.1:8640`
- Relay 2 qRPC: `http://127.0.0.1:9640`

Public RPC:
- HTTPS JSON-RPC: `https://testnet-core-rpc.synergy-network.io/rpc`
- Raw public qRPC listener: `0.0.0.0:5641`
- Raw public WS listener: `0.0.0.0:5661`

Atlas API:
- `https://testnet-atlas-api.synergy-network.io/api/v1/network/summary`
- `https://testnet-atlas-api.synergy-network.io/api/v1/blocks`
- `https://testnet-atlas-api.synergy-network.io/api/v1/dag/status`

## Required Preflight Before Future Live Mutation

Capture and attach the following before any reset, restart, installer deployment, release promotion, firewall change, or topology change:
- node, control-panel, and Atlas repo commit/tag
- GitHub Actions run IDs, artifact URLs, and checksums
- all five validator heights, hashes, genesis hashes, runtime binary checksums, and workspace resource versions
- committed QC and canonical-lock file existence/latest height on each validator
- latest block timestamp and validator host wall-clock delta
- relayer height, hash, peers, and runtime checksum
- public RPC height and hash
- Atlas API/indexer height and database latest indexed block
- observer, bootnode, and seed service status
- validator and relayer peer tables
- direct public-to-validator exposure check
- block interval statistics over latest 50, 120, and 300 blocks
- whether Atlas is indexing and whether DAG rows exist
- whether any validator is divergent, lagging, or failing health checks

## Post-Rollout Proof Requirements

Do not describe a future rollout as complete until the final report proves:
- all five validators agree by height/hash
- relayers are synced
- public RPC is synced
- Atlas is synced and current
- latest block timestamps are close to wall clock
- block cadence is near 2 seconds over latest 50/120/300 blocks
- release package and live runtime checksums match
- no direct public-node-to-validator peers exist
- no active deployable `Testnet-Beta`, `testnet-beta`, `testbeta`, chain 1262, or chain 1263 identity material remains
