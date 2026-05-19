# Synergy Testnet v2 Chain 1264 Readiness Audit

Date: 2026-05-19

Scope: source tree and live non-mutating preflight for Synergy public TESTNET v2.

Canonical identity:
- chain_id: 1264
- chain_id_hex: 0x4f0
- network_id: synergy-testnet-v2
- genesis validators: 5
- cluster_count: 1
- cluster_id: 0
- quorum: 4 of 5 active genesis validators
- target block cadence: 2 seconds

## Live Preflight Snapshot

Status before any live mutation:
- Repos were clean at node `cdd4f51` / `v12.2.12`, control panel `5656bcc` / `v12.2.12`, and explorer `3e87c36`.
- GitHub Actions control-panel release run `26073464285` completed successfully for Linux, macOS, and Windows. The publish-to-releases-repo job was skipped.
- Validators 1-5 agreed at height `1817`, block hash `fb6462d6105d0b51f9424313ea619f0385fe90be36c33a58d0c5fc497333a211`.
- Relayer 1 and Relayer 2 agreed at height `1817` and had private validator peers plus RPC/indexer support peers through the relayer surface.
- Public RPC `https://testnet-core-rpc.synergy-network.io` returned height `1817`, hash `fb6462d6105d0b51f9424313ea619f0385fe90be36c33a58d0c5fc497333a211`.
- Atlas API `https://testnet-atlas.synergy-network.io/api/v1/network/summary` returned chain `1264`, latest block `1817`, average block time `2`, and current `indexedAt`.
- Latest Atlas blocks were sourced from block header timestamps, not indexer insertion time.
- Live block header timestamp at height `1817` was `1779160986`, while validator host wall time was around `1779173169-1779173184`, so the header clock was stale by about `12180-12200` seconds while still advancing by exactly 2 seconds per block.
- Installed control-panel resources on validators still reported `12.2.10+10af72192276`; the running node binary sampled on Validator 1 was the newer `e5b4202694f39139cd58268c14575e8270a8a1257cadee36ad043613eab0e06d`. This is release/runtime drift.

No live reset, validator restart, chain-data wipe, installer deployment, firewall change, topology change, or release promotion was performed during this audit.

## Implementation Status

| Area | Status | Wired into live runtime | Tests observed | Limitations / notes |
| --- | --- | --- | --- | --- |
| PQC enforcement through aegis-pqvm | Partially implemented | Partially | Handshake and vote/QC guard tests exist and some pass | Legacy crypto names remain in places. Full no-fallback audit is not complete. |
| Canonical serialization | Partially implemented | Partially | Some canonical genesis/artifact tests exist | Need full proof for every consensus-critical type and unordered-map exclusion. |
| Transaction admission | Partially implemented | Yes for legacy RPC/P2P admission | Focused validation paths exist | Full aegis-pqvm end-to-end admission proof still incomplete. |
| DAG mempool | Partially implemented | Partially | DAG helper tests exist | Live Atlas DAG table currently has zero vertices because post-reset signed DAG transactions have not been observed. |
| Deterministic execution | Scaffold/partial | Partially | Determinism scripts exist | Full repeated execution/thread-count/state-root proof still incomplete. |
| PoSy finalization | Implemented in legacy runtime | Yes | Consensus tests pass | New typed PoSy module is not the sole runtime path. |
| QC formation and verification | Implemented in legacy runtime | Yes | Consensus tests pass; P2P module tests pass serially | Need finish full invalid-context matrix across live legacy path. |
| Canonical block locks | Implemented | Yes | P2P module tests pass serially | Canonical-lock tests required isolation to avoid stale temp lock state. |
| Precommit guard | Implemented in legacy path | Yes | Focused guard tests exist | New typed PreCommitGuard is not fully wired as a standalone module. |
| Anti-divergence detection | Partial | Partially | Unit tests exist | Automatic fleet reconciliation is not proven live. |
| Self-quarantine | Partial | Partially | Unit tests exist | Needs end-to-end live acceptance coverage. |
| Reconciliation | Scaffold/partial | Not fully | Limited tests | Archive-assisted self-heal is not proven live. |
| Archive validator/snapshot sync | Scaffold/partial | Not live | Package scaffolding exists | Archive snapshot production and validator fast-sync are not proven on live TESTNET. |
| Validator onboarding | Partial | Control panel/runtime partial | Onboarding tests exist | Must still enforce standard lifecycle through `STAKE_REQUIRED -> STAKE_CONFIRMED` before activation. |
| Staking requirement | Partial | Partially | Some tests/docs exist | Needs full canonical finalized stake-lock verification in live onboarding flow. |
| Node Control Panel live status | Partial | UI partial | UI tests exist in control-panel repo | Validator hosts are still installed with stale resource version `12.2.10+10af72192276`. |
| Atlas DAG display | Partial | Atlas live but DAG empty | API/frontend tests exist | Atlas block indexing works; DAG display cannot show real topology until valid signed DAG txs are indexed. |
| Reset tooling | Implemented but high risk | Available | Installer validation scripts exist | Any reset must be explicit chain 1264 procedure preserving configs, keys, logs, and evidence. |
| Release packaging | Partial | Artifacts built | GitHub Actions green for OS artifacts | Publish-to-releases-repo skipped; validator hosts have runtime/package drift. |
| Relayer topology | Partially enforced | Live peer tables mostly match intended relayer bridge | Manual peer audit done | Need automated topology verification script and firewall/listener audit before declaring complete. |
| Observer behavior | Partial | Observer peers through relayers | Manual peer audit observed observer as non-genesis peer | Need tests proving observer cannot vote/propose/count toward quorum. |

