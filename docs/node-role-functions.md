# Synergy Node Role Functions

## Overview

- This document lists the functions assigned to all 19 specialized Synergy node roles.
- It distinguishes between:
- `Shared`: functions present in every specialized node binary.
- `Shared subset`: functions shared by more than one role, but not by all 19 roles.
- `Role-specific`: functions assigned to a single role or to a narrow class of roles.
- `Current runtime note`: what the current role-bound runtime actually starts or enforces today.
- Source of truth for role surfaces: `src/role_profiles.rs`.
- Source of truth for current role-bound startup behavior: `src/role_runtime.rs`.
- Bootnodes are not a separate node role in the 19-role taxonomy. A bootnode is a node started with `bootstrap_only = true`.
- Seed servers are services, not node binaries.

## Shared Functions Across All 19 Specialized Binaries

- `Shared`: load configuration from `node.toml` and compatible template inputs.
- `Shared`: validate `identity.role` and `role.compiled_profile` before startup.
- `Shared`: refuse startup when the binary role and the config role do not match.
- `Shared`: initialize logging, log rotation, and console/file log output.
- `Shared`: create and manage data, chain, log, and PID-file paths.
- `Shared`: initialize wallet/testnet state and restore token state when present.
- `Shared`: expose the same lifecycle CLI commands: `init`, `start`, `stop`, `restart`, `status`, `logs`, `keygen`, `register`, `sync`, `list-templates`, `version`.
- `Shared`: emit `data/role-runtime.json` describing the active binary, role, and enabled surfaces.

## Shared Conditional Functions

- `Shared subset`: P2P bootstrap and peer discovery are shared by roles whose profile requires P2P, plus any node run in `bootstrap_only` mode.
- `Shared subset`: bootstrap sources are shared across P2P-capable roles: hardcoded bootnodes, `_dnsaddr.bootstrap` TXT records, and seed-service fallbacks.
- `Shared subset`: chain sync is shared by P2P-capable roles that are not running in `bootstrap_only` mode.
- `Shared subset`: local RPC/WS/GRPC startup is shared only by roles whose bounded profile exposes RPC-class surfaces.
- `Shared subset`: PoSy consensus startup is shared only by roles whose bounded profile includes `consensus`.
- `Shared subset`: PQC-managed local role services are shared by Committee, Audit Validator, Oracle, Aegis Cryptography, Governance Auditor, Treasury Controller, and Security Council nodes.
- `Shared subset`: governance-manager startup is shared by Governance Auditor, Treasury Controller, and Security Council nodes.

## Class I: Consensus Nodes

### Validator Node

- `Shared`: config/profile validation, logging, data-path setup, runtime reporting, and CLI lifecycle commands.
- `Shared subset`: P2P bootstrap, peer discovery, and chain sync.
- `Shared subset`: local RPC/WS startup.
- `Shared subset`: PoSy consensus engine startup.
- `Role-specific`: propose, vote, and commit blocks under Proof of Synergy.
- `Role-specific`: run mempool, state-transition, and validator execution duties.
- `Role-specific`: optionally self-register and self-stake when validator auto-registration is enabled.
- `Current runtime note`: the binary starts P2P, sync, RPC, and consensus when the config allows it.

### Committee Node

- `Shared`: config/profile validation, logging, data-path setup, runtime reporting, and CLI lifecycle commands.
- `Shared subset`: P2P bootstrap and chain sync.
- `Shared subset`: PoSy consensus engine startup.
- `Shared subset`: Aegis-verifier surface shared with Validator.
- `Role-specific`: committee synchronization and epoch-rotation listening.
- `Role-specific`: validator rotation and cluster coordination.
- `Current runtime note`: the binary instantiates `EntropyBeacon` and `ValidatorRotation` and performs validator rotation at startup.

### Archive Validator Node

- `Shared`: config/profile validation, logging, data-path setup, runtime reporting, and CLI lifecycle commands.
- `Shared subset`: P2P bootstrap and chain sync.
- `Shared subset`: read-oriented RPC surface.
- `Role-specific`: maintain full historical chain state.
- `Role-specific`: build proofs and snapshots from retained history.
- `Role-specific`: support archive and snapshot retention functions that are not validator-authority functions.
- `Current runtime note`: the binary is role-bounded and excludes consensus startup, but deeper archive-local managers are still represented as bounded service surfaces rather than separate local service objects.

### Audit Validator Node

- `Shared`: config/profile validation, logging, data-path setup, runtime reporting, and CLI lifecycle commands.
- `Shared subset`: P2P bootstrap and chain sync.
- `Role-specific`: independently verify consensus outputs and quorum-certificate integrity.
- `Role-specific`: run state-diff and divergence-detection functions.
- `Role-specific`: emit alerting and audit-oriented validation outputs.
- `Current runtime note`: the binary starts `CartelDetectionEngine` and `WhistleblowerSystem` as audit-local services.

