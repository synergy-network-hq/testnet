# Synergy Testnet 1264 Read-Only Preflight

Date: 2026-05-20

This preflight is read-only. No validator restart, reset, installer deployment, firewall change, database wipe, or service mutation was performed while collecting this snapshot.

## Source and Release State

- Node repository: `synergy-network-hq/testnet`
- Node branch: `main`
- Latest pushed node commit: `403663e` (`docs: complete validator checksum preflight`)
- Runtime fix commit under tag build: `22cff3e` (`fix: self quarantine on canonical lock divergence`)
- Node tag under trusted artifact build: `v12.2.24`
- Node GitHub Actions run: `26184477201`
- Node run status after capture refresh: `v12.2.24` succeeded and published Linux, macOS, Windows, and `latest.json` release artifacts.
- Control Panel repository: `synergy-network-hq/synergy-node-control-panel`
- Latest pushed Control Panel commit: `4a866c6` (`fix: verify onboarding public sync target identity`)
- Control Panel `v12.2.24` workflow run `26185416441` failed before installer build because `rg` was missing on the runner; the fix must be promoted under a new tag.

## Validator State

| Validator | Latest height | Hash | Parent hash | Block timestamp | Host time | Delta | Runtime checksum / note |
| --- | ---: | --- | --- | ---: | ---: | ---: | --- |
| Validator 1 | 12543 | `5cbf7a3ead399d04972d5f4eb9b844d35bafa4d5ea6f0e9f1bf686095912ae4e` | `447dd79d730a665566fc476530d85558436add84f2c80e64720ff51e9840a3ea` | 1779305065 | 1779305069 | 4s | `1325ef8f36d51ec6d01a166b22710c3fba170af3e3d691aa990bf4d788f289fc` |
| Validator 2 | 12543 | `5cbf7a3ead399d04972d5f4eb9b844d35bafa4d5ea6f0e9f1bf686095912ae4e` | `447dd79d730a665566fc476530d85558436add84f2c80e64720ff51e9840a3ea` | 1779305065 | 1779305068 | 3s | `1325ef8f36d51ec6d01a166b22710c3fba170af3e3d691aa990bf4d788f289fc` |
| Validator 3 | 11668 | `5b3bed3ac4377db5451fdc68366f31b14103fb867a4d8b756a29472435f444a2` | `3238715aed3dcadee39a2279dc3e524d9d6f28a080557bbf45f51457b616036b` | 1779302405 | 1779305070 | 2665s | `1325ef8f36d51ec6d01a166b22710c3fba170af3e3d691aa990bf4d788f289fc`; divergent/stuck. |
| Validator 4 | 12542 | `447dd79d730a665566fc476530d85558436add84f2c80e64720ff51e9840a3ea` | `9aa5e5dfb0ba21ef3d319743870cbedc28c4a495ec733d67a999ab5a47fcf4a8` | 1779305063 | 1779305066 | 3s | `1325ef8f36d51ec6d01a166b22710c3fba170af3e3d691aa990bf4d788f289fc` |
| Validator 5 | 12543 | `5cbf7a3ead399d04972d5f4eb9b844d35bafa4d5ea6f0e9f1bf686095912ae4e` | `447dd79d730a665566fc476530d85558436add84f2c80e64720ff51e9840a3ea` | 1779305065 | 1779305069 | 4s | `1325ef8f36d51ec6d01a166b22710c3fba170af3e3d691aa990bf4d788f289fc` |

Validator 3 divergence evidence:
- Local tip remains height `11668`.
- Existing logs show a canonical lock conflict at height `11668`: local canonical lock binds `5b3bed3ac4377db5451fdc68366f31b14103fb867a4d8b756a29472435f444a2` and rejects conflicting block `b8871be5fcb4f3b8069ecfd23287ed9ab1e0c747684c2edeaaf7e52214c0f915`.
- Validators 1, 2, 4, and 5 continue advancing, so the network has not fully stalled, but the fleet is not healthy.

QC and canonical lock files:
- Validators sampled have `data/committed_qcs.jsonl` and `data/canonical_locks.json`.
- Current `committed_qcs.jsonl` sizes are about 959 MB on Validator 3 and about 1.03 GB on the advancing validators.
- Existing backup copies under `backups/pre-v12.2.*` are present and must be preserved as evidence.

