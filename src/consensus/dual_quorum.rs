use crate::block::Block;
use crate::crypto::pqc::{
    PQCAlgorithm, PQCCiphertext, PQCManager, PQCPrivateKey, PQCPublicKey, PQCSignature,
};
use crate::validator::ValidatorManager;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_512};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

lazy_static::lazy_static! {
    static ref EPHEMERAL_VALIDATOR_KEYS: Arc<Mutex<HashMap<String, (PQCPublicKey, PQCPrivateKey)>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub validator_address: String,
    pub block_hash: String,
    pub epoch_number: u64,
    pub signature: PQCSignature,
    #[serde(default)]
    pub signer_public_key: Vec<u8>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumCertificate {
    pub block_hash: String,
    pub epoch_number: u64,
    pub aggregate_signature: Vec<u8>,
    pub participant_bitmap: Vec<u8>,
    pub cumulative_weight: f64,
    pub validation_quorum_met: bool,
    pub cooperation_quorum_met: bool,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateSignature {
    pub combined_signature: Vec<u8>,
    pub participation_bitmap: Vec<u8>,
    pub message_hash: Vec<u8>,
    pub participant_count: usize,
}

#[derive(Debug)]
pub struct DualQuorumConsensus {
    pub validator_manager: Arc<ValidatorManager>,
    pub pqc_manager: Arc<Mutex<PQCManager>>,
    pub minimum_validator_count: usize,
    pub validation_quorum_threshold: f64,
    pub cooperation_quorum_threshold: f64,
    pub vote_timeout: u64,
    pub block_timeout: u64,
    pub current_epoch: u64,
    pub votes: HashMap<String, Vec<Vote>>, // block_hash -> votes
    pub quorum_certificates: HashMap<String, QuorumCertificate>, // block_hash -> QC
}

impl DualQuorumConsensus {
    pub fn new(
        validator_manager: Arc<ValidatorManager>,
        pqc_manager: Arc<Mutex<PQCManager>>,
        minimum_validator_count: usize,
    ) -> Self {
        DualQuorumConsensus {
            validator_manager,
            pqc_manager,
            minimum_validator_count: minimum_validator_count.max(1),
            validation_quorum_threshold: 0.67,
            cooperation_quorum_threshold: 0.51,
            vote_timeout: 3,
            block_timeout: 5,
            current_epoch: 0,
            votes: HashMap::new(),
            quorum_certificates: HashMap::new(),
        }
    }

    pub fn start_consensus_round(
        &mut self,
        proposed_block: &Block,
    ) -> Result<QuorumCertificate, String> {
        let block_hash = proposed_block.hash.clone();
        let epoch_number = self.current_epoch;

        // Phase 1: Proposal validation
        self.validate_block_proposal(proposed_block)?;

        // Phase 2: Voting
        let votes = self.collect_votes(&block_hash, epoch_number)?;

        // Phase 3: Commitment
        self.check_quorums_and_commit(&block_hash, epoch_number, &votes)
    }

    fn validate_block_proposal(&self, block: &Block) -> Result<(), String> {
        if !Self::is_block_hash_valid(block) {
            return Err("Invalid block hash payload".to_string());
        }

        // Verify leader signature
        let leader = self.get_block_leader(block)?;

        if !self.verify_block_signature(block, &leader) {
            return Err("Invalid block signature".to_string());
        }

        // Verify all transactions in the block
        for tx in &block.transactions {
            self.verify_transaction(tx)?;
        }

        Ok(())
    }

    fn collect_votes(&mut self, block_hash: &str, epoch_number: u64) -> Result<Vec<Vote>, String> {
        let active_validators = self.validator_manager.get_active_validators();
        let mut votes = Vec::new();

        for validator in &active_validators {
            // Simulate vote creation - in real implementation, this would come from network
            let vote = self.create_vote(&validator.address, block_hash, epoch_number)?;
            votes.push(vote);
        }

        self.votes.insert(block_hash.to_string(), votes.clone());
        Ok(votes)
    }

    fn create_vote(
        &self,
        validator_address: &str,
        block_hash: &str,
        epoch_number: u64,
    ) -> Result<Vote, String> {
        let timestamp = Self::current_timestamp();
        let message = format!("{}:{}:{}", validator_address, block_hash, epoch_number);

        let (public_key, private_key) = self.get_or_create_validator_keypair(validator_address)?;

        let mut pqc_manager = self.pqc_manager.lock().unwrap();
        let signature = pqc_manager.sign(&private_key, message.as_bytes())?;

        Ok(Vote {
            validator_address: validator_address.to_string(),
            block_hash: block_hash.to_string(),
            epoch_number,
            signature,
            signer_public_key: public_key.key_data,
            timestamp,
        })
    }

    fn check_quorums_and_commit(
        &mut self,
        block_hash: &str,
        epoch_number: u64,
        votes: &[Vote],
    ) -> Result<QuorumCertificate, String> {
        let cumulative_weight = self.calculate_cumulative_vote_weight(votes);
        let validator_count = votes.len();
        let active_validators = self.validator_manager.get_active_validators();
        let total_validators = active_validators.len();

        if total_validators < self.minimum_validator_count {
            return Err(format!(
                "Insufficient active validators: {} active, {} required",
                total_validators, self.minimum_validator_count
            ));
        }

        if validator_count < self.minimum_validator_count {
            return Err(format!(
                "Insufficient validator votes: {} votes, {} required",
                validator_count, self.minimum_validator_count
            ));
        }

        // Check validation quorum (weighted by synergy score)
        let validation_quorum_met = cumulative_weight > self.validation_quorum_threshold;

        // Check cooperation quorum (simple count)
        let cooperation_quorum_met =
            validator_count as f64 / total_validators as f64 > self.cooperation_quorum_threshold;

        if validation_quorum_met && cooperation_quorum_met {
            // Create quorum certificate
            let qc = self.create_quorum_certificate(block_hash, epoch_number, votes)?;
            self.quorum_certificates
                .insert(block_hash.to_string(), qc.clone());
            Ok(qc)
        } else {
            Err("Quorum thresholds not met".to_string())
        }
    }

    fn calculate_cumulative_vote_weight(&self, votes: &[Vote]) -> f64 {
        let mut total_weight = 0.0;

        for vote in votes {
            if let Some(validator) = self
                .validator_manager
                .get_validator(&vote.validator_address)
            {
                // Use normalized synergy score as vote weight
                total_weight += validator.synergy_score / 100.0;
            }
        }

        total_weight
    }

    fn create_quorum_certificate(
        &self,
        block_hash: &str,
        epoch_number: u64,
        votes: &[Vote],
    ) -> Result<QuorumCertificate, String> {
        // Aggregate signatures
        let aggregate_sig = self.aggregate_signatures(votes)?;

        // Create participation bitmap
        let participant_bitmap = self.create_participant_bitmap(votes);

        // Calculate cumulative weight
        let cumulative_weight = self.calculate_cumulative_vote_weight(votes);

        Ok(QuorumCertificate {
            block_hash: block_hash.to_string(),
            epoch_number,
            aggregate_signature: aggregate_sig.combined_signature,
            participant_bitmap,
            cumulative_weight,
            validation_quorum_met: true,
            cooperation_quorum_met: true,
            timestamp: Self::current_timestamp(),
        })
    }

    fn aggregate_signatures(&self, votes: &[Vote]) -> Result<AggregateSignature, String> {
        // Sort votes by validator address for deterministic ordering
        let mut sorted_votes = votes.to_vec();
        sorted_votes.sort_by(|a, b| a.validator_address.cmp(&b.validator_address));

        // Create participation bitmap
        let participant_bitmap = self.create_participant_bitmap(&sorted_votes);

        // Collect all individual signatures and verify each one before aggregation.
        let mut signatures = Vec::new();

        for vote in &sorted_votes {
            let message = format!(
                "{}:{}:{}",
                vote.validator_address, vote.block_hash, vote.epoch_number
            );
            let public_key = PQCPublicKey {
                algorithm: vote.signature.algorithm.clone(),
                key_data: vote.signer_public_key.clone(),
                key_id: format!("vote_{}", vote.validator_address),
                created_at: vote.timestamp,
            };

            let valid = {
                let pqc_manager = self.pqc_manager.lock().unwrap();
                pqc_manager
                    .verify(&public_key, &vote.signature, message.as_bytes())
                    .map_err(|err| format!("vote signature verify error: {err}"))?
            };

            if !valid {
                return Err(format!(
                    "invalid vote signature from validator {}",
                    vote.validator_address
                ));
            }

            signatures.push(vote.signature.signature_data.clone());
        }

        // Deterministically bind all individual signatures into a compact attestation digest.
        let mut hasher = Sha3_512::new();
        for sig in &signatures {
            hasher.update((sig.len() as u64).to_be_bytes());
            hasher.update(sig);
        }
        let combined_signature = hasher.finalize().to_vec();

        // Use first vote's message hash as common message hash
        let message_hash = if let Some(first_vote) = sorted_votes.first() {
            first_vote.signature.message_hash.clone()
        } else {
            Vec::new()
        };

        Ok(AggregateSignature {
            combined_signature,
            participation_bitmap: participant_bitmap.clone(),
            message_hash,
            participant_count: sorted_votes.len(),
        })
    }

    fn create_participant_bitmap(&self, votes: &[Vote]) -> Vec<u8> {
        let active_validators = self.validator_manager.get_active_validators();
        let mut bitmap = vec![0u8; (active_validators.len() + 7) / 8];

        for (i, validator) in active_validators.iter().enumerate() {
            let byte_index = i / 8;
            let bit_index = i % 8;

            if votes
                .iter()
                .any(|v| v.validator_address == validator.address)
            {
                bitmap[byte_index] |= 1 << bit_index;
            }
        }

        bitmap
    }

    fn get_block_leader(&self, block: &Block) -> Result<String, String> {
        // In PoSy, the leader is determined by the block's validator_id field
        if block.validator_id.is_empty() {
            Err("No validator specified in block".to_string())
        } else {
            Ok(block.validator_id.clone())
        }
    }

    fn get_or_create_validator_keypair(
        &self,
        validator_address: &str,
    ) -> Result<(PQCPublicKey, PQCPrivateKey), String> {
        if let Ok(cache) = EPHEMERAL_VALIDATOR_KEYS.lock() {
            if let Some((public_key, private_key)) = cache.get(validator_address) {
                return Ok((public_key.clone(), private_key.clone()));
            }
        }

        let mut pqc_manager = self.pqc_manager.lock().unwrap();
        let generated = pqc_manager
            .generate_keypair(PQCAlgorithm::FNDSA)
            .map_err(|e| format!("Failed to generate validator keypair: {e}"))?;

        if let Ok(mut cache) = EPHEMERAL_VALIDATOR_KEYS.lock() {
            cache.insert(
                validator_address.to_string(),
                (generated.0.clone(), generated.1.clone()),
            );
        }

        Ok(generated)
    }

    fn verify_block_signature(&self, block: &Block, leader_address: &str) -> bool {
        if block.block_signature.is_empty() || block.proposer_public_key.is_empty() {
            return false;
        }

        let public_key_obj = PQCPublicKey {
            algorithm: PQCAlgorithm::FNDSA,
            key_data: block.proposer_public_key.clone(),
            key_id: format!("block_{}", leader_address),
            created_at: block.timestamp,
        };

        let signature_obj = PQCSignature {
            algorithm: PQCAlgorithm::FNDSA,
            signature_data: block.block_signature.clone(),
            message_hash: block.hash.as_bytes().to_vec(),
            public_key_id: format!("block_{}", leader_address),
            created_at: block.timestamp,
        };

        let pqc_manager = self.pqc_manager.lock().unwrap();
        pqc_manager
            .verify(&public_key_obj, &signature_obj, block.hash.as_bytes())
            .unwrap_or(false)
    }

    fn is_block_hash_valid(block: &Block) -> bool {
        let expected = format!(
            "{:?}{}{}{}{}{}",
            block.block_index,
            block.previous_hash,
            block.validator_id,
            block.nonce,
            block.timestamp,
            block.transactions_root
        );
        blake3::hash(expected.as_bytes()).to_hex().to_string() == block.hash
    }

    fn verify_transaction(&self, _tx: &crate::transaction::Transaction) -> Result<(), String> {
        // Verify transaction signature
        // Verify sender balance
        // Verify nonce
        // Execute contract if applicable

        // Simplified for now
        Ok(())
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

#[derive(Debug, Clone)]
pub struct EntropyBeacon {
    pub current_epoch: u64,
    pub epoch_randomness: Vec<u8>,
    pub previous_qc_hash: String,
    pub nonce: u64,
    pub pqc_manager: Arc<Mutex<PQCManager>>,
    pub mlkem_keypairs: HashMap<u64, (PQCPublicKey, PQCPrivateKey)>, // Store keypairs per epoch
}

impl EntropyBeacon {
    pub fn new(pqc_manager: Arc<Mutex<PQCManager>>) -> Self {
        EntropyBeacon {
            current_epoch: 0,
            epoch_randomness: Vec::new(),
            previous_qc_hash: String::new(),
            nonce: 0,
            pqc_manager,
            mlkem_keypairs: HashMap::new(),
        }
    }

    pub fn generate_epoch_randomness(&mut self, previous_qc: &QuorumCertificate) -> Vec<u8> {
        self.current_epoch += 1;
        self.previous_qc_hash = self.hash_qc(previous_qc);
        self.nonce += 1;

        // Generate ML-KEM shared secret for entropy
        let mut pqc_manager = self.pqc_manager.lock().unwrap();

        // Generate a persistent keypair for this epoch
        let (pub_key, priv_key) = pqc_manager
            .generate_keypair(PQCAlgorithm::MLKEM1024)
            .expect("Failed to generate ML-KEM keypair for epoch");

        // Store the keypair for potential later decapsulation
        self.mlkem_keypairs
            .insert(self.current_epoch, (pub_key.clone(), priv_key.clone()));

        // Encapsulate to get shared secret
        let (_ciphertext, shared_secret) = pqc_manager
            .encapsulate(&pub_key)
            .expect("Failed to encapsulate ML-KEM for epoch randomness");

        // Create entropy input - this uses the shared secret from ML-KEM encapsulation
        let mut input = Vec::new();
        input.extend(&shared_secret.secret); // Shared secret contributes to entropy
        input.extend(self.current_epoch.to_be_bytes());
        input.extend(self.previous_qc_hash.as_bytes());
        input.extend(Self::current_timestamp().to_be_bytes());
        input.extend(self.nonce.to_be_bytes());

        // Hash with SHA3-512 to create the actual epoch randomness
        let mut hasher = Sha3_512::new();
        hasher.update(&input);
        let hash = hasher.finalize();

        // Store the computed randomness
        self.epoch_randomness = hash.to_vec();

        self.epoch_randomness.clone()
    }

    // Method to decapsulate and verify the shared secret (for cross-validation between validators)
    pub fn decapsulate_epoch_randomness(
        &self,
        epoch: u64,
        ciphertext: &PQCCiphertext,
    ) -> Result<Vec<u8>, String> {
        if let Some((_, priv_key)) = self.mlkem_keypairs.get(&epoch) {
            let pqc_manager = self.pqc_manager.lock().unwrap();
            let shared_secret = pqc_manager
                .decapsulate(priv_key, ciphertext)
                .map_err(|e| format!("Failed to decapsulate epoch randomness: {}", e))?;
            Ok(shared_secret.secret)
        } else {
            Err("No keypair found for epoch".to_string())
        }
    }

    fn hash_qc(&self, qc: &QuorumCertificate) -> String {
        let serialized = serde_json::to_string(qc).unwrap_or_default();
        let mut hasher = Sha3_512::new();
        hasher.update(serialized.as_bytes());
        let hash = hasher.finalize();
        hex::encode(hash)
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

#[derive(Debug, Clone)]
pub struct ValidatorRotation {
    pub validator_manager: Arc<ValidatorManager>,
    pub entropy_beacon: Arc<Mutex<EntropyBeacon>>,
    pub target_cluster_size: usize,
}

impl ValidatorRotation {
    pub fn new(
        validator_manager: Arc<ValidatorManager>,
        entropy_beacon: Arc<Mutex<EntropyBeacon>>,
    ) -> Self {
        ValidatorRotation {
            validator_manager,
            entropy_beacon,
            target_cluster_size: 30,
        }
    }

    pub fn rotate_validators(&self) {
        let active_validators = self.validator_manager.get_active_validators();
        let epoch_randomness = self.get_current_epoch_randomness();

        // Calculate number of clusters
        let num_clusters =
            (active_validators.len() as f64 / self.target_cluster_size as f64).ceil() as usize;

        // Assign validators to clusters using deterministic randomness
        for (i, validator) in active_validators.iter().enumerate() {
            let cluster_id =
                self.assign_to_cluster(&validator.address, &epoch_randomness, num_clusters);
            // Update validator's cluster assignment
        }
    }

    fn assign_to_cluster(
        &self,
        validator_address: &str,
        epoch_randomness: &[u8],
        num_clusters: usize,
    ) -> usize {
        // Create hash of epoch_randomness + validator_address
        let mut hasher = Sha3_512::new();
        hasher.update(epoch_randomness);
        hasher.update(validator_address.as_bytes());
        let hash = hasher.finalize();

        // Use first 8 bytes as cluster assignment
        let cluster_hash = u64::from_be_bytes(hash[..8].try_into().unwrap());
        (cluster_hash % num_clusters as u64) as usize
    }

    fn get_current_epoch_randomness(&self) -> Vec<u8> {
        let beacon = self.entropy_beacon.lock().unwrap();
        beacon.epoch_randomness.clone()
    }
}