## Class II: Interoperability Nodes

### Relayer Node

- `Shared`: config/profile validation, logging, data-path setup, runtime reporting, and CLI lifecycle commands.
- `Shared subset`: P2P bootstrap and chain sync.
- `Shared subset`: WS/API-class surface for SXCP relay traffic.
- `Role-specific`: relay SXCP attestations and interoperability events.
- `Role-specific`: package attestations and maintain witness-registry-facing liveness.
- `Role-specific`: maintain relayer heartbeat and relayer-set presence.
- `Current runtime note`: the binary registers itself with SXCP state and emits periodic relayer heartbeats in a background worker.

### Witness Node

- `Shared`: config/profile validation, logging, data-path setup, runtime reporting, and CLI lifecycle commands.
- `Role-specific`: observe external systems and capture proof material.
- `Role-specific`: submit witness attestations and signed receipts.
- `Role-specific`: provide telemetry for external observation pipelines.
- `Shared subset`: attestation-oriented behavior shared with Relayer and Oracle, but without relay authority.
- `Current runtime note`: the binary is role-bounded and excludes validator, governance, and service-plane startup; witness-local managers are still represented as bounded service surfaces.

### Oracle Node

- `Shared`: config/profile validation, logging, data-path setup, runtime reporting, and CLI lifecycle commands.
- `Role-specific`: fetch external data from oracle sources.
- `Role-specific`: authenticate sources and normalize quoted data before submission.
- `Role-specific`: submit attestation-ready oracle outputs.
- `Shared subset`: attestation behavior shared with Witness and Relayer.
- `Current runtime note`: the binary instantiates `SynergyOracle` as its oracle-local service.

### UMA Coordinator Node

- `Shared`: config/profile validation, logging, data-path setup, runtime reporting, and CLI lifecycle commands.
- `Role-specific`: orchestrate UMA identity refresh and mapping flows.
- `Role-specific`: verify identity-to-network mappings.
- `Role-specific`: maintain audit logging for UMA coordination.
- `Shared subset`: interoperability-plane posture shared with Oracle, Witness, Relayer, and Cross-Chain Verifier.
- `Current runtime note`: the binary is dedicated to the UMA profile and excludes other planes, but its coordinator-local service is currently represented as a bounded runtime surface placeholder.

### Cross-Chain Verifier Node

- `Shared`: config/profile validation, logging, data-path setup, runtime reporting, and CLI lifecycle commands.
- `Role-specific`: verify cross-chain proofs and receipts.
- `Role-specific`: check finality and bind verification scope to allowed domains.
- `Role-specific`: issue verification receipts after proof acceptance.
- `Shared subset`: interoperability-plane verification shared with Relayer and UMA Coordinator, but with a verifier-only scope.
- `Current runtime note`: the binary is dedicated to the cross-chain verifier profile and excludes other planes, but its verifier-local service is currently represented as a bounded runtime surface placeholder.

## Class III: Execution, Data, and Cryptography Nodes

### SynQ Execution Node

- `Shared`: config/profile validation, logging, data-path setup, runtime reporting, and CLI lifecycle commands.
- `Role-specific`: execute SynQ workloads inside the bounded SynQ execution plane.
- `Role-specific`: produce execution traces, determinism checks, and predeploy simulations.
- `Shared subset`: execution-data plane shared with Analytics/Simulation, Aegis Cryptography, and Data Availability.
- `Current runtime note`: the binary is dedicated to the SynQ execution profile and excludes other planes, but the execution-local service is currently represented as a bounded runtime surface placeholder.

### Analytics and Simulation Node

- `Shared`: config/profile validation, logging, data-path setup, runtime reporting, and CLI lifecycle commands.
- `Role-specific`: run simulation workloads and risk-model computation.
- `Role-specific`: perform anomaly detection and analytics reporting.
- `Shared subset`: execution-data plane shared with SynQ Execution and Data Availability, but without consensus or interoperability authority.
- `Current runtime note`: the binary is dedicated to the analytics/simulation profile and excludes other planes, but the analytics-local service is currently represented as a bounded runtime surface placeholder.

### Aegis Cryptography Node

- `Shared`: config/profile validation, logging, data-path setup, runtime reporting, and CLI lifecycle commands.
- `Shared subset`: PQC key-management posture shared with Committee, Audit Validator, Oracle, and governance/security nodes.
- `Role-specific`: run Aegis verification and KMS-bridge functions.
- `Role-specific`: manage key lifecycle and attestation-signing duties.
- `Role-specific`: provide audit-log output for cryptographic authority events.
- `Current runtime note`: the binary keeps a dedicated `PQCManager` alive and binds itself to the Aegis-only service surface; deeper Aegis service orchestration is still represented as a bounded runtime surface placeholder.

### Data-Availability Node

