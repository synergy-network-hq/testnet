# Synergy Node Roles for Operators

## Overview

- This document explains the 19 specialized Synergy node roles in plain English.
- It separates:
- `Shared`: work that every specialized node app performs.
- `Shared subset`: work that only some groups of node roles share.
- `Role-specific`: work that belongs to one role or a narrow class of roles.
- Use this alongside `node-role-functions.md` when you need the implementation-level details.
- Bootnodes are not a 20th node type. A bootnode is any compatible node started in `bootstrap_only = true` mode.
- Seed servers are support services, not node binaries.

## Functions Shared by All 19 Specialized Node Apps

- `Shared`: load the node's configuration and identity data.
- `Shared`: confirm that the selected binary matches the configured role before startup.
- `Shared`: create and maintain the node's working directories, logs, and runtime files.
- `Shared`: support the same operator lifecycle commands such as `init`, `start`, `stop`, `restart`, `status`, `logs`, and `version`.
- `Shared`: write runtime status information so operators can confirm which role is running.
- `Shared`: restore local chain, wallet, and test-state data when that data is present.

## Functions Shared by Some Role Groups

- `Shared subset`: join the P2P network, discover peers, and maintain peer connectivity.
- `Shared subset`: sync chain data from other peers.
- `Shared subset`: expose RPC, WebSocket, or other service endpoints for clients or internal tools.
- `Shared subset`: participate in Proof of Synergy consensus.
- `Shared subset`: manage governance, treasury, or emergency response workflows.
- `Shared subset`: handle cryptographic signing, attestation, and key-management duties.
- `Shared subset`: fetch, verify, relay, or publish outside-network data.
- `Shared subset`: provide read-only indexing, explorer, and light-client services.

## Class I: Consensus Nodes

### Validator Node

- `Shared`: joins peers, syncs the chain, maintains logs, and enforces role validation.
- `Shared subset`: participates in consensus and can expose local RPC services when allowed.
- `Role-specific`: proposes blocks.
- `Role-specific`: validates transactions and state changes.
- `Role-specific`: votes on and commits blocks under Proof of Synergy.
- `Role-specific`: can self-register and self-stake when that path is enabled.

### Committee Node

- `Shared`: joins peers, syncs chain data, and runs the same operator lifecycle commands as other roles.
- `Shared subset`: participates in consensus alongside validators.
- `Shared subset`: shares validator-trust and verification responsibilities with Validator nodes.
- `Role-specific`: watches committee membership and epoch changes.
- `Role-specific`: coordinates validator rotation.
- `Role-specific`: helps maintain validator-cluster stability.

### Archive Validator Node

- `Shared`: joins peers, syncs chain data, and maintains role-bounded runtime files and logs.
- `Shared subset`: can expose read-focused RPC access.
- `Role-specific`: keeps the full historical chain state instead of only the current operating state.
- `Role-specific`: supports historical queries, snapshots, and proof generation from retained history.
- `Role-specific`: focuses on storage and retrieval of long-term chain history, not block-authority duties.

### Audit Validator Node

- `Shared`: joins peers, syncs the chain, validates its role on startup, and maintains audit logs.
- `Role-specific`: independently checks whether consensus outputs are correct.
- `Role-specific`: looks for block, state, or validator behavior that does not match expected results.
- `Role-specific`: raises audit and divergence alerts when the network appears inconsistent.
- `Role-specific`: supports independent review of network integrity.

## Class II: Interoperability Nodes

### Relayer Node

- `Shared`: joins peers, syncs chain data, and runs standard lifecycle operations.
- `Shared subset`: exposes service endpoints needed for interoperability traffic.
- `Role-specific`: relays cross-system messages and attestation events.
- `Role-specific`: packages and forwards interoperability proof material.
- `Role-specific`: maintains relayer liveness and relayer-set presence.

### Witness Node

- `Shared`: runs with the same role validation, logging, and operator controls as other roles.
- `Shared subset`: shares attestation-style duties with Relayer and Oracle roles.
- `Role-specific`: observes outside systems and collects proof material.
- `Role-specific`: submits signed witness observations and receipts.
- `Role-specific`: feeds observation telemetry into the broader interoperability pipeline.

### Oracle Node

- `Shared`: runs with the same role validation, logging, and operator controls as other roles.
- `Shared subset`: shares attestation-style duties with Witness and Relayer roles.
- `Role-specific`: pulls data from outside sources.
- `Role-specific`: checks and normalizes outside data before submission.
- `Role-specific`: produces oracle outputs that the network can trust and consume.

### UMA Coordinator Node

- `Shared`: runs with the same startup validation, logging, and operator controls as other roles.
- `Shared subset`: works within the same interoperability family as Oracle, Witness, Relayer, and Cross-Chain Verifier nodes.
- `Role-specific`: coordinates UMA identity and mapping flows.
- `Role-specific`: checks that identity-to-network mappings are correct.
- `Role-specific`: keeps audit trails for UMA coordination activity.

### Cross-Chain Verifier Node

- `Shared`: runs with the same startup validation, logging, and operator controls as other roles.
- `Shared subset`: shares verification-focused interoperability work with Relayer and UMA Coordinator nodes.
- `Role-specific`: verifies cross-chain proofs and receipts.
- `Role-specific`: checks finality before accepting outside-chain proof material.
- `Role-specific`: issues verification results after proof review.

## Class III: Execution, Data, and Cryptography Nodes

### SynQ Execution Node

- `Shared`: runs with the same role validation, logs, runtime reporting, and operator commands as other roles.
- `Shared subset`: belongs to the same execution-and-data family as Analytics and Simulation, Aegis Cryptography, and Data Availability nodes.
- `Role-specific`: executes SynQ workloads.
- `Role-specific`: produces execution traces and determinism checks.
- `Role-specific`: supports workload simulation before broader deployment.

