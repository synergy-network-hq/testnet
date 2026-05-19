# Synergy Testnet v2 Chain 1264 Readiness Audit

Date: 2026-05-19

Scope: source tree, trusted GitHub Actions artifacts, and live rollout evidence for Synergy public TESTNET v2.

Canonical identity:
- chain_id: 1264
- chain_id_hex: 0x4f0
- network_id: synergy-testnet-v2
- product/network name: Synergy Testnet
- genesis validators: 5
- cluster_count: 1
- cluster_id: 0
- quorum: 4 of 5 active genesis validators
- target block cadence: 2 seconds

## Current Release Evidence

- Node repository: `synergy-network-hq/testnet`
- Node tag: `v12.2.19`
- Node commit: `c3b2585`
- Node GitHub Actions run: `26107094704`
- Node run status: Linux, macOS, Windows, and unified manifest succeeded.
- Control Panel repository: `synergy-network-hq/synergy-node-control-panel`
- Control Panel tag: `v12.2.19`
- Control Panel commit: `eb82d19`
- Control Panel GitHub Actions run: `26108441314`
- Control Panel run status: release workflow succeeded and published installer artifacts.

Trusted node runtime checksums:
- Linux `synergy-testnet-linux-amd64`: `f4d155867e179510c0fab90d33b6d74b64b650a7d2f978a734e80f7c77a25d7c`
- macOS and Windows artifacts were built by run `26107094704`; record platform checksums from the release manifest before the next live rollout.

Trusted Control Panel Linux package:
- `synergy-node-control-panel_12.2.19_amd64.deb`
- digest: `sha256:5ed100c2b5d061b9aa2520806d74bf982b2f4de02214efe7e30222b3f087dcc3`
- verified package manifest:
  - `app_version`: `12.2.19`
  - `workspace_resource_version`: `12.2.19`
  - `chain_id`: `1264`
  - `chain_id_hex`: `0x4f0`
  - `network_id`: `synergy-testnet-v2`
  - `genesis_hash`: `f79011f2aaddd40b120d47ba723104fafe3c998d4a17097fae018914b95f1789`

## Live Deployment Status

Deployed:
- all five genesis validators have Control Panel package `12.2.19`
- all five genesis validator runtime binaries hash to `f4d155867e179510c0fab90d33b6d74b64b650a7d2f978a734e80f7c77a25d7c`
- both relayers have runtime binary hash `f4d155867e179510c0fab90d33b6d74b64b650a7d2f978a734e80f7c77a25d7c`
- Atlas API and indexer are using relayer-only local tunnels:
  - primary: `http://127.0.0.1:8640`
  - fallback: `http://127.0.0.1:9640`
- public HTTPS RPC is available at `https://testnet-core-rpc.synergy-network.io/rpc`

Post-reset evidence observed during rollout:
- QCs and canonical locks were created from height 1 onward on validators.
- Latest block timestamps are close to wall clock after the v12.2.19 timestamp/QC-persistence rollout.
- Public RPC advanced 299 blocks in 598.025 seconds during the final observation window.
- Final short cadence sample observed 18 blocks in 34.842 seconds.
- Simultaneous validator sample at height 691 showed all five validators on hash `87a3338b82167705bd3d22cad9173c746042dac0fa4fa2d1470c7b60ab4c3b70`.
- Public RPC sample at height 917 showed hash `cc90af52ad7a99ff01bb095f34589f152eed3733eec34ec015bf12c9e4794e57` with current block timestamp.
- Atlas DB was backed up and reset before reindexing.
- Atlas API latest-block rows show block production timestamps separately from inserted/indexed time.
- DAG status is empty because no post-reset signed DAG transactions have been indexed:
  - `vertexCount`: `0`
  - `transactionCount`: `0`
  - `frontierCount`: `0`

Resolved live cadence finding:
- v12.2.19 changed committed QC persistence to append-only `data/committed_qcs.jsonl`, kept compatibility with legacy `committed_qcs.json`, skipped duplicate appends, and avoided persisting P2P QCs before a block was appended to the canonical tip.
- This fix was deployed from the trusted GitHub Actions release artifact, not an ad hoc local binary.
- Bootstrap/seed deleted-inode drift was corrected during rollout by replacing retired `synergy-testbeta` process launch paths with the current Testnet runtime and moving old inactive builds to a retired-builds archive on the service host.

## Implementation Status

