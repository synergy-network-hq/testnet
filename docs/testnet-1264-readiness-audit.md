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
- Node tag: `v12.2.18`
- Node commit: `ad96256133af76ad670ea7f5f532ada454d06f4b`
- Node GitHub Actions run: `26103018856`
- Node run status: Linux, macOS, Windows, and unified manifest succeeded.
- Control Panel repository: `synergy-network-hq/synergy-node-control-panel`
- Control Panel tag: `v12.2.18`
- Control Panel commit: `6d2ff941facc2d07a310dd2a3b8321223b04f276`
- Control Panel GitHub Actions run: `26104388942`
- Control Panel run status: Linux, macOS, and Windows installers succeeded.

Trusted node runtime checksums:
- Linux `synergy-testnet-linux-amd64`: `828ebdf05cb52ae31f2694b19d0b2c2663c3c43e1bae31e99ded91ccd5592689`
- macOS `synergy-testnet-macos-arm64`: `f8675d0800426feec856359f23116fb32744744c1f342e627246972eb021a5fc`
- Windows `synergy-testnet-windows-amd64.exe`: `191bc7276f2d0b7eaca7eb6a1901142dde6b903b08c8dc64d72508d109c73ea5`

Trusted Control Panel Linux package:
- `synergy-node-control-panel_12.2.18_amd64.deb`
- digest: `sha256:29f33189fdf5ebc6cc30258196ed33d82b98e4280e4b9bedd254f8ccb94f1c40`
- verified package manifest:
  - `app_version`: `12.2.18`
  - `workspace_resource_version`: `12.2.18+47907c98de8e`
  - `chain_id`: `1264`
  - `chain_id_hex`: `0x4f0`
  - `network_id`: `synergy-testnet-v2`
  - `genesis_hash`: `f79011f2aaddd40b120d47ba723104fafe3c998d4a17097fae018914b95f1789`

## Live Deployment Status

Deployed:
- all five genesis validators have Control Panel package `12.2.18`
- all five genesis validator runtime binaries hash to `828ebdf05cb52ae31f2694b19d0b2c2663c3c43e1bae31e99ded91ccd5592689`
- both relayers have runtime binary hash `828ebdf05cb52ae31f2694b19d0b2c2663c3c43e1bae31e99ded91ccd5592689`
- Atlas API and indexer are using relayer-only local tunnels:
  - primary: `http://127.0.0.1:8640`
  - fallback: `http://127.0.0.1:9640`
- public HTTPS RPC is available at `https://testnet-core-rpc.synergy-network.io/rpc`

Post-reset evidence observed during rollout:
- QCs and canonical locks were created from height 1 onward on validators.
- Latest block timestamps are close to wall clock after the v12.2.18 timestamp/leader-timeout fix.
- Validator-local post-reset cadence sample at height 187:
  - latest timestamp delta: 4 seconds
  - 50-block average: 2.184 seconds
  - 120-block average: 2.076 seconds
  - available 187-block average: 2.054 seconds
- Atlas DB was backed up and reset before reindexing.
- Atlas API latest-block rows show block production timestamps separately from inserted/indexed time.
- DAG status is empty because no post-reset signed DAG transactions have been indexed:
  - `vertexCount`: `0`
  - `transactionCount`: `0`
  - `frontierCount`: `0`

## Implementation Status

