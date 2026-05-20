# Synergy Testnet 1264 Preflight for v12.2.25 Rollout

Date: 2026-05-20

This is a read-only preflight captured before any live v12.2.25 mutation. No validator restart, chain reset, installer deployment, database wipe, firewall change, or service topology change was performed while collecting this evidence.

## Trusted Artifact State

- Node repository: `synergy-network-hq/testnet`
- Node tag: `v12.2.25`
- Node commit: `2f7b832`
- Node GitHub Actions run: `26185757460`
- Node workflow status: succeeded for Linux, macOS, Windows, and unified `latest.json`.
- Node release URL: `https://github.com/synergy-network-hq/testnet/releases/tag/v12.2.25`
- Node Linux `synergy-testnet` checksum: `bc74a3ae1a480c5dae351ebed2707c5c34b3bf9046f0b60ea68900bd2caf467a`
- Node macOS `synergy-testnet` checksum: `a9032478dacce5bd89ef4d72b3ceb063074820e53154df231581667decce6824`
- Node Windows `synergy-testnet` checksum: `04fd8891cee7d81b42f55bee7fdea3ec3449d96193adc7d80c7918021399218d`
- Node Linux RPC gateway checksum: `2aae13444d6046ddc0aefa9b6adb69c2459c03e2968e78073e974168d2a6cb1f`
- Node Linux indexer/explorer checksum: `4374238229bc5ec358ec853f75b72e932a7eba8e55e2360bac420c65a9a4e3ab`

- Control Panel repository: `synergy-network-hq/synergy-node-control-panel`
- Control Panel tag: `v12.2.25`
- Control Panel commit: `342deb9`
- Control Panel GitHub Actions run: `26186714962`
- Control Panel workflow status: succeeded for Linux, macOS, and Windows installer builds.
- Control Panel release URL: `https://github.com/synergy-network-hq/synergy-node-control-panel-releases/releases/tag/v12.2.25`
- Control Panel Linux `.deb` checksum: `03e49542343ff82f77307a8262465f14cb83010613b7c6b946c4b565469cd7b8`
- Control Panel Linux AppImage checksum: `1f0e225a391ded4cb6f79295468e992cc3e5f23cb5eb2089aded271fcbee018f`
- Control Panel macOS DMG checksum: `7724995bbe4b7ec3a2bf7790b9835cd52dc4e3c9aebf2c838a6836672224ba66`
- Control Panel macOS ZIP checksum: `4ceca096800fc790c6ad855f434d45672c2c6ef805674fc304e80d056e1957c6`
- Control Panel Windows installer checksum: `8690e0534835e457ddad42a10abb90137da29c6b5a4a1d096db020f07a688350`

## Validator State

| Validator | Height | Hash | Parent hash | Block timestamp | Host time | Delta | Genesis hash | Runtime checksum | Installed Control Panel |
| --- | ---: | --- | --- | ---: | ---: | ---: | --- | --- | --- |
| Validator 1 | 13556 | `50198d0e098c31cc716b1bed68c71264a4d19eb7dc48c28855d7a60143a13618` | `ec2efd4650afb7d330bb591091ae0a4e6c28b1419527b549e072424abcd7311d` | 1779308142 | 1779308144 | 2s | `f79011f2aaddd40b120d47ba723104fafe3c998d4a17097fae018914b95f1789` | `1325ef8f36d51ec6d01a166b22710c3fba170af3e3d691aa990bf4d788f289fc` | `12.2.21` |
| Validator 2 | 13556 | `50198d0e098c31cc716b1bed68c71264a4d19eb7dc48c28855d7a60143a13618` | `ec2efd4650afb7d330bb591091ae0a4e6c28b1419527b549e072424abcd7311d` | 1779308142 | 1779308144 | 2s | `f79011f2aaddd40b120d47ba723104fafe3c998d4a17097fae018914b95f1789` | `1325ef8f36d51ec6d01a166b22710c3fba170af3e3d691aa990bf4d788f289fc` | `12.2.21` |
| Validator 3 | 11668 | `5b3bed3ac4377db5451fdc68366f31b14103fb867a4d8b756a29472435f444a2` | `3238715aed3dcadee39a2279dc3e524d9d6f28a080557bbf45f51457b616036b` | 1779302405 | 1779308144 | 5739s | `f79011f2aaddd40b120d47ba723104fafe3c998d4a17097fae018914b95f1789` | `1325ef8f36d51ec6d01a166b22710c3fba170af3e3d691aa990bf4d788f289fc` | `12.2.21` |
| Validator 4 | 13555 | `ec2efd4650afb7d330bb591091ae0a4e6c28b1419527b549e072424abcd7311d` | `f4fea981ee4f71ad90bf7ca3182cb7f2aad835e42e2d9811b3e8279a1ae0f565` | 1779308140 | 1779308143 | 3s | `f79011f2aaddd40b120d47ba723104fafe3c998d4a17097fae018914b95f1789` | `1325ef8f36d51ec6d01a166b22710c3fba170af3e3d691aa990bf4d788f289fc` | `12.2.21` |
| Validator 5 | 13556 | `50198d0e098c31cc716b1bed68c71264a4d19eb7dc48c28855d7a60143a13618` | `ec2efd4650afb7d330bb591091ae0a4e6c28b1419527b549e072424abcd7311d` | 1779308142 | 1779308145 | 3s | `f79011f2aaddd40b120d47ba723104fafe3c998d4a17097fae018914b95f1789` | `1325ef8f36d51ec6d01a166b22710c3fba170af3e3d691aa990bf4d788f289fc` | `12.2.21` |

