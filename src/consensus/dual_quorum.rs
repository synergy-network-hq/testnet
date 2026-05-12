use crate::block::Block;
use crate::crypto::pqc::{
    PQCAlgorithm, PQCCiphertext, PQCManager, PQCPrivateKey, PQCPublicKey, PQCSignature,
};
use crate::validator::{
    consensus_membership_validators, Validator, ValidatorManager, ValidatorPerformanceUpdate,
    TESTNET_BETA_VALIDATOR_CLUSTER_SIZE, VALIDATOR_MANAGER,
};
use crate::{debug, warn};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_512};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

lazy_static::lazy_static! {
    static ref EPHEMERAL_VALIDATOR_KEYS: Arc<Mutex<HashMap<String, (PQCPublicKey, PQCPrivateKey)>>> =
        Arc::new(Mutex::new(HashMap::new()));
    static ref NETWORK_VOTE_MAILBOX: Arc<Mutex<HashMap<String, Vec<Vote>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    static ref OBSERVED_VOTES: Arc<Mutex<HashMap<String, Vote>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref EQUIVOCATION_EVIDENCE_LOG: Arc<Mutex<HashMap<String, VoteEquivocationEvidence>>> =
        Arc::new(Mutex::new(HashMap::new()));
    static ref PROCESSED_EQUIVOCATION_EVIDENCE: Arc<Mutex<BTreeSet<String>>> =
        Arc::new(Mutex::new(BTreeSet::new()));
    static ref LOCAL_VOTE_LOCK_FILE_MUTEX: Mutex<()> = Mutex::new(());
}

const TWO_THIRDS_QUORUM_THRESHOLD: f64 = 2.0 / 3.0;
const QUORUM_COMPARISON_EPSILON: f64 = 0.000_000_001;