### Analytics and Simulation Node

- `Shared`: runs with the same role validation, logs, runtime reporting, and operator commands as other roles.
- `Shared subset`: shares the execution-and-data family with SynQ Execution and Data Availability roles.
- `Role-specific`: runs analytics and simulation workloads.
- `Role-specific`: performs anomaly and risk analysis.
- `Role-specific`: produces reporting for operators and analysts.

### Aegis Cryptography Node

- `Shared`: runs with the same role validation, logs, runtime reporting, and operator commands as other roles.
- `Shared subset`: shares post-quantum and signing responsibilities with Committee, Audit Validator, Oracle, and governance/security roles.
- `Role-specific`: runs Aegis cryptographic verification functions.
- `Role-specific`: manages key lifecycle operations and signing support.
- `Role-specific`: records sensitive cryptographic events for audit review.

### Data Availability Node

- `Shared`: runs with the same role validation, logs, runtime reporting, and operator commands as other roles.
- `Shared subset`: shares the execution-and-data family with SynQ Execution and Analytics and Simulation roles.
- `Role-specific`: stores and serves data-availability payloads.
- `Role-specific`: maintains proof indexes for availability checks.
- `Role-specific`: supports audits that confirm network data can still be fetched when needed.

## Class IV: Governance and Security Nodes

### Governance Auditor Node

- `Shared`: runs with the same role validation, logs, runtime reporting, and operator commands as other roles.
- `Shared subset`: shares governance-manager duties with Treasury Controller and Security Council nodes.
- `Role-specific`: reviews governance proposals and governance decisions.
- `Role-specific`: checks vote integrity and governance process correctness.
- `Role-specific`: produces governance review outputs and signed audit records.

### Treasury Controller Node

- `Shared`: runs with the same role validation, logs, runtime reporting, and operator commands as other roles.
- `Shared subset`: shares governance-manager duties with Governance Auditor and Security Council nodes.
- `Role-specific`: controls treasury actions within treasury policy boundaries.
- `Role-specific`: coordinates disbursement and multi-party authorization flows.
- `Role-specific`: enforces extra review and control around treasury operations.

### Security Council Node

- `Shared`: runs with the same role validation, logs, runtime reporting, and operator commands as other roles.
- `Shared subset`: shares governance-manager duties with Governance Auditor and Treasury Controller nodes.
- `Role-specific`: handles emergency response and network containment actions.
- `Role-specific`: records incidents and emergency authorization activity.
- `Role-specific`: enforces stronger approval requirements for emergency actions.

## Class V: Service and Access Nodes

### RPC Gateway Node

- `Shared`: runs with the same role validation, logs, runtime reporting, and operator commands as other roles.
- `Shared subset`: joins the P2P network to sync chain state and shares RPC-style service exposure with other RPC-capable roles.
- `Role-specific`: provides public or internal gateway access to the network.
- `Role-specific`: applies rate limiting, access control, and request routing.
- `Role-specific`: exposes access services without inheriting validator or governance authority.

### Indexer and Explorer Node

- `Shared`: runs with the same role validation, logs, runtime reporting, and operator commands as other roles.
- `Shared subset`: shares the service-and-access family with RPC Gateway and Observer / Light nodes.
- `Role-specific`: ingests chain data into queryable indexes.
- `Role-specific`: powers explorer and search-style experiences.
- `Role-specific`: serves read-heavy data access for wallets, dashboards, and explorers.

### Observer / Light Node

- `Shared`: runs with the same role validation, logs, runtime reporting, and operator commands as other roles.
- `Shared subset`: shares the service-and-access family with RPC Gateway and Indexer and Explorer nodes.
- `Role-specific`: performs light sync instead of full-state operation.
- `Role-specific`: verifies headers and lightweight proofs.
- `Role-specific`: supports read-only wallet and observer use cases with minimal local state.

## Bootnodes and Seed Servers

### Bootnode

- `Shared`: uses the same networking stack and role-safe runtime foundation as other Synergy nodes.
- `Role-specific`: serves as a known entry point so new nodes can find the network.
- `Role-specific`: should run in `bootstrap_only = true` mode when it is intended to act only as a bootnode.
- `Role-specific`: should not be treated as a validator, relayer, or public service gateway unless intentionally configured to do more than bootstrapping.

### Seed Server

- `Role-specific`: publishes peer and bootstrap information for new nodes.
- `Role-specific`: can expose DNS and HTTP discovery information without being a blockchain node.
- `Role-specific`: supports remote operators who need a stable place to fetch current bootstrap targets.

## Shared-Function Cross Reference

- `Consensus`: Validator, Committee.
- `P2P bootstrap and sync`: Validator, Committee, Archive Validator, Audit Validator, Relayer, RPC Gateway, plus nodes intentionally started in bootstrap-only mode.
- `RPC and service access`: Validator, Archive Validator, Relayer, RPC Gateway, plus any role intentionally configured to expose compatible service endpoints.
- `Governance workflows`: Governance Auditor, Treasury Controller, Security Council.
- `Cryptographic and attestation-heavy workflows`: Committee, Audit Validator, Oracle, Aegis Cryptography, Governance Auditor, Treasury Controller, Security Council, plus interoperability roles that submit signed observations.
- `Explorer and read-only access`: RPC Gateway, Indexer and Explorer, Observer / Light.

## Current Operator Note

- The 19 node apps now have dedicated role-bound binaries.
- Some roles already start clear role-local services today.
- Some roles are still bounded correctly by binary and profile, but their deeper role-local subsystems still need more hardening before public production use.
- For public testing, operators should treat role binding as real, but they should still verify the current implementation status of the specific role they plan to run.