| Area | Status | Wired into live runtime | Tests observed | Limitations / notes |
| --- | --- | --- | --- | --- |
| PQC enforcement through aegis-pqvm | Partially implemented | Partially | PQC-only guard passed in GitHub Actions; lifecycle stake tests now use real `aegis_pqvm` signatures | Full no-fallback audit across every consensus-critical path remains incomplete. |
| Canonical serialization | Partially implemented | Partially | Existing serialization/genesis checks run in CI | Full proof for every consensus-critical type and unordered-map exclusion remains incomplete. |
| Transaction admission | Partially implemented | Yes for legacy RPC/P2P admission | Focused validation paths exist | Full end-to-end aegis-pqvm admission proof remains incomplete. |
| DAG mempool | Partially implemented | Partially | DAG helper/API paths exist | Live DAG is empty after reset until valid signed DAG transactions are submitted. The current portable wallet helper signs through the wallet PQC CLI, not the mandatory `aegis-pqvm` transaction-key path required to prove DAG ingestion end to end. |
| Deterministic execution | Scaffold/partial | Partially | Determinism scripts/tests exist | Full repeated execution/thread-count/state-root proof remains incomplete. |
| PoSy finalization | Implemented in legacy runtime | Yes | Live QCs and canonical locks from height 1 onward | New typed PoSy module is not the sole runtime path. |
| QC formation and verification | Implemented in legacy runtime | Yes | Consensus/P2P tests and live finality observed | Need full invalid-context matrix across live legacy path. |
| Canonical block locks | Implemented | Yes | Live canonical locks observed from height 1 onward | Standalone typed lock module is not the only enforcement path. |
| Precommit guard | Implemented in legacy path | Yes | Focused guard tests exist | New typed PreCommitGuard is not fully wired as the sole guard. |
| Anti-divergence detection | Partial | Partially | Unit tests exist | Automatic fleet reconciliation is not proven live. |
| Self-quarantine | Partial | Partially | Unit tests exist | Needs end-to-end live acceptance coverage. |
| Reconciliation | Scaffold/partial | Not fully | Limited tests | Archive-assisted self-heal is not proven live. |
| Archive validator/snapshot sync | Scaffold/partial | Not live | Archive package asset tests pass locally | Linux/macOS package structure now exists and fails closed for missing binary/signing/notarization inputs. Snapshot production, API serving, and validator fast-sync commands are intentionally fail-closed stubs until full verified archive storage/install logic is wired. |
| Validator onboarding | Partial | Control Panel/runtime partial | Lifecycle order and stake-gate tests pass locally | Must still prove full `STAKE_REQUIRED -> STAKE_CONFIRMED -> ACTIVE` live flow. Legacy direct register/approve CLI and RPC shortcuts are now disabled in source. |
| Staking requirement | Partial | Partially | Wrong chain/network/signature, under-stake, exact stake, over-stake, bad status, identity mismatch, and no-skip tests pass locally | Needs canonical finalized stake-lock verification in live onboarding flow. |
| Node Control Panel live status | Partial | UI/package partial | UI tests exist in control-panel repo | Live hosts have current package, but full UI state-panel acceptance was not reverified visually in this rollout. |
| Atlas DAG display | Partial | Atlas live, DAG empty | API/frontend paths exist | Empty DAG is expected until real signed DAG transactions are indexed. |
| Reset tooling | Implemented but high risk | Available | Installer validation scripts exist | Resets must preserve keys/config/logs/evidence and use chain 1264 only. |
| Release packaging | Implemented for v12.2.19 | Yes | Node and Control Panel release workflows succeeded | Release packaging should keep rejecting stale 1262/1263/Testnet-Beta deployable material. |
| Relayer topology | Partially enforced | Live peer tables match validator-private relayer bridge | Manual peer audit done | Need continuous topology verification in CI/ops. |
| Observer behavior | Partial | Observer not active in final sample | Manual preflight saw no observer qRPC state | Need tests proving observer cannot vote/propose/count toward quorum. |

## Source Fixes Included in v12.2.19

Timestamp/cadence:
- The proposer no longer stamps delayed blocks as only `parent.timestamp + block_time_secs`.
- Timestamp selection catches up to wall clock while remaining monotonic and bounded.
- Validators reject timestamps outside consensus bounds.

Leader timeout/liveness:
- Followers no longer treat a fresh tip as immediately timed out because its header timestamp lags local wall time.
- Leader timeout now uses local tip-observation elapsed time, while header timestamp validation remains separate.

Support-node catch-up:
- Support-node block sync responses are bounded.
- Consensus vote/proposal/QC paths remain prioritized over bulk catch-up traffic.

QC persistence:
- Committed QCs are no longer persisted by rewriting the full QC map every block.
- New durable file: `data/committed_qcs.jsonl`.
- Legacy load compatibility remains for `data/committed_qcs.json`.
- Reset tooling now removes both `committed_qcs.json` and `committed_qcs.jsonl`.
- P2P block application records a QC only after the block extends the local canonical tip.

Follower QC validation:
- Follower roles hydrate canonical genesis validators before verifying reset-chain QCs.
- Non-validator roles can validate height-1 QCs without being allowed to vote/propose/count toward quorum.

## Tests Run

Passing local checks before tag:
- `cargo fmt`
- `cargo test leader --lib`
- `cargo test block_sync --lib`
- `cargo test committed_qc_store --lib`
- `cargo test p2p::networking::tests --lib`
- `cargo test consensus::dual_quorum::tests --lib`
- `cargo check --manifest-path control-service/Cargo.toml --bin control-service --no-default-features`
- `SKIP_BUNDLED_ASSET_GIT_CLEAN_CHECK=1 bash scripts/release/validate-bundled-assets.sh`
- `git diff --check`

Passing GitHub Actions:
- `synergy-network-hq/testnet` run `26107094704`
- `synergy-network-hq/synergy-node-control-panel` run `26108441314`

Additional local checks after this audit update:
- `cargo test validator_lifecycle --lib`
- `cargo test archive_validator --lib`
- `bash -n archive-validator/package-archive-validator.sh archive-validator/setup-archive-validator.sh archive-validator/macos/build-macos-pkg.sh archive-validator/macos/preinstall archive-validator/macos/postinstall archive-validator/macos/uninstall-macos.sh`
- `git diff --check`

## Known Limitations

- Full typed-module overhaul is not complete; the live runtime still uses the legacy consensus path plus hardened guards.
- Full Aegis PQC end-to-end proof remains incomplete for every artifact class listed in the long-form requirements.
- Archive validator snapshot creation/verification is scaffolded but not live-proven.
- The `synergy-archive` serve/create/verify commands and `synergy-node sync-from-archive` / `self-heal-from-archive` commands refuse to mutate or serve until the real verified storage/snapshot implementation is wired.
- Automatic self-heal and rejoin is not live-proven.
- Non-genesis staking/onboarding safety is partially implemented but not fully live-accepted.
- Atlas DAG is correctly empty after reset because no signed DAG transactions have been indexed yet.
- Historical directories/backups may retain old names for forensics. Active systemd units and deployed runtime manifests must not.