| Area | Status | Wired into live runtime | Tests observed | Limitations / notes |
| --- | --- | --- | --- | --- |
| PQC enforcement through aegis-pqvm | Partially implemented | Partially | PQC-only guard passed in GitHub Actions | Full no-fallback audit across every consensus-critical path remains incomplete. |
| Canonical serialization | Partially implemented | Partially | Existing serialization/genesis checks run in CI | Full proof for every consensus-critical type and unordered-map exclusion remains incomplete. |
| Transaction admission | Partially implemented | Yes for legacy RPC/P2P admission | Focused validation paths exist | Full end-to-end aegis-pqvm admission proof remains incomplete. |
| DAG mempool | Partially implemented | Partially | DAG helper/API paths exist | Live DAG is empty after reset until valid signed DAG transactions are submitted. |
| Deterministic execution | Scaffold/partial | Partially | Determinism scripts/tests exist | Full repeated execution/thread-count/state-root proof remains incomplete. |
| PoSy finalization | Implemented in legacy runtime | Yes | Live QCs and canonical locks from height 1 onward | New typed PoSy module is not the sole runtime path. |
| QC formation and verification | Implemented in legacy runtime | Yes | Consensus/P2P tests and live finality observed | Need full invalid-context matrix across live legacy path. |
| Canonical block locks | Implemented | Yes | Live canonical locks observed from height 1 onward | Standalone typed lock module is not the only enforcement path. |
| Precommit guard | Implemented in legacy path | Yes | Focused guard tests exist | New typed PreCommitGuard is not fully wired as the sole guard. |
| Anti-divergence detection | Partial | Partially | Unit tests exist | Automatic fleet reconciliation is not proven live. |
| Self-quarantine | Partial | Partially | Unit tests exist | Needs end-to-end live acceptance coverage. |
| Reconciliation | Scaffold/partial | Not fully | Limited tests | Archive-assisted self-heal is not proven live. |
| Archive validator/snapshot sync | Scaffold/partial | Not live | Package scaffolding exists | Archive snapshot production and validator fast-sync are not proven on live TESTNET. |
| Validator onboarding | Partial | Control Panel/runtime partial | Onboarding tests exist | Must still prove full `STAKE_REQUIRED -> STAKE_CONFIRMED -> ACTIVE` live flow. |
| Staking requirement | Partial | Partially | Some tests/docs exist | Needs canonical finalized stake-lock verification in live onboarding flow. |
| Node Control Panel live status | Partial | UI/package partial | UI tests exist in control-panel repo | Live hosts have current package, but full UI state-panel acceptance was not reverified visually in this rollout. |
| Atlas DAG display | Partial | Atlas live, DAG empty | API/frontend paths exist | Empty DAG is expected until real signed DAG transactions are indexed. |
| Reset tooling | Implemented but high risk | Available | Installer validation scripts exist | Resets must preserve keys/config/logs/evidence and use chain 1264 only. |
| Release packaging | Implemented for v12.2.18 | Yes | Node and Control Panel release workflows succeeded | Release packaging should keep rejecting stale 1262/1263/Testnet-Beta deployable material. |
| Relayer topology | Partially enforced | Live peer tables match validator-private relayer bridge | Manual peer audit done | Need continuous topology verification in CI/ops. |
| Observer behavior | Partial | Observer not active in final sample | Manual preflight saw no observer qRPC state | Need tests proving observer cannot vote/propose/count toward quorum. |

## Source Fixes Included in v12.2.18

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

Follower QC validation:
- Follower roles hydrate canonical genesis validators before verifying reset-chain QCs.
- Non-validator roles can validate height-1 QCs without being allowed to vote/propose/count toward quorum.

## Tests Run

Passing local checks before tag:
- `cargo fmt`
- `cargo test leader --lib`
- `cargo test block_sync --lib`
- `cargo check --manifest-path control-service/Cargo.toml --bin control-service --no-default-features`
- `SKIP_BUNDLED_ASSET_GIT_CLEAN_CHECK=1 bash scripts/release/validate-bundled-assets.sh`
- `git diff --check`

Passing GitHub Actions:
- `synergy-network-hq/testnet` run `26103018856`
- `synergy-network-hq/synergy-node-control-panel` run `26104388942`

## Known Limitations

- Full typed-module overhaul is not complete; the live runtime still uses the legacy consensus path plus hardened guards.
- Full Aegis PQC end-to-end proof remains incomplete for every artifact class listed in the long-form requirements.
- Archive validator snapshot creation/verification is scaffolded but not live-proven.
- Automatic self-heal and rejoin is not live-proven.
- Non-genesis staking/onboarding safety is partially implemented but not fully live-accepted.
- Atlas DAG is correctly empty after reset because no signed DAG transactions have been indexed yet.
- Historical directories/backups may retain old names for forensics. Active systemd units and deployed runtime manifests must not.
