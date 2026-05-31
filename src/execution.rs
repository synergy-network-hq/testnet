use crate::crypto::aegis_pqvm::{SYNERGY_RECEIPT_ROOT_V1, SYNERGY_STATE_ROOT_V1};
use crate::synergy_types::{Block, CanonicalSerialize, Hash, Transaction, TxId};
use crate::synq_admission::SynQVerificationSummary;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionState {
    pub balances_nwei: BTreeMap<String, u128>,
    pub verified_authorizations: BTreeMap<TxId, Hash>,
    pub synq_verifications: BTreeMap<TxId, SynQVerificationSummary>,
    pub synq_errors: BTreeMap<TxId, (String, String)>,
}

impl ExecutionState {
    pub fn new() -> Self {
        Self {
            balances_nwei: BTreeMap::new(),
            verified_authorizations: BTreeMap::new(),
            synq_verifications: BTreeMap::new(),
            synq_errors: BTreeMap::new(),
        }
    }

    pub fn with_balance(mut self, account: &str, amount_nwei: u128) -> Self {
        self.balances_nwei.insert(account.to_string(), amount_nwei);
        self
    }

    pub fn mark_authorized(&mut self, tx: &Transaction) -> Result<TxId, String> {
        let tx_id = tx_id(tx)?;
        self.verified_authorizations
            .insert(tx_id.clone(), tx.canonical_tx_bytes_hash()?);
        match crate::synq_admission::verify_transaction_payload_for_chain_admission(
            tx,
            current_unix_timestamp(),
        ) {
            Ok(Some(summary)) => {
                self.synq_verifications.insert(tx_id.clone(), summary);
            }
            Ok(None) => {}
            Err(error) => {
                self.synq_errors
                    .insert(tx_id.clone(), (error.code().to_string(), error.to_string()));
                return Err(error.to_string());
            }
        }
        Ok(tx_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionGraph {
    pub batches: Vec<Vec<TxId>>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReceiptStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TransactionReceipt {
    pub tx_id: TxId,
    pub status: ReceiptStatus,
    pub gas_used: u64,
    pub error: String,
    pub state_root_after: Hash,
    pub synq_verification: Option<SynQVerificationSummary>,
    pub synq_error_code: Option<String>,
    pub synq_error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    pub state: ExecutionState,
    pub receipts: Vec<TransactionReceipt>,
    pub state_root_after: Hash,
    pub receipt_root: Hash,
}

pub fn execute_block(block: &Block, state: &ExecutionState) -> Result<ExecutionResult, String> {
    let graph = build_execution_graph(&block.transactions)?;
    let batches = split_into_parallel_batches(&graph);
    let mut working_state = state.clone();
    let mut receipts = Vec::new();
    for batch in batches {
        let mut batch_receipts =
            execute_batch_parallel(&batch, &block.transactions, &mut working_state)?;
        receipts.append(&mut batch_receipts);
    }
    receipts = merge_results_in_canonical_order(receipts);
    let state_root_after = compute_state_root_after(&working_state)?;
    let receipt_root = compute_receipt_root(&receipts)?;
    Ok(ExecutionResult {
        state: working_state,
        receipts,
        state_root_after,
        receipt_root,
    })
}

pub fn build_execution_graph(transactions: &[Transaction]) -> Result<ExecutionGraph, String> {
    let mut resource_owner = BTreeMap::<String, TxId>::new();
    let mut tx_depth = BTreeMap::<TxId, usize>::new();
    let mut batches: Vec<Vec<TxId>> = Vec::new();
    for tx in transactions {
        let id = tx_id(tx)?;
        let mut depth = 0usize;
        for resource in &tx.write_set_hint {
            if let Some(parent) = resource_owner.get(resource) {
                depth = depth.max(tx_depth.get(parent).copied().unwrap_or(0).saturating_add(1));
            }
        }
        tx_depth.insert(id.clone(), depth);
        for resource in &tx.write_set_hint {
            resource_owner.insert(resource.clone(), id.clone());
        }
        if batches.len() <= depth {
            batches.resize(depth + 1, Vec::new());
        }
        batches[depth].push(id);
    }
    for batch in &mut batches {
        batch.sort();
    }
    Ok(ExecutionGraph { batches })
}

pub fn split_into_parallel_batches(graph: &ExecutionGraph) -> Vec<Vec<TxId>> {
    graph.batches.clone()
}

pub fn execute_batch_parallel(
    batch: &[TxId],
    transactions: &[Transaction],
    state: &mut ExecutionState,
) -> Result<Vec<TransactionReceipt>, String> {
    let by_id = transactions
        .iter()
        .map(|tx| tx_id(tx).map(|id| (id, tx)))
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let mut receipts = Vec::new();
    for tx_id in batch {
        let tx = by_id
            .get(tx_id)
            .ok_or_else(|| format!("transaction {} missing from batch input", tx_id.0))?;
        receipts.push(execute_transaction(tx_id.clone(), tx, state)?);
    }
    Ok(receipts)
}

pub fn merge_results_in_canonical_order(
    mut receipts: Vec<TransactionReceipt>,
) -> Vec<TransactionReceipt> {
    receipts.sort_by(|a, b| a.tx_id.cmp(&b.tx_id));
    receipts
}

pub fn compute_state_root_after(state: &ExecutionState) -> Result<Hash, String> {
    serde_json::to_vec(&state.balances_nwei)
        .map(|bytes| Hash::from_domain_bytes(SYNERGY_STATE_ROOT_V1, &bytes))
        .map_err(|error| format!("state root serialize failed: {error}"))
}

pub fn compute_receipt_root(receipts: &[TransactionReceipt]) -> Result<Hash, String> {
    serde_json::to_vec(receipts)
        .map(|bytes| Hash::from_domain_bytes(SYNERGY_RECEIPT_ROOT_V1, &bytes))
        .map_err(|error| format!("receipt root serialize failed: {error}"))
}

fn execute_transaction(
    id: TxId,
    tx: &Transaction,
    state: &mut ExecutionState,
) -> Result<TransactionReceipt, String> {
    let canonical_hash = tx.canonical_tx_bytes_hash()?;
    match state.verified_authorizations.get(&id) {
        Some(recorded) if *recorded == canonical_hash => {}
        Some(_) => {
            return Err(format!(
                "transaction {} bytes changed after PQC authorization verification",
                id.0
            ));
        }
        None => {
            return Err(format!(
                "transaction {} missing verified Aegis PQC authorization context",
                id.0
            ));
        }
    }

    let sender = tx.sender_uma_or_account.clone();
    let receiver = tx.receiver_uma_or_account.clone();
    let total_debit = tx.amount_nwei.saturating_add(tx.max_fee_nwei);
    let sender_balance = state.balances_nwei.get(&sender).copied().unwrap_or(0);
    let synq_verification = state.synq_verifications.get(&id).cloned();
    let synq_error = state.synq_errors.get(&id).cloned();
    let receipt = if sender_balance >= total_debit {
        state
            .balances_nwei
            .insert(sender.clone(), sender_balance - total_debit);
        let receiver_balance = state.balances_nwei.get(&receiver).copied().unwrap_or(0);
        state
            .balances_nwei
            .insert(receiver, receiver_balance.saturating_add(tx.amount_nwei));
        TransactionReceipt {
            tx_id: id,
            status: ReceiptStatus::Success,
            gas_used: tx.gas_limit.min(21_000),
            error: String::new(),
            state_root_after: compute_state_root_after(state)?,
            synq_verification,
            synq_error_code: synq_error.as_ref().map(|(code, _)| code.clone()),
            synq_error_message: synq_error.map(|(_, message)| message),
        }
    } else {
        TransactionReceipt {
            tx_id: id,
            status: ReceiptStatus::Failed,
            gas_used: tx.gas_limit.min(21_000),
            error: "INSUFFICIENT_FUNDS".to_string(),
            state_root_after: compute_state_root_after(state)?,
            synq_verification,
            synq_error_code: synq_error.as_ref().map(|(code, _)| code.clone()),
            synq_error_message: synq_error.map(|(_, message)| message),
        }
    };
    Ok(receipt)
}

fn current_unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn tx_id(tx: &Transaction) -> Result<TxId, String> {
    Ok(TxId::from_hash(Hash::from_domain_bytes(
        "SYNERGY_EXECUTION_TX_ID_V1",
        &tx.canonical_bytes()?,
    )))
}

pub fn verified_context_for_block(
    transactions: &[Transaction],
) -> Result<BTreeMap<TxId, Hash>, String> {
    let mut context = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for tx in transactions {
        let id = tx_id(tx)?;
        if !seen.insert(id.clone()) {
            return Err(format!("duplicate transaction {} in block", id.0));
        }
        context.insert(id, tx.canonical_tx_bytes_hash()?);
    }
    Ok(context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synergy_types::{
        AegisPqKeyId, AegisPqSignature, ChainId, Epoch, Height, NetworkId, UmaId,
    };

    fn tx(sender: &str, receiver: &str, nonce: u64, amount: u128, write: &str) -> Transaction {
        Transaction {
            version: 1,
            chain_id: ChainId::synergy_testnet_v2(),
            network_id: NetworkId::synergy_testnet_v2(),
            epoch: Epoch(0),
            sender_uma_or_account: sender.to_string(),
            receiver_uma_or_account: receiver.to_string(),
            account_nonce_or_sequence: nonce,
            amount_nwei: amount,
            gas_limit: 21_000,
            max_fee_nwei: 1,
            ttl_height: Height(100),
            explicit_dependencies: Vec::new(),
            read_set_hint: Vec::new(),
            write_set_hint: vec![write.to_string()],
            payload: Vec::new(),
            signer_uma_id: UmaId(sender.to_string()),
            aegis_pq_key_id: AegisPqKeyId("key".to_string()),
            aegis_pq_signature: AegisPqSignature {
                algorithm: "fndsa".to_string(),
                signature_bytes: vec![1, 2, 3],
            },
        }
    }

    fn block(transactions: Vec<Transaction>) -> Block {
        Block {
            header: crate::synergy_types::BlockHeader {
                version: 1,
                chain_id: ChainId::synergy_testnet_v2(),
                network_id: NetworkId::synergy_testnet_v2(),
                height: Height(1),
                round: crate::synergy_types::Round(0),
                epoch: Epoch(0),
                cluster_id: crate::synergy_types::ClusterId(0),
                parent_block_hash: Hash::zero(),
                parent_state_root: Hash::zero(),
                last_finalized_qc_hash: Hash::zero(),
                proposer_validator_id: crate::synergy_types::ValidatorId("v1".to_string()),
                proposer_uma_id: UmaId("uma-v1".to_string()),
                proposer_key_id: AegisPqKeyId("key".to_string()),
                active_validator_set_hash: Hash::zero(),
                eligible_validator_set_hash: Hash::zero(),
                cluster_map_hash: Hash::zero(),
                proposer_schedule_hash: Hash::zero(),
                protocol_config_hash: Hash::zero(),
                dag_frontier_root: Hash::zero(),
                tx_order_root: Hash::zero(),
                tx_count: transactions.len() as u64,
                evidence_root: Hash::zero(),
                state_root_before: Hash::zero(),
                state_root_after: Hash::zero(),
                receipt_root: Hash::zero(),
                app_version: 1,
                execution_version: 1,
                dag_version: 1,
                aegis_pqvm_version: "aegis-pqvm".to_string(),
                timestamp_ms_consensus_bounded: 0,
            },
            transactions,
            proposer_signature: AegisPqSignature {
                algorithm: "fndsa".to_string(),
                signature_bytes: vec![1],
            },
        }
    }

    fn authorized_state(transactions: &[Transaction]) -> ExecutionState {
        let mut state = ExecutionState::new()
            .with_balance("alice", 1_000_000)
            .with_balance("bob", 1_000_000)
            .with_balance("carol", 0)
            .with_balance("dave", 0);
        state.verified_authorizations = verified_context_for_block(transactions).unwrap();
        state
    }

    #[test]
    fn same_block_executed_repeatedly_produces_same_state_root() {
        let transactions = vec![
            tx("alice", "carol", 0, 10, "alice"),
            tx("bob", "dave", 0, 20, "bob"),
        ];
        let block = block(transactions.clone());
        let state = authorized_state(&transactions);
        let first = execute_block(&block, &state).unwrap().state_root_after;
        for _ in 0..100 {
            assert_eq!(
                execute_block(&block, &state).unwrap().state_root_after,
                first
            );
        }
    }

    #[test]
    fn failed_receipt_is_deterministic_and_conflicts_execute_in_order() {
        let transactions = vec![
            tx("alice", "carol", 0, 10, "alice"),
            tx("alice", "dave", 1, 2_000_000, "alice"),
        ];
        let block = block(transactions.clone());
        let state = authorized_state(&transactions);
        let a = execute_block(&block, &state).unwrap();
        let b = execute_block(&block, &state).unwrap();
        assert_eq!(a.receipts, b.receipts);
        assert!(a
            .receipts
            .iter()
            .any(|receipt| receipt.status == ReceiptStatus::Failed));
    }

    #[test]
    fn receipt_preserves_synq_verification_summary() {
        let transaction = tx("alice", "carol", 0, 10, "synq-contract");
        let id = tx_id(&transaction).unwrap();
        let block = block(vec![transaction.clone()]);
        let mut state = authorized_state(&[transaction]);
        state.synq_verifications.insert(
            id,
            crate::synq_admission::SynQVerificationSummary {
                chain_id: crate::synergy_types::SYNERGY_TESTNET_V2_CHAIN_ID,
                normalized_network_id: "synergy-testnet".to_string(),
                node_network_id: crate::synergy_types::SYNERGY_TESTNET_V2_NETWORK_ID.to_string(),
                domain: "SYNQ_CONTRACT_DEPLOY_V1".to_string(),
                algorithm: "ML-DSA-65".to_string(),
                signer: "tsynq1fixture".to_string(),
                payload_hash: [7; 32],
                bytecode_hash: Some([1; 32]),
                manifest_hash: Some([2; 32]),
                abi_hash: Some([3; 32]),
                verified_at_admission: true,
            },
        );

        let result = execute_block(&block, &state).unwrap();
        let receipt = result.receipts.first().expect("receipt");
        assert_eq!(
            receipt
                .synq_verification
                .as_ref()
                .map(|summary| summary.domain.as_str()),
            Some("SYNQ_CONTRACT_DEPLOY_V1")
        );
    }

    #[test]
    fn missing_or_altered_authorization_context_fails_closed() {
        let mut transaction = tx("alice", "carol", 0, 10, "alice");
        let original_block = block(vec![transaction.clone()]);
        let state = ExecutionState::new().with_balance("alice", 100);
        assert!(execute_block(&original_block, &state).is_err());

        let mut state = authorized_state(&[transaction.clone()]);
        transaction.amount_nwei = 11;
        let altered = block(vec![transaction]);
        assert!(execute_block(&altered, &state).is_err());
        state.verified_authorizations.clear();
        assert!(execute_block(&altered, &state).is_err());
    }
}
