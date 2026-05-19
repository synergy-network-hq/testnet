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

Trusted runtime artifacts must come from the public canonical repository:

- Node source and binaries: `synergy-network-hq/testnet`
- Node Control Panel: `synergy-network-hq/synergy-node-control-panel`
- Atlas, when applicable: `synergy-network-hq/synergy-atlas`

Do not deploy ad hoc local binaries while the public GitHub Actions artifact path is available.

## Required Preflight Before Live Mutation

Capture and attach the following before any reset, restart, installer deployment, release promotion, firewall change, or topology change:

- node, control-panel, and Atlas repo commit/tag
- GitHub Actions run IDs, artifact URLs, and checksums
- all five validator heights, hashes, genesis hashes, runtime binary checksums, and workspace resource versions
- committed QC file existence and latest QC height on each validator
- canonical lock file existence and latest canonical lock height on each validator
- latest state root where exposed
- latest block timestamp and validator host wall-clock delta
- Relayer-1 and Relayer-2 height, hash, peers, and runtime checksum
- RPC gateway height, hash, peers, and runtime checksum
- Node-EXP/indexer height, hash, peers, and runtime checksum
- Atlas API chain summary and Atlas database latest indexed block when database access works
- observer, bootnode, and seed service status
- validator and relayer peer tables
- direct public-to-validator peer check
- validator public service port exposure check
- block interval statistics over latest 50, 120, and 300 blocks
- whether Atlas is indexing and whether DAG rows exist
- whether any validator is divergent, lagging, or failing health checks

## Mutation Plan Template

Before executing a live change, record:

- exact nodes and services affected
- exact artifact and checksum being installed
- exact state directories or files being cleared
- why the change is safe
- rollback artifact or backup path
- evidence and logs that must be preserved

## Controlled Reset / Start Sequence

Use only after trusted release artifacts are installed everywhere and the preflight shows no unhandled safety blocker.

1. Stop public support services first: Atlas/indexer, RPC gateway node, Node-EXP, observer follower process, archive/follower nodes, and support-node catch-up services.
2. Stop relayers.
3. Stop validators.
4. Install release artifact/runtime everywhere.
5. Clear runtime state only where applicable: chain files, `chain.json`, `committed_qcs.json`, `canonical_locks.json`, DAG state, vote locks, proposal caches, consensus temp files, local finalized cache, stale per-network state directories, stale registry cache when it conflicts with canonical genesis, and Atlas/indexer block/account/transaction/DAG tables.
6. Preserve keys, configs, logs, divergence evidence, WireGuard state, service credentials, TLS certificates, and OS user permissions.
7. Start bootnodes.
8. Start all five genesis validators.
9. Wait for QCs and canonical locks from height 1 onward.
10. Verify validators agree by height/hash and are advancing.
11. Start relayers.
12. Verify relayers sync through validator private-plane peers and do not reject canonical QC/registry context.
13. Start RPC gateway and Node-EXP.
14. Start Atlas backend/indexer.
15. Start observer.
16. Monitor catch-up while enforcing priority isolation so support-node sync traffic cannot starve consensus traffic.

## Post-Rollout Proof

Do not describe the rollout as complete until the final report proves:

- all five validators agree by height/hash
- relayers are synced
- public RPC is synced
- Atlas is synced and current
- latest block timestamps are close to wall clock
- block cadence is near 2 seconds over latest 50/120/300 blocks
- release package and live runtime checksums match
- no direct public-node-to-validator peers exist
- no deployable `Testnet-Beta`, `testnet-beta`, `testbeta`, chain 1262, or chain 1263 identity material remains
