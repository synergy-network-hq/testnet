use crate::crypto::aegis_pqvm::{AegisPqvmSigner, AegisPqvmVerifier};
use crate::dag_mempool::{BlockSelectionLimits, DagAdmissionResult, DagMempool};
use crate::synergy_types::{
    AegisPqKeyId, AegisPqKeyRole, AegisPqSignature, CanonicalSerialize, ChainId, Epoch, Height,
    NetworkId, Transaction, TxDependency, TxDependencyType, TxId, UmaId,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AegisTxBuildOptions {
    pub signer_uma_id: String,
    pub sender: String,
    pub receiver: String,
    pub nonce: u64,
    pub amount_nwei: u128,
    pub gas_limit: u64,
    pub max_fee_nwei: u128,
    pub ttl_height: u64,
    pub epoch: u64,
    pub read_set_hint: Vec<String>,
    pub write_set_hint: Vec<String>,
    pub explicit_dependencies: Vec<String>,
    pub payload: Vec<u8>,
}

impl Default for AegisTxBuildOptions {
    fn default() -> Self {
        Self {
            signer_uma_id: "aegis-dag-fixture-sender".to_string(),
            sender: "aegis-dag-fixture-sender".to_string(),
            receiver: "aegis-dag-fixture-receiver".to_string(),
            nonce: 0,
            amount_nwei: 1,
            gas_limit: 10,
            max_fee_nwei: 1_000,
            ttl_height: 100,
            epoch: 0,
            read_set_hint: Vec::new(),
            write_set_hint: vec!["fixture-resource".to_string()],
            explicit_dependencies: Vec::new(),
            payload: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AegisSignedTxReport {
    pub tx_id: TxId,
    pub key_id: AegisPqKeyId,
    pub key_role: AegisPqKeyRole,
    pub signature_verification_result: String,
    pub dag_node_id: TxId,
    pub admission_result: DagAdmissionResult,
    pub canonical_tx_bytes_hex: String,
    pub transaction: Transaction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AegisDagFixtureReport {
    pub chain_id: u64,
    pub network_id: String,
    pub signer_uma_id: String,
    pub key_id: AegisPqKeyId,
    pub key_role: AegisPqKeyRole,
    pub transactions: Vec<AegisSignedTxReport>,
    pub ready_frontier: Vec<TxId>,
    pub selected_ancestor_closed_set: Vec<TxId>,
    pub tx_order_root: String,
    pub dag_frontier_root: String,
    pub atlas_ingestion_status: String,
}

pub fn sign_with_new_aegis_transaction_key(
    options: AegisTxBuildOptions,
) -> Result<AegisSignedTxReport, String> {
    let mut signer = AegisPqvmSigner::initialize_required().map_err(|error| error.to_string())?;
    let key_id = signer
        .generate_and_register_key(
            &options.signer_uma_id,
            vec![AegisPqKeyRole::Transaction],
            Epoch(options.epoch),
        )
        .map_err(|error| error.to_string())?;
    let tx = sign_with_existing_aegis_transaction_key(&mut signer, &key_id, options)?;
    let verifier = signer.verifier();
    report_for_transaction(&verifier, tx, key_id)
}

pub fn build_fixture_report() -> Result<AegisDagFixtureReport, String> {
    let signer_uma_id = "aegis-dag-fixture-sender".to_string();
    let mut signer = AegisPqvmSigner::initialize_required().map_err(|error| error.to_string())?;
    let key_id = signer
        .generate_and_register_key(&signer_uma_id, vec![AegisPqKeyRole::Transaction], Epoch(0))
        .map_err(|error| error.to_string())?;

    let tx0 = sign_with_existing_aegis_transaction_key(
        &mut signer,
        &key_id,
        AegisTxBuildOptions {
            signer_uma_id: signer_uma_id.clone(),
            sender: signer_uma_id.clone(),
            nonce: 0,
            write_set_hint: vec!["fixture-account-sequence".to_string()],
            ..AegisTxBuildOptions::default()
        },
    )?;

    let tx0_id = tx_id_from_signed_tx(&signer.verifier(), &tx0)?;
    let tx1 = sign_with_existing_aegis_transaction_key(
        &mut signer,
        &key_id,
        AegisTxBuildOptions {
            signer_uma_id: signer_uma_id.clone(),
            sender: signer_uma_id.clone(),
            nonce: 1,
            write_set_hint: vec!["fixture-account-sequence".to_string()],
            explicit_dependencies: vec![tx0_id.0.clone()],
            payload: b"explicit dependency on nonce 0".to_vec(),
            ..AegisTxBuildOptions::default()
        },
    )?;

    let tx2 = sign_with_existing_aegis_transaction_key(
        &mut signer,
        &key_id,
        AegisTxBuildOptions {
            signer_uma_id: signer_uma_id.clone(),
            sender: "aegis-dag-fixture-independent".to_string(),
            nonce: 0,
            write_set_hint: vec!["fixture-independent".to_string()],
            payload: b"independent transaction".to_vec(),
            ..AegisTxBuildOptions::default()
        },
    )?;

    let verifier = signer.verifier();
    let mut mempool = DagMempool::new(&verifier, Epoch(0), Height(0));
    let mut reports = Vec::new();
    for tx in [tx0, tx1, tx2] {
        reports.push(report_for_transaction_in_mempool(
            &verifier,
            &mut mempool,
            tx,
            key_id.clone(),
        )?);
    }
    let ready_frontier = mempool.ready_frontier();
    let selected_ancestor_closed_set = mempool.ancestor_closed_set(
        &ready_frontier,
        BlockSelectionLimits {
            max_txs: 10,
            max_gas: 1_000_000,
        },
    )?;
    let tx_order_root = mempool.compute_tx_order_root(&selected_ancestor_closed_set)?;
    let dag_frontier_root = mempool.compute_dag_frontier_root()?;

    Ok(AegisDagFixtureReport {
        chain_id: ChainId::synergy_testnet_v2().0,
        network_id: NetworkId::synergy_testnet_v2().0,
        signer_uma_id,
        key_id,
        key_role: AegisPqKeyRole::Transaction,
        transactions: reports,
        ready_frontier,
        selected_ancestor_closed_set,
        tx_order_root: tx_order_root.to_hex(),
        dag_frontier_root: dag_frontier_root.to_hex(),
        atlas_ingestion_status: "not_attempted: typed Aegis DAG transaction RPC is not wired yet; no fabricated Atlas rows were created".to_string(),
    })
}

fn sign_with_existing_aegis_transaction_key(
    signer: &mut AegisPqvmSigner,
    key_id: &AegisPqKeyId,
    options: AegisTxBuildOptions,
) -> Result<Transaction, String> {
    let mut tx = Transaction {
        version: 1,
        chain_id: ChainId::synergy_testnet_v2(),
        network_id: NetworkId::synergy_testnet_v2(),
        epoch: Epoch(options.epoch),
        sender_uma_or_account: options.sender,
        receiver_uma_or_account: options.receiver,
        account_nonce_or_sequence: options.nonce,
        amount_nwei: options.amount_nwei,
        gas_limit: options.gas_limit,
        max_fee_nwei: options.max_fee_nwei,
        ttl_height: Height(options.ttl_height),
        explicit_dependencies: options
            .explicit_dependencies
            .into_iter()
            .map(|tx_id| TxDependency {
                dependency_type: TxDependencyType::ExplicitDependency,
                tx_id: TxId(tx_id),
            })
            .collect(),
        read_set_hint: options.read_set_hint,
        write_set_hint: options.write_set_hint,
        payload: options.payload,
        signer_uma_id: UmaId(options.signer_uma_id),
        aegis_pq_key_id: key_id.clone(),
        aegis_pq_signature: AegisPqSignature {
            algorithm: String::new(),
            signature_bytes: Vec::new(),
        },
    };
    tx.aegis_pq_signature = signer
        .sign_transaction(&tx.signing_bytes()?, key_id)
        .map_err(|error| error.to_string())?;
    Ok(tx)
}

fn report_for_transaction(
    verifier: &AegisPqvmVerifier,
    tx: Transaction,
    key_id: AegisPqKeyId,
) -> Result<AegisSignedTxReport, String> {
    let mut mempool = DagMempool::new(verifier, tx.epoch, Height(0));
    report_for_transaction_in_mempool(verifier, &mut mempool, tx, key_id)
}

fn report_for_transaction_in_mempool(
    verifier: &AegisPqvmVerifier,
    mempool: &mut DagMempool<'_>,
    tx: Transaction,
    key_id: AegisPqKeyId,
) -> Result<AegisSignedTxReport, String> {
    verifier
        .verify_transaction_signature_checked(&tx)
        .map_err(|error| error.to_string())?;
    let canonical_tx_bytes = tx.canonical_bytes()?;
    let admission_result = mempool.admit_transaction(tx.clone())?;
    Ok(AegisSignedTxReport {
        tx_id: admission_result.tx_id.clone(),
        key_id,
        key_role: AegisPqKeyRole::Transaction,
        signature_verification_result: "verified_through_aegis_pqvm".to_string(),
        dag_node_id: admission_result.tx_id.clone(),
        admission_result,
        canonical_tx_bytes_hex: hex::encode(canonical_tx_bytes),
        transaction: tx,
    })
}

fn tx_id_from_signed_tx(verifier: &AegisPqvmVerifier, tx: &Transaction) -> Result<TxId, String> {
    let mut mempool = DagMempool::new(verifier, tx.epoch, Height(0));
    Ok(mempool.admit_transaction(tx.clone())?.tx_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_aegis_transaction_key_signs_verifies_and_admits_to_dag() {
        let report = sign_with_new_aegis_transaction_key(AegisTxBuildOptions::default()).unwrap();
        assert_eq!(report.key_role, AegisPqKeyRole::Transaction);
        assert_eq!(
            report.signature_verification_result,
            "verified_through_aegis_pqvm"
        );
        assert!(report.admission_result.ready);
        assert!(!report.canonical_tx_bytes_hex.is_empty());
    }

    #[test]
    fn fixture_uses_dependencies_and_no_wallet_cli_path() {
        let report = build_fixture_report().unwrap();
        assert_eq!(report.chain_id, 1264);
        assert_eq!(report.network_id, "synergy-testnet-v2");
        assert_eq!(report.transactions.len(), 3);
        assert!(report
            .transactions
            .iter()
            .all(|tx| tx.signature_verification_result == "verified_through_aegis_pqvm"));
        assert!(report.transactions.iter().any(|tx| !tx
            .admission_result
            .missing_dependencies
            .is_empty()
            || tx.transaction.account_nonce_or_sequence > 0));
        assert!(report.atlas_ingestion_status.contains("not_attempted"));
    }

    #[test]
    fn altered_transaction_bytes_fail_aegis_verification() {
        let mut signer = AegisPqvmSigner::initialize_required().unwrap();
        let key_id = signer
            .generate_and_register_key("alice", vec![AegisPqKeyRole::Transaction], Epoch(0))
            .unwrap();
        let mut tx = sign_with_existing_aegis_transaction_key(
            &mut signer,
            &key_id,
            AegisTxBuildOptions {
                signer_uma_id: "alice".to_string(),
                sender: "alice".to_string(),
                ..AegisTxBuildOptions::default()
            },
        )
        .unwrap();
        tx.amount_nwei = tx.amount_nwei.saturating_add(1);
        let verifier = signer.verifier();
        assert!(verifier.verify_transaction_signature_checked(&tx).is_err());
    }
}
