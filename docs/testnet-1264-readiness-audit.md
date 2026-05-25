# Synergy Testnet v2 Chain 1264 Readiness Audit

Date: 2026-05-20

Scope: source tree, trusted GitHub Actions artifacts, and live rollout evidence for Synergy public TESTNET v2. This audit is intentionally conservative: it distinguishes source changes, CI artifacts, deployed runtime behavior, and live acceptance proof.

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

## Current Source and Release Evidence

- Node repository: `synergy-network-hq/testnet`
- Last deployed fully observed release: `v12.2.19`
- Latest node source commit on `testnet/main`: `b019c61` (`fix: wire aegis pqvm transaction submission path`)
- Latest node source tag with green trusted artifacts: `v12.2.25`
- Node GitHub Actions run for `v12.2.25`: `26185757460`
- Node run status at this audit update: succeeded for Linux, macOS, Windows, and `latest.json` publication.
- Previous node source commit `c6090c0` added a local `aegis-pqvm` transaction-key DAG CLI proof.
- Node source commit `b019c61` wires the generated `aegis-pqvm` transaction envelope into a public RPC method, `synergy_submitAegisTransaction`, and validates the legacy carrier transaction through the typed Aegis transaction-key path. This is source-only until a new trusted artifact is tagged, built, and deployed.
- Control Panel repository: `synergy-network-hq/synergy-node-control-panel`
- Last deployed fully observed Control Panel release: `v12.2.19`
- Latest Control Panel source commit on `main`: `342deb9` (`fix: install release validation tools`)
- Control Panel `v12.2.22` release run `26183645638` failed in bundled asset validation because the workflow checked out the canonical node repo into `testnet-source/` while the validator script only looked for `../config/genesis.json`.
- The validation script now prefers `testnet-source/config/genesis.json` and falls back to `../config/genesis.json`.
- Control Panel `v12.2.24` workflow run `26185416441` failed before installer build because the runner did not have `rg` for bundled-asset validation.
- Control Panel `v12.2.25` workflow run `26186714962` succeeded for Linux, macOS, and Windows installers after adding explicit release validation tool installation.

Trusted deployed node runtime checksums:
- Earlier deployed Linux `v12.2.19` runtime: `f4d155867e179510c0fab90d33b6d74b64b650a7d2f978a734e80f7c77a25d7c`
- Latest live validator samples currently hash to `1325ef8f36d51ec6d01a166b22710c3fba170af3e3d691aa990bf4d788f289fc`; this must be tied back to its trusted release manifest before any final rollout claim.
- `v12.2.25` trusted node checksums were published in `latest.json`:
  - Linux `synergy-testnet`: `bc74a3ae1a480c5dae351ebed2707c5c34b3bf9046f0b60ea68900bd2caf467a`
  - macOS `synergy-testnet`: `a9032478dacce5bd89ef4d72b3ceb063074820e53154df231581667decce6824`
  - Windows `synergy-testnet`: `04fd8891cee7d81b42f55bee7fdea3ec3449d96193adc7d80c7918021399218d`

Last verified Control Panel Linux package:
- `synergy-node-control-panel_12.2.25_amd64.deb`
- digest: `sha256:03e49542343ff82f77307a8262465f14cb83010613b7c6b946c4b565469cd7b8`
- verified package manifest:
  - `app_version`: `12.2.25`
  - `workspace_resource_version`: `12.2.25`
  - `chain_id`: `1264`
  - `chain_id_hex`: `0x4f0`
  - `network_id`: `synergy-testnet-v2`
  - `genesis_hash`: `f79011f2aaddd40b120d47ba723104fafe3c998d4a17097fae018914b95f1789`

## Live Deployment Status