#[cfg(test)]
lazy_static::lazy_static! {
    static ref TEST_LOCAL_VOTE_LOCK_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub validator_address: String,
    pub block_hash: String,
    #[serde(default)]
    pub block_index: u64,
    pub epoch_number: u64,
    #[serde(default)]
    pub round_number: u64,
    pub signature: PQCSignature,
    #[serde(default)]
    pub signer_public_key: Vec<u8>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteEquivocationEvidence {
    pub validator_address: String,
    pub epoch_number: u64,
    pub block_index: u64,
    pub round_number: u64,
    pub first_vote: Vote,
    pub conflicting_vote: Vote,
    pub detected_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocalVoteLock {
    validator_address: String,
    block_hash: String,
    block_index: u64,
    epoch_number: u64,
    first_round_number: u64,
    latest_round_number: u64,
    proposer: String,
    created_at: u64,
    updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumCertificate {
    pub block_hash: String,
    pub epoch_number: u64,
    pub round_number: u64,
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
    pub penalization_enabled: bool,
    pub minimum_validator_count: usize,
    pub validator_vote_threshold: usize,
    pub validation_quorum_threshold: f64,
    pub cooperation_quorum_threshold: f64,
    pub vote_timeout: u64,
    pub block_timeout: u64,
    pub current_epoch: u64,
    pub current_round_by_height: HashMap<u64, u64>,
    pub votes: HashMap<String, Vec<Vote>>, // block_hash -> votes
    pub quorum_certificates: HashMap<String, QuorumCertificate>, // block_hash -> QC
    verified_vote_signatures: Mutex<HashSet<String>>,
}

impl DualQuorumConsensus {
    pub fn new(
        validator_manager: Arc<ValidatorManager>,
        pqc_manager: Arc<Mutex<PQCManager>>,
        penalization_enabled: bool,
        minimum_validator_count: usize,
        validator_vote_threshold: usize,
        vote_timeout_secs: u64,
        block_timeout_secs: u64,
    ) -> Self {
        DualQuorumConsensus {
            validator_manager,
            pqc_manager,
            penalization_enabled,
            minimum_validator_count: minimum_validator_count.max(1),
            validator_vote_threshold: validator_vote_threshold.max(1),
            validation_quorum_threshold: TWO_THIRDS_QUORUM_THRESHOLD,
            cooperation_quorum_threshold: 0.51,
            vote_timeout: vote_timeout_secs.max(1),
            block_timeout: block_timeout_secs.max(1),
            current_epoch: 0,
            current_round_by_height: HashMap::new(),
            votes: HashMap::new(),
            quorum_certificates: HashMap::new(),
            verified_vote_signatures: Mutex::new(HashSet::new()),
        }
    }

    pub fn start_consensus_round(
        &mut self,
        proposed_block: &Block,
        minimum_round_number: u64,
    ) -> Result<QuorumCertificate, String> {
        let block_hash = proposed_block.hash.clone();
        let epoch_number = self.current_epoch;
        let local_validator_address = self
            .resolve_local_validator_address_for_round()
            .ok_or_else(|| "Local validator address is not configured".to_string())?;
        let round_number = self.allocate_round_number(
            proposed_block.block_index,
            epoch_number,
            &local_validator_address,
            minimum_round_number,
        );

        // Phase 1: Proposal validation
        self.validate_block_proposal(proposed_block)?;

        // Phase 2: Voting
        let votes = self.collect_votes(proposed_block, &block_hash, epoch_number, round_number)?;

        // Phase 3: Commitment
        self.check_quorums_and_commit(&block_hash, epoch_number, round_number, &votes)
    }

    fn validate_block_proposal(&self, block: &Block) -> Result<(), String> {
        Self::validate_block_proposal_static(block)
    }

    pub fn validate_block_proposal_static(block: &Block) -> Result<(), String> {
        if !Self::is_block_hash_valid(block) {
            return Err("Invalid block hash payload".to_string());
        }

        // Verify leader signature
        let leader = Self::get_block_leader_static(block)?;

        if !Self::verify_block_signature_static(block, &leader) {
            return Err("Invalid block signature".to_string());
        }

        // Verify all transactions in the block
        for tx in &block.transactions {
            Self::verify_transaction_static(tx)?;
        }

        Ok(())
    }

    fn collect_votes(
        &mut self,
        proposed_block: &Block,
        block_hash: &str,
        epoch_number: u64,
        round_number: u64,
    ) -> Result<Vec<Vote>, String> {
        let active_validators = self.collect_live_validators();
        if active_validators.len() < self.minimum_validator_count {
            return Err(format!(
                "Insufficient live validators: {} active on the network, {} required",
                active_validators.len(),
                self.minimum_validator_count
            ));
        }

        let expected_validators = active_validators
            .iter()
            .map(|validator| validator.address.clone())
            .collect::<BTreeSet<_>>();
        let local_validator_address = self
            .resolve_local_validator_address_for_round()
            .ok_or_else(|| "Local validator address is not configured".to_string())?;
        if !expected_validators.contains(&local_validator_address) {
            return Err(format!(
                "Local validator {} is not eligible for this consensus round",
                local_validator_address
            ));
        }

        Self::register_local_vote_intent(
            &local_validator_address,
            proposed_block,
            epoch_number,
            round_number,
        )?;
        let local_vote = Self::create_vote_for_validator(
            &local_validator_address,
            proposed_block,
            epoch_number,
            round_number,
        )?;
        self.register_local_vote_or_slash(&local_vote)?;
        let mut votes = vec![local_vote];

        Self::reset_network_vote_mailbox(block_hash, epoch_number, round_number);

        let remote_validators = expected_validators
            .iter()
            .filter(|address| *address != &local_validator_address)
            .count();
        if remote_validators > 0 {
            if let Some(network) = crate::p2p::get_p2p_network() {
                let notified =
                    network.broadcast_vote_request(proposed_block, epoch_number, round_number);
                debug!(
                    "consensus",
                    "Broadcasted vote request",
                    "block_hash" => block_hash.to_string(),
                    "epoch" => epoch_number,
                    "round" => round_number,
                    "remote_validators" => remote_validators as u64,
                    "notified_peers" => notified as u64
                );
            } else {
                warn!(
                    "consensus",
                    "Consensus round has remote validators but no active P2P network",
                    "block_hash" => block_hash.to_string(),
                    "epoch" => epoch_number,
                    "round" => round_number
                );
            }
        }

        let deadline = Instant::now() + Duration::from_secs(self.vote_timeout.max(1));
        while Instant::now() < deadline {
            self.apply_recorded_equivocations();
            votes.retain(|vote| self.vote_is_eligible(vote));

            let pending_votes =
                Self::snapshot_network_votes(block_hash, epoch_number, round_number);
            self.merge_remote_votes(
                &mut votes,
                &expected_validators,
                block_hash,
                epoch_number,
                round_number,
                pending_votes,
            );

            if self.has_commit_quorum(&active_validators, &votes) {
                break;
            }

            thread::sleep(Duration::from_millis(100));
        }

        // Drain the mailbox one final time after the wait window closes so votes
        // that arrive right on the timeout edge still count toward this round.
        let pending_votes = Self::snapshot_network_votes(block_hash, epoch_number, round_number);
        self.merge_remote_votes(
            &mut votes,
            &expected_validators,
            block_hash,
            epoch_number,
            round_number,
            pending_votes,
        );

        self.apply_recorded_equivocations();
        votes.retain(|vote| self.vote_is_eligible(vote));
        self.record_vote_participation(&votes);
        if self.penalization_enabled {
            self.record_missed_vote_timeouts(&active_validators, &votes);
        }

        Self::reset_network_vote_mailbox(block_hash, epoch_number, round_number);
        self.votes.insert(block_hash.to_string(), votes.clone());
        Ok(votes)
    }

    pub fn build_local_vote_for_proposal(
        proposed_block: &Block,
        epoch_number: u64,
        round_number: u64,
    ) -> Result<Vote, String> {
        Self::validate_block_proposal_static(proposed_block)?;

        let local_validator_address = Self::resolve_local_validator_address()
            .ok_or_else(|| "Local validator address is not configured for voting".to_string())?;
        let local_validator_is_active =
            consensus_membership_validators(VALIDATOR_MANAGER.get_active_validators())
                .into_iter()
                .any(|validator| validator.address == local_validator_address);
        if !local_validator_is_active {
            return Err(format!(
                "Local validator {} is not an active consensus validator",
                local_validator_address
            ));
        }

        Self::register_local_vote_intent(
            &local_validator_address,
            proposed_block,
            epoch_number,
            round_number,
        )?;
        let vote = Self::create_vote_for_validator(
            &local_validator_address,
            proposed_block,
            epoch_number,
            round_number,
        )?;
        if let Some(evidence) = Self::register_local_vote_attempt(&vote) {
            return Err(format!(
                "Refusing to double-sign for validator {} at height {} in epoch {} round {}",
                evidence.validator_address,
                evidence.block_index,
                evidence.epoch_number,
                evidence.round_number
            ));
        }

        Ok(vote)
    }

    pub fn record_network_vote(vote: Vote) {
        if vote.validator_address.trim().is_empty() || vote.block_hash.trim().is_empty() {
            return;
        }

        if Self::register_vote_observation(&vote).is_some() {
            return;
        }

        let key = Self::vote_mailbox_key(&vote.block_hash, vote.epoch_number, vote.round_number);
        if let Ok(mut mailbox) = NETWORK_VOTE_MAILBOX.lock() {
            let entry = mailbox.entry(key).or_default();
            if entry
                .iter()
                .all(|existing| existing.validator_address != vote.validator_address)
            {
                entry.push(vote);
            }
        }
    }

    fn create_vote_for_validator(
        validator_address: &str,
        proposed_block: &Block,
        epoch_number: u64,
        round_number: u64,
    ) -> Result<Vote, String> {
        let timestamp = Self::current_timestamp();
        let message = Self::vote_signature_payload(
            validator_address,
            &proposed_block.hash,
            proposed_block.block_index,
            epoch_number,
            round_number,
        );

        let (public_key, private_key) = Self::get_or_create_validator_keypair(validator_address)?;

        let mut pqc_manager = PQCManager::new();
        let signature = pqc_manager.sign(&private_key, message.as_bytes())?;

        Ok(Vote {
            validator_address: validator_address.to_string(),
            block_hash: proposed_block.hash.clone(),
            block_index: proposed_block.block_index,
            epoch_number,
            round_number,
            signature,
            signer_public_key: public_key.key_data,
            timestamp,
        })
    }

    fn merge_remote_votes(
        &self,
        votes: &mut Vec<Vote>,
        expected_validators: &BTreeSet<String>,
        block_hash: &str,
        epoch_number: u64,
        round_number: u64,
        pending_votes: Vec<Vote>,
    ) {
        let mut seen_validators = votes
            .iter()
            .map(|vote| vote.validator_address.clone())
            .collect::<BTreeSet<_>>();
        let mut cached_votes = Vec::new();
        let mut uncached_votes = Vec::new();

        for vote in pending_votes {
            if vote.block_hash != block_hash
                || vote.epoch_number != epoch_number
                || vote.round_number != round_number
            {
                continue;
            }
            if Self::has_equivocation_evidence(
                &vote.validator_address,
                vote.epoch_number,
                vote.block_index,
                vote.round_number,
            ) {
                warn!(
                    "consensus",
                    "Discarding equivocating vote",
                    "validator" => vote.validator_address.clone(),
                    "block_hash" => vote.block_hash.clone(),
                    "height" => vote.block_index,
                    "epoch" => vote.epoch_number,
                    "round" => vote.round_number
                );
                continue;
            }
            if !expected_validators.contains(&vote.validator_address) {
                continue;
            }
            if !self.vote_is_eligible(&vote) {
                continue;
            }
            if !seen_validators.insert(vote.validator_address.clone()) {
                continue;
            }

            let cache_key = Self::vote_signature_cache_key(&vote);
            if self.vote_signature_cache_contains(&cache_key) {
                cached_votes.push(vote);
            } else {
                uncached_votes.push((vote, cache_key));
            }
        }

        votes.extend(cached_votes);

        let mut handles = Vec::new();
        for (vote, cache_key) in uncached_votes {
            handles.push(Self::spawn_vote_signature_verification_with_key(
                vote, cache_key,
            ));
        }

        for handle in handles {
            let Ok((vote, cache_key, verification)) = handle.join() else {
                warn!(
                    "consensus",
                    "Remote vote verification worker panicked",
                    "block_hash" => block_hash.to_string(),
                    "epoch" => epoch_number,
                    "round" => round_number
                );
                continue;
            };

            if let Err(error) = verification {
                warn!(
                    "consensus",
                    "Discarding invalid remote vote",
                    "validator" => vote.validator_address.clone(),
                    "block_hash" => vote.block_hash.clone(),
                    "epoch" => vote.epoch_number,
                    "round" => vote.round_number,
                    "error" => error
                );
                continue;
            }

            self.cache_verified_vote_signature(cache_key);
            votes.push(vote);
        }
    }

    fn spawn_vote_signature_verification_with_key(
        vote: Vote,
        cache_key: String,
    ) -> thread::JoinHandle<(Vote, String, Result<(), String>)> {
        thread::spawn(move || {
            let verification = Self::verify_vote_signature_uncached(&vote);
            (vote, cache_key, verification)
        })
    }

    fn check_quorums_and_commit(
        &mut self,
        block_hash: &str,
        epoch_number: u64,
        round_number: u64,
        votes: &[Vote],
    ) -> Result<QuorumCertificate, String> {
        let cumulative_weight = self.calculate_cumulative_vote_weight(votes);
        let validator_count = votes.len();
        let active_validators = self.collect_live_validators();
        let total_validators = active_validators.len();

        if total_validators < self.minimum_validator_count {
            return Err(format!(
                "Insufficient active validators: {} active, {} required",
                total_validators, self.minimum_validator_count
            ));
        }

        let required_validator_votes = self.required_validator_votes(total_validators);
        if validator_count < required_validator_votes {
            return Err(format!(
                "Insufficient validator votes: {} votes, {} required for quorum",
                validator_count, required_validator_votes
            ));
        }

        // Check validation quorum against the total live validator weight for the round.
        let total_live_weight = self.total_validator_weight(&active_validators);
        let validation_ratio = if total_live_weight > 0.0 {
            cumulative_weight / total_live_weight
        } else {
            0.0
        };
        let validation_quorum_met =
            validation_ratio + QUORUM_COMPARISON_EPSILON >= self.validation_quorum_threshold;

        // Check cooperation quorum using a BFT-style supermajority count.
        let cooperation_ratio = validator_count as f64 / total_validators as f64;
        let cooperation_quorum_met = cooperation_ratio + QUORUM_COMPARISON_EPSILON
            >= self.cooperation_quorum_threshold
            && validator_count >= required_validator_votes;

        if validation_quorum_met && cooperation_quorum_met {
            // Create quorum certificate
            let qc =
                self.create_quorum_certificate(block_hash, epoch_number, round_number, votes)?;
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

    fn total_validator_weight(&self, validators: &[Validator]) -> f64 {
        validators
            .iter()
            .map(|validator| (validator.synergy_score / 100.0).max(0.0))
            .sum()
    }

    fn required_validator_votes(&self, total_validators: usize) -> usize {
        self.validator_vote_threshold
            .max(1)
            .min(total_validators.max(1))
    }

    fn has_commit_quorum(&self, live_validators: &[Validator], votes: &[Vote]) -> bool {
        if live_validators.is_empty() {
            return false;
        }

        let required_validator_votes = self.required_validator_votes(live_validators.len());
        if votes.len() < required_validator_votes {
            return false;
        }

        let total_live_weight = self.total_validator_weight(live_validators);
        if total_live_weight <= 0.0 {
            return false;
        }

        let cumulative_weight = self.calculate_cumulative_vote_weight(votes);
        (cumulative_weight / total_live_weight) + QUORUM_COMPARISON_EPSILON
            >= self.validation_quorum_threshold
    }

    fn record_missed_vote_timeouts(&self, live_validators: &[Validator], votes: &[Vote]) {
        if !self.penalization_enabled {
            return;
        }

        let received_votes = votes
            .iter()
            .map(|vote| vote.validator_address.clone())
            .collect::<BTreeSet<_>>();

        for validator in live_validators {
            if received_votes.contains(&validator.address) {
                continue;
            }

            self.validator_manager
                .update_performance(ValidatorPerformanceUpdate {
                    validator_address: validator.address.clone(),
                    update_type: "block_missed".to_string(),
                    value: None,
                    timestamp: Self::current_timestamp(),
                });

            warn!(
                "consensus",
                "Validator missed vote deadline",
                "validator" => validator.address.clone()
            );
        }
    }

    fn record_vote_participation(&self, votes: &[Vote]) {
        for vote in votes {
            self.validator_manager
                .update_performance(ValidatorPerformanceUpdate {
                    validator_address: vote.validator_address.clone(),
                    update_type: "vote_cast".to_string(),
                    value: None,
                    timestamp: Self::current_timestamp(),
                });
        }
    }

    fn create_quorum_certificate(
        &self,
        block_hash: &str,
        epoch_number: u64,
        round_number: u64,
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
            round_number,
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
            self.verify_vote_signature(vote)?;
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
        let active_validators = self.collect_live_validators();
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

    fn get_block_leader_static(block: &Block) -> Result<String, String> {
        // In PoSy, the leader is determined by the block's validator_id field
        if block.validator_id.is_empty() {
            Err("No validator specified in block".to_string())
        } else {
            Ok(block.validator_id.clone())
        }
    }

    fn collect_live_validators(&self) -> Vec<Validator> {
        let active_validators =
            consensus_membership_validators(self.validator_manager.get_active_validators());
        let active_by_address = active_validators
            .into_iter()
            .map(|validator| (validator.address.clone(), validator))
            .collect::<HashMap<_, _>>();

        let mut live_addresses = BTreeSet::new();

        if let Some(local_validator_address) = self.resolve_local_validator_address_for_round() {
            if active_by_address.contains_key(&local_validator_address) {
                live_addresses.insert(local_validator_address);
            }
        }

        if let Some(network) = crate::p2p::get_p2p_network() {
            for validator_address in network.get_status_ready_validator_addresses() {
                if active_by_address.contains_key(&validator_address) {
                    live_addresses.insert(validator_address);
                }
            }
        }

        live_addresses
            .into_iter()
            .filter_map(|address| active_by_address.get(&address).cloned())
            .collect()
    }

    fn resolve_local_validator_address() -> Option<String> {
        let from_env = crate::config::resolve_runtime_validator_address();

        if from_env.is_some() {
            return from_env;
        }

        let active_validators =
            consensus_membership_validators(VALIDATOR_MANAGER.get_active_validators());
        if active_validators.len() == 1 {
            active_validators
                .first()
                .map(|validator| validator.address.clone())
        } else {
            None
        }
    }

    fn resolve_local_validator_address_for_round(&self) -> Option<String> {
        Self::resolve_local_validator_address().or_else(|| {
            let active_validators =
                consensus_membership_validators(self.validator_manager.get_active_validators());
            if active_validators.len() == 1 {
                active_validators
                    .first()
                    .map(|validator| validator.address.clone())
            } else {
                None
            }
        })
    }

    fn get_or_create_validator_keypair(
        validator_address: &str,
    ) -> Result<(PQCPublicKey, PQCPrivateKey), String> {
        if let Ok(cache) = EPHEMERAL_VALIDATOR_KEYS.lock() {
            if let Some((public_key, private_key)) = cache.get(validator_address) {
                return Ok((public_key.clone(), private_key.clone()));
            }
        }

        let mut pqc_manager = PQCManager::new();
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

    fn verify_block_signature_static(block: &Block, leader_address: &str) -> bool {
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

        let pqc_manager = PQCManager::new();
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

    fn verify_transaction_static(_tx: &crate::transaction::Transaction) -> Result<(), String> {
        // Verify transaction signature
        // Verify sender balance
        // Verify nonce
        // Execute contract if applicable

        // Simplified for now
        Ok(())
    }

    fn verify_vote_signature(&self, vote: &Vote) -> Result<(), String> {
        let cache_key = Self::vote_signature_cache_key(vote);
        if self.vote_signature_cache_contains(&cache_key) {
            return Ok(());
        }

        Self::verify_vote_signature_uncached(vote)?;
        self.cache_verified_vote_signature(cache_key);
        Ok(())
    }

    fn verify_vote_signature_uncached(vote: &Vote) -> Result<(), String> {
        let message = Self::vote_signature_payload(
            &vote.validator_address,
            &vote.block_hash,
            vote.block_index,
            vote.epoch_number,
            vote.round_number,
        );
        let public_key = PQCPublicKey {
            algorithm: vote.signature.algorithm.clone(),
            key_data: vote.signer_public_key.clone(),
            key_id: format!("vote_{}", vote.validator_address),
            created_at: vote.timestamp,
        };

        let pqc_manager = PQCManager::new();
        let valid = pqc_manager
            .verify(&public_key, &vote.signature, message.as_bytes())
            .map_err(|err| format!("vote signature verify error: {err}"))?;

        if valid {
            Ok(())
        } else {
            Err(format!(
                "invalid vote signature from validator {}",
                vote.validator_address
            ))
        }
    }

    fn vote_signature_cache_contains(&self, cache_key: &str) -> bool {
        self.verified_vote_signatures
            .lock()
            .map(|cache| cache.contains(cache_key))
            .unwrap_or(false)
    }

    fn cache_verified_vote_signature(&self, cache_key: String) {
        if let Ok(mut cache) = self.verified_vote_signatures.lock() {
            if cache.len() > 8192 {
                cache.clear();
            }
            cache.insert(cache_key);
        }
    }

    fn vote_signature_cache_key(vote: &Vote) -> String {
        let mut hasher = Sha3_512::new();
        hasher.update(vote.validator_address.as_bytes());
        hasher.update(vote.block_hash.as_bytes());
        hasher.update(vote.block_index.to_be_bytes());
        hasher.update(vote.epoch_number.to_be_bytes());
        hasher.update(vote.round_number.to_be_bytes());
        hasher.update(format!("{:?}", vote.signature.algorithm).as_bytes());
        hasher.update((vote.signer_public_key.len() as u64).to_be_bytes());
        hasher.update(&vote.signer_public_key);
        hasher.update((vote.signature.signature_data.len() as u64).to_be_bytes());
        hasher.update(&vote.signature.signature_data);
        hex::encode(hasher.finalize())
    }

    fn vote_signature_payload(
        validator_address: &str,
        block_hash: &str,
        block_index: u64,
        epoch_number: u64,
        round_number: u64,
    ) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            validator_address, block_index, round_number, block_hash, epoch_number
        )
    }

    fn local_vote_lock_key(validator_address: &str, epoch_number: u64, block_index: u64) -> String {
        format!("{epoch_number}:{block_index}:{validator_address}")
    }

    fn local_vote_lock_path() -> PathBuf {
        #[cfg(test)]
        {
            if let Ok(path) = TEST_LOCAL_VOTE_LOCK_PATH.lock() {
                if let Some(path) = path.clone() {
                    return path;
                }
            }
        }

        crate::utils::resolve_data_path("data/consensus_vote_locks.json")
    }

    fn load_local_vote_locks_unlocked() -> Result<HashMap<String, LocalVoteLock>, String> {
        let path = Self::local_vote_lock_path();
        if !path.exists() {
            return Ok(HashMap::new());
        }

        let data = fs::read(&path)
            .map_err(|err| format!("failed to read local vote lock file {:?}: {err}", path))?;
        if data.is_empty() {
            return Ok(HashMap::new());
        }

        serde_json::from_slice::<HashMap<String, LocalVoteLock>>(&data)
            .map_err(|err| format!("failed to parse local vote lock file {:?}: {err}", path))
    }

    fn persist_local_vote_locks_unlocked(
        locks: &HashMap<String, LocalVoteLock>,
    ) -> Result<(), String> {
        let path = Self::local_vote_lock_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("failed to create local vote lock directory: {err}"))?;
        }

        let tmp_path = path.with_extension("json.tmp");
        let serialized = serde_json::to_vec_pretty(locks)
            .map_err(|err| format!("failed to encode local vote locks: {err}"))?;

        let mut options = OpenOptions::new();
        options.create(true).truncate(true).write(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options
            .open(&tmp_path)
            .map_err(|err| format!("failed to open local vote lock temp file: {err}"))?;
        file.write_all(&serialized)
            .map_err(|err| format!("failed to write local vote lock temp file: {err}"))?;
        file.sync_all()
            .map_err(|err| format!("failed to sync local vote lock temp file: {err}"))?;
        drop(file);

        fs::rename(&tmp_path, &path)
            .map_err(|err| format!("failed to replace local vote lock file: {err}"))
    }

    fn register_local_vote_intent(
        validator_address: &str,
        proposed_block: &Block,
        epoch_number: u64,
        round_number: u64,
    ) -> Result<(), String> {
        let _guard = LOCAL_VOTE_LOCK_FILE_MUTEX
            .lock()
            .map_err(|_| "local vote lock file mutex is poisoned".to_string())?;
        let mut locks = Self::load_local_vote_locks_unlocked()?;
        let key =
            Self::local_vote_lock_key(validator_address, epoch_number, proposed_block.block_index);
        let now = Self::current_timestamp();

        if let Some(existing) = locks.get_mut(&key) {
            if existing.block_hash == proposed_block.hash {
                existing.latest_round_number = existing.latest_round_number.max(round_number);
                existing.updated_at = now;
                Self::persist_local_vote_locks_unlocked(&locks)?;
                return Ok(());
            }

            if round_number > existing.latest_round_number {
                warn!(
                    "consensus",
                    "Advancing local vote lock to higher-round proposal",
                    "validator" => validator_address.to_string(),
                    "height" => proposed_block.block_index,
                    "epoch" => epoch_number,
                    "previous_hash" => existing.block_hash.clone(),
                    "previous_proposer" => existing.proposer.clone(),
                    "previous_latest_round" => existing.latest_round_number,
                    "next_hash" => proposed_block.hash.clone(),
                    "next_proposer" => proposed_block.validator_id.clone(),
                    "next_round" => round_number
                );
                existing.block_hash = proposed_block.hash.clone();
                existing.proposer = proposed_block.validator_id.clone();
                existing.latest_round_number = round_number;
                existing.updated_at = now;
                Self::persist_local_vote_locks_unlocked(&locks)?;
                return Ok(());
            }

            return Err(format!(
                "already locally voted for different block at height {}: locked_hash={}, locked_proposer={}, locked_epoch={}, locked_first_round={}, locked_latest_round={}, requested_hash={}, requested_proposer={}, requested_epoch={}, requested_round={}",
                proposed_block.block_index,
                existing.block_hash,
                existing.proposer,
                existing.epoch_number,
                existing.first_round_number,
                existing.latest_round_number,
                proposed_block.hash,
                proposed_block.validator_id,
                epoch_number,
                round_number
            ));
        }

        locks.insert(
            key,
            LocalVoteLock {
                validator_address: validator_address.to_string(),
                block_hash: proposed_block.hash.clone(),
                block_index: proposed_block.block_index,
                epoch_number,
                first_round_number: round_number,
                latest_round_number: round_number,
                proposer: proposed_block.validator_id.clone(),
                created_at: now,
                updated_at: now,
            },
        );

        Self::persist_local_vote_locks_unlocked(&locks)
    }

    fn vote_observation_key(
        validator_address: &str,
        epoch_number: u64,
        block_index: u64,
        round_number: u64,
    ) -> String {
        format!("{epoch_number}:{block_index}:{round_number}:{validator_address}")
    }

    fn observe_vote(
        vote: &Vote,
        persist_equivocation_evidence: bool,
    ) -> Option<VoteEquivocationEvidence> {
        let key = Self::vote_observation_key(
            &vote.validator_address,
            vote.epoch_number,
            vote.block_index,
            vote.round_number,
        );

        let mut observed_votes = OBSERVED_VOTES.lock().ok()?;
        match observed_votes.get(&key) {
            // Idempotent replays of the exact same vote are allowed.
            Some(existing) if existing.block_hash == vote.block_hash => None,
            Some(existing) => {
                let evidence = VoteEquivocationEvidence {
                    validator_address: vote.validator_address.clone(),
                    block_index: vote.block_index,
                    epoch_number: vote.epoch_number,
                    round_number: vote.round_number,
                    first_vote: existing.clone(),
                    conflicting_vote: vote.clone(),
                    detected_at: Self::current_timestamp(),
                };

                if persist_equivocation_evidence {
                    if let Ok(mut evidence_log) = EQUIVOCATION_EVIDENCE_LOG.lock() {
                        evidence_log.insert(key, evidence.clone());
                    }
                }

                Some(evidence)
            }
            None => {
                observed_votes.insert(key, vote.clone());
                None
            }
        }
    }

    fn register_vote_observation(vote: &Vote) -> Option<VoteEquivocationEvidence> {
        Self::observe_vote(vote, true)
    }

    fn register_local_vote_attempt(vote: &Vote) -> Option<VoteEquivocationEvidence> {
        Self::observe_vote(vote, false)
    }

    fn has_equivocation_evidence(
        validator_address: &str,
        epoch_number: u64,
        block_index: u64,
        round_number: u64,
    ) -> bool {
        let key =
            Self::vote_observation_key(validator_address, epoch_number, block_index, round_number);
        EQUIVOCATION_EVIDENCE_LOG
            .lock()
            .ok()
            .map(|log| log.contains_key(&key))
            .unwrap_or(false)
    }

    fn register_local_vote_or_slash(&self, vote: &Vote) -> Result<(), String> {
        if let Some(evidence) = Self::register_local_vote_attempt(vote) {
            return Err(format!(
                "Validator {} attempted conflicting votes at height {} in epoch {} round {}",
                evidence.validator_address,
                evidence.block_index,
                evidence.epoch_number,
                evidence.round_number
            ));
        }

        Ok(())
    }

    fn pending_equivocation_evidence(&self) -> Vec<VoteEquivocationEvidence> {
        let processed = PROCESSED_EQUIVOCATION_EVIDENCE
            .lock()
            .ok()
            .map(|entries| entries.clone())
            .unwrap_or_default();

        EQUIVOCATION_EVIDENCE_LOG
            .lock()
            .ok()
            .map(|entries| {
                entries
                    .iter()
                    .filter(|(key, _)| !processed.contains(*key))
                    .map(|(_, evidence)| evidence.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn apply_recorded_equivocations(&self) {
        for evidence in self.pending_equivocation_evidence() {
            self.apply_equivocation_evidence(&evidence);
        }
    }

    fn apply_equivocation_evidence(&self, evidence: &VoteEquivocationEvidence) {
        let key = Self::vote_observation_key(
            &evidence.validator_address,
            evidence.epoch_number,
            evidence.block_index,
            evidence.round_number,
        );

        let should_process = if let Ok(mut processed) = PROCESSED_EQUIVOCATION_EVIDENCE.lock() {
            processed.insert(key)
        } else {
            false
        };
        if !should_process {
            return;
        }

        match self
            .validator_manager
            .slash_validator(&evidence.validator_address, "double_sign")
        {
            Ok(_) => {
                warn!(
                    "consensus",
                    "Slashed validator for vote equivocation",
                    "validator" => evidence.validator_address.clone(),
                    "height" => evidence.block_index,
                    "epoch" => evidence.epoch_number,
                    "round" => evidence.round_number
                );
            }
            Err(error) => {
                warn!(
                    "consensus",
                    "Failed to slash equivocating validator",
                    "validator" => evidence.validator_address.clone(),
                    "height" => evidence.block_index,
                    "epoch" => evidence.epoch_number,
                    "round" => evidence.round_number,
                    "error" => error
                );
            }
        }
    }

    fn vote_is_eligible(&self, vote: &Vote) -> bool {
        if Self::has_equivocation_evidence(
            &vote.validator_address,
            vote.epoch_number,
            vote.block_index,
            vote.round_number,
        ) {
            return false;
        }

        consensus_membership_validators(self.validator_manager.get_active_validators())
            .into_iter()
            .any(|validator| validator.address == vote.validator_address)
    }

    fn vote_mailbox_key(block_hash: &str, epoch_number: u64, round_number: u64) -> String {
        format!("{epoch_number}:{round_number}:{block_hash}")
    }

    fn reset_network_vote_mailbox(block_hash: &str, epoch_number: u64, round_number: u64) {
        let key = Self::vote_mailbox_key(block_hash, epoch_number, round_number);
        if let Ok(mut mailbox) = NETWORK_VOTE_MAILBOX.lock() {
            mailbox.remove(&key);
        }
    }

    fn snapshot_network_votes(block_hash: &str, epoch_number: u64, round_number: u64) -> Vec<Vote> {
        let key = Self::vote_mailbox_key(block_hash, epoch_number, round_number);
        NETWORK_VOTE_MAILBOX
            .lock()
            .ok()
            .and_then(|mailbox| mailbox.get(&key).cloned())
            .unwrap_or_default()
    }

    fn latest_observed_round_for_validator(
        validator_address: &str,
        epoch_number: u64,
        block_index: u64,
    ) -> u64 {
        OBSERVED_VOTES
            .lock()
            .ok()
            .and_then(|observed_votes| {
                observed_votes
                    .values()
                    .filter(|vote| {
                        vote.validator_address == validator_address
                            && vote.epoch_number == epoch_number
                            && vote.block_index == block_index
                    })
                    .map(|vote| vote.round_number)
                    .max()
            })
            .unwrap_or(0)
    }

    fn allocate_round_number(
        &mut self,
        block_index: u64,
        epoch_number: u64,
        validator_address: &str,
        minimum_round_number: u64,
    ) -> u64 {
        let next_round = self.current_round_by_height.entry(block_index).or_insert(0);
        let requested_floor = minimum_round_number.saturating_sub(1);
        let observed_floor =
            Self::latest_observed_round_for_validator(validator_address, epoch_number, block_index);
        *next_round = (*next_round).max(requested_floor).max(observed_floor);
        *next_round += 1;
        *next_round
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    #[cfg(test)]
    fn reset_test_vote_tracking() {
        if let Ok(mut mailbox) = NETWORK_VOTE_MAILBOX.lock() {
            mailbox.clear();
        }
        if let Ok(mut observed) = OBSERVED_VOTES.lock() {
            observed.clear();
        }
        if let Ok(mut evidence) = EQUIVOCATION_EVIDENCE_LOG.lock() {
            evidence.clear();
        }
        if let Ok(mut processed) = PROCESSED_EQUIVOCATION_EVIDENCE.lock() {
            processed.clear();
        }
    }

    #[cfg(test)]
    fn set_test_local_vote_lock_path(path: Option<PathBuf>) {
        if let Ok(mut test_path) = TEST_LOCAL_VOTE_LOCK_PATH.lock() {
            *test_path = path;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::pqc::PQCAlgorithm;
    use crate::validator::{ValidatorRegistration, ValidatorStatus};
    use std::fs;
    use std::path::PathBuf;

    fn approved_validator_manager(addresses: &[&str]) -> Arc<ValidatorManager> {
        let manager = Arc::new(ValidatorManager::new());
        for address in addresses {
            manager
                .register_validator(ValidatorRegistration {
                    address: (*address).to_string(),
                    public_key: format!("{}-key", address),
                    name: format!("{address} validator"),
                    stake_amount: 1_000,
                    submitted_at: 0,
                    registration_tx_hash: format!("{address}-registration"),
                })
                .expect("validator registration should succeed");
            manager
                .approve_validator(address)
                .expect("validator approval should succeed");
        }
        manager
    }

    fn signed_block(block_index: u64, nonce: u64, validator_id: &str) -> Block {
        let mut block = Block::new(
            block_index,
            vec![],
            "parent-hash".to_string(),
            validator_id.to_string(),
            nonce,
        );

        let mut pqc_manager = PQCManager::new();
        let (public_key, private_key) = pqc_manager
            .generate_keypair(PQCAlgorithm::FNDSA)
            .expect("FN-DSA key generation should succeed");
        let signature = pqc_manager
            .sign(&private_key, block.hash.as_bytes())
            .expect("block signing should succeed");
        block.proposer_public_key = public_key.key_data;
        block.block_signature = signature.signature_data;
        block.block_signature_algorithm = "fndsa".to_string();
        block
    }

    fn temp_vote_lock_path(test_name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        std::env::temp_dir()
            .join(format!("synergy-{test_name}-{unique}"))
            .join("data")
            .join("consensus_vote_locks.json")
    }

    #[test]
    fn equivocation_evidence_slashes_conflicting_validator() {
        DualQuorumConsensus::reset_test_vote_tracking();

        let validator_manager = approved_validator_manager(&["validator1", "validator2"]);
        let pqc_manager = Arc::new(Mutex::new(PQCManager::new()));
        let consensus = DualQuorumConsensus::new(
            Arc::clone(&validator_manager),
            Arc::clone(&pqc_manager),
            true,
            1,
            1,
            8,
            5,
        );

        let first_block = signed_block(7, 1, "validator1");
        let conflicting_block = signed_block(7, 2, "validator1");

        let first_vote =
            DualQuorumConsensus::create_vote_for_validator("validator2", &first_block, 12, 1)
                .expect("first vote should be created");
        assert!(DualQuorumConsensus::register_vote_observation(&first_vote).is_none());

        let conflicting_vote =
            DualQuorumConsensus::create_vote_for_validator("validator2", &conflicting_block, 12, 1)
                .expect("conflicting vote should be created");
        let evidence = DualQuorumConsensus::register_vote_observation(&conflicting_vote)
            .expect("conflicting vote should emit equivocation evidence");

        consensus.apply_recorded_equivocations();

        let validator = validator_manager
            .get_validator("validator2")
            .expect("validator should still exist");
        assert_eq!(validator.status, ValidatorStatus::Slashed);
        assert_eq!(validator.double_signs, 1);
        assert_eq!(validator.equivocation_evidence_count, 1);
        assert_eq!(evidence.block_index, 7);
        assert_eq!(evidence.epoch_number, 12);
        assert_eq!(evidence.round_number, 1);
    }

    #[test]
    fn validator_can_repeat_same_block_vote_in_new_round() {
        DualQuorumConsensus::reset_test_vote_tracking();

        let validator_manager = approved_validator_manager(&["validator1", "validator2"]);
        let block = signed_block(9, 1, "validator1");

        let first_vote =
            DualQuorumConsensus::create_vote_for_validator("validator2", &block, 21, 1)
                .expect("round one vote should be created");
        assert!(DualQuorumConsensus::register_vote_observation(&first_vote).is_none());

        let next_round_vote =
            DualQuorumConsensus::create_vote_for_validator("validator2", &block, 21, 2)
                .expect("round two vote should be created");
        assert!(DualQuorumConsensus::register_vote_observation(&next_round_vote).is_none());

        let validator = validator_manager
            .get_validator("validator2")
            .expect("validator should still exist");
        assert_eq!(validator.status, ValidatorStatus::Active);
        assert_eq!(validator.double_signs, 0);
    }

    #[test]
    fn validator_conflicting_vote_in_later_round_is_allowed_for_liveness() {
        DualQuorumConsensus::reset_test_vote_tracking();

        let validator_manager = approved_validator_manager(&["validator1", "validator2"]);
        let pqc_manager = Arc::new(Mutex::new(PQCManager::new()));
        let consensus = DualQuorumConsensus::new(
            Arc::clone(&validator_manager),
            Arc::clone(&pqc_manager),
            true,
            1,
            1,
            8,
            5,
        );
        let first_block = signed_block(10, 1, "validator1");
        let conflicting_block = signed_block(10, 2, "validator1");

        let first_vote =
            DualQuorumConsensus::create_vote_for_validator("validator2", &first_block, 22, 1)
                .expect("round one vote should be created");
        assert!(DualQuorumConsensus::register_vote_observation(&first_vote).is_none());

        let conflicting_vote =
            DualQuorumConsensus::create_vote_for_validator("validator2", &conflicting_block, 22, 2)
                .expect("round two vote should be created");
        assert!(
            DualQuorumConsensus::register_vote_observation(&conflicting_vote).is_none(),
            "conflicting later-round vote should be treated as view-change liveness, not equivocation"
        );

        consensus.apply_recorded_equivocations();

        let validator = validator_manager
            .get_validator("validator2")
            .expect("validator should still exist");
        assert_eq!(validator.status, ValidatorStatus::Active);
        assert_eq!(validator.double_signs, 0);
        assert_eq!(validator.equivocation_evidence_count, 0);
    }

    #[test]
    fn local_vote_intent_persists_same_height_lock_before_signing() {
        DualQuorumConsensus::reset_test_vote_tracking();

        let path = temp_vote_lock_path("local-vote-intent");
        DualQuorumConsensus::set_test_local_vote_lock_path(Some(path.clone()));

        let block = signed_block(13, 1, "validator1");
        let conflicting_block = signed_block(13, 2, "validator1");

        DualQuorumConsensus::register_local_vote_intent("validator2", &block, 40, 1)
            .expect("first local vote intent should persist");
        DualQuorumConsensus::register_local_vote_intent("validator2", &block, 40, 2)
            .expect("same block hash may be repeated in a later round");

        let locks = DualQuorumConsensus::load_local_vote_locks_unlocked()
            .expect("persisted vote locks should load");
        let key = DualQuorumConsensus::local_vote_lock_key("validator2", 40, 13);
        assert_eq!(locks[&key].block_hash, block.hash);
        assert_eq!(locks[&key].first_round_number, 1);
        assert_eq!(locks[&key].latest_round_number, 2);

        let stale_error = DualQuorumConsensus::register_local_vote_intent(
            "validator2",
            &conflicting_block,
            40,
            2,
        )
        .expect_err("conflicting local vote intent in the same round should be rejected");
        assert!(
            stale_error.contains("already locally voted for different block"),
            "unexpected local vote lock error: {stale_error}"
        );

        DualQuorumConsensus::register_local_vote_intent("validator2", &conflicting_block, 40, 3)
            .expect("higher-round conflicting proposal should advance the local vote lock");

        let locks = DualQuorumConsensus::load_local_vote_locks_unlocked()
            .expect("updated vote locks should load");
        assert_eq!(locks[&key].block_hash, conflicting_block.hash);
        assert_eq!(locks[&key].first_round_number, 1);
        assert_eq!(locks[&key].latest_round_number, 3);

        DualQuorumConsensus::set_test_local_vote_lock_path(None);
        if let Some(root) = path.parent().and_then(|data| data.parent()) {
            let _ = fs::remove_dir_all(root);
        }
    }

    #[test]
    fn verified_vote_signature_cache_key_binds_signature_material() {
        DualQuorumConsensus::reset_test_vote_tracking();

        let validator_manager = approved_validator_manager(&["validator1", "validator2"]);
        let pqc_manager = Arc::new(Mutex::new(PQCManager::new()));
        let consensus = DualQuorumConsensus::new(
            Arc::clone(&validator_manager),
            Arc::clone(&pqc_manager),
            true,
            1,
            1,
            8,
            5,
        );

        let block = signed_block(8, 1, "validator1");
        let vote = DualQuorumConsensus::create_vote_for_validator("validator2", &block, 12, 1)
            .expect("vote should be created");

        consensus
            .verify_vote_signature(&vote)
            .expect("first verification should succeed");
        let cache_key = DualQuorumConsensus::vote_signature_cache_key(&vote);
        assert!(consensus
            .verified_vote_signatures
            .lock()
            .expect("cache lock")
            .contains(&cache_key));

        let mut tampered_vote = vote.clone();
        tampered_vote.signature.signature_data.push(0);
        assert_ne!(
            cache_key,
            DualQuorumConsensus::vote_signature_cache_key(&tampered_vote)
        );
    }

    #[test]
    fn merge_remote_votes_accepts_verified_votes_and_caches_signatures() {
        DualQuorumConsensus::reset_test_vote_tracking();

        let validator_manager =
            approved_validator_manager(&["validator1", "validator2", "validator3", "validator4"]);
        let pqc_manager = Arc::new(Mutex::new(PQCManager::new()));
        let consensus = DualQuorumConsensus::new(
            Arc::clone(&validator_manager),
            Arc::clone(&pqc_manager),
            true,
            1,
            1,
            8,
            5,
        );

        let block = signed_block(9, 1, "validator1");
        let local_vote =
            DualQuorumConsensus::create_vote_for_validator("validator1", &block, 12, 1)
                .expect("local vote should be created");
        let remote_vote_a =
            DualQuorumConsensus::create_vote_for_validator("validator2", &block, 12, 1)
                .expect("remote vote should be created");
        let remote_vote_b =
            DualQuorumConsensus::create_vote_for_validator("validator3", &block, 12, 1)
                .expect("remote vote should be created");

        let expected_validators = ["validator1", "validator2", "validator3", "validator4"]
            .into_iter()
            .map(String::from)
            .collect::<BTreeSet<_>>();
        let remote_cache_key = DualQuorumConsensus::vote_signature_cache_key(&remote_vote_a);
        let mut votes = vec![local_vote];

        consensus.merge_remote_votes(
            &mut votes,
            &expected_validators,
            &block.hash,
            12,
            1,
            vec![remote_vote_a, remote_vote_b],
        );

        assert_eq!(votes.len(), 3);
        assert!(consensus.vote_signature_cache_contains(&remote_cache_key));
    }

    #[test]
    fn round_allocation_respects_view_floor() {
        DualQuorumConsensus::reset_test_vote_tracking();

        let validator_manager = approved_validator_manager(&["validator1"]);
        let pqc_manager = Arc::new(Mutex::new(PQCManager::new()));
        let mut consensus = DualQuorumConsensus::new(
            Arc::clone(&validator_manager),
            Arc::clone(&pqc_manager),
            true,
            1,
            1,
            8,
            5,
        );

        assert_eq!(consensus.allocate_round_number(4, 1, "validator1", 3), 3);
        assert_eq!(consensus.allocate_round_number(4, 1, "validator1", 3), 4);
        assert_eq!(consensus.allocate_round_number(4, 1, "validator1", 1), 5);
        assert_eq!(consensus.allocate_round_number(5, 1, "validator1", 1), 1);
    }

    #[test]
    fn round_allocation_skips_rounds_already_used_by_local_validator() {
        DualQuorumConsensus::reset_test_vote_tracking();

        let validator_manager = approved_validator_manager(&["validator1", "validator2"]);
        let pqc_manager = Arc::new(Mutex::new(PQCManager::new()));
        let mut consensus = DualQuorumConsensus::new(
            Arc::clone(&validator_manager),
            Arc::clone(&pqc_manager),
            true,
            1,
            1,
            8,
            5,
        );

        let remote_leader_block = signed_block(14, 1, "validator1");
        let prior_local_vote = DualQuorumConsensus::create_vote_for_validator(
            "validator2",
            &remote_leader_block,
            41,
            2,
        )
        .expect("prior local vote should be created");
        assert!(DualQuorumConsensus::register_local_vote_attempt(&prior_local_vote).is_none());

        assert_eq!(
            consensus.allocate_round_number(14, 41, "validator2", 2),
            3,
            "a local validator that already voted in round 2 must advance to round 3"
        );
    }

    #[test]
    fn missed_vote_timeouts_are_ignored_when_penalization_is_disabled() {
        DualQuorumConsensus::reset_test_vote_tracking();

        let validator_manager = approved_validator_manager(&["validator1", "validator2"]);
        let pqc_manager = Arc::new(Mutex::new(PQCManager::new()));
        let consensus = DualQuorumConsensus::new(
            Arc::clone(&validator_manager),
            Arc::clone(&pqc_manager),
            false,
            1,
            1,
            8,
            5,
        );

        let before = validator_manager
            .get_validator("validator2")
            .expect("validator should exist")
            .clone();

        consensus.record_missed_vote_timeouts(std::slice::from_ref(&before), &[]);

        let after = validator_manager
            .get_validator("validator2")
            .expect("validator should exist");
        assert_eq!(after.uptime_percentage, before.uptime_percentage);
        assert_eq!(after.task_accuracy, before.task_accuracy);
        assert_eq!(after.reputation_score, before.reputation_score);
        assert_eq!(after.missed_vote_window, before.missed_vote_window);
        assert_eq!(
            after.consecutive_missed_votes,
            before.consecutive_missed_votes
        );
        assert_eq!(after.status, ValidatorStatus::Active);
    }

    #[test]
    fn four_of_six_equal_weight_votes_satisfy_two_thirds_validation_quorum() {
        let validator_manager = approved_validator_manager(&[
            "validator1",
            "validator2",
            "validator3",
            "validator4",
            "validator5",
            "validator6",
        ]);
        let pqc_manager = Arc::new(Mutex::new(PQCManager::new()));
        let consensus = DualQuorumConsensus::new(
            Arc::clone(&validator_manager),
            Arc::clone(&pqc_manager),
            false,
            3,
            4,
            2,
            6,
        );
        let active_validators =
            consensus_membership_validators(validator_manager.get_active_validators());
        let votes = ["validator1", "validator2", "validator3", "validator4"]
            .into_iter()
            .map(|validator_address| Vote {
                validator_address: validator_address.to_string(),
                block_hash: "block-hash".to_string(),
                block_index: 42,
                epoch_number: 1,
                round_number: 1,
                signature: PQCSignature {
                    algorithm: PQCAlgorithm::FNDSA,
                    signature_data: Vec::new(),
                    message_hash: Vec::new(),
                    public_key_id: String::new(),
                    created_at: 0,
                },
                signer_public_key: Vec::new(),
                timestamp: 0,
            })
            .collect::<Vec<_>>();

        assert!(
            consensus.has_commit_quorum(&active_validators, &votes),
            "4 of 6 equal-weight votes is exactly two thirds and must not wait for a fifth vote"
        );
    }

    #[test]
    fn local_conflicting_vote_attempt_is_rejected_without_self_slashing() {
        DualQuorumConsensus::reset_test_vote_tracking();

        let validator_manager = approved_validator_manager(&["validator1", "validator2"]);
        let pqc_manager = Arc::new(Mutex::new(PQCManager::new()));
        let consensus = DualQuorumConsensus::new(
            Arc::clone(&validator_manager),
            Arc::clone(&pqc_manager),
            true,
            1,
            1,
            8,
            5,
        );

        let first_block = signed_block(11, 1, "validator1");
        let conflicting_block = signed_block(11, 2, "validator1");

        let first_vote =
            DualQuorumConsensus::create_vote_for_validator("validator2", &first_block, 30, 1)
                .expect("first local vote should be created");
        consensus
            .register_local_vote_or_slash(&first_vote)
            .expect("first local vote should be accepted");

        let conflicting_vote =
            DualQuorumConsensus::create_vote_for_validator("validator2", &conflicting_block, 30, 1)
                .expect("conflicting local vote should be created");
        let error = consensus
            .register_local_vote_or_slash(&conflicting_vote)
            .expect_err("conflicting local vote should be rejected");
        assert!(
            error.contains("attempted conflicting votes"),
            "unexpected local conflict error: {error}"
        );

        let validator = validator_manager
            .get_validator("validator2")
            .expect("validator should still exist");
        assert_eq!(validator.status, ValidatorStatus::Active);
        assert_eq!(validator.double_signs, 0);
        assert_eq!(validator.equivocation_evidence_count, 0);

        let evidence = EQUIVOCATION_EVIDENCE_LOG.lock().expect("evidence log lock");
        let local_key = DualQuorumConsensus::vote_observation_key("validator2", 30, 11, 1);
        assert!(
            !evidence.contains_key(&local_key),
            "local conflicting vote should not persist slashable evidence"
        );
    }

    #[test]
    fn identical_vote_replay_is_idempotent() {
        DualQuorumConsensus::reset_test_vote_tracking();

        let validator_manager = approved_validator_manager(&["validator1", "validator2"]);
        let pqc_manager = Arc::new(Mutex::new(PQCManager::new()));
        let consensus = DualQuorumConsensus::new(
            Arc::clone(&validator_manager),
            Arc::clone(&pqc_manager),
            true,
            1,
            1,
            8,
            5,
        );

        let block = signed_block(12, 1, "validator1");
        let vote = DualQuorumConsensus::create_vote_for_validator("validator2", &block, 31, 1)
            .expect("vote should be created");

        consensus
            .register_local_vote_or_slash(&vote)
            .expect("first local vote should be accepted");
        consensus
            .register_local_vote_or_slash(&vote)
            .expect("replaying the same vote should be idempotent");

        let validator = validator_manager
            .get_validator("validator2")
            .expect("validator should still exist");
        assert_eq!(validator.status, ValidatorStatus::Active);
        assert_eq!(validator.double_signs, 0);
        assert_eq!(validator.equivocation_evidence_count, 0);

        let evidence = EQUIVOCATION_EVIDENCE_LOG.lock().expect("evidence log lock");
        let local_key = DualQuorumConsensus::vote_observation_key("validator2", 31, 12, 1);
        assert!(
            !evidence.contains_key(&local_key),
            "idempotent replay should not persist slashable evidence"
        );
    }

    #[test]
    fn epoch_randomness_is_deterministic_for_shared_qc() {
        let previous_qc = QuorumCertificate {
            block_hash: "shared-block-hash".to_string(),
            epoch_number: 7,
            round_number: 3,
            aggregate_signature: vec![1, 2, 3],
            participant_bitmap: vec![0x1f],
            cumulative_weight: 5.0,
            validation_quorum_met: true,
            cooperation_quorum_met: true,
            timestamp: 1_777_000_000,
        };
        let mut beacon_a = EntropyBeacon::new(Arc::new(Mutex::new(PQCManager::new())));
        let mut beacon_b = EntropyBeacon::new(Arc::new(Mutex::new(PQCManager::new())));

        let randomness_a = beacon_a.generate_epoch_randomness(&previous_qc);
        let randomness_b = beacon_b.generate_epoch_randomness(&previous_qc);

        assert_eq!(randomness_a, randomness_b);
    }

    #[test]
    fn epoch_randomness_ignores_qc_timestamp_differences() {
        let previous_qc_a = QuorumCertificate {
            block_hash: "shared-block-hash".to_string(),
            epoch_number: 7,
            round_number: 3,
            aggregate_signature: vec![1, 2, 3],
            participant_bitmap: vec![0x1f],
            cumulative_weight: 5.0,
            validation_quorum_met: true,
            cooperation_quorum_met: true,
            timestamp: 1_777_000_000,
        };
        let mut previous_qc_b = previous_qc_a.clone();
        previous_qc_b.timestamp += 42;

        let mut beacon_a = EntropyBeacon::new(Arc::new(Mutex::new(PQCManager::new())));
        let mut beacon_b = EntropyBeacon::new(Arc::new(Mutex::new(PQCManager::new())));

        let randomness_a = beacon_a.generate_epoch_randomness(&previous_qc_a);
        let randomness_b = beacon_b.generate_epoch_randomness(&previous_qc_b);

        assert_eq!(randomness_a, randomness_b);
    }

    #[test]
    fn epoch_randomness_ignores_local_beacon_epoch_drift() {
        let previous_qc = QuorumCertificate {
            block_hash: "shared-block-hash".to_string(),
            epoch_number: 7,
            round_number: 3,
            aggregate_signature: vec![1, 2, 3],
            participant_bitmap: vec![0x1f],
            cumulative_weight: 5.0,
            validation_quorum_met: true,
            cooperation_quorum_met: true,
            timestamp: 1_777_000_000,
        };
        let mut beacon_a = EntropyBeacon::new(Arc::new(Mutex::new(PQCManager::new())));
        let mut beacon_b = EntropyBeacon::new(Arc::new(Mutex::new(PQCManager::new())));

        // Simulate nodes that have taken a different number of local transition
        // attempts before observing the same epoch-boundary QC.
        beacon_a.current_epoch = 2;
        beacon_b.current_epoch = 19;

        let randomness_a = beacon_a.generate_epoch_randomness(&previous_qc);
        let randomness_b = beacon_b.generate_epoch_randomness(&previous_qc);

        assert_eq!(randomness_a, randomness_b);
        assert_eq!(beacon_a.current_epoch, 8);
        assert_eq!(beacon_b.current_epoch, 8);
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
        let next_epoch = previous_qc.epoch_number.saturating_add(1);
        self.current_epoch = next_epoch;
        self.previous_qc_hash = self.hash_qc(previous_qc);
        self.nonce += 1;

        // Keep a per-epoch ML-KEM keypair for future cross-validation hooks, but
        // derive the public epoch randomness deterministically from shared chain
        // state so every validator computes the same leader rotation.
        if !self.mlkem_keypairs.contains_key(&next_epoch) {
            let mut pqc_manager = self.pqc_manager.lock().unwrap();

            let (pub_key, priv_key) = pqc_manager
                .generate_keypair(PQCAlgorithm::MLKEM1024)
                .expect("Failed to generate ML-KEM keypair for epoch");

            self.mlkem_keypairs.insert(next_epoch, (pub_key, priv_key));
        }

        // Epoch randomness must be identical across validators at the same chain
        // tip. Only use deterministic inputs derived from the previous QC.
        let mut input = Vec::new();
        input.extend(next_epoch.to_be_bytes());
        input.extend(self.previous_qc_hash.as_bytes());

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
        let mut hasher = Sha3_512::new();
        hasher.update(qc.block_hash.as_bytes());
        hasher.update(qc.epoch_number.to_be_bytes());
        hasher.update(qc.round_number.to_be_bytes());
        hasher.update(&qc.aggregate_signature);
        hasher.update(&qc.participant_bitmap);
        hasher.update([qc.validation_quorum_met as u8]);
        hasher.update([qc.cooperation_quorum_met as u8]);
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
            target_cluster_size: TESTNET_BETA_VALIDATOR_CLUSTER_SIZE,
        }
    }

    pub fn rotate_validators(&self) {
        let active_validators = self.validator_manager.get_active_validators();
        let epoch_randomness = self.get_current_epoch_randomness();

        // Calculate number of clusters
        let num_clusters =
            (active_validators.len() as f64 / self.target_cluster_size as f64).ceil() as usize;

        // Assign validators to clusters using deterministic randomness
        for validator in &active_validators {
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
