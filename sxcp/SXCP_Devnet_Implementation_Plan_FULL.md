# Implementing the Synergy Cross-Chain Protocol (SXCP) in the Devnet

## Introduction

The Synergy Cross-Chain Protocol (SXCP) provides a bridge-less interoperability layer for transferring assets and data between blockchains. SXCP eliminates custodial bridges by combining post-quantum multi-party computation (MPC) with deterministic state proofs. The protocol operates in two modes:

1. **Atomic Swap Mode**: two parties lock tokens on their respective chains under hash- and time-locked conditions; relayers attest to the revelation of the hash pre-image.
2. **Unified MPC Custody Mode**: users deposit assets in a vault contract on the source chain; a threshold subset of relayer nodes validates the deposit and produces an aggregated post-quantum signature; the witness contract on the destination chain verifies the signature and releases (or mints) corresponding assets.

Key components include a relayer network holding post-quantum key shares, a Witness Registry smart contract that manages authorized relayers and slashing, chain-specific vault contracts (bridge contracts) that lock or release assets, and a governance layer controlling parameters such as quorum thresholds, key rotation frequency and emergency procedures. All signatures are produced using Aegis PQC algorithms such as ML-DSA and aggregated via the Aegis Aggregation Engine.

This manual provides a detailed plan to integrate SXCP into the Synergy devnet and extend the node control panel to support the new node types required for cross-chain functionality. The tasks below assume the reader has access to the Synergy devnet and node-control-panel repositories and that the team will deploy to public testnets such as Ethereum Sepolia and at least one additional network (e.g., Polygon’s Mumbai or Avalanche Fuji). The plan is structured in stages: core protocol implementation in the devnet, smart-contract development, modifications to the node control panel, testing and deployment, and future considerations.

---

## 1. Devnet Modifications

### 1.1 Relayer Network Module

SXCP relies on a distributed relayer network where each relayer holds a share of a post-quantum private key generated through distributed key generation (DKG). Relayers monitor events on source chains, validate state transitions and collaboratively produce threshold signatures attesting to cross-chain events. To support this in the devnet:

1. Add a Relayer role to the node software. The existing validator node type handles PoSy consensus but lacks event watching or threshold signing. Add a relayer service that:
2. Maintains a local key share αᵢ derived via a DKG protocol. Implement the DKG steps: each relayer generates a secret polynomial of degree t-1, exchanges encrypted polynomial evaluations, and derives its private share; the collective public key is PK = Σ PKᵢ. Use Aegis PQC’s ML-KEM for key exchange and ML-DSA for signing. The code should leverage the Aegis PQC library integrated into the devnet (from the Aegis PQC Core specification).
3. Listens to vault contract events on supported chains. When it detects a deposit or atomic-swap lock, it verifies the transaction details and independently signs the event hash using its share αᵢ. Partial signatures are broadcast to other relayers.
4. Aggregates valid partial signatures into a complete signature when at least t valid shares are collected. Integrate the Aegis Aggregation Engine to merge multiple ML-DSA signatures into a compact signature that reduces on-chain verification costs by ~70%.
5. Submits the aggregated signature and event metadata to the Witness Registry contract on the destination chain. Relayers should also send heartbeat messages to the registry to maintain their reputation scores.
6. Implement key lifecycle management. Use the Aegis Key Lifecycle Manager to generate new key pairs at epoch boundaries, distribute shares, retire old keys and record proof of destruction. Incorporate the quantum-randomness beacon for deterministic yet unpredictable relayer rotation. The beacon uses ML-KEM secrets, epoch identifiers and previous quorum certificates to generate entropy.
7. Add slashing and reputation tracking. The relayer module should interact with the Witness Registry to update its reputation score, track attestation count and record slashing events for misbehavior. If a relayer fails to sign valid events within a timeout or signs fraudulent events, the registry will reduce its reputation and potentially remove it from the quorum.
8. Networking configuration. Open separate ports for relayer peer-to-peer (P2P) communication, threshold signature exchange and gRPC/JSON-RPC endpoints. For example, use ports 39638 (P2P), 49638 (RPC) and 59638 (WebSocket) for relayers to avoid conflicts with validator nodes. Update the devnet’s `network-config.toml` and templates (e.g., `templates/cross-chain-verifier.toml`) to include these ports and specify bootnodes for relayer discovery.

### 1.2 Witness Registry Integration

The Witness Registry is a smart contract deployed on each participating chain that stores the public keys of authorized relayers, defines quorum thresholds (t of n), tracks reputation scores and applies slashing penalties. The devnet must integrate with this registry as follows:

1. Implement an off-chain registry interface. Add a module to the devnet node that interacts with the Witness Registry via JSON-RPC or gRPC. Functions should include: registering/unregistering relayers, updating reputation scores, retrieving quorum parameters, submitting aggregated signatures and fetching attestation logs.
2. Expose RPC methods. Extend the devnet node’s JSON-RPC API (documented in the API reference) with endpoints such as `synergy_registerRelayer`, `synergy_getRelayerSet`, `synergy_submitAttestation` and `synergy_getAttestations`. Each method should call the appropriate function on the on-chain Witness Registry and return results to clients or the control panel.
3. Maintain local state. Store local copies of registry metadata (public keys, threshold values, reputation scores) to reduce on-chain queries. Ensure that updates from the registry are propagated to all relayer nodes.