Current read-only preflight evidence gathered on 2026-05-20:
- Validators 1, 2, 4, and 5 were advancing, but not sampled at the exact same height/hash before mutation.
- Validator 3 was divergent/stuck at height `11668` on hash `5b3bed3ac4377db5451fdc68366f31b14103fb867a4d8b756a29472435f444a2`.
- Validator 3 logs show canonical lock conflict evidence at height `11668`: the local canonical lock binds `5b3bed3ac4377db5451fdc68366f31b14103fb867a4d8b756a29472435f444a2` and rejects conflicting block `b8871be5fcb4f3b8069ecfd23287ed9ab1e0c747684c2edeaaf7e52214c0f915`.
- Validator 3 continues refusing vote requests for later heights because proposals do not extend local tip `11668`. This is live divergence and must not be described as stable.
- Public RPC peer table sampled with two peers, `relay1` and `relay2`, which matches the relayer-only public RPC topology requirement for that sample.
- Public RPC block interval samples ending near height `12260` were about 3 seconds, not the 2 second target:
  - latest 50 blocks: average `3.0408s`, min `2s`, max `6s`
  - latest 120 blocks: average `3.0s`, min `2s`, max `6s`
  - latest 300 blocks: average `3.0033s`, min `2s`, max `6s`
- Public RPC and Atlas timestamps are sane in the latest sample; block production timestamp is close to host wall clock.
- Atlas API and indexer are using relayer-only local tunnels:
  - primary: `http://127.0.0.1:8640`
  - fallback: `http://127.0.0.1:9640`
- public HTTPS RPC is available at `https://testnet-core-rpc.synergy-network.io/rpc`
- Atlas summary sampled chain `1264`, latest block around `12254`, active validators `5`, peer count `7`, and current indexed time.
- Atlas DAG status remains empty:
  - `vertexCount`: `0`
  - `transactionCount`: `0`
  - `frontierCount`: `0`
  - This is still expected only because no live post-reset real signed DAG transactions have been submitted and indexed.

Evidence-preserving live repair work performed after preflight:
- Validator 3 divergence evidence was backed up on-host at `/home/rob/.synergy/backups/v3-divergence-pre-v12.2.25-20260520T201929Z`.
- Validator 3 was upgraded to the trusted Control Panel `12.2.25` package and trusted node runtime checksum `bc74a3ae1a480c5dae351ebed2707c5c34b3bf9046f0b60ea68900bd2caf467a`, then restarted from a clean local chain-following state while preserving keys, configs, logs, and evidence.
- While Validator 3 was rebuilding, Validator 1 encountered a same-height canonical conflict at height `13781`. Validator 1 had canonical lock block `e5b3540999f5ee8aa29e8aecc3bda503216fe6218eb47cb5a001fd7a11cb51be`, while Validators 2/4/5 and public RPC agreed on block `a1c25fb8821e43fbd3be3718032c527b9b9e67b375b4d62ecf59e00f84cd83a1`.
- Validator 1 divergence evidence was backed up on-host at `/home/justin/.synergy/backups/v1-divergence-13781-pre-v12.2.25-20260520T203850Z`.
- Validator 1 was upgraded to Control Panel `12.2.25` and trusted node runtime checksum `bc74a3ae1a480c5dae351ebed2707c5c34b3bf9046f0b60ea68900bd2caf467a`. Only height `13781+` divergent consensus artifacts and volatile proposal/vote caches were removed after backup; persisted canonical chain data through height `13780` was retained.
- Post-repair same-height sample at height `13905` showed Validator 1, Validator 2, Validator 4, Validator 5, and public RPC all on hash `00a555dee612ca8b568bff4b23ce4b9ed0b7738717b4409033615845389c34de`.
- Validator 3 remained in genesis catch-up during this audit update and must not be counted as fully reconciled until it reaches the current canonical head and matches the validator quorum by height/hash.

Deployed status from the previous accepted rollout remains useful historical context but is no longer current-health proof:
- v12.2.19 produced QCs and canonical locks from height 1 onward after reset.
- v12.2.19 fixed stale block header timestamps and large committed QC rewrite pressure.
- v12.2.19 was installed from trusted GitHub Actions artifacts, not ad hoc local binaries.

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
- Bootstrap/seed deleted-inode drift was corrected during rollout by replacing retired `synergy-testnet` process launch paths with the current Testnet runtime and moving old inactive builds to a retired-builds archive on the service host.

## Implementation Status

