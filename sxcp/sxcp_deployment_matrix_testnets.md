# SXCP + SCETP Deployment Matrix (Synergy Devnet + Sepolia + Polygon Testnet + Solana Devnet + Bitcoin Testnet)

This document converts the **SXCP + Wallet (SCETP) on‑chain inventory** into an actionable **deployment matrix** covering:

- **Synergy devnet** (Synergy chain)
- **Ethereum Sepolia** (EVM)
- **Polygon Amoy** (EVM; Polygon testnet)
- **Solana devnet** (non‑EVM)
- **Bitcoin testnet** (UTXO)

It specifies:

1) **What gets deployed on each network**  
2) **What must be deployed first** (dependency order)  
3) **Which addresses/IDs must be configured into which components**  
4) The **minimum event + API surface** the relayer network and Synergy devnet nodes must implement

> Terminology: this doc avoids the banned term you called out. SXCP is treated as a **custody‑free, direct‑verification interoperability protocol** (coordination + attestations + proofs), not a custodial system.

---

## 0) Notation and identifiers

### 0.1 Placeholder naming

- `SY_*` = Synergy devnet deployment outputs  
- `SEP_*` = Ethereum Sepolia deployment outputs  
- `AMOY_*` = Polygon Amoy deployment outputs  
- `SOL_*` = Solana devnet program IDs / PDA accounts  
- `BTC_*` = Bitcoin testnet script templates / policy IDs (not contract addresses)

### 0.2 Chain identifiers (for config tables)

- **Ethereum Sepolia chainId**: `11155111`  
- **Polygon Amoy chainId**: `80002`  
- **Solana cluster**: `devnet` (RPC typically `https://api.devnet.solana.com`)  
- **Bitcoin network**: `testnet`

> For EVM chain IDs and Solana RPC, confirm against your chosen RPC provider(s) at deployment time.

---

## 1) Contract / Program suites (by chain family)

### 1.1 SXCP core suite (EVM / Synergy)

Core:

- `SXCPVault`
- `WitnessRegistry`
- `SignatureVerifier`
- `GovernanceModule`
- `AuditLogger`

Supporting:

- `StateProofValidator`
- `FinalityChecker`
- `CompensationPool`
- `RelayerRegistry` (optional; can be merged with `WitnessRegistry`)

### 1.2 Wallet (SCETP) foundation (Synergy chain only)

- `IdentityRegistry`
- `SNSRegistry`
- `UMARegistry`
- `RecoveryModule`
- `PolicyConfigRegistry`
- `AuditReceiptRegistry` (optional but strongly recommended for SCETP receipts)

### 1.3 SXCP equivalents for Solana

Programs:

- `sxcp_vault_program`
- `witness_registry_program`
- `audit_logger_program`
- `governance_program`
- `comp_pool_program`
- `signature_verifier_program` (or embed verifier logic in vault program)
- Optional: `state_proof_validator_program`, `finality_checker_program`

### 1.4 SXCP equivalents for Bitcoin testnet

On‑chain artifacts are standardized **script + transaction templates** (swap‑style coordination):

- `BTC_HTLC_LOCK_SCRIPT_TEMPLATE`
- `BTC_CLAIM_TX_TEMPLATE`
- `BTC_REFUND_TX_TEMPLATE`
- Optional: `BTC_INTENT_MARKER_TEMPLATE` (e.g., OP_RETURN metadata)

---

## 2) High‑level matrix (what lives where)

