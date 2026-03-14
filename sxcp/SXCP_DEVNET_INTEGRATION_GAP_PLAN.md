# SXCP Devnet Integration Gap Plan (Sepolia + Amoy + Synergy Devnet)

## Scope

This document defines:

1. What is already implemented in this repo for SXCP phase-1.
2. What is still required to make SXCP actually operate with your Synergy devnet.
3. The exact order to execute remaining work.

Primary reference implementation added in this pass:

- `/Users/devpup/Desktop/synergy-devnet/sxcp/sxcp_external_chains/evm`

## What Is Already Set Up

### A. Deployable EVM SXCP Suite Exists

Implemented contracts and deploy tooling for two EVM testnets:

- `GovernanceModule`
- `WitnessRegistry` (epoch + threshold aware)
- `SignatureVerifier` (phase-1 ECDSA threshold bundle verification)
- `FinalityChecker` (per-chain finality settings)
- `StateProofValidator` (root validation gate)
- `AuditLogger`
- `SXCPIntentHub` (V2-style intent + attestation, no pooled custody)

### B. Two-Testnet Tooling Exists

- Hardhat project configured for Sepolia + Amoy.
- Deploy scripts, endpoint wiring scripts, and demo flow scripts.
- Unit tests for attestation verification and replay-safe consumption.
- Runbook:
  - `/Users/devpup/Desktop/synergy-devnet/sxcp/sxcp_external_chains/evm/TESTNET_RUNBOOK.md`

### C. Devnet Config Artifact Generator Exists

Script added:

- `scripts/export-devnet-relayer-config.js`

This emits:

- `runtime/devnet-sxcp-relayer-config.json`

for relayer nodes to consume as machine configuration input.

## What Is Still Required (Critical Path)

## P0 - Required Before “SXCP Works With Devnet” Can Be Claimed

1. Deploy to real Sepolia and Amoy with funded deployer.
2. Wire both directions (Sepolia -> Amoy and Amoy -> Sepolia).
3. Register the exact relayer addresses used for attestation signing into:
   - EVM `WitnessRegistry` on both chains, and
   - Synergy devnet via `synergy_registerRelayer`.
4. Stand up a long-running relayer daemon on interop nodes (machine-06/07/08/09 equivalent roles) that does:
   - source-chain event subscription (`IntentCommitted`),
   - source finality waiting,
   - proof-root construction,
   - threshold signing coordination,
   - destination submission (`verifyAttestationBundle`),
   - devnet audit/report submission (`synergy_submitAttestation` + heartbeat).
5. Persist relayer state in durable storage (SQLite/Postgres), including:
   - processed event checkpoint,
   - bundle status,
   - retry backoff state,
   - replay cache.
6. Add production key management path:
   - private keys must not live in plaintext env files on nodes.
   - use encrypted secret storage or HSM/KMS-backed signing worker.

## P1 - Required for Protocol Correctness/Security Alignment

1. Replace phase-1 ECDSA verifier path with ML-DSA aggregate verification integration (Aegis/PQC path).
2. Replace synthetic `stateProofRoot` generation with canonical chain receipt/log proof pipeline.
3. Harden `StateProofValidator` to verify actual inclusion proofs, not allow-list root toggles.
4. Add reorg handling policy:
   - min finality depth per chain,
   - revalidation on detected reorg,
   - attest invalidation handling.
5. Enforce signer liveness and penalties end-to-end:
   - heartbeat timeout,
   - missed-attestation tracking,
   - slashing hooks and appeals.

## P2 - Required for Operations at Scale

1. Observability:
   - metrics for queue depth, time-to-finality, verify failures, replay rejects.
   - dashboards and alerts (Prometheus/Grafana).
2. Control panel integration:
   - show per-chain deployment addresses,
   - relayer quorum status,
   - attestation pipeline health,
   - failed proof diagnostics.
3. CI/CD:
   - contract test + lint + deploy simulation jobs,
   - signed release process for relayer daemon binaries.
4. Chaos tests:
   - relayer outage,
   - lagging signer,
   - malformed bundle submissions,
   - duplicate/replay stress.

## Devnet Integration Work Breakdown

### Step 1 - Chain Deployment and Wiring

Run from:

- `/Users/devpup/Desktop/synergy-devnet/sxcp/sxcp_external_chains/evm`

Commands:

```bash
npm install
npm run deploy:sepolia
npm run deploy:amoy
npm run wire:sepolia-to-amoy
npm run wire:amoy-to-sepolia
npm run export:devnet-config
```

Output required:

- `deployments/sepolia.json`
- `deployments/amoy.json`
- `runtime/devnet-sxcp-relayer-config.json`

### Step 2 - Relayer Runtime (Missing Code)

You still need a daemon/service process (not just one-shot scripts) with:

1. Source watchers (websocket + polling fallback).
2. Quorum coordinator (collect signatures, threshold gate).
3. Destination submitter (gas policy + retry policy).
4. Devnet reporter (RPC methods in `rpc-methods.md` SXCP section).

Recommended placement:

- `scripts/devnet15/` for bootstrap wrappers
- new service under `src/` or `services/sxcp-relayer/`

### Step 3 - Devnet Node Assignment

For your topology, keep SXCP live components on interop roles:

- relayer engine: machine-06
- verifier/coordinator: machine-07
- oracle/proof builder: machine-08
- witness/backup signer: machine-09

If running your 13-machine profile, map equivalent roles directly; do not tie this to old machine IDs only.

### Step 4 - End-to-End Acceptance Test

Definition of done for initial devnet readiness:

1. Publish intent on Sepolia.
2. Attestation reaches quorum on interop relayers.
3. Bundle verified on Amoy.
4. Attestation appears in Synergy devnet SXCP RPC (`synergy_getAttestations`).
5. Replay submission is rejected.
6. One relayer is slashed and rotated back without halting quorum processing.

## Current Known Limitations

1. EVM verifier is phase-1 ECDSA, not final PQC aggregate verification.
2. Proof validation is simplified and must be replaced with canonical proof checks.
3. Non-EVM chain implementations in `sxcp_external_chains/{solana,cosmos,substrate}` remain scaffold-level and are not production-ready.
4. There is no daemonized relayer coordinator in this pass; only deploy/demo scripts are provided.

## Recommended Immediate Next Build

Build a single `sxcp-relayer` service that consumes:

- `runtime/devnet-sxcp-relayer-config.json`
- `hosts.env` / inventory mapping

and performs:

- watch -> finalize -> sign -> submit -> report loop,
- with persisted checkpoints and alerting hooks.

Once that daemon exists, integrate it into your machine installers/systemd units for the interop nodes.
