use super::cartel_detection::{CartelDetectionEngine, VoteRecord};
use super::dao_governance::{DAOGovernance, GovernanceProposal, ProposalStatus};
use super::dual_quorum::{
    DualQuorumConsensus, EntropyBeacon, QuorumCertificate, ValidatorRotation, Vote,
};
use super::synergy_score::SynergyScoreCalculator;
use super::vrf::{VRFConsensus, VRFSeed};
use crate::block::{Block, BlockChain};
use crate::crypto::pqc::{PQCAlgorithm, PQCManager, PQCPrivateKey, PQCPublicKey};
use crate::genesis::canonical_genesis;
use crate::rpc::rpc_server::{SHARED_CHAIN, TX_POOL};
use crate::token::TOKEN_MANAGER;
use crate::validator::{
    Validator, ValidatorManager, ValidatorPerformanceUpdate, TESTNET_BETA_VALIDATOR_CLUSTER_SIZE,
    VALIDATOR_MANAGER,
};
use crate::wallet::WALLET_MANAGER;
use crate::{info, warn};
use base64::{engine::general_purpose, Engine as _};
use hex;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_512};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// CHAIN_PATH will be resolved at runtime using project root
fn get_chain_path() -> String {
    crate::utils::resolve_data_path("data/chain.json")
        .to_str()
        .unwrap_or("data/chain.json")
        .to_string()
}
const VALIDATOR_REGISTRY_PATH: &str = "data/validator_registry.json";
const VERBOSE_CONSENSUS_LOGS: bool = false;