| Component | Synergy devnet | Ethereum Sepolia | Polygon Amoy | Solana devnet | Bitcoin testnet |
|---|---:|---:|---:|---:|---:|
| IdentityRegistry / SNS / UMA / Recovery / Policy / AuditReceipt | ✅ | ❌ | ❌ | ❌ | ❌ |
| SXCPVault | ✅ | ✅ | ✅ | ✅ (program) | ❌ |
| WitnessRegistry | ✅ | ✅ | ✅ | ✅ (program) | ❌ |
| SignatureVerifier | ✅ | ✅ | ✅ | ✅ (program or embedded) | ❌ |
| GovernanceModule | ✅ | ✅ | ✅ | ✅ (program) | ❌ |
| AuditLogger | ✅ | ✅ | ✅ | ✅ (program) | ❌ |
| StateProofValidator | ✅ (optional phase‑gated) | ✅ (optional phase‑gated) | ✅ (optional phase‑gated) | optional | ❌ |
| FinalityChecker | ✅ (optional phase‑gated) | ✅ (optional phase‑gated) | ✅ (optional phase‑gated) | optional | ❌ |
| CompensationPool | ✅ | ✅ | ✅ | ✅ (program) | ❌ |
| RelayerRegistry | optional | optional | optional | optional | ❌ |
| BTC script/tx templates | ❌ | ❌ | ❌ | ❌ | ✅ |

---

## 3) Deployment order + dependency matrix (per network)

### 3.1 Synergy devnet deployment order (Synergy chain)

> Goal: deploy wallet foundation first (Identity/UMA/etc), then deploy SXCP suite.

| Step | Deploy / Initialize | Required? | Depends on | Output ID |
|---:|---|---:|---|---|
| SY‑1 | `GovernanceModule` (Synergy) | ✅ | — | `SY_GOV` |
| SY‑2 | `IdentityRegistry` | ✅ | — (optionally `SY_GOV` for global params) | `SY_ID` |
| SY‑3 | `RecoveryModule` (if separate) | ✅ | `SY_ID` | `SY_RECOVERY` |
| SY‑4 | `PolicyConfigRegistry` | ✅ | `SY_ID` | `SY_POLICY` |
| SY‑5 | `UMARegistry` | ✅ | `SY_ID`, `SY_RECOVERY`, `SY_POLICY` | `SY_UMA` |
| SY‑6 | `SNSRegistry` | ✅ | `SY_ID` (identity ownership checks) | `SY_SNS` |
| SY‑7 | `AuditReceiptRegistry` (SCETP receipts) | optional | `SY_ID` (optional) | `SY_SCETP_AUDIT` |
| SY‑8 | `CompensationPool` | ✅ | `SY_GOV` | `SY_COMP` |
| SY‑9 | `WitnessRegistry` | ✅ | `SY_GOV` (admin), `SY_COMP` (slashing funding path) | `SY_WITNESS` |
| SY‑10 | `RelayerRegistry` (if separate) | optional | `SY_GOV`, `SY_WITNESS` | `SY_RELAYER_REG` |
| SY‑11 | `SignatureVerifier` | ✅ | `SY_WITNESS` | `SY_SIGVERIFY` |
| SY‑12 | `StateProofValidator` | optional (phase‑gated) | `SY_GOV` | `SY_PROOFVAL` |
| SY‑13 | `FinalityChecker` | optional (phase‑gated) | `SY_GOV` | `SY_FINALITY` |
| SY‑14 | `AuditLogger` | ✅ | `SY_GOV` | `SY_AUDIT` |
| SY‑15 | `SXCPVault` | ✅ | `SY_WITNESS`, `SY_SIGVERIFY`, `SY_AUDIT`, `SY_GOV`, `SY_COMP`, (`SY_PROOFVAL`/`SY_FINALITY` if used) | `SY_VAULT` |
| SY‑16 | Post‑deploy config (section 4) | ✅ | all above | — |

### 3.2 Ethereum Sepolia deployment order (EVM)