## Public and Relayer Plane

Public RPC sample:
- Latest height: `12550`
- Latest hash: `067e01176d00b56d0eac0e98940e0e9bbf00bdc47cac4b9cfa1b3db63e404aff`
- Latest timestamp: `1779305086`
- Local collection time: `1779305094`
- Timestamp delta: 8 seconds

Public RPC block interval statistics ending at height `12550`:
- Latest 50 blocks: average `3.0204s`, min `2s`, max `6s`
- Latest 120 blocks: average `3.0168s`, min `2s`, max `6s`
- Latest 300 blocks: average `3.0201s`, min `2s`, max `7s`

Relayer 1:
- Latest height: `12570`
- Latest hash: `75d3c6185e6da6c6be3b66b68a8db4921d435e7e919536c59921c15874f44170`
- Runtime path: `/opt/synergy/testnet/relayer/bin/synergy-testnet-linux-amd64`
- Runtime checksum: `f4d155867e179510c0fab90d33b6d74b64b650a7d2f978a734e80f7c77a25d7c`
- Metrics included `synergy_chain_height 12570`.

Relayer 2:
- Latest height: `12569`
- Latest hash: `666c81d02e59b380a25f4be70f8b4039bd6dfd207653d4d1ed234cd108f82ce1`
- Runtime path: `/opt/synergy/testnet/relayer/bin/synergy-testnet-linux-amd64`
- Runtime checksum: `f4d155867e179510c0fab90d33b6d74b64b650a7d2f978a734e80f7c77a25d7c`
- Metrics included `synergy_chain_height 12569`.

RPC/Explorer host:
- SSH using the documented `synergyvps` route currently fails with `Permission denied (publickey,password)`.
- Public HTTPS RPC and Atlas API are reachable, so public-plane evidence is collected through those surfaces until host access is restored.

Atlas API:
- Chain ID: `1264`
- Latest indexed block: `12550`
- Average block time reported by Atlas: `3.0417s`
- Active validators: `5`
- Peer count: `7`
- `indexedAt`: `2026-05-20T19:24:52.652Z`
- Source RPC: `http://127.0.0.1:8640`
- Fallback RPC: `http://127.0.0.1:9640`

Atlas DAG:
- Enabled: `true`
- Data source: `atlas_indexed_committed_dag`
- Vertices: `0`
- Transactions: `0`
- Frontier entries: `0`
- Latest vertex: `null`
- This is not an end-to-end DAG proof. No fabricated DAG rows were inserted.

## Current Health Assessment

- Chain is advancing with four live validators, but Validator 3 is divergent and stuck.
- The target block cadence is 2 seconds; current public RPC samples are about 3 seconds over 50, 120, and 300 blocks.
- Public block timestamps are close to wall clock.
- Atlas is indexing current blocks and shows the same current chain identity.
- DAG ingestion remains unproven live because no real post-reset `aegis-pqvm` signed transactions have been submitted through a live typed transaction path and indexed.
- The non-genesis onboarding stale height source over 6000 has not been traced in this preflight pass and remains an open P0 investigation item.
- Fee collector deposits, validator rewards, archive node status, and non-genesis onboarding state were not fully verified in this preflight pass.

## Mutation Plan Required Before Any Live Change

Do not mutate live services until:
1. A synchronized node `v12.2.25` tag exists and trusted GitHub Actions artifacts are green with checksums recorded.
2. Control Panel `v12.2.25` builds green and bundles matching trusted runtime binaries.
3. A fresh preflight re-confirms Validator 3 status, relayer/RPC/Atlas state, and artifact checksums.
4. A Validator 3 evidence-preserving reconciliation plan is written, including exact files to back up, exact state to clear or roll back, and a rollback artifact.

Planned live mutation, once allowed:
- Install only trusted release artifacts, not local binaries.
- Preserve Validator 3 configs, keys, logs, canonical-lock conflict evidence, committed QCs, and backups.
- Reconcile Validator 3 from canonical peers or verified archive data; do not regenerate genesis and do not copy keys between validators.
- Re-measure 50/120/300 block cadence and all validator hashes after reconciliation.