### 1.3 Vault / Bridge Contract Event Listener

The bridge contracts (also called vault contracts) lock and release assets on each chain, validate aggregated signatures and emit events for relayers. The devnet must watch these contracts and react accordingly:

1. Event subscription. Add an event listener in the relayer module for each supported chain. For the devnet chain, this listener watches deposit events and atomic-swap locks; for external testnets (Sepolia, etc.) it watches deposit or mint events. Use the chain’s WebSocket or RPC endpoint and filter events by contract address and event signature.
2. Event validation. When a deposit event is detected, validate the transaction (sender address, token amount, nonce, time lock) and compute the event hash. Each relayer signs the hash with its private share and broadcasts the partial signature.
3. State-proof construction. For MPC custody mode, include the deposit event in a Merkle tree and compute the Merkle root. Provide proof parameters to the Witness Registry when submitting the aggregated signature.

### 1.4 Governance and DAO Integration

SXCP relies on DAO governance (using Synergy Scores from PoSy validators) to adjust quorum thresholds, key rotation schedules and emergency procedures. For the devnet:

1. Implement governance proposals. Add a module to handle proposals related to the Witness Registry or relayer network, such as adding/removing relayers, adjusting t and n, setting slashing penalties, or triggering emergency pauses. Use the existing governance functions in the PoSy oracle contract; proposals should require a 67% weighted majority to pass.
2. Expose governance RPC. Provide endpoints like `synergy_proposeRelayerChange`, `synergy_voteOnProposal` and `synergy_executeProposal`, which the control panel and validators can call. Make sure proposals are time-locked (e.g., 48 hours) to allow dissenting validators to exit.
3. Emergency rollback. Implement the “pause” state and rollback mechanism described in the specification. When invoked, the system should stop relayer activity, reconstruct the valid pre-exploit state from Merkle proofs and resume once patched.

### 1.5 Synergy Score and PoSy Adaptations

Since relayers are new actors, the Proof-of-Synergy consensus must be extended to account for relayer performance:

1. Synergy Score integration. Modify the Synergy Oracle to include relayer metrics (attestation accuracy, latency, uptime, slashing history) in the Synergy Score computation. Extend the `ValidatorMetrics` and `EpochSnapshot` structures to include fields for relayer tasks. The contribution index used in the score should reward timely and accurate attestations, with slashing penalties reducing the score.
2. Cluster selection for relayers. Determine how relayers are assigned to clusters or whether they form an independent cluster. Bridge selection in PoSy currently chooses the top K validators by normalized Synergy Score to relay messages; extend this logic to include relayers or create a separate relayer selection algorithm based on reputation scores.
3. Reward distribution. Define how relayers are rewarded (Synergy Points, fees) for successful attestations. Integrate these rewards into the existing reward weighting (task_accuracy, uptime, collaboration) in the consensus configuration.

### 1.6 Networking and Configuration Changes

1. Port allocations: allocate ports for relayer nodes (e.g., 39638 P2P, 49638 RPC, 59638 WebSocket) and cross-chain verifiers. Update `SYNERGY_DEVNET_PORTS_AND_PROTOCOLS.txt` and create new templates (e.g., `relayer.toml`, `cross-chain-vault.toml`).
2. Bootnodes: define bootnodes for the relayer network; these nodes coordinate DKG and maintain the list of active relayers. Bootnodes should run both PoSy validator and relayer services or run dedicated relayer nodes.
3. Config generation: extend the `generate_templates.sh` script to produce templates for relayers and cross-chain verifiers. Include sections for `[relayer]` specifying witness registry addresses, threshold values, external chain RPC endpoints, and timeouts.

---

## 2. Smart Contract Development

### 2.1 Witness Registry Contract

Create a smart contract (in Solidity or SynQ) to manage relayer membership and maintain attestation data. Key components:

- Relayer struct: store the relayer’s ML-DSA public key, reputation score, attestation count, slashing history and active status.
- Quorum parameters: maintain total relayer count n and required signature count t. Use a default threshold of ⌈2⁄3 n⌉ for Byzantine fault tolerance.
- Reputation and slashing: functions to increment attestation counts, adjust reputation scores and penalize malicious behavior (e.g., missing attestations, fraudulent signatures). Penalties should be proportional to the severity and reduce future participation rights.
- Registration / deregistration: functions for relayers to register by submitting their public key and for the DAO to remove relayers. Registration should require a governance approval or staking requirement.
- Attestation log: emit events for each attestation, recording timestamp, event hash, participating relayers and aggregated signature. This enables public audit trails and dispute resolution.
- Threshold verification: a function `verifyAttestation(bytes eventHash, bytes aggregateSig)` that uses the aggregated public key and threshold parameter to verify the signature. In MPC mode, also verify the Merkle proof of the event.
- Governance hooks: allow the DAO to update quorum thresholds, slashing parameters, key rotation schedules and emergency procedures.