| Step | Deploy / Initialize | Required? | Depends on | Output address |
|---:|---|---:|---|---|
| SEP‑1 | `GovernanceModule` | ✅ | — | `SEP_GOV` |
| SEP‑2 | `CompensationPool` | ✅ | `SEP_GOV` | `SEP_COMP` |
| SEP‑3 | `WitnessRegistry` | ✅ | `SEP_GOV`, `SEP_COMP` | `SEP_WITNESS` |
| SEP‑4 | `RelayerRegistry` (if separate) | optional | `SEP_GOV`, `SEP_WITNESS` | `SEP_RELAYER_REG` |
| SEP‑5 | `SignatureVerifier` | ✅ | `SEP_WITNESS` | `SEP_SIGVERIFY` |
| SEP‑6 | `StateProofValidator` | optional (phase‑gated) | `SEP_GOV` | `SEP_PROOFVAL` |
| SEP‑7 | `FinalityChecker` | optional (phase‑gated) | `SEP_GOV` | `SEP_FINALITY` |
| SEP‑8 | `AuditLogger` | ✅ | `SEP_GOV` | `SEP_AUDIT` |
| SEP‑9 | `SXCPVault` | ✅ | `SEP_WITNESS`, `SEP_SIGVERIFY`, `SEP_AUDIT`, `SEP_GOV`, `SEP_COMP`, (`SEP_PROOFVAL`/`SEP_FINALITY` if used) | `SEP_VAULT` |
| SEP‑10 | Post‑deploy config (section 4) | ✅ | all above | — |

### 3.3 Polygon Amoy deployment order (EVM)

> Same contract suite and order as Sepolia; only config differs.

| Step | Deploy / Initialize | Required? | Depends on | Output address |
|---:|---|---:|---|---|
| AMOY‑1 | `GovernanceModule` | ✅ | — | `AMOY_GOV` |
| AMOY‑2 | `CompensationPool` | ✅ | `AMOY_GOV` | `AMOY_COMP` |
| AMOY‑3 | `WitnessRegistry` | ✅ | `AMOY_GOV`, `AMOY_COMP` | `AMOY_WITNESS` |
| AMOY‑4 | `RelayerRegistry` (if separate) | optional | `AMOY_GOV`, `AMOY_WITNESS` | `AMOY_RELAYER_REG` |
| AMOY‑5 | `SignatureVerifier` | ✅ | `AMOY_WITNESS` | `AMOY_SIGVERIFY` |
| AMOY‑6 | `StateProofValidator` | optional (phase‑gated) | `AMOY_GOV` | `AMOY_PROOFVAL` |
| AMOY‑7 | `FinalityChecker` | optional (phase‑gated) | `AMOY_GOV` | `AMOY_FINALITY` |
| AMOY‑8 | `AuditLogger` | ✅ | `AMOY_GOV` | `AMOY_AUDIT` |
| AMOY‑9 | `SXCPVault` | ✅ | `AMOY_WITNESS`, `AMOY_SIGVERIFY`, `AMOY_AUDIT`, `AMOY_GOV`, `AMOY_COMP`, (`AMOY_PROOFVAL`/`AMOY_FINALITY` if used) | `AMOY_VAULT` |
| AMOY‑10 | Post‑deploy config (section 4) | ✅ | all above | — |

### 3.4 Solana devnet deployment order (non‑EVM)

> Solana uses programs and program‑derived accounts (PDAs). This matrix assumes you standardize **one PDA per registry**, plus one PDA per lock/transfer record.

| Step | Deploy / Initialize | Required? | Depends on | Output |
|---:|---|---:|---|---|
| SOL‑1 | `governance_program` | ✅ | — | `SOL_GOV_PROG` |
| SOL‑2 | `comp_pool_program` | ✅ | `SOL_GOV_PROG` (admin authority PDA) | `SOL_COMP_PROG` (+ `SOL_COMP_PDA`) |
| SOL‑3 | `witness_registry_program` | ✅ | `SOL_GOV_PROG`, `SOL_COMP_PROG` | `SOL_WITNESS_PROG` (+ `SOL_WITNESS_PDA`) |
| SOL‑4 | `signature_verifier_program` (or embed) | ✅ | `SOL_WITNESS_PROG` | `SOL_SIGVERIFY_PROG` |
| SOL‑5 | `audit_logger_program` | ✅ | `SOL_GOV_PROG` | `SOL_AUDIT_PROG` (+ `SOL_AUDIT_PDA`) |
| SOL‑6 | optional `state_proof_validator_program` | optional | `SOL_GOV_PROG` | `SOL_PROOFVAL_PROG` |
| SOL‑7 | optional `finality_checker_program` | optional | `SOL_GOV_PROG` | `SOL_FINALITY_PROG` |
| SOL‑8 | `sxcp_vault_program` | ✅ | `SOL_WITNESS_PROG`, `SOL_SIGVERIFY_PROG`, `SOL_AUDIT_PROG`, `SOL_COMP_PROG`, optional proof/finality programs | `SOL_VAULT_PROG` |
| SOL‑9 | Post‑deploy config (section 4) | ✅ | all above | — |