Validator state findings:
- Validators 1, 2, 4, and 5 are advancing and parent-aligned in the sample.
- Validator 3 remains divergent and stuck at height `11668`.
- Validator 3 previously logged canonical lock conflict evidence at height `11668`; that evidence must be preserved before repair.
- All sampled validator genesis hashes match canonical Testnet chain 1264 genesis.
- All sampled live validator binaries are still the older runtime checksum `1325ef8f36d51ec6d01a166b22710c3fba170af3e3d691aa990bf4d788f289fc`, not the trusted v12.2.25 runtime checksum.
- Installed Control Panel package version on all sampled validators is `12.2.21`, not `12.2.25`.
- `committed_qcs.jsonl` and `canonical_locks.json` exist on validators; examples are about `1.1G` and `7.5M` respectively on advancing validators.

## Relayers and Public Plane

| Node | Height | Hash | Timestamp | Runtime checksum | Notes |
| --- | ---: | --- | ---: | --- | --- |
| Relayer 1 | 13573 | `3ee500be508c239c5cb1731c6956c2657533f67d956f4b7f3b07785f54f01284` | 1779308195 | `f4d155867e179510c0fab90d33b6d74b64b650a7d2f978a734e80f7c77a25d7c` | `chainId` 1264, `networkId` 1264, not syncing |
| Relayer 2 | 13573 | `3ee500be508c239c5cb1731c6956c2657533f67d956f4b7f3b07785f54f01284` | 1779308195 | `f4d155867e179510c0fab90d33b6d74b64b650a7d2f978a734e80f7c77a25d7c` | `chainId` 1264, `networkId` 1264, not syncing |
| Public RPC | 13579 | `be96a3c7827f29a72c7a74cb2ab5c7ad1dfeff1c4622cef254b1f01cc8e246cd` | 1779308212 | not sampled over SSH | `chainId` 1264, `networkId` 1264, timestamp delta 7s |
| Atlas API | 13584 | not returned by summary | not returned by summary | not sampled over SSH | `chainId` 1264, indexed at `2026-05-20T20:17:10.944Z` |

Atlas DAG:
- DAG enabled: `true`
- data source: `atlas_indexed_committed_dag`
- vertex count: `0`
- transaction count: `0`
- frontier count: `0`
- latest vertex: `null`

Public RPC block interval sample:
- Latest 50 blocks: average `3.0816s`, min `2s`, max `6s`
- Latest 120 blocks: average `3.0420s`, min `2s`, max `6s`
- Latest 300 blocks: average `3.0234s`, min `2s`, max `7s`

## Required Mutation Plan

No live mutation may proceed outside this plan.

1. Preserve Validator 3 evidence first:
   - copy Validator 3 configs, logs, `chain.json`, `committed_qcs.jsonl`, `canonical_locks.json`, backups, and current runtime/process evidence into a timestamped backup on Validator 3.
   - do not delete or overwrite keys, identities, configs, logs, WireGuard state, TLS material, or evidence.
2. Install trusted `v12.2.25` Control Panel package on Validator 3 and refresh Validator 3 runtime from the packaged release artifact only.
3. Stop only Validator 3 validator process, preserve the divergent data backup, clear only Validator 3 chain-following state required to resync from canonical chain 1264, and restart from canonical genesis/config/keys.
4. Verify Validator 3 catches up and agrees by height/hash with the other validators. If it does not recover, stop and report; do not roll the remaining validators.
5. Once all five validators are healthy, roll the package/runtime update across Validators 1, 2, 4, and 5 one at a time, preserving configs/keys/logs/evidence and checking quorum/cadence after each step.
6. Update relayers after validators are stable, then RPC/Explorer/Atlas support nodes.
7. Re-measure validator alignment, public RPC, Atlas indexing, DAG status, and 50/120/300 block cadence after every role group.

Known blockers before claiming stable:
- Validator 3 is currently divergent.
- Live runtime/package drift remains.
- Cadence is about 3s, not 2s.
- Atlas DAG remains empty because no real post-reset `aegis-pqvm` signed DAG transactions have been live-submitted and indexed.
- Non-genesis onboarding, archive fast sync, rewards, fees, wallet/dApp connection, and full end-to-end PQC coverage remain incomplete or unproven.
