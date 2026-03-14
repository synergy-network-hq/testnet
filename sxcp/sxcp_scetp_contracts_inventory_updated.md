# SXCP + Wallet (SCETP) On‑Chain Contract / Program Inventory (Updated)

> Note on terminology: the wallet spec uses **SCETP** (“Same‑Chain External Transfer Protocol”). You referenced “SECTP”; I’m treating that as SCETP.

This document lists **all on‑chain components** needed to fully implement SXCP in the Synergy devnet and integrate with external testnets (EVM + non‑EVM), **plus** the on‑chain components required by the Synergy Wallet architecture (Identity / Routing / Recovery / Policy / Audit).

---

## 1) Synergy Network (Synergy chain / devnet) — Required on‑chain components

### 1.1 Wallet foundation (required for SCETP + SXCP)

1. **IdentityRegistry (Synergy Identity + State Machine)**
   - Purpose: canonical registry for Synergy identities and their **Identity State Model** (Active / Restricted / Recovery‑Pending / Recovered / Decommissioned), plus identity‑governed authority boundaries.
   - Responsibilities:
     - Register / update Synergy Identity Key (SIK) verification material (classical, post‑quantum, or hybrid).
     - Enforce monotonic state transitions + quorum/time‑delay requirements.
     - Expose identity status to other modules (UMA, recovery, policy, SXCP).
   - Core data:
     - identityId (Synergy address), state, SIK public keys, key‑rotation counters, recovery config pointer, policy config pointer.

2. **SNSRegistry (Synergy Naming System)**
   - Purpose: map **SNS names** (e.g., `alice.syn`) → **Synergy addresses only** (not to contracts/endpoints).
   - Responsibilities:
     - Register/transfer/update names.
     - Emit versioned, auditable updates.

3. **UMARegistry (Universal Meta‑Address routing registry)**
   - Purpose: identity‑governed, versioned routing records: Synergy identity → chain‑specific address (by chain identifier).
   - Responsibilities:
     - Propose / approve / activate routing updates (time‑delayed and/or quorum‑gated).
     - Maintain version counters and revocation / validity windows.
     - Enforce “routing is non‑authoritative”: routing can suggest destinations but never sign/execute.
   - Core data:
     - (identityId, chainId) → {version, destinationAddress, validityWindow, status, activationTime, revocationFlag}

4. **RecoveryModule (Guardian + Recovery State Machine)**
   - Purpose: on‑chain enforcement of recovery triggers and recovery authorization (guardian quorum, multi‑sig, time‑delay).
   - Responsibilities:
     - Manage guardian sets / thresholds / delay windows.
     - Trigger Recovery‑Pending / Recovery‑Confirmed / Recovered transitions.
     - During recovery, freeze or restrict UMA updates and require elevated approvals.
   - Note: can be its own contract or tightly integrated into IdentityRegistry.

5. **PolicyConfigRegistry (identity‑governed policy configuration; local enforcement)**
   - Purpose: store canonical, versioned policy configuration under identity control so multiple devices can converge on the same policy state.
   - Responsibilities:
     - Accept policy updates under identity governance controls (guardian quorum, delayed activation).
     - Provide “effective policy version” view for clients.
   - Important: policy enforcement is still pre‑signature **on device**; this contract is for governance/state distribution.

6. **AuditReceiptRegistry (SCETP receipts)**
   - Purpose: optional Synergy‑chain “receipt” event for same‑chain external transfers.
   - Responsibilities:
     - Record events containing: sender identity, recipient identity, chain identifier, external tx hash.
   - Note: implement as a lightweight contract that *only* emits receipts + provides indexing hooks.

### 1.2 SXCP protocol layer (required for cross‑chain coordination)

These contracts are described as **core** SXCP contracts and supporting modules.

1. **SXCPVault**
   - Purpose: lock/release logic and transfer lifecycle coordination hooks (including timeout reversion paths).
   - Scope: on Synergy chain for Synergy‑native assets, and mirrored on external chains for their assets.

2. **WitnessRegistry**
   - Purpose: manage relayer membership, status, and legitimacy checks; reference point for verifier logic.
   - Scope: deployed on Synergy chain and mirrored on external chains.