Deploy the Witness Registry on the devnet and on each external testnet. For testing, use Sepolia addresses; later, update addresses when deploying to mainnets.

### 2.2 Vault (Bridge) Contracts

Develop a pair of chain-specific vault contracts that manage asset资产 transfers. Each contract should support both atomic swap and MPC custody modes:

1. Atomic Swap functions:
2. `initiateSwap(bytes32 hash, uint256 amount, address counterparty, uint64 timeout)`: lock tokens under a hash and time-lock; emit `SwapInitiated` event.
3. `completeSwap(bytes32 preimage)`: verify that `hash(preimage)` matches the stored hash, release tokens to the counterparty and emit `SwapCompleted` event. Require a signature from the relayer network confirming preimage revelation.
4. `refundSwap()`: allow the initiator to reclaim tokens after timeout if the swap is not completed.
5. MPC Custody functions:
6. `deposit(uint256 amount, address recipient)`: lock tokens on the source chain; emit `Deposit` event with details and a nonce.
7. `release(bytes32 eventHash, bytes aggregateSig, bytes proof)`: verify the aggregated ML-DSA signature and Merkle proof. If valid, mint or release the corresponding amount of tokens on the destination chain. Emit `Release` event.
8. Relayer management: include a reference to the Witness Registry contract to verify aggregated signatures and fetch quorum parameters. Optionally implement a slashing mechanism in the vault contract as an extra safeguard (e.g., require a bond for relayers and slash the bond if invalid signatures are submitted).
9. Security: incorporate the Aegis PQC verification library to check aggregated signatures; ensure that all cryptographic operations use post-quantum algorithms such as ML-DSA; guard against replay attacks by including chain IDs and nonces in the signed messages.

### 2.3 DAO and Governance Contracts

Use existing PoSy governance contracts to manage SXCP parameters. Key functions include:

- `proposeParameterChange(bytes32 parameter, uint256 newValue)`
- `voteOnProposal(uint256 proposalId, bool support)`
- `executeProposal(uint256 proposalId)`
- `emergencyPause()`
- `emergencyRollback(bytes32 snapshotRoot)`

---

## 3. Node-Control-Panel Enhancements

The node-control-panel repository automates node setup via YAML recipes and TOML templates. To support SXCP:

### 3.1 New Node Types and Recipes

1. Relayer node: add a recipe `relayer.yml`.
2. Cross-chain verifier node: add `cross-chain-verifier.yml`.
3. Witness Registry deployment: `witness-registry-deploy.yml`.
4. Vault contract deployment: `vault-deploy.yml`.
5. Upgrade tasks and key rotation recipes.

Example skeleton for `relayer.yml`:

```yaml
node_type: relayer
role: sxcp-relayer
steps:
  - name: recipe
    description: Validating relayer setup recipe
    progress: 0
  - name: init
    description: Initializing relayer environment
    progress: 10
  - name: directories
    description: Creating sandbox and key directories
    progress: 25
  - name: dkg
    description: Running distributed key generation to derive PQC key share
    progress: 40
  - name: binary
    description: Preparing Synergy relayer binary
    progress: 60
  - name: config
    description: Preparing configuration files for the relayer (registry address, RPC endpoints, threshold)
    progress: 75
  - name: register
    description: Registering the relayer in the Witness Registry contract
    progress: 90
  - name: start
    description: Starting the relayer node and syncing cross-chain events
    progress: 100
```

### 3.2 Templates and Configuration

1. Create TOML templates for relayers and verifiers.
2. Define network IDs, ports, consensus algorithm.
3. Add [relayer] configuration sections.
4. Update template generation scripts.
5. Extend control panel UI and CLI.

### 3.3 Key Generation and Management

1. Integrate Aegis PQC key tools.
2. Implement scheduled key rotation (every 2,160 blocks).
3. Add slashing and appeal monitoring UI.

## 4. Testing and Deployment Plan

1. Local integration tests.
2. Testnet deployment on Sepolia.
3. Secondary testnet deployment (e.g., Polygon Mumbai).
4. Performance and adversarial testing.
5. Governance testing.
6. Documentation and user guides.

## 5. Future Considerations

1. Additional chains.
2. SynQ integration.
3. AI-assisted attestations.
4. Control panel dashboards.
5. Hardware Security Modules (HSM).

## Conclusion

Implementing the Synergy Cross-Chain Protocol in the devnet requires significant modifications to node software, smart contracts and operational tooling. By following the tasks outlined above—adding relayer roles with distributed key generation and threshold signing, deploying witness registry and vault contracts, adapting PoSy and governance modules, extending the node control panel with new recipes and templates, and rigorously testing across multiple testnets—the Synergy team can achieve a bridge-less, quantum-secure cross-chain infrastructure. These steps will enable early experimentation with SXCP on Sepolia and other testnets, laying the foundation for a secure multi-chain ecosystem.