| Area | Status | Wired into live runtime | Tests observed | Limitations / notes |
| --- | --- | --- | --- | --- |
| PQC enforcement through aegis-pqvm | Partially implemented | Partially | PQC-only guard passed in GitHub Actions; lifecycle stake tests now use real `aegis_pqvm` signatures; local `aegis_tx_tool` tests prove transaction-key signing/verification through `aegis-pqvm` | Full no-fallback audit across every consensus-critical path remains incomplete. Live transaction RPC/Atlas DAG admission proof remains incomplete. |
| Canonical serialization | Partially implemented | Partially | Existing serialization/genesis checks run in CI | Full proof for every consensus-critical type and unordered-map exclusion remains incomplete. |
| Transaction admission | Partially implemented | Yes for legacy RPC/P2P admission | Focused validation paths exist | Full end-to-end aegis-pqvm admission proof remains incomplete. |
| DAG mempool | Partially implemented | Partially | `cargo test -p synergy-testnet aegis_tx_tool -- --nocapture` passed; `cargo test -p synergy-testnet dag_mempool -- --nocapture` passed; `synergy-node dag submit-test-fixture --real-aegis-pqvm` locally created independent, sequence-dependent, explicit-dependency, and conflict transactions with `wallet_cli_used=false` and `demo_data_used=false` | Live DAG is still empty after reset. Commit `b019c61` wires the source RPC path for real Aegis envelopes, but it is not yet in a tagged trusted artifact or deployed live. Atlas DAG indexing through this path remains unproven. |
| Deterministic execution | Scaffold/partial | Partially | Determinism scripts/tests exist | Full repeated execution/thread-count/state-root proof remains incomplete. |
| PoSy finalization | Implemented in legacy runtime | Yes | Live QCs and canonical locks from height 1 onward | New typed PoSy module is not the sole runtime path. |
| QC formation and verification | Implemented in legacy runtime | Yes | Consensus/P2P tests and live finality observed | Need full invalid-context matrix across live legacy path. |
| Canonical block locks | Implemented | Yes | Live canonical locks observed from height 1 onward | Standalone typed lock module is not the only enforcement path. |
| Precommit guard | Implemented in legacy path | Yes | Focused guard tests exist | New typed PreCommitGuard is not fully wired as the sole guard. |
| Anti-divergence detection | Partial | Partially | `cargo test -p synergy-testnet consensus::anti_divergence -- --nocapture` passed after `22cff3e` | Live Validator 3 is currently divergent at height `11668`. The new source fix records self-quarantine on future canonical-lock conflicts, but it has not been deployed or live-accepted. |
| Self-quarantine | Partial | Source implemented, deployment pending | `self_quarantine_record_persists_canonical_lock_conflict_evidence` and `divergent_validator_quarantines_and_reconciliation_blocks_duties` passed | The live fleet already contains a divergent Validator 3 from before this fix. Controlled evidence-preserving reconciliation is still required. |
| Reconciliation | Scaffold/partial | Not fully | Limited tests | Archive-assisted self-heal is not proven live. |
| Archive validator/snapshot sync | Scaffold/partial | Not live | Archive package asset tests pass locally | Linux/macOS package structure now exists and fails closed for missing binary/signing/notarization inputs. Snapshot production, API serving, and validator fast-sync commands are intentionally fail-closed stubs until full verified archive storage/install logic is wired. |
| Validator onboarding | Partial | Control Panel/runtime partial | Lifecycle order and stake-gate tests pass locally | Must still prove full `STAKE_REQUIRED -> STAKE_CONFIRMED -> ACTIVE` live flow. Legacy direct register/approve CLI and RPC shortcuts are now disabled in source. |
| Staking requirement | Partial | Partially | Wrong chain/network/signature, under-stake, exact stake, over-stake, bad status, identity mismatch, and no-skip tests pass locally | Needs canonical finalized stake-lock verification in live onboarding flow. |
| Node Control Panel live status | Partial | UI/package partial | UI tests exist in control-panel repo | Live hosts have current package, but full UI state-panel acceptance was not reverified visually in this rollout. |
| Atlas DAG display | Partial | Atlas live, DAG empty | API/frontend paths exist | Empty DAG is expected until real signed DAG transactions are indexed. |
| Reset tooling | Implemented but high risk | Available | Installer validation scripts exist | Resets must preserve keys/config/logs/evidence and use chain 1264 only. |
| Release packaging | Implemented for v12.2.25 artifacts | Partially deployed live | Node `v12.2.25` run `26185757460` and Control Panel `v12.2.25` run `26186714962` succeeded. Validators 1 and 3 have been upgraded to the trusted `v12.2.25` runtime after evidence-preserving repair. Validators 2, 4, and 5 remain on the older runtime until Validator 3 catches up or a new safe rolling plan exists. | Package drift is not fully eliminated. Commit `b019c61` requires a new trusted artifact before deployment. |
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