### 3.5 Bitcoin testnet “deployment order” (UTXO)

> There is no contract deployment step. You **standardize** the scripts and enable relayer/watch tooling.

| Step | Define / Initialize | Required? | Depends on | Output |
|---:|---|---:|---|---|
| BTC‑1 | Standardize `BTC_HTLC_LOCK_SCRIPT_TEMPLATE` | ✅ | — | template ID + version |
| BTC‑2 | Standardize `BTC_CLAIM_TX_TEMPLATE` | ✅ | BTC‑1 | template ID + version |
| BTC‑3 | Standardize `BTC_REFUND_TX_TEMPLATE` | ✅ | BTC‑1 | template ID + version |
| BTC‑4 | (Optional) `BTC_INTENT_MARKER_TEMPLATE` | optional | — | template ID + version |
| BTC‑5 | Configure relayer indexers + policy engine | ✅ | BTC‑1..4 | config bundle |

---

## 4) Wiring matrix (what address/ID must be set where)

This section is the “connective tissue” that makes the suite function.

### 4.1 Core SXCP wiring (applies to Synergy + all EVM chains)

| Component | Needs reference to | Why | How to set |
|---|---|---|---|
| `SXCPVault` | `WitnessRegistry` | validate relayer membership/quorum; rate limits; slashing hooks | constructor param or `setWitnessRegistry(addr)` |
| `SXCPVault` | `SignatureVerifier` | verify threshold / aggregate attestations | constructor param or `setSignatureVerifier(addr)` |
| `SXCPVault` | `AuditLogger` | record lifecycle + attestation metadata | constructor param or `setAuditLogger(addr)` |
| `SXCPVault` | `GovernanceModule` | admin (pause/upgrade/params), chain allow‑list, timeouts | constructor param + `transferOwnership(GOV)` |
| `SXCPVault` | `CompensationPool` | fee sink and/or victim compensation pathways | constructor param or `setCompPool(addr)` |
| `SXCPVault` | `StateProofValidator` (optional) | verify Merkle proofs of receipts/logs | `setStateProofValidator(addr)` |
| `SXCPVault` | `FinalityChecker` (optional) | confirm blocks finalized, detect reorg | `setFinalityChecker(addr)` |
| `SignatureVerifier` | `WitnessRegistry` | load public keys / keysets; signer set | constructor param or `setRegistry(addr)` |
| `WitnessRegistry` | `GovernanceModule` | admin actions (register, slash, quorum changes) | constructor param + ownership |
| `WitnessRegistry` | `CompensationPool` (optional) | slashing -> compensation funding path | `setCompPool(addr)` |
| `AuditLogger` | `GovernanceModule` | admin actions; retention; batch anchoring | ownership |
| `CompensationPool` | `GovernanceModule` | admin actions; payout rules | ownership |

### 4.2 Wallet wiring (Synergy chain only)

| Component | Needs reference to | Why | How to set |
|---|---|---|---|
| `RecoveryModule` | `IdentityRegistry` | recovery state transitions + key rotation controls | constructor param |
| `PolicyConfigRegistry` | `IdentityRegistry` | policy state anchored to identity governance | constructor param |
| `UMARegistry` | `IdentityRegistry`, `RecoveryModule`, `PolicyConfigRegistry` | identity‑controlled routing + restrictions during recovery + policy gates | constructor param(s) |
| `SNSRegistry` | `IdentityRegistry` | ownership and authorization checks | constructor param |
| `AuditReceiptRegistry` | `IdentityRegistry` (optional) | validate identities and canonical identity IDs | constructor param |

### 4.3 Cross‑network wiring (required for multi‑chain operation)

Each `SXCPVault` must know which remote “endpoint” is authoritative for each supported chain family.

**Minimum required mappings on each chain** (store in `SXCPVault` or `GovernanceModule`):