macro_rules! consensus_log {
    ($($arg:tt)*) => {
        if VERBOSE_CONSENSUS_LOGS {
            println!($($arg)*);
        }
    };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynergyScores {
    pub scores: HashMap<String, f64>,
    pub last_updated: u64,
}

#[derive(Debug)]
pub struct ProofOfSynergy {
    pub chain: Arc<Mutex<BlockChain>>,
    pub validator_manager: Arc<ValidatorManager>,
    pub synergy_scores: SynergyScores,
    pub block_time: u64,
    pub epoch_length: u64,
    pub min_validators: usize,
    pub cluster_size: usize,
    pub vrf_enabled: bool,
    pub vrf_seed_interval: u64,
    pub max_synergy_points: u64,
    pub reward_weights: RewardWeights,
    pub vrf_consensus: VRFConsensus,
    pub current_vrf_seed: Option<VRFSeed>,

    // New PoSy components
    pub synergy_calculator: Arc<SynergyScoreCalculator>,
    pub dual_quorum_consensus: Arc<Mutex<DualQuorumConsensus>>,
    pub entropy_beacon: Arc<Mutex<EntropyBeacon>>,
    pub validator_rotation: Arc<ValidatorRotation>,
    pub dao_governance: Arc<Mutex<DAOGovernance>>,
    pub cartel_detection: Arc<Mutex<CartelDetectionEngine>>,
    pub pqc_manager: Arc<Mutex<PQCManager>>,

    // State tracking
    pub current_epoch: u64,
    pub epoch_votes: HashMap<u64, Vec<Vote>>,
    pub quorum_certificates: HashMap<u64, QuorumCertificate>,
    pub governance_proposals: HashMap<String, GovernanceProposal>,
}

#[derive(Debug, Clone)]
pub struct RewardWeights {
    pub task_accuracy: f64,
    pub uptime: f64,
    pub collaboration: f64,
}

// Track leader rotation within epochs
lazy_static::lazy_static! {
    static ref EPOCH_LEADER_ROTATION: Arc<Mutex<(u64, Vec<String>, usize)>> =
        Arc::new(Mutex::new((0, Vec::new(), 0))); // (epoch, top_k_validators, current_index)
    static ref EPHEMERAL_LEADER_KEYS: Arc<Mutex<HashMap<String, (PQCPublicKey, PQCPrivateKey)>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

impl ProofOfSynergy {
    pub fn new() -> Self {
        // Use the global shared chain instance
        let chain = Arc::clone(&SHARED_CHAIN);

        // Use global validator manager
        let validator_manager = Arc::clone(&VALIDATOR_MANAGER);

        // Initialize PQC manager
        let pqc_manager = Arc::new(Mutex::new(PQCManager::new()));

        // Initialize synergy score calculator
        let synergy_calculator = Arc::new(SynergyScoreCalculator::new(
            Arc::clone(&validator_manager),
            Arc::clone(&pqc_manager),
        ));

        // Initialize entropy beacon
        let entropy_beacon = Arc::new(Mutex::new(EntropyBeacon::new(Arc::clone(&pqc_manager))));

        // Initialize validator rotation
        let validator_rotation = Arc::new(ValidatorRotation::new(
            Arc::clone(&validator_manager),
            Arc::clone(&entropy_beacon),
        ));

        // Initialize DAO governance
        let dao_governance = Arc::new(Mutex::new(DAOGovernance::new(
            Arc::clone(&validator_manager),
            Arc::clone(&synergy_calculator),
            Arc::clone(&pqc_manager),
        )));

        // Initialize cartel detection
        let cartel_detection = Arc::new(Mutex::new(CartelDetectionEngine::new(
            Arc::clone(&validator_manager),
            Arc::clone(&synergy_calculator),
        )));

        // Load validator registry from file or initialize genesis validators
        if let Err(e) = validator_manager.load_registry(VALIDATOR_REGISTRY_PATH) {
            println!(
                "🔧 No validator registry found — initializing with genesis validators: {}",
                e
            );
            Self::initialize_genesis_validators(&validator_manager);

            // Save the registry after initializing genesis validators
            if let Err(save_err) = validator_manager.save_registry(VALIDATOR_REGISTRY_PATH) {
                println!(
                    "⚠️ Failed to save validator registry after genesis initialization: {}",
                    save_err
                );
            } else {
                println!("✅ Validator registry saved to {}", VALIDATOR_REGISTRY_PATH);
            }
        } else {
            // Registry exists.  Re-read genesis.json so any validators that were
            // added after the node's first run (e.g. multi-node setups where the
            // genesis.json was populated after initial launch) are registered and
            // approved, not just staked.
            println!("🔧 Validator registry loaded, ensuring genesis validators have stakes");
            Self::ensure_genesis_validator_stakes(&validator_manager);

            // Persist any newly-registered validators back to disk.
            if let Err(save_err) = validator_manager.save_registry(VALIDATOR_REGISTRY_PATH) {
                println!(
                    "⚠️ Failed to save validator registry after genesis stake check: {}",
                    save_err
                );
            } else {
                println!("✅ Validator registry saved after genesis stake check");
            }
        }

        let synergy_scores = Self::load_synergy_scores().unwrap_or_else(|| {
            println!("🔧 No synergy scores found — initializing empty scores.");
            SynergyScores {
                scores: HashMap::new(),
                last_updated: Self::current_timestamp(),
            }
        });

        let consensus_cfg = crate::config::load_node_config(None)
            .ok()
            .map(|cfg| cfg.consensus);

        // Load consensus timing from env/config for deterministic testnet-beta tuning.
        let block_time = std::env::var("SYNERGY_CONSENSUS_BLOCK_TIME_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .or_else(|| consensus_cfg.as_ref().map(|c| c.block_time_secs))
            .unwrap_or(5)
            .max(1);

        let epoch_length = std::env::var("SYNERGY_CONSENSUS_EPOCH_LENGTH")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .or_else(|| consensus_cfg.as_ref().map(|c| c.epoch_length))
            .unwrap_or(1000)
            .max(1);

        let min_validators = std::env::var("SYNERGY_CONSENSUS_MIN_VALIDATORS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .or_else(|| consensus_cfg.as_ref().map(|c| c.min_validators))
            .unwrap_or(5)
            .max(1);

        // Initialize dual quorum consensus after loading the minimum validator requirement.
        let dual_quorum_consensus = Arc::new(Mutex::new(DualQuorumConsensus::new(
            Arc::clone(&validator_manager),
            Arc::clone(&pqc_manager),
            min_validators,
        )));

        let cluster_size = consensus_cfg
            .as_ref()
            .map(|c| c.validator_cluster_size)
            .unwrap_or(TESTNET_BETA_VALIDATOR_CLUSTER_SIZE);
        let vrf_enabled = consensus_cfg
            .as_ref()
            .map(|c| c.vrf_enabled)
            .unwrap_or(true);
        let vrf_seed_interval = consensus_cfg
            .as_ref()
            .map(|c| c.vrf_seed_epoch_interval)
            .unwrap_or(1000);
        let max_synergy_points = consensus_cfg
            .as_ref()
            .map(|c| c.max_synergy_points_per_epoch)
            .unwrap_or(100);

        let reward_weights = RewardWeights {
            task_accuracy: consensus_cfg
                .as_ref()
                .map(|c| c.reward_weighting.task_accuracy)
                .unwrap_or(0.5),
            uptime: consensus_cfg
                .as_ref()
                .map(|c| c.reward_weighting.uptime)
                .unwrap_or(0.3),
            collaboration: consensus_cfg
                .as_ref()
                .map(|c| c.reward_weighting.collaboration)
                .unwrap_or(0.2),
        };

        let vrf_consensus = VRFConsensus::new();
        let current_vrf_seed = if vrf_enabled {
            Some(VRFSeed::generate())
        } else {
            None
        };

        ProofOfSynergy {
            chain,
            validator_manager,
            synergy_scores,
            block_time,
            epoch_length,
            min_validators,
            cluster_size,
            vrf_enabled,
            vrf_seed_interval,
            max_synergy_points,
            reward_weights,
            vrf_consensus,
            current_vrf_seed,
            synergy_calculator,
            dual_quorum_consensus,
            entropy_beacon,
            validator_rotation,
            dao_governance,
            cartel_detection,
            pqc_manager,
            current_epoch: 0,
            epoch_votes: HashMap::new(),
            quorum_certificates: HashMap::new(),
            governance_proposals: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) {
        let active_validators = self.validator_manager.get_active_validators();
        let live_validator_addresses =
            Self::collect_live_validator_addresses(&self.validator_manager);
        let chain = self.chain.lock().unwrap();
        println!(
            "🔧 Chain loaded. Latest height: {}",
            chain.last().map_or(0, |b| b.block_index)
        );
        println!(
            "🔧 Validator registry loaded. Active validators: {}",
            active_validators.len()
        );
        println!(
            "🔧 Synergy scores loaded. Total entries: {}",
            self.synergy_scores.scores.len()
        );
        println!(
            "🔧 Live validator participants currently visible: {}",
            live_validator_addresses.len()
        );
        println!(
            "🔧 Minimum active validators required for block production: {}",
            self.min_validators
        );
    }

    pub fn execute(&mut self) {
        info!("consensus", "Starting Proof of Synergy consensus engine");

        let chain = Arc::clone(&self.chain);
        let validator_manager = Arc::clone(&self.validator_manager);
        let synergy_calculator = Arc::clone(&self.synergy_calculator);
        let dual_quorum_consensus = Arc::clone(&self.dual_quorum_consensus);
        let entropy_beacon = Arc::clone(&self.entropy_beacon);
        let validator_rotation = Arc::clone(&self.validator_rotation);
        let dao_governance = Arc::clone(&self.dao_governance);
        let cartel_detection = Arc::clone(&self.cartel_detection);
        let pqc_manager = Arc::clone(&self.pqc_manager);
        let block_time_secs = self.block_time.max(1);
        let epoch_length = self.epoch_length.max(1);
        let min_validators = self.min_validators.max(1);

        thread::spawn(move || {
            let mut last_block_time = SystemTime::now();
            let mut consecutive_failures = 0;
            let mut current_epoch = 0;
            // View-change state: tracks how many times the scheduled leader has timed out
            // for the current block height. Increments on each leader timeout, causing
            // select_leader_for_block to rotate to the next candidate in the top-K list.
            let mut view_offset: usize = 0;
            // When we first started waiting for the leader at the current height.
            let mut block_wait_start: Option<SystemTime> = None;
            // The chain height at which view_offset was last reset.
            let mut last_committed_height: u64 = 0;
            // How long to wait for a leader proposal before rotating to the next candidate.
            // At least 15 seconds; otherwise 3× the configured block time.
            let leader_timeout_secs = (block_time_secs * 3).max(15);

            loop {
                let current_time = SystemTime::now();
                let elapsed = current_time
                    .duration_since(last_block_time)
                    .unwrap_or_default();

                if elapsed >= Duration::from_secs(block_time_secs) {
                    let pool = TX_POOL.lock().unwrap();
                    let chain_guard = chain.lock().unwrap();

                    if let Some(latest_block) = chain_guard.last() {
                        // Reset view-change state whenever the chain has advanced.
                        if latest_block.block_index != last_committed_height {
                            last_committed_height = latest_block.block_index;
                            view_offset = 0;
                            block_wait_start = None;
                        }

                        // Check for epoch boundary
                        if Self::is_epoch_boundary(latest_block.block_index, epoch_length) {
                            Self::handle_epoch_transition(
                                &mut current_epoch,
                                &chain_guard,
                                &validator_manager,
                                &synergy_calculator,
                                &dual_quorum_consensus,
                                &entropy_beacon,
                                &validator_rotation,
                                &dao_governance,
                                &cartel_detection,
                                epoch_length,
                            );
                        }

                        // Get active validators
                        let active_validators = validator_manager.get_active_validators();
                        let registry_active_count = active_validators.len();
                        let live_validator_addresses =
                            Self::collect_live_validator_addresses(&validator_manager);
                        let live_validator_address_set = live_validator_addresses
                            .iter()
                            .cloned()
                            .collect::<HashSet<_>>();
                        let live_active_validators: Vec<Validator> = active_validators
                            .into_iter()
                            .filter(|validator| {
                                live_validator_address_set.contains(&validator.address)
                            })
                            .collect();
                        consensus_log!(
                            "🔍 Found {} registry-active validators and {} live validator participants",
                            registry_active_count,
                            live_active_validators.len()
                        );

                        if live_active_validators.len() < min_validators {
                            println!(
                                "⏳ Insufficient live validators for block production: {} live, {} registry-active, {} required.",
                                live_active_validators.len(),
                                registry_active_count,
                                min_validators
                            );
                            thread::sleep(Duration::from_secs(1));
                            continue;
                        }

                        consensus_log!(
                            "🎯 Selecting leader for block {}",
                            latest_block.block_index + 1
                        );

                        // Clone latest_block before we might need to drop the guard
                        let latest_block_clone = latest_block.clone();

                        // Phase 1: Leader selection using entropy beacon and synergy scores
                        // Use next block index for leader selection (current block + 1)
                        let next_block_index = latest_block_clone.block_index + 1;
                        let selected_validator = Self::select_leader_for_block(
                            &live_active_validators,
                            next_block_index,
                            &synergy_calculator,
                            &entropy_beacon,
                            epoch_length,
                            view_offset,
                        );

                        let local_validator_address = Self::resolve_local_validator_address();
                        if local_validator_address.as_deref()
                            != Some(selected_validator.address.as_str())
                        {
                            // Track how long we have been waiting for the leader at this height.
                            let wait_start = *block_wait_start.get_or_insert(current_time);
                            let wait_elapsed =
                                current_time.duration_since(wait_start).unwrap_or_default();

                            if wait_elapsed >= Duration::from_secs(leader_timeout_secs) {
                                // Leader did not produce within the timeout window.
                                // Rotate to the next candidate in the top-K list.
                                view_offset += 1;
                                block_wait_start = Some(current_time);
                                warn!(
                                    "consensus",
                                    "Leader proposal timeout — rotating to next candidate",
                                    "timed_out_leader" => selected_validator.address.clone(),
                                    "new_view_offset" => view_offset,
                                    "waited_secs" => wait_elapsed.as_secs(),
                                    "block_height" => next_block_index
                                );
                                // Penalise the timed-out leader's synergy score so that
                                // chronically offline validators sink in the epoch ranking
                                // and are naturally displaced from the top-K rotation.
                                Self::apply_leader_timeout_penalty(
                                    &validator_manager,
                                    &selected_validator.address,
                                    next_block_index,
                                    view_offset,
                                );
                            } else {
                                info!(
                                    "consensus",
                                    "Local validator is not the scheduled leader; waiting for remote proposal",
                                    "leader" => selected_validator.address.clone(),
                                    "local_validator" => local_validator_address.unwrap_or_default(),
                                    "visible_validators" => live_active_validators.len() as u64
                                );
                            }
                            drop(chain_guard);
                            drop(pool);
                            last_block_time = current_time;
                            thread::sleep(Duration::from_millis(250));
                            continue;
                        }

                        consensus_log!("LEADER SELECTED: {}", selected_validator.address);
                        consensus_log!("Getting transactions from pool...");

                        let transactions = if pool.is_empty() {
                            consensus_log!("Pool is empty");
                            vec![]
                        } else {
                            consensus_log!("Pool has {} transactions", pool.len());
                            pool.clone()
                        };

                        consensus_log!("Creating processed transactions vec...");
                        let mut processed_transactions = Vec::new();

                        consensus_log!("Processing {} transactions...", transactions.len());
                        // Process transactions with full validation
                        for tx in &transactions {
                            if Self::validate_transaction(tx, &pqc_manager) {
                                processed_transactions.push(tx.clone());

                                // Update wallet nonce
                            } else {
                                println!(
                                    "❌ Invalid transaction from {}: failed validation",
                                    tx.sender
                                );
                            }
                        }

                        consensus_log!(
                            "Creating block proposal with {} processed transactions...",
                            processed_transactions.len()
                        );
                        use std::io::{self, Write};
                        io::stdout().flush().unwrap();

                        // Phase 2: Block proposal
                        consensus_log!("Calling create_block_proposal...");
                        io::stdout().flush().unwrap();
                        let new_block = Self::create_block_proposal(
                            &latest_block_clone,
                            &selected_validator,
                            processed_transactions,
                            &pqc_manager,
                        );
                        consensus_log!("Block proposal created!");
                        io::stdout().flush().unwrap();

                        // Phase 3: Dual-quorum consensus
                        consensus_log!("Starting dual-quorum consensus...");
                        io::stdout().flush().unwrap();

                        info!("consensus", "Starting dual-quorum consensus",
                              "block_height" => new_block.block_index,
                              "block_hash" => new_block.hash.clone(),
                              "epoch" => current_epoch,
                              "validator" => selected_validator.address.clone());

                        let quorum_certificate = Self::execute_dual_quorum_consensus(
                            &new_block,
                            &validator_manager,
                            &dual_quorum_consensus,
                            current_epoch,
                            view_offset,
                        );

                        consensus_log!("Dual-quorum consensus complete!");
                        io::stdout().flush().unwrap();

                        consensus_log!("Matching on quorum_certificate result...");
                        io::stdout().flush().unwrap();

                        match quorum_certificate {
                            Ok(qc) => {
                                // Block committed - update chain.
                                // Reset view-change state: the chain has advanced, so the next
                                // block starts with the primary scheduled leader again.
                                view_offset = 0;
                                block_wait_start = None;
                                last_block_time = current_time;
                                drop(chain_guard);
                                drop(pool);

                                {
                                    let mut chain_guard = chain.lock().unwrap();
                                    chain_guard.add_block(new_block.clone());
                                    chain_guard.save_to_file(&get_chain_path());
                                }

                                // Apply state transitions for included transactions (token transfers, staking, etc.)
                                let token_manager = TOKEN_MANAGER.clone();
                                let mut applied_txs = 0u64;
                                let mut failed_txs = 0u64;
                                for tx in &new_block.transactions {
                                    match token_manager
                                        .process_transaction_in_block(tx, new_block.block_index)
                                    {
                                        Ok(_) => applied_txs += 1,
                                        Err(e) => {
                                            failed_txs += 1;
                                            warn!(
                                                "consensus",
                                                "Failed to apply transaction state",
                                                "tx_hash" => tx.hash(),
                                                "error" => e
                                            );
                                        }
                                    }
                                }

                                // Persist token state for explorer continuity across restarts (best-effort).
                                if let Err(e) = token_manager.save_state("data/token_state.json") {
                                    warn!("consensus", "Failed to persist token state", "error" => e.to_string());
                                }

                                // Broadcast the committed block to peers (best-effort).
                                if let Some(p2p) = crate::p2p::get_p2p_network() {
                                    p2p.broadcast_block(&new_block);
                                }

                                // Update validator performance
                                let performance_update = ValidatorPerformanceUpdate {
                                    validator_address: selected_validator.address.clone(),
                                    update_type: "block_produced".to_string(),
                                    value: None,
                                    timestamp: Self::current_timestamp(),
                                };
                                validator_manager.update_performance(performance_update.clone());

                                // Distribute rewards to cluster based on PoSy protocol
                                // Rewards are awarded to the cluster, then distributed among validators
                                // in that cluster based on their normalized Synergy Scores
                                let reward_amount = 5 * 10u64.pow(9);

                                // Get the cluster that the selected validator belongs to
                                if let Some(cluster) = validator_manager
                                    .get_validator_cluster(&selected_validator.address)
                                {
                                    // Get all validators in the cluster
                                    let cluster_validator_addresses = &cluster.validators;

                                    info!("consensus", "Cluster reward distribution",
                                          "cluster_id" => cluster.id,
                                          "cluster_size" => cluster_validator_addresses.len() as u64,
                                          "selected_validator" => selected_validator.address.clone(),
                                          "reward_amount" => reward_amount);

                                    // Calculate raw synergy scores for all validators in the cluster
                                    // Then normalize within the cluster (PoSy Equation 13: SS_v,normalized = SS_v / Σ_i∈V SS_i)
                                    let mut cluster_validators_with_scores: Vec<(String, f64)> =
                                        Vec::new();
                                    let mut raw_scores: Vec<f64> = Vec::new();
                                    let mut validator_details: Vec<serde_json::Value> = Vec::new();

                                    for validator_address in cluster_validator_addresses {
                                        if let Some(validator) =
                                            validator_manager.get_validator(validator_address)
                                        {
                                            let components = synergy_calculator
                                                .calculate_synergy_score(&validator);
                                            // Calculate raw score: SS_v = (S_v * R_v * C_v) / P_v (PoSy Equation 12)
                                            let raw_score = (components.stake_weight
                                                * components.reputation
                                                * components.contribution_index)
                                                / components.cartelization_penalty;
                                            raw_scores.push(raw_score);
                                            cluster_validators_with_scores
                                                .push((validator_address.clone(), raw_score));

                                            // Log validator details
                                            validator_details.push(serde_json::json!({
                                                "address": validator_address,
                                                "name": validator.name,
                                                "raw_synergy_score": raw_score,
                                                "normalized_synergy_score": components.normalized_score,
                                                "stake_weight": components.stake_weight,
                                                "reputation": components.reputation,
                                                "contribution_index": components.contribution_index,
                                                "cartelization_penalty": components.cartelization_penalty
                                            }));
                                        }
                                    }

                                    info!("consensus", "Cluster validator synergy scores",
                                          "cluster_id" => cluster.id,
                                          "validators" => serde_json::to_string(&validator_details).unwrap_or_default());

                                    // Normalize scores within the cluster so they sum to 1.0 (PoSy Equation 13)
                                    if !raw_scores.is_empty() {
                                        let total_score: f64 = raw_scores.iter().sum();
                                        if total_score > 0.0 {
                                            // Normalize: SS_v,normalized = SS_v / Σ_i∈cluster SS_i
                                            for (idx, (_, score)) in cluster_validators_with_scores
                                                .iter_mut()
                                                .enumerate()
                                            {
                                                *score = raw_scores[idx] / total_score;
                                            }

                                            // Log normalized distribution
                                            let normalized_dist: Vec<serde_json::Value> = cluster_validators_with_scores.iter()
                                                .map(|(addr, score)| serde_json::json!({
                                                    "validator": addr,
                                                    "normalized_share": score,
                                                    "reward_portion": (reward_amount as f64 * score) as u64
                                                }))
                                                .collect();

                                            info!("consensus", "Cluster reward distribution (normalized)",
                                                  "cluster_id" => cluster.id,
                                                  "total_raw_score" => total_score,
                                                  "distribution" => serde_json::to_string(&normalized_dist).unwrap_or_default());

                                            // Distribute rewards to cluster
                                            match token_manager.distribute_cluster_rewards(
                                                &cluster_validators_with_scores,
                                                reward_amount,
                                            ) {
                                                Ok(result) => {
                                                    println!(
                                                        "✅ Cluster rewards distributed: {}",
                                                        result
                                                    );
                                                    info!("consensus", "Cluster rewards distributed successfully",
                                                          "cluster_id" => cluster.id,
                                                          "result" => result);
                                                }
                                                Err(e) => {
                                                    println!("❌ Failed to distribute cluster rewards: {}", e);
                                                    warn!("consensus", "Failed to distribute cluster rewards",
                                                          "cluster_id" => cluster.id,
                                                          "error" => e);
                                                }
                                            }
                                        } else {
                                            println!("⚠️ Cluster has no valid synergy scores, using fallback distribution");
                                            warn!("consensus", "Cluster has no valid synergy scores, using equal distribution",
                                                  "cluster_id" => cluster.id);
                                            // Fallback: distribute equally if no valid scores
                                            let equal_share =
                                                1.0 / cluster_validator_addresses.len() as f64;
                                            let equal_scores: Vec<(String, f64)> =
                                                cluster_validator_addresses
                                                    .iter()
                                                    .map(|addr| (addr.clone(), equal_share))
                                                    .collect();
                                            match token_manager.distribute_cluster_rewards(
                                                &equal_scores,
                                                reward_amount,
                                            ) {
                                                Ok(result) => {
                                                    println!("✅ Cluster rewards distributed (equal): {}", result);
                                                    info!("consensus", "Cluster rewards distributed (equal fallback)",
                                                          "cluster_id" => cluster.id,
                                                          "result" => result);
                                                }
                                                Err(e) => {
                                                    println!("❌ Failed to distribute cluster rewards: {}", e);
                                                    warn!("consensus", "Failed to distribute cluster rewards (equal fallback)",
                                                          "cluster_id" => cluster.id,
                                                          "error" => e);
                                                }
                                            }
                                        }
                                    } else {
                                        println!("⚠️ No validators found in cluster, using legacy distribution");
                                        warn!("consensus", "No validators found in cluster, using legacy distribution",
                                              "cluster_id" => cluster.id);
                                        // Fallback to legacy method if cluster is empty
                                        match token_manager.distribute_validator_rewards(
                                            &selected_validator.address,
                                            reward_amount,
                                        ) {
                                            Ok(result) => {
                                                println!(
                                                    "✅ Validator rewards distributed (legacy): {}",
                                                    result
                                                );
                                                info!("consensus", "Validator rewards distributed (legacy fallback)",
                                                      "validator" => selected_validator.address.clone(),
                                                      "result" => result);
                                            }
                                            Err(e) => {
                                                println!(
                                                    "❌ Failed to distribute validator rewards: {}",
                                                    e
                                                );
                                                warn!("consensus", "Failed to distribute validator rewards (legacy)",
                                                      "validator" => selected_validator.address.clone(),
                                                      "error" => e);
                                            }
                                        }
                                    }
                                } else {
                                    println!("⚠️ Validator {} not in a cluster, using legacy distribution", selected_validator.address);
                                    warn!("consensus", "Validator not in a cluster, using legacy distribution",
                                          "validator" => selected_validator.address.clone());
                                    // Fallback to legacy method if validator not in cluster
                                    match token_manager.distribute_validator_rewards(
                                        &selected_validator.address,
                                        reward_amount,
                                    ) {
                                        Ok(result) => {
                                            println!(
                                                "✅ Validator rewards distributed (legacy): {}",
                                                result
                                            );
                                            info!("consensus", "Validator rewards distributed (legacy fallback)",
                                                  "validator" => selected_validator.address.clone(),
                                                  "result" => result);
                                        }
                                        Err(e) => {
                                            println!(
                                                "❌ Failed to distribute validator rewards: {}",
                                                e
                                            );
                                            warn!("consensus", "Failed to distribute validator rewards (legacy)",
                                                  "validator" => selected_validator.address.clone(),
                                                  "error" => e);
                                        }
                                    }
                                }

                                // Update synergy scores
                                Self::update_synergy_scores(
                                    &validator_manager,
                                    &synergy_calculator,
                                    &selected_validator.address,
                                );

                                // Record vote for cartel detection
                                Self::record_vote_for_cartel_detection(
                                    &cartel_detection,
                                    &selected_validator.address,
                                    new_block.block_index,
                                    true,
                                    Self::current_timestamp(),
                                    epoch_length,
                                );

                                // Check for governance proposals
                                Self::check_governance_proposals(
                                    &dao_governance,
                                    new_block.block_index,
                                );

                                // Clear transaction pool
                                {
                                    let mut pool = TX_POOL.lock().unwrap();
                                    if !pool.is_empty() {
                                        pool.clear();
                                    }
                                }

                                last_block_time = current_time;
                                consecutive_failures = 0;

                                // Get synergy score components for detailed logging
                                let synergy_components =
                                    synergy_calculator.calculate_synergy_score(&selected_validator);

                                // Get cluster info if available
                                let cluster_info = validator_manager
                                    .get_validator_cluster(&selected_validator.address)
                                    .map(|c| {
                                        serde_json::json!({
                                            "cluster_id": c.id,
                                            "cluster_size": c.validators.len(),
                                            "total_stake": c.total_stake,
                                            "average_synergy_score": c.average_synergy_score
                                        })
                                    });

                                info!(
                                    "consensus",
                                    "Block committed",
                                    "height" => new_block.block_index,
                                    "hash" => new_block.hash.clone(),
                                    "previous_hash" => new_block.previous_hash.clone(),
                                    "timestamp" => new_block.timestamp,
                                    "epoch" => current_epoch,
                                    "block_in_epoch" => new_block.block_index % epoch_length,
                                    "validator" => selected_validator.address.clone(),
                                    "validator_name" => selected_validator.name.clone(),
                                    "synergy_score" => format!("{:.2}", selected_validator.synergy_score),
                                    "synergy_score_components" => serde_json::json!({
                                        "stake_weight": synergy_components.stake_weight,
                                        "reputation": synergy_components.reputation,
                                        "contribution_index": synergy_components.contribution_index,
                                        "cartelization_penalty": synergy_components.cartelization_penalty,
                                        "normalized_score": synergy_components.normalized_score
                                    }).to_string(),
                                    "cluster_info" => cluster_info.as_ref().map(|c| c.to_string()).unwrap_or_default(),
                                    "txs" => new_block.transactions.len() as u64,
                                    "txs_applied" => applied_txs,
                                    "txs_failed" => failed_txs,
                                    "qc_validation_quorum_met" => qc.validation_quorum_met,
                                    "qc_cooperation_quorum_met" => qc.cooperation_quorum_met,
                                    "qc_epoch_number" => qc.epoch_number,
                                    "qc_cumulative_weight" => qc.cumulative_weight,
                                    "qc_timestamp" => qc.timestamp,
                                    "reward_amount" => reward_amount
                                );
                            }
                            Err(e) => {
                                warn!("consensus", "QC error - block proposal failed", "error" => e.clone());
                                use std::io::{self, Write};
                                io::stdout().flush().unwrap();
                                println!("⚠️ Block proposal failed: {}", e);
                                consecutive_failures += 1;

                                // Apply penalty to proposer for failed block
                                Self::apply_proposer_penalty(
                                    &validator_manager,
                                    &selected_validator.address,
                                );
                            }
                        }
                    } else {
                        consecutive_failures += 1;
                        if consecutive_failures > 10 {
                            println!("⚠️ No genesis block found. Please check blockchain initialization.");
                            thread::sleep(Duration::from_secs(block_time_secs));
                        }
                    }
                }

                thread::sleep(Duration::from_millis(100));
            }
        });
    }

    fn initialize_genesis_validators(validator_manager: &Arc<ValidatorManager>) {
        println!("🔧 INITIALIZE_GENESIS_VALIDATORS CALLED - START");
        match canonical_genesis() {
            Ok(genesis) => {
                println!(
                    "🔧 Found {} canonical genesis validators",
                    genesis.validators().len()
                );
                for validator in genesis.validators() {
                    let address = validator.operator_address.as_str();
                    let registration = crate::validator::ValidatorRegistration {
                        address: validator.operator_address.clone(),
                        public_key: validator.consensus_public_key.clone(),
                        name: validator.moniker.clone(),
                        stake_amount: validator.stake_nwei,
                        submitted_at: Self::current_timestamp(),
                        registration_tx_hash: "genesis".to_string(),
                    };

                    if validator_manager.get_validator(address).is_none() {
                        match validator_manager.register_validator(registration) {
                            Ok(_) => {
                                if let Err(error) = validator_manager.approve_validator(address) {
                                    println!(
                                        "⚠️ Failed to approve genesis validator {}: {}",
                                        address, error
                                    );
                                    continue;
                                }
                                println!(
                                    "✅ Genesis validator {} registered and approved",
                                    address
                                );
                            }
                            Err(error) => {
                                println!(
                                    "⚠️ Failed to register genesis validator {}: {}",
                                    address, error
                                );
                                continue;
                            }
                        }
                    }

                    validator_manager.update_validator_stake(address, validator.stake_nwei);
                }
            }
            Err(error) => {
                println!("⚠️ Failed to load canonical genesis validators: {}", error);
            }
        }
        println!("🔧 INITIALIZE_GENESIS_VALIDATORS CALLED - END");
    }

    fn resolve_local_validator_address() -> Option<String> {
        crate::config::resolve_runtime_validator_address()
    }

    fn collect_live_validator_addresses(validator_manager: &Arc<ValidatorManager>) -> Vec<String> {
        let active_validator_addresses = validator_manager
            .get_active_validators()
            .into_iter()
            .map(|validator| validator.address)
            .collect::<HashSet<_>>();
        let mut live_validator_addresses = HashSet::new();

        if let Some(local_validator_address) = Self::resolve_local_validator_address() {
            if active_validator_addresses.contains(&local_validator_address) {
                live_validator_addresses.insert(local_validator_address);
            }
        }

        if let Some(network) = crate::p2p::get_p2p_network() {
            for validator_address in network.get_connected_validator_addresses() {
                if active_validator_addresses.contains(&validator_address) {
                    live_validator_addresses.insert(validator_address);
                }
            }
        }

        let mut live_validator_addresses = live_validator_addresses.into_iter().collect::<Vec<_>>();
        live_validator_addresses.sort();
        live_validator_addresses
    }

    fn ensure_genesis_validator_stakes(validator_manager: &Arc<ValidatorManager>) {
        println!("🔧 ENSURING_GENESIS_VALIDATOR_STAKES - START");
        match canonical_genesis() {
            Ok(genesis) => {
                for validator in genesis.validators() {
                    let address = validator.operator_address.as_str();
                    if validator_manager.get_validator(address).is_none() {
                        let registration = crate::validator::ValidatorRegistration {
                            address: validator.operator_address.clone(),
                            public_key: validator.consensus_public_key.clone(),
                            name: validator.moniker.clone(),
                            stake_amount: validator.stake_nwei,
                            submitted_at: Self::current_timestamp(),
                            registration_tx_hash: "genesis".to_string(),
                        };

                        match validator_manager.register_validator(registration) {
                            Ok(_) => {
                                if let Err(error) = validator_manager.approve_validator(address) {
                                    println!(
                                        "⚠️ Failed to approve late-joined genesis validator {}: {}",
                                        address, error
                                    );
                                    continue;
                                }
                                println!(
                                    "✅ Late-joined genesis validator {} registered and approved",
                                    address
                                );
                            }
                            Err(error) => {
                                println!(
                                    "⚠️ Failed to register late-joined genesis validator {}: {}",
                                    address, error
                                );
                                continue;
                            }
                        }
                    }

                    validator_manager.update_validator_stake(address, validator.stake_nwei);
                }
            }
            Err(error) => {
                println!(
                    "⚠️ Failed to ensure canonical genesis validator stakes: {}",
                    error
                );
            }
        }
        println!("🔧 ENSURING_GENESIS_VALIDATOR_STAKES - END");
    }

    fn load_synergy_scores() -> Option<SynergyScores> {
        let scores_path = "data/synergy_scores.json";
        if std::path::Path::new(scores_path).exists() {
            if let Ok(contents) = std::fs::read_to_string(scores_path) {
                if let Ok(scores) = serde_json::from_str::<SynergyScores>(&contents) {
                    return Some(scores);
                }
            }
        }
        None
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    // New PoSy Helper Methods

    fn is_epoch_boundary(block_index: u64, epoch_length: u64) -> bool {
        // Epoch boundary occurs when block_index is divisible by epoch_length
        // Epoch 0: blocks 0-(epoch_length-1), Epoch 1: next epoch_length blocks, etc.
        block_index > 0 && block_index % epoch_length == 0
    }

    fn handle_epoch_transition(
        current_epoch: &mut u64,
        chain: &BlockChain,
        validator_manager: &Arc<ValidatorManager>,
        synergy_calculator: &Arc<SynergyScoreCalculator>,
        dual_quorum_consensus: &Arc<Mutex<DualQuorumConsensus>>,
        entropy_beacon: &Arc<Mutex<EntropyBeacon>>,
        validator_rotation: &Arc<ValidatorRotation>,
        dao_governance: &Arc<Mutex<DAOGovernance>>,
        cartel_detection: &Arc<Mutex<CartelDetectionEngine>>,
        epoch_length: u64,
    ) {
        *current_epoch += 1;
        println!("🔄 Epoch Transition: Starting epoch {}", current_epoch);

        // 1. Generate new epoch randomness
        let previous_qc = Self::get_previous_quorum_certificate(chain, epoch_length);
        let mut beacon = entropy_beacon.lock().unwrap();
        let _epoch_randomness = beacon.generate_epoch_randomness(&previous_qc);
        drop(beacon);

        // 2. Rotate validators using new entropy
        validator_rotation.rotate_validators();

        // 3. Recalculate synergy scores
        Self::recalculate_all_synergy_scores(validator_manager, synergy_calculator);

        // 4. Detect cartels and apply penalties
        let mut cartel_engine = cartel_detection.lock().unwrap();
        let cartel_penalties = cartel_engine.detect_cartels(*current_epoch);
        cartel_engine.apply_cartel_penalties(&cartel_penalties);

        // 5. Update governance proposals
        let mut governance = dao_governance.lock().unwrap();
        Self::update_governance_proposals(&mut governance, *current_epoch);

        // 6. Reset dual quorum consensus state
        let mut consensus = dual_quorum_consensus.lock().unwrap();
        consensus.current_epoch = *current_epoch;

        println!("🔄 Epoch Transition: Completed epoch {}", current_epoch);
    }

    fn get_previous_quorum_certificate(chain: &BlockChain, epoch_length: u64) -> QuorumCertificate {
        // Retrieve the actual QC from the chain at the epoch boundary
        // For now, we'll create a QC based on the most recent block in the chain
        if let Some(block) = chain.last() {
            // Create a placeholder QC based on the block's info
            // In the actual implementation, blocks would contain QCs from consensus
            QuorumCertificate {
                block_hash: block.hash.clone(),
                epoch_number: block.block_index / epoch_length.max(1), // Calculate epoch from block index
                round_number: 1,
                aggregate_signature: block.block_signature.clone(),
                participant_bitmap: vec![0xFF], // Placeholder bitmap
                cumulative_weight: 1.0,         // Placeholder weight
                validation_quorum_met: true,
                cooperation_quorum_met: true,
                timestamp: Self::current_timestamp(),
            }
        } else {
            // Genesis case - return default QC
            QuorumCertificate {
                block_hash: "genesis_block".to_string(),
                epoch_number: 0,
                round_number: 0,
                aggregate_signature: Vec::new(),
                participant_bitmap: Vec::new(),
                cumulative_weight: 0.0,
                validation_quorum_met: true,
                cooperation_quorum_met: true,
                timestamp: Self::current_timestamp(),
            }
        }
    }

    fn select_leader_for_block(
        validators: &[Validator],
        block_height: u64,
        synergy_calculator: &Arc<SynergyScoreCalculator>,
        entropy_beacon: &Arc<Mutex<EntropyBeacon>>,
        epoch_length: u64,
        view_offset: usize,
    ) -> Validator {
        consensus_log!(
            "🔍 [select_leader_for_block] START - block_height: {}, validators: {}",
            block_height,
            validators.len()
        );

        if validators.is_empty() {
            println!("⚠️ [select_leader_for_block] No validators, returning genesis validator");
            return Validator::new(
                "synv1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t1".to_string(),
                "genesis_key".to_string(),
                "Genesis Validator".to_string(),
                1000,
            );
        }

        // Calculate current epoch from configured epoch length.
        let current_epoch = block_height / epoch_length;
        let block_in_epoch = block_height % epoch_length;

        // Check if we need to recalculate leader priorities (at epoch start or if not initialized)
        let mut rotation = EPOCH_LEADER_ROTATION.lock().unwrap();
        let needs_recalculation = rotation.0 != current_epoch || rotation.1.is_empty();

        if needs_recalculation {
            consensus_log!(
                "🔄 [select_leader_for_block] Recalculating leader priorities for epoch {}",
                current_epoch
            );

            // Get current epoch randomness
            let beacon = entropy_beacon.lock().unwrap();
            let epoch_randomness = beacon.epoch_randomness.clone();
            drop(beacon);

            // Calculate priority for each validator using Equation 17 from PoSy spec
            let mut validator_priorities = Vec::new();

            consensus_log!(
                "🔄 [select_leader_for_block] Calculating priorities for {} validators",
                validators.len()
            );
            for validator in validators.iter() {
                // Calculate priority: H(r_e || validatorID_v) * SS_v,normalized (PoSy Equation 17)
                let mut hasher = Sha3_512::new();
                hasher.update(&epoch_randomness);
                hasher.update(validator.address.as_bytes());
                let hash = hasher.finalize();

                // Get normalized synergy score
                let components = synergy_calculator.calculate_synergy_score(validator);
                let normalized_score = components.normalized_score;

                // Calculate priority value
                let priority_value =
                    u64::from_be_bytes(hash[..8].try_into().unwrap()) as f64 * normalized_score;

                validator_priorities.push((validator.clone(), priority_value));
            }

            // Sort by priority value (descending)
            validator_priorities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

            // Select top K validators for round-robin (K = min(10, |validators|) as per PoSy)
            // Use all validators if we have 3 or fewer, otherwise use min(10, validators.len())
            let k = std::cmp::min(10, validators.len());
            let top_k_addresses: Vec<String> = validator_priorities
                .iter()
                .take(k)
                .map(|(v, _)| v.address.clone())
                .collect();

            info!("consensus", "Selected top K validators for epoch", 
                  "k" => k, 
                  "epoch" => current_epoch, 
                  "validators" => format!("{:?}", top_k_addresses));
            println!(
                "🏆 [select_leader_for_block] Selected top {} validators for epoch {}: {:?}",
                k, current_epoch, top_k_addresses
            );
            consensus_log!(
                "🏆 [select_leader_for_block] Selected top {} validators for epoch {}: {:?}",
                k,
                current_epoch,
                top_k_addresses
            );

            // Update rotation state
            rotation.0 = current_epoch;
            rotation.1 = top_k_addresses;
            rotation.2 = 0; // Reset index for new epoch
        }

        // Use round-robin within epoch (PoSy: top K validators rotate).
        // view_offset is added so that when the primary leader times out, the next
        // candidate in the sorted list is tried without waiting for the next block.
        let rotation_index = (block_in_epoch as usize + view_offset) % rotation.1.len();
        let leader_address = rotation.1[rotation_index].clone();
        // Update stored index for logging/debugging.
        rotation.2 = rotation_index + 1;
        drop(rotation);

        // Find and return the selected validator
        let selected_validator = validators.iter()
            .find(|v| v.address == leader_address)
            .cloned()
            .unwrap_or_else(|| {
                // Fallback if validator not found (shouldn't happen)
                println!("⚠️ [select_leader_for_block] Selected leader {} not found, using first validator", leader_address);
                consensus_log!("⚠️ [select_leader_for_block] Selected leader {} not found, using first validator", leader_address);
                validators[0].clone()
            });

        info!("consensus", "Selected leader for block",
              "block_height" => block_height,
              "epoch" => current_epoch,
              "block_in_epoch" => block_in_epoch,
              "rotation_index" => rotation_index,
              "view_offset" => view_offset,
              "leader" => selected_validator.address.clone());
        println!("🏆 [select_leader_for_block] Selected leader for block {} (epoch {}, block_in_epoch {}, rotation_index {}): {}", 
                      block_height, current_epoch, block_in_epoch, rotation_index, selected_validator.address);
        consensus_log!(
            "🏆 [select_leader_for_block] Selected leader for block {} (epoch {}, block {}): {}",
            block_height,
            current_epoch,
            block_in_epoch,
            selected_validator.address
        );
        selected_validator
    }

    fn create_block_proposal(
        previous_block: &Block,
        leader: &Validator,
        transactions: Vec<crate::transaction::Transaction>,
        pqc_manager: &Arc<Mutex<PQCManager>>,
    ) -> Block {
        // Create block and attach a real FN-DSA signature over the block hash.
        let mut block = Block::new(
            previous_block.block_index + 1,
            transactions,
            previous_block.hash.clone(),
            leader.address.clone(),
            previous_block.nonce + 1, // Simple nonce increment
        );

        let mut pqc = pqc_manager.lock().unwrap();
        let (leader_public_key, leader_private_key) =
            Self::get_or_create_leader_keypair(&leader.address, &mut pqc);

        if let Ok(signature) = pqc.sign(&leader_private_key, block.hash.as_bytes()) {
            block.proposer_public_key = leader_public_key.key_data;
            block.block_signature = signature.signature_data;
            block.block_signature_algorithm = "fndsa".to_string();
        }

        block
    }

    fn execute_dual_quorum_consensus(
        block: &Block,
        _validator_manager: &Arc<ValidatorManager>,
        dual_quorum_consensus: &Arc<Mutex<DualQuorumConsensus>>,
        current_epoch: u64,
        view_offset: usize,
    ) -> Result<QuorumCertificate, String> {
        consensus_log!(
            "🔒 [execute_dual_quorum_consensus] Attempting to lock dual_quorum_consensus..."
        );
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
        let mut consensus = dual_quorum_consensus.lock().unwrap();
        consensus_log!(
            "✅ [execute_dual_quorum_consensus] Locked! Setting current_epoch to {}",
            current_epoch
        );
        io::stdout().flush().unwrap();
        consensus.current_epoch = current_epoch;
        let minimum_round_number = (view_offset as u64).saturating_add(1);

        consensus_log!("📞 [execute_dual_quorum_consensus] Calling start_consensus_round...");
        io::stdout().flush().unwrap();
        // Execute the dual-quorum consensus process
        let result = consensus.start_consensus_round(block, minimum_round_number);
        consensus_log!("✅ [execute_dual_quorum_consensus] start_consensus_round returned!");
        io::stdout().flush().unwrap();
        result
    }

    fn validate_transaction(
        tx: &crate::transaction::Transaction,
        pqc_manager: &Arc<Mutex<PQCManager>>,
    ) -> bool {
        // 1. Verify transaction signature (best-effort)
        // NOTE: Testnet Beta currently allows transactions even if the sender public key
        // isn't known locally (remote wallets). If a public key is available, we verify.
        let public_key = Self::get_transaction_public_key(&tx.sender);
        if let Some(public_key) = public_key {
            let pqc = pqc_manager.lock().unwrap();
            // Use raw_hash() for signature verification (without prefix)
            let message_bytes = match hex::decode(tx.raw_hash()) {
                Ok(bytes) => bytes,
                Err(_) => return false,
            };

            let signature_obj = crate::crypto::pqc::PQCSignature {
                algorithm: crate::crypto::pqc::PQCAlgorithm::FNDSA,
                signature_data: tx.signature.clone(),
                message_hash: message_bytes.clone(),
                public_key_id: public_key.key_id.clone(),
                created_at: tx.timestamp,
            };

            let signature_valid = pqc
                .verify(&public_key, &signature_obj, &message_bytes)
                .unwrap_or(false);

            if !signature_valid {
                return false;
            }
        } else {
            warn!(
                "consensus",
                "No public key for sender - skipping signature verification",
                "sender" => tx.sender.clone()
            );
        }

        // 2. Verify sender balance via token manager to reflect on-chain state
        let token_manager = TOKEN_MANAGER.clone();
        let required = tx.amount.saturating_add(tx.get_fee());
        if token_manager.get_balance(&tx.sender, "SNRG") < required {
            return false;
        }

        // 3. Verify nonce using wallet manager metadata when available
        if let Ok(wallet_manager) = WALLET_MANAGER.lock() {
            if let Some(wallet) = wallet_manager.get_wallet(&tx.sender) {
                let expected_nonce = wallet.nonce.saturating_sub(1);
                if tx.nonce != expected_nonce {
                    return false;
                }
            }
        }

        // 4. Execute contract if applicable (simplified)
        if tx.receiver.starts_with("contract_") {
            // Execute contract in sandboxed environment
            // Verify state changes
            // For now, assume valid
        }

        true
    }

    fn update_synergy_scores(
        validator_manager: &Arc<ValidatorManager>,
        synergy_calculator: &Arc<SynergyScoreCalculator>,
        validator_address: &str,
    ) {
        if let Some(validator) = validator_manager.get_validator(validator_address) {
            let components = synergy_calculator.calculate_synergy_score(&validator);
            validator_manager.update_synergy_score(validator_address, components.normalized_score);
            println!(
                "📊 Updated synergy score for {}: {:.2}",
                validator_address, components.normalized_score
            );
        }
    }

    fn record_vote_for_cartel_detection(
        cartel_detection: &Arc<Mutex<CartelDetectionEngine>>,
        validator_address: &str,
        block_height: u64,
        voted_for_winner: bool,
        timestamp: u64,
        epoch_length: u64,
    ) {
        let mut engine = cartel_detection.lock().unwrap();
        let current_epoch = block_height / epoch_length.max(1);

        let vote_record = VoteRecord {
            validator_address: validator_address.to_string(),
            block_height,
            voted_for_winner,
            vote_timestamp: timestamp,
            signature: Vec::new(), // Simplified
        };

        engine.record_vote(current_epoch, vote_record);
    }

    fn check_governance_proposals(dao_governance: &Arc<Mutex<DAOGovernance>>, block_index: u64) {
        let mut governance = dao_governance.lock().unwrap();

        // Collect proposals that need transition (to avoid borrow checker issues)
        let proposals_to_transition: Vec<(String, ProposalStatus, u64, u64, u64)> = governance
            .proposals
            .iter()
            .map(|(id, p)| {
                (
                    id.clone(),
                    p.status.clone(),
                    p.discussion_end,
                    p.voting_end,
                    p.execution_timestamp,
                )
            })
            .collect();

        // Check if any proposals need transition
        for (proposal_id, status, discussion_end, voting_end, execution_timestamp) in
            proposals_to_transition
        {
            if status == ProposalStatus::Discussion && block_index >= discussion_end {
                governance.transition_proposal_to_voting(&proposal_id).ok();
            }

            if status == ProposalStatus::Voting && block_index >= voting_end {
                governance.finalize_voting(&proposal_id).ok();
            }

            if status == ProposalStatus::Approved && block_index >= execution_timestamp {
                governance.execute_approved_proposal(&proposal_id).ok();
            }
        }
    }

    fn apply_proposer_penalty(validator_manager: &Arc<ValidatorManager>, validator_address: &str) {
        // Must mutate through the registry lock so the change is actually persisted.
        if let Ok(mut registry) = validator_manager.registry.lock() {
            if let Some(validator) = registry.validators.get_mut(validator_address) {
                validator.reputation_score = (validator.reputation_score * 0.99).max(0.0);
                validator.calculate_synergy_score();
                println!(
                    "⚠️ Applied proposer penalty to {}: reputation reduced to {:.2}, synergy score now {:.2}",
                    validator_address, validator.reputation_score, validator.synergy_score
                );
            }
        }
    }

    /// Called when the view-change timer fires because the scheduled leader failed to
    /// broadcast a block proposal within the timeout window.  Uses the existing
    /// `record_missed_block` path so that uptime, accuracy, reputation, and slashing
    /// penalty are all updated consistently with the rest of the PoSy rules.
    fn apply_leader_timeout_penalty(
        validator_manager: &Arc<ValidatorManager>,
        validator_address: &str,
        block_height: u64,
        view_offset: usize,
    ) {
        if let Ok(mut registry) = validator_manager.registry.lock() {
            if let Some(validator) = registry.validators.get_mut(validator_address) {
                // record_missed_block → record_missed_vote: decrements uptime/accuracy/reputation
                // and increments slashing_penalty + missed_vote_window atomically.
                validator.record_missed_block();

                let new_score = validator.synergy_score;
                let new_rep = validator.reputation_score;
                let new_penalty = validator.slashing_penalty;
                let missed_window = validator.missed_vote_window;

                warn!(
                    "consensus",
                    "Leader timeout penalty applied",
                    "validator" => validator_address,
                    "block_height" => block_height,
                    "view_offset" => view_offset,
                    "synergy_score" => format!("{:.4}", new_score),
                    "reputation_score" => format!("{:.4}", new_rep),
                    "slashing_penalty" => format!("{:.4}", new_penalty),
                    "missed_vote_window" => missed_window
                );
            }
        }
    }

    fn recalculate_all_synergy_scores(
        validator_manager: &Arc<ValidatorManager>,
        synergy_calculator: &Arc<SynergyScoreCalculator>,
    ) {
        let validators = validator_manager.get_active_validators();

        for validator in validators {
            let components = synergy_calculator.calculate_synergy_score(&validator);
            validator_manager.update_synergy_score(&validator.address, components.normalized_score);
        }

        println!("📊 Recalculated synergy scores for all validators");
    }

    fn update_governance_proposals(governance: &mut DAOGovernance, current_epoch: u64) {
        // Check for expired proposals
        let expired_proposals: Vec<String> = governance
            .proposals
            .iter()
            .filter(|(_, proposal)| {
                proposal.status != ProposalStatus::Executed
                    && proposal.status != ProposalStatus::Rejected
                    && current_epoch > (proposal.execution_timestamp / 1000) as u64 + 1
            })
            .map(|(id, _)| id.clone())
            .collect();

        for proposal_id in expired_proposals {
            governance
                .update_proposal_status(&proposal_id, ProposalStatus::Expired)
                .ok();
        }
    }

    fn get_or_create_leader_keypair(
        validator_address: &str,
        pqc_manager: &mut PQCManager,
    ) -> (PQCPublicKey, PQCPrivateKey) {
        if let Ok(cache) = EPHEMERAL_LEADER_KEYS.lock() {
            if let Some((public_key, private_key)) = cache.get(validator_address) {
                return (public_key.clone(), private_key.clone());
            }
        }

        let generated = pqc_manager
            .generate_keypair(PQCAlgorithm::FNDSA)
            .unwrap_or_else(|_| {
                (
                    PQCPublicKey {
                        algorithm: PQCAlgorithm::FNDSA,
                        key_data: Vec::new(),
                        key_id: format!("fallback_pub_{validator_address}"),
                        created_at: Self::current_timestamp(),
                    },
                    PQCPrivateKey {
                        algorithm: PQCAlgorithm::FNDSA,
                        key_data: Vec::new(),
                        public_key_id: format!("fallback_pub_{validator_address}"),
                        created_at: Self::current_timestamp(),
                    },
                )
            });

        if let Ok(mut cache) = EPHEMERAL_LEADER_KEYS.lock() {
            cache.insert(
                validator_address.to_string(),
                (generated.0.clone(), generated.1.clone()),
            );
        }

        generated
    }

    fn get_transaction_public_key(address: &str) -> Option<crate::crypto::pqc::PQCPublicKey> {
        if let Ok(wallet_manager) = WALLET_MANAGER.lock() {
            if let Some(wallet) = wallet_manager.get_wallet(address) {
                // Public keys are stored as base64 in identity.json; support both hex and base64.
                let key_bytes = hex::decode(&wallet.public_key)
                    .or_else(|_| general_purpose::STANDARD.decode(wallet.public_key.as_bytes()));
                if let Ok(key_bytes) = key_bytes {
                    return Some(PQCPublicKey {
                        algorithm: PQCAlgorithm::FNDSA,
                        key_data: key_bytes,
                        key_id: format!("wallet_{}", address),
                        created_at: wallet.created_at,
                    });
                }
            }
        }
        None
    }
}