- `Shared`: config/profile validation, logging, data-path setup, runtime reporting, and CLI lifecycle commands.
- `Role-specific`: maintain data-availability storage and shard-serving functions.
- `Role-specific`: maintain proof indexing and availability-audit outputs.
- `Shared subset`: execution-data plane shared with SynQ Execution and Analytics/Simulation, but specialized for storage and proof availability.
- `Current runtime note`: the binary is dedicated to the data-availability profile and excludes other planes, but the DA-local service is currently represented as a bounded runtime surface placeholder.

## Class IV: Governance and Security Nodes

### Governance Auditor Node

- `Shared`: config/profile validation, logging, data-path setup, runtime reporting, and CLI lifecycle commands.
- `Shared subset`: governance-manager startup shared with Treasury Controller and Security Council.
- `Role-specific`: audit governance proposals, vote integrity, and review scope.
- `Role-specific`: produce reporting and signed governance-review artifacts.
- `Current runtime note`: the binary instantiates `DAOGovernance` as the bounded governance manager for this role.

### Treasury Controller Node

- `Shared`: config/profile validation, logging, data-path setup, runtime reporting, and CLI lifecycle commands.
- `Shared subset`: governance-manager startup shared with Governance Auditor and Security Council.
- `Role-specific`: execute treasury policy within bounded treasury scope.
- `Role-specific`: coordinate multisig/disbursement control and treasury audit functions.
- `Role-specific`: enforce dual-control posture for treasury actions.
- `Current runtime note`: the binary instantiates the shared `DAOGovernance` manager but remains role-bound to treasury-only service surfaces.

### Security Council Node

- `Shared`: config/profile validation, logging, data-path setup, runtime reporting, and CLI lifecycle commands.
- `Shared subset`: governance-manager startup shared with Governance Auditor and Treasury Controller.
- `Role-specific`: apply emergency-scope and containment policy.
- `Role-specific`: perform incident logging and bounded emergency authorization.
- `Role-specific`: enforce dual-authorization posture for emergency actions.
- `Current runtime note`: the binary instantiates the shared `DAOGovernance` manager but remains role-bound to security-council service surfaces.

## Class V: Service and Access Nodes

### RPC Gateway Node

- `Shared`: config/profile validation, logging, data-path setup, runtime reporting, and CLI lifecycle commands.
- `Shared subset`: P2P bootstrap, peer discovery, and chain sync.
- `Shared subset`: RPC/WS startup shared with other RPC-capable roles.
- `Role-specific`: expose gateway, upstream routing, and request-entry surfaces.
- `Role-specific`: apply rate limiting, authentication/authorization, and edge-cache behavior.
- `Role-specific`: provide public access without inheriting validator or governance authority.
- `Current runtime note`: the binary is role-bounded and starts P2P plus RPC-class startup for the gateway profile, while gateway-local management is still represented as a bounded runtime surface placeholder.

### Indexer and Explorer Node

- `Shared`: config/profile validation, logging, data-path setup, runtime reporting, and CLI lifecycle commands.
- `Role-specific`: ingest chain data for index construction.
- `Role-specific`: expose query API, search, and explorer-backend functions.
- `Shared subset`: service-access plane shared with RPC Gateway and Observer/Light, but specialized for indexing and query.
- `Current runtime note`: the binary is role-bounded and excludes consensus and governance planes, but indexer-local management is currently represented as a bounded runtime surface placeholder.

### Observer / Light Node

- `Shared`: config/profile validation, logging, data-path setup, runtime reporting, and CLI lifecycle commands.
- `Role-specific`: perform header sync and light-proof verification.
- `Role-specific`: provide wallet-feed and read-only observer outputs.
- `Shared subset`: service-access plane shared with RPC Gateway and Indexer/Explorer, but specialized for minimal-state verification.
- `Current runtime note`: the binary is role-bounded and excludes consensus, treasury, and interoperability planes, but observer-local management is currently represented as a bounded runtime surface placeholder.

## Shared-Function Cross Reference

- `Consensus startup`: Validator, Committee.
- `P2P bootstrap and sync`: Validator, Committee, Archive Validator, Audit Validator, Relayer, RPC Gateway, plus any binary started in `bootstrap_only` mode.
- `RPC/WS-class startup`: Validator, Archive Validator, Relayer, RPC Gateway.
- `PQC-managed local services`: Committee, Audit Validator, Oracle, Aegis Cryptography, Governance Auditor, Treasury Controller, Security Council.
- `Governance manager`: Governance Auditor, Treasury Controller, Security Council.
- `Telemetry / metrics posture`: all roles, though the exposed ports differ by profile.

## Current State Note

- The 19 binaries are now dedicated, role-bound executables.
- The role boundary is enforced at startup by profile validation and by excluding unrelated planes.
- Some roles already start dedicated local managers today.
- Some roles currently expose a bounded role surface and dedicated binary without yet having a separate long-lived local service manager behind every declared function.
- When this document says a function is role-specific, that means it belongs to that role's bounded surface even if the deeper subsystem is still being hardened behind that binary.