## Source Fixes Pending Trusted Artifact Deployment

`c6090c0`:
- Added `synergy-node tx create-aegis`, `tx sign-aegis`, `tx submit-aegis`, and `dag submit-test-fixture --real-aegis-pqvm`.
- Uses the real `aegis-pqvm` transaction-key path with `AegisPqvmSigner::initialize_required`, transaction-role key registration, `SYNERGY_TX_V1` signing, and verification through `AegisPqvmVerifier`.
- The helper refuses to claim live Atlas ingestion when the typed transaction RPC path is not wired.

`22cff3e`:
- Records canonical-lock conflicts as `data/validator_quarantine.json` self-quarantine evidence.
- Blocks validator proposal/vote/QC duties and inbound vote-request handling when the local node is self-quarantined.
- Preserves canonical-lock conflict details for later reconciliation instead of continuing normal consensus participation.
- This fix directly addresses the live Validator 3 failure mode observed at height `11668`, but it is source-only until released and deployed.

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

Passing local checks for the pending `v12.2.25` source:
- `cargo fmt --check`
- `git diff --check`
- `cargo test -p synergy-testnet aegis_tx_tool -- --nocapture`
- `cargo run -p synergy-testnet --bin synergy-node -- dag submit-test-fixture --real-aegis-pqvm --chain-id 1264 --network-id synergy-testnet-v2`
- `cargo test -p synergy-testnet self_quarantine_record_persists_canonical_lock_conflict_evidence -- --nocapture`
- `cargo test -p synergy-testnet divergent_validator_quarantines_and_reconciliation_blocks_duties -- --nocapture`
- `cargo test -p synergy-testnet consensus::anti_divergence -- --nocapture`
- Control Panel: `SKIP_BUNDLED_ASSET_GIT_CLEAN_CHECK=1 ./scripts/release/validate-bundled-assets.sh`
- Control Panel: `bash -n scripts/release/validate-bundled-assets.sh`
- Control Panel: `cargo check --manifest-path control-service/Cargo.toml --bin control-service --no-default-features`

Passing local checks for source commit `b019c61`:
- `cargo fmt`
- `cargo test -p synergy-testnet aegis_tx_tool -- --nocapture`
- `cargo test -p synergy-testnet dag_mempool -- --nocapture`

## Known Limitations

- Full typed-module overhaul is not complete; the live runtime still uses the legacy consensus path plus hardened guards.
- Full Aegis PQC end-to-end proof remains incomplete for every artifact class listed in the long-form requirements.
- The new `aegis-pqvm` DAG transaction helper and `synergy_submitAegisTransaction` source path are not yet a live end-to-end DAG proof because commit `b019c61` still needs a trusted artifact, live deployment, real transaction submission, finality, and Atlas DB/API/frontend evidence.
- Archive validator snapshot creation/verification is scaffolded but not live-proven.
- The `synergy-archive` serve/create/verify commands and `synergy-node sync-from-archive` / `self-heal-from-archive` commands refuse to mutate or serve until the real verified storage/snapshot implementation is wired.
- Automatic self-heal and rejoin is not live-proven.
- Validator 3 is no longer left on the divergent local lock, but it is still rebuilding from genesis and must catch up to the canonical head before the fleet can be called healthy.
- Validator 1 experienced a same-height conflict at height `13781` during Validator 3 catch-up. Evidence was preserved and the node was repaired back to canonical height `13780`; the old-runtime conflict mechanism remains a critical incident requiring regression hardening and final verification.
- Current public RPC block interval samples are around 3 seconds, so the 2 second cadence target is not yet satisfied.
- Non-genesis staking/onboarding safety is partially implemented but not fully live-accepted.
- Atlas DAG is correctly empty after reset because no signed DAG transactions have been indexed yet.
- Historical directories/backups may retain old names for forensics. Active systemd units and deployed runtime manifests must not.