| Mapping | Example key | Example value |
|---|---|---|
| Remote vault endpoint | `chainId/cluster` | EVM: contract address; Solana: program ID; Bitcoin: script template ID |
| Remote witness registry | `chainId/cluster` | EVM: contract address; Solana: program ID; Bitcoin: N/A |
| Chain family | `chainId/cluster` | `EVM` / `SOLANA` / `UTXO` |
| Finality parameters | `chainId/cluster` | confirmations / slot depth / “finalized” rule |
| Proof model toggle | `chainId/cluster` | `ATTESTATION_ONLY` vs `PROOF+ATTESTATION` |

**Deployment sequencing implication:**  
You can’t complete cross‑network configuration until *each network’s vault + registry addresses/program IDs exist*.

---

## 5) Minimum on‑chain event surface to index (relayers + explorers)

Relayers must be able to deterministically detect the start and end of each lifecycle.

### 5.1 Required EVM events (Sepolia + Polygon Amoy)

**From `SXCPVault`:**

- `CrossChainTransferInitiated(…all parameters…)` — emitted after `lockAssets` sets transfer to PENDING.  
- `CrossChainTransferCompleted(transferId, recipient, amount, sourceChain, sourceBlock, completionTimestamp)` — emitted on destination chain after successful completion.  
- Atomic‑swap mode (if enabled for the chain): `HashLockCreated`, `HashLockClaimed` (secret revealed), `HashLockRefunded`.

**From `WitnessRegistry`:**

- `RelayerRegistered(relayer, publicKeyBytes)`
- `AttestationRecorded(transferId, signers[], valid)`
- `SlashingExecuted(relayer, amount, reason)`

**From `AuditLogger` (or AuditReceiptRegistry if merged):**

- `AttestationLogged` / `BatchRootAnchored` (exact naming up to implementation, but must be indexable)

### 5.2 Required Solana “events” (program logs + account writes)

Solana does not have EVM events; you standardize:

- **Program log messages** (Anchor events or structured logs) for:
  - `TransferInitiated`
  - `TransferCompleted`
  - `HashLockCreated/Claimed/Refunded` (atomic swaps)
  - `RelayerRegistered`, `Slashed`, `AttestationRecorded`
- **Account state changes**:
  - PDA storing transfer/lock state transitions (`PENDING → COMPLETED → REVERTED`)
  - Witness registry PDA updates (members + quorum config)

### 5.3 Required Bitcoin observables (UTXO)

Relayers/indexers must track:

- UTXOs matching `BTC_HTLC_LOCK_SCRIPT_TEMPLATE`
- Spending path used:
  - **Claim path** (must reveal preimage in witness/script data)
  - **Refund path** (after timelock)
- (Optional) OP_RETURN markers if you implement `BTC_INTENT_MARKER_TEMPLATE`

---

## 6) Minimum RPC / API surface for relayers and Synergy devnet nodes

### 6.1 Chain RPC capabilities required (by chain family)

**EVM (Sepolia + Polygon Amoy):**
Relayers must be able to:

- Subscribe/poll logs for `CrossChainTransferInitiated` / completion events (`eth_getLogs` / websocket logs)
- Pull block header fields including receipts root (`eth_getBlockByNumber`)
- Pull tx receipts (`eth_getTransactionReceipt`)
- Call token contracts for balance verification (`eth_call` to `balanceOf`), and vault/token metadata
- Submit transactions to destination chain vault (`eth_sendRawTransaction`)

**Solana devnet:**
Relayers must be able to:

- Subscribe/poll for program logs / signatures (`logsSubscribe`, `getSignaturesForAddress`)
- Fetch transactions and confirmation/finality status (`getTransaction`, `getSignatureStatuses`)
- Read PDA account state (`getAccountInfo`, `getProgramAccounts`)
- Submit txs/instructions to vault and registry programs

**Bitcoin testnet:**
Relayers must be able to:

- Monitor for script matches / new UTXOs (indexer or `scantxoutset`)
- Fetch raw transactions and decode witness/script paths (`getrawtransaction`, `decoderawtransaction`)
- Detect timelock maturity / block confirmations (`getblock`, `getblockcount`)
- Broadcast claim/refund transactions as needed (typically wallet‑driven; relayers primarily observe)

### 6.2 Internal relayer network API (minimum viable)

Even if you later move to pure P2P gossip, the devnet needs a deterministic “thin waist” between:

- chain observers
- threshold‑signature participants
- submission agents
- explorers / UI

**Minimum endpoints (REST or gRPC):**

1) **Transfer status**

- `GET /transfers/{transferId}` → `{ status, sourceChain, destinationChain, amount, timestamps, txHashes }`

1) **Observation ingest**

- `POST /observations` → submit `{ chainId, vault, eventType, transferId, blockNumber, txHash, proofBundle }`

1) **Attestation lifecycle**

- `POST /attestations/request` → request signing for `{ transferId, messageHash }`
- `POST /attestations/partial` → submit partial signature
- `GET /attestations/{transferId}` → `{ aggregateSignature, signerSet, quorum, createdAt }`

1) **Submission queue**

- `POST /submissions` → enqueue `{ destinationChain, callData, gasParams }`
- `GET /submissions/{id}` → status and txHash

1) **Relayer health**

- `GET /health` and `GET /metrics`

### 6.3 Minimum Synergy devnet node API extensions (to support SXCP relayers)

Your Synergy node stack should expose (in addition to standard chain RPC):

- **Event subscription** for Synergy SXCP + wallet contracts (identity, UMA updates, receipts)
- **Finality indicator** (if Synergy has its own notion of finalized blocks distinct from head)
- **Historical log access** (relayers must be able to reconstruct proofs deterministically)

---

## 7) Post‑deploy configuration checklist (all networks)

Perform these steps on **every chain** where the suite is deployed.

### 7.1 Governance + admin

- Set admin/owner of each component to the chain’s `GovernanceModule` (or your devnet admin key)
- Configure pause/upgrade roles

### 7.2 Register relayers / quorum

- `WitnessRegistry.registerRelayer(relayerAddress, publicKeyBytes)` for each relayer  
- Set quorum threshold (e.g., t-of-n), timeouts, and slashing rules

### 7.3 Register supported chains (cross‑network endpoints)

On each chain’s governance/vault:

- Add entries for:
  - Synergy devnet
  - Ethereum Sepolia
  - Polygon Amoy
  - Solana devnet
  - Bitcoin testnet (script template IDs)

### 7.4 Register supported assets

On each chain’s `SXCPVault`:

- For EVM: allowlist ERC‑20s (and native token if supported)
- For Solana: allowlist SPL mints + token accounts
- For Bitcoin: script template version allow‑list (the “asset” is BTC itself)

### 7.5 Dry‑run flows to validate wiring

- EVM → EVM transfer: Sepolia → Amoy (small ERC‑20)  
- EVM → Solana atomic swap: Sepolia ↔ Solana devnet  
- BTC ↔ EVM atomic swap: BTC testnet ↔ Sepolia (small amounts)

---

## Appendix A — Suggested “minimum interfaces” (function names)

These are suggested names aligned with the specification examples; adapt to your codebase.

### A.1 `SXCPVault` (EVM / Synergy)

- `lockAssets(amount, destinationChainId, destinationAddress, transferId, timeoutBlock)`
- `submitAttestation(transferId, attestation, proofBundle)`
- `finalizeTransfer(transferId, …)`
- `revertTimeout(transferId)` (timeout path)
- Atomic‑swap mode:
  - `createHashLock(secretHash, amount, counterparty, timeoutBlock)`
  - `revealAndClaim(lockId, secret)`
  - `refundExpired(lockId)`

### A.2 `WitnessRegistry`

- `registerRelayer(relayer, publicKey)`
- `slashRelayer(relayer, amount, reason)`
- `verifyAggregateSignature(messageHash, aggregateSig)`
- `recordAttestation(transferId, signers, valid)`

### A.3 `SignatureVerifier`

- `verifyAggregate(messageHash, aggregateSig, signerSet)`
- `loadPublicKeys()`

---