## Current Fixes In This Working Tree

Block timestamp / cadence diagnosis:
- The consensus proposer previously set every new block timestamp to `previous.timestamp + block_time_secs`.
- When real block production was delayed, header timestamps continued to show perfect 2-second cadence while drifting behind wall time.
- Atlas, public RPC, relayers, and Postgres/API were accurately carrying this stale header timestamp. Atlas frontend was not the root cause.

Source changes now made in the working tree:
- `src/consensus/consensus_algorithm.rs` now computes a bounded consensus timestamp:
  - on-time production preserves target cadence,
  - delayed production catches the header timestamp up to the proposer wall-clock second,
  - cached retry proposals keep the same block hash and timestamp for the same height.
- `src/p2p/networking.rs` now applies a stricter block-sync response policy when a validator serves a non-active-validator/support peer:
  - max 4 blocks per response,
  - 100 ms write timeout,
  - normal non-validator nodes keep the existing 16-block / 1-second response budget,
  - consensus vote/proposal/block messages remain on the priority path outside the shared background queue.
- `scripts/testnet/verify-relayer-topology.sh` now provides a read-only operator topology check without embedding credentials.

## Tests Run

Passing:
- `cargo fmt`
- `cargo test -p synergy-testnet consensus_algorithm::tests -- --nocapture`
- `cargo test -p synergy-testnet p2p::networking::tests -- --nocapture --test-threads=1`
- `cargo test -p synergy-testnet dual_quorum::tests -- --nocapture`
- `cargo test -p synergy-testnet block_sync_response -- --nocapture --test-threads=1`
- `cargo test -p synergy-testnet validator_role_is_detected -- --nocapture`
- `cargo test -p synergy-testnet dispatch_peer_message_keeps_votes_off_the_background_queue -- --nocapture`
- `bash -n scripts/testnet/verify-relayer-topology.sh`
- `cargo build -p synergy-testnet --release --bin synergy-testnet`
  - local artifact hash: `2633b78040e4ab7ce53090c7a9036c415b5e33fcc6fcf1c153c8664294792cbb`
  - result: passed with existing dead-code warnings.

Notes:
- P2P module tests must be run serially because they share the test validator key registry and canonical-lock temp state.

## Required Before Live Mutation

Before any restart, reset, redeploy, firewall change, topology change, or release promotion:
- Capture fresh validator heights/hashes across all five validators.
- Capture public RPC and Atlas/indexer heights.
- Capture relayer peer tables.
- Capture block interval statistics from both header timestamps and wall-clock/indexer insertion times.
- Back up configs and logs.
- Preserve divergence/canonical-lock/evidence data.
- State the exact live change, why it is safe, and the rollback artifact.

## Known Limitations

- The live chain is aligned by hash, but the deployed consensus binary still emits stale block header timestamps until the bounded timestamp fix is built and rolled out.
- The package and live runtime are not aligned on validator hosts.
- Atlas does not need a chain reset for the timestamp display; it is rendering the canonical block header timestamp it receives.
- Postgres direct SSH access to the Atlas host using the supplied `synergyvps` path failed in the current session; public Atlas API checks succeeded.
- Full archive-validator, self-healing reconciliation, typed PoSy/DAG/execution overhaul, and non-genesis staking/onboarding enforcement remain incomplete unless separately proven by tests and live rollout evidence.