3. **SignatureVerifier**
   - Purpose: verify aggregated / threshold attestations (post‑quantum capable on Synergy chain; chain‑specific verifiers on other chains).

4. **GovernanceModule**

- Purpose: parameter management (thresholds, supported chains, slashing params), emergency controls, upgrade governance.

1. **AuditLogger**

- Purpose: immutable log of SXCP transfer lifecycle events, attestations, and outcomes.
- Note: can be integrated with SCETP AuditReceiptRegistry or kept separate (recommended to keep distinct event types).

#### SXCP supporting modules (may be separate or library‑linked, but still “on‑chain components”)

1. **StateProofValidator**
2. **FinalityChecker**
3. **CompensationPool**
4. **RelayerRegistry** (if not merged into WitnessRegistry)

---

## 2) External chains — what must be deployed on each network

### 2.1 EVM chains (e.g., Ethereum Sepolia, Base Sepolia, Polygon Amoy)

Deploy the **full SXCP EVM contract set**:

- `SXCPVault` (EVM)
- `WitnessRegistry` (EVM)
- `SignatureVerifier` (EVM)
- `StateProofValidator` (EVM)
- `FinalityChecker` (EVM)
- `AuditLogger` (EVM)
- `GovernanceModule` (EVM)
- `CompensationPool` (EVM)
- `RelayerRegistry` (EVM; optional if merged)

SCETP requirement on EVM chains: **no contracts required** (SCETP uses identity routing + native EVM execution).

### 2.2 Solana (non‑EVM)

Solana uses **programs** rather than EVM contracts. Deploy program equivalents:

- `sxcp_vault_program`
- `witness_registry_program`
- `signature_verifier_program` (or verifier logic embedded in vault program)
- `state_proof_validator_program` (only if proofs must be validated on‑chain)
- `finality_checker_program` (often minimized; finality can be treated as an off‑chain relayer responsibility)
- `audit_logger_program`
- `governance_program`
- `comp_pool_program`

SCETP requirement on Solana: **no additional on‑chain programs required** beyond standard SPL token/native transfer.

### 2.3 Cosmos SDK chains (non‑EVM)

Two implementation paths:

A) CosmWasm contracts

- `sxcp_vault_wasm`
- `witness_registry_wasm`
- `signature_verifier_wasm`
- `audit_logger_wasm`
- `governance_wasm`
- (optional) `state_proof_validator_wasm`, `finality_checker_wasm`, `comp_pool_wasm`

B) Native Cosmos SDK module

- `x/sxcp` module implementing vault + registry + verifier + audit + governance.

SCETP requirement: none on‑chain (native send), unless you want optional receipt logging in a Cosmos module (not required by wallet spec).

### 2.4 Substrate / Polkadot parachains (non‑EVM)

Implement as **pallets**:

- `pallet-sxcp-vault`
- `pallet-sxcp-witness-registry`
- `pallet-sxcp-signature-verifier`
- `pallet-sxcp-audit`
- `pallet-sxcp-governance`
- (optional) `pallet-sxcp-proof-validator`, `pallet-sxcp-finality`, `pallet-sxcp-comp-pool`

### 2.5 UTXO chains (Bitcoin testnet/signet)

No deployable smart contracts in the EVM sense; required on‑chain artifacts are **standardized script/transaction templates** for SXCP’s swap-style coordination:

- **HTLC‑style lock script template** (hash‑lock + time‑lock; claim path + refund path)
- **Refund transaction template** (pre‑signed or safely pre‑constructed under policy)
- **Claim transaction template** (requires preimage and satisfies script constraints)
- (optional) **Commitment marker template** (e.g., OP_RETURN metadata) if you want explicit on‑chain intent markers for indexing

SCETP requirement: none (standard Bitcoin transactions).

---

## 3) Practical “minimum viable” deployment set for initial devnet testing

If you are starting with Synergy devnet + Ethereum Sepolia + Solana devnet, the minimum is:

- Synergy chain: Wallet foundation contracts (Identity/SNS/UMA/Recovery/Policy/AuditReceipt) + SXCP core set.
- Sepolia: SXCP core contracts + verifier/finality modules required by your chosen proof model.
- Solana: SXCP vault + witness registry + audit + governance (verifier logic embedded as needed).
