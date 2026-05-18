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
use crate::p2p::networking::P2PNetwork;
use crate::rpc::rpc_server::{
    prune_transaction_hashes_from_pool, transaction_hashes, SHARED_CHAIN, SYNC_MANAGER, TX_POOL,
};
use crate::token::TOKEN_MANAGER;
use crate::validator::{
    apply_validator_activation_transaction, consensus_membership_validators,
    is_validator_activation_transaction, replay_validator_activation_transactions, Validator,
    ValidatorManager, TESTNET_VALIDATOR_CLUSTER_SIZE, VALIDATOR_MANAGER,
};
use crate::wallet::WALLET_MANAGER;
use crate::{debug, info, warn};
use base64::{engine::general_purpose, Engine as _};
use hex;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_512};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

fn staking_payload(tx: &crate::transaction::Transaction) -> Option<serde_json::Value> {
    let data = tx.data.as_deref()?;
    let payload = data.strip_prefix("stake:")?;
    serde_json::from_str::<serde_json::Value>(payload).ok()
}

fn staking_validator_address(tx: &crate::transaction::Transaction) -> Option<String> {
    staking_payload(tx)?
        .get("validator")
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn staking_amount_nwei(tx: &crate::transaction::Transaction) -> Option<u64> {
    staking_payload(tx)?
        .get("amount")
        .and_then(|value| value.as_u64())
}

fn snrg_balance_required_for_transaction(tx: &crate::transaction::Transaction) -> u64 {
    if tx
        .data
        .as_deref()
        .map(|data| data.starts_with("stake:"))
        .unwrap_or(false)
    {
        return staking_amount_nwei(tx).unwrap_or(tx.amount);
    }

    tx.amount.saturating_add(tx.get_fee())
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
    pub status_ready_gate_enabled: bool,
    pub status_ready_min_validators: usize,
    pub status_ready_genesis_grace_secs: u64,
    pub allow_genesis_status_bypass: bool,
    pub mesh_settle_secs: u64,
    pub leader_timeout_secs: u64,
    pub vote_timeout_secs: u64,
    pub block_timeout_secs: u64,
    pub penalization_enabled: bool,
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
    static ref EPOCH_LEADER_ROTATION: Arc<Mutex<(u64, Vec<String>, usize, Vec<String>)>> =
        Arc::new(Mutex::new((0, Vec::new(), 0, Vec::new()))); // (epoch, top_k_validators, current_index, candidate_set)
    static ref EPHEMERAL_LEADER_KEYS: Arc<Mutex<HashMap<String, (PQCPublicKey, PQCPrivateKey)>>> =
        Arc::new(Mutex::new(HashMap::new()));
    static ref PROPOSAL_CACHE_LOCK: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    static ref LAST_CONSENSUS_CHAIN_PERSIST: Arc<Mutex<Option<(u64, Instant)>>> =
        Arc::new(Mutex::new(None));
    static ref CONSENSUS_CHAIN_PERSIST_IN_FLIGHT: AtomicBool = AtomicBool::new(false);
}

#[cfg(test)]
lazy_static::lazy_static! {
    static ref TEST_PROPOSAL_CACHE_DIR: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));
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

        let chain_snapshot = {
            let chain_guard = chain.lock().unwrap();
            chain_guard.clone()
        };
        let token_manager = TOKEN_MANAGER.clone();
        let (activation_replayed, activation_failed) = replay_validator_activation_transactions(
            &chain_snapshot,
            &token_manager,
            &validator_manager,
        );
        if activation_replayed > 0 || activation_failed > 0 {
            info!(
                "consensus",
                "Replayed validator activation transactions into registry",
                "replayed" => activation_replayed,
                "failed" => activation_failed
            );
        }
        if activation_replayed > 0 {
            if let Err(error) = validator_manager.save_registry(VALIDATOR_REGISTRY_PATH) {
                warn!(
                    "consensus",
                    "Failed to persist replayed validator activations",
                    "error" => error.to_string()
                );
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

        // Load consensus timing from env/config for deterministic testnet tuning.
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
            .unwrap_or(3)
            .max(1);

        let validator_vote_threshold = std::env::var("SYNERGY_CONSENSUS_VALIDATOR_VOTE_THRESHOLD")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .or_else(|| consensus_cfg.as_ref().map(|c| c.validator_vote_threshold))
            .unwrap_or(3)
            .max(1);

        let status_ready_gate_enabled = consensus_cfg
            .as_ref()
            .map(|c| c.status_ready_gate_enabled)
            .unwrap_or(true);
        let status_ready_min_validators = consensus_cfg
            .as_ref()
            .map(|c| c.status_ready_min_validators)
            .unwrap_or(0);
        let status_ready_genesis_grace_secs = consensus_cfg
            .as_ref()
            .map(|c| c.status_ready_genesis_grace_secs)
            .unwrap_or(15);
        let allow_genesis_status_bypass = consensus_cfg
            .as_ref()
            .map(|c| c.allow_genesis_status_bypass)
            .unwrap_or(true);
        let mesh_settle_secs = consensus_cfg
            .as_ref()
            .map(|c| c.mesh_settle_secs)
            .unwrap_or(3);
        let leader_timeout_secs = consensus_cfg
            .as_ref()
            .map(|c| c.leader_timeout_secs)
            .unwrap_or(0);
        let vote_timeout_secs = consensus_cfg
            .as_ref()
            .map(|c| c.vote_timeout_secs)
            .unwrap_or(8)
            .max(1);
        let block_timeout_secs = consensus_cfg
            .as_ref()
            .map(|c| c.block_timeout_secs)
            .unwrap_or(5)
            .max(1);
        let penalization_enabled = consensus_cfg
            .as_ref()
            .map(|c| c.penalization_enabled)
            .unwrap_or(true);

        // Initialize dual quorum consensus after loading the minimum validator requirement.
        let dual_quorum_consensus = Arc::new(Mutex::new(DualQuorumConsensus::new(
            Arc::clone(&validator_manager),
            Arc::clone(&pqc_manager),
            penalization_enabled,
            min_validators,
            validator_vote_threshold,
            vote_timeout_secs,
            block_timeout_secs,
        )));

        let cluster_size = consensus_cfg
            .as_ref()
            .map(|c| c.validator_cluster_size)
            .unwrap_or(TESTNET_VALIDATOR_CLUSTER_SIZE);
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
            status_ready_gate_enabled,
            status_ready_min_validators,
            status_ready_genesis_grace_secs,
            allow_genesis_status_bypass,
            mesh_settle_secs,
            leader_timeout_secs,
            vote_timeout_secs,
            block_timeout_secs,
            penalization_enabled,
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
        let active_validators =
            consensus_membership_validators(self.validator_manager.get_active_validators());
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
        println!(
            "🔧 Status-ready gate: enabled={}, required={}, genesis_grace_secs={}, allow_genesis_bypass={}",
            self.status_ready_gate_enabled,
            if self.status_ready_min_validators == 0 {
                self.min_validators
            } else {
                self.status_ready_min_validators.max(1)
            },
            self.status_ready_genesis_grace_secs,
            false
        );
        println!(
            "🔧 Mesh settle/window timeouts: settle_secs={}, leader_timeout_secs={}, vote_timeout_secs={}, block_timeout_secs={}",
            self.mesh_settle_secs,
            self.effective_leader_timeout_secs(),
            self.vote_timeout_secs,
            self.block_timeout_secs
        );
        println!(
            "🔧 Validator penalization enabled: {}",
            self.penalization_enabled
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
        let status_ready_gate_enabled = self.status_ready_gate_enabled;
        let status_ready_required_validators = if self.status_ready_min_validators == 0 {
            min_validators
        } else {
            self.status_ready_min_validators.max(1)
        };
        let status_ready_genesis_grace_secs = self.status_ready_genesis_grace_secs;
        let allow_genesis_status_bypass = false;
        let mesh_settle_secs = self.mesh_settle_secs;
        let penalization_enabled = self.penalization_enabled;
        let leader_timeout_secs = self.effective_leader_timeout_secs();

        thread::spawn(move || {
            let mut last_block_time = chain
                .lock()
                .unwrap()
                .last()
                .map(|block| Self::system_time_from_unix_timestamp(block.timestamp))
                .unwrap_or_else(SystemTime::now);
            let mut consecutive_failures = 0;
            let mut current_epoch = chain
                .lock()
                .unwrap()
                .last()
                .map(|block| Self::epoch_for_block(block.block_index, epoch_length))
                .unwrap_or(0);
            if let Ok(mut consensus) = dual_quorum_consensus.lock() {
                consensus.current_epoch = current_epoch;
            }
            let mut mesh_ready_since: Option<Instant> = None;
            let mut status_sync_grace_since: Option<Instant> = None;
            let mut genesis_status_gate_bypassed = false;
            let mut last_committed_height: u64 = 0;
            let mut last_logged_view_timeout: Option<(u64, usize)> = None;

            loop {
                let current_time = SystemTime::now();
                let elapsed = current_time
                    .duration_since(last_block_time)
                    .unwrap_or_default();

                if elapsed >= Duration::from_secs(block_time_secs) {
                    let pool = TX_POOL.lock().unwrap();
                    let chain_guard = chain.lock().unwrap();

                    if let Some(latest_block) = chain_guard.last() {
                        if latest_block.block_index != last_committed_height {
                            last_committed_height = latest_block.block_index;
                            last_logged_view_timeout = None;
                            last_block_time =
                                Self::system_time_from_unix_timestamp(latest_block.timestamp);
                            drop(chain_guard);
                            drop(pool);
                            thread::sleep(Duration::from_millis(100));
                            continue;
                        }

                        let target_epoch =
                            Self::epoch_for_next_block(latest_block.block_index, epoch_length);
                        while current_epoch < target_epoch {
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

                        // Get active validators, then reduce them to the shared consensus
                        // membership before leader or quorum math uses the set.
                        let registry_active_validators = validator_manager.get_active_validators();
                        let registry_active_count = registry_active_validators.len();
                        let active_validators =
                            consensus_membership_validators(registry_active_validators);
                        let consensus_active_count = active_validators.len();
                        let live_validator_addresses =
                            Self::collect_live_validator_addresses(&validator_manager);
                        let live_validator_address_set = live_validator_addresses
                            .iter()
                            .cloned()
                            .collect::<HashSet<_>>();
                        let live_active_validators: Vec<Validator> = active_validators
                            .iter()
                            .cloned()
                            .filter(|validator| {
                                live_validator_address_set.contains(&validator.address)
                            })
                            .collect();
                        consensus_log!(
                            "🔍 Found {} registry-active validators, {} consensus members, and {} live validator participants",
                            registry_active_count,
                            consensus_active_count,
                            live_active_validators.len()
                        );

                        if live_active_validators.len() < min_validators {
                            mesh_ready_since = None;
                            status_sync_grace_since = None;
                            genesis_status_gate_bypassed = false;
                            println!(
                                "⏳ Insufficient live validators for block production: {} live, {} consensus-active, {} registry-active, {} required.",
                                live_active_validators.len(),
                                consensus_active_count,
                                registry_active_count,
                                min_validators
                            );
                            drop(chain_guard);
                            drop(pool);
                            thread::sleep(Duration::from_secs(1));
                            continue;
                        }

                        if let Some(network) = crate::p2p::get_p2p_network() {
                            if status_ready_gate_enabled {
                                let status_ready_validators = live_validator_addresses.len();
                                let is_genesis_height = latest_block.block_index == 0;
                                if !is_genesis_height {
                                    genesis_status_gate_bypassed = false;
                                }
                                if status_ready_validators < status_ready_required_validators
                                    && !(is_genesis_height && genesis_status_gate_bypassed)
                                {
                                    match status_sync_grace_since {
                                        Some(grace_since)
                                            if allow_genesis_status_bypass
                                                && is_genesis_height
                                                && grace_since.elapsed()
                                                    >= Duration::from_secs(
                                                        status_ready_genesis_grace_secs,
                                                    ) =>
                                        {
                                            genesis_status_gate_bypassed = true;
                                            warn!(
                                                "consensus",
                                                "Bypassing validator mesh status gate at genesis after grace window",
                                                "live_validators" => live_active_validators.len() as u64,
                                                "status_ready_validators" => status_ready_validators as u64,
                                                "required_validators" => status_ready_required_validators as u64,
                                                "grace_secs" => status_ready_genesis_grace_secs
                                            );
                                            status_sync_grace_since = Some(grace_since);
                                        }
                                        Some(_) => {
                                            mesh_ready_since = None;
                                            info!(
                                                "consensus",
                                                "Waiting for validator mesh status sync before block production",
                                                "status_ready_validators" => status_ready_validators as u64,
                                                "required_validators" => status_ready_required_validators as u64,
                                                "grace_secs" => status_ready_genesis_grace_secs
                                            );
                                            drop(chain_guard);
                                            drop(pool);
                                            thread::sleep(Duration::from_secs(1));
                                            continue;
                                        }
                                        None => {
                                            status_sync_grace_since = Some(Instant::now());
                                            mesh_ready_since = None;
                                            info!(
                                                "consensus",
                                                "Waiting for validator mesh status sync before block production",
                                                "status_ready_validators" => status_ready_validators as u64,
                                                "required_validators" => status_ready_required_validators as u64,
                                                "grace_secs" => status_ready_genesis_grace_secs
                                            );
                                            drop(chain_guard);
                                            drop(pool);
                                            thread::sleep(Duration::from_secs(1));
                                            continue;
                                        }
                                    }
                                } else {
                                    status_sync_grace_since = None;
                                }
                            } else {
                                status_sync_grace_since = None;
                                genesis_status_gate_bypassed = false;
                            }

                            let required_sync_support =
                                status_ready_required_validators.saturating_sub(1).max(1);
                            let best_validator_height = network
                                .get_best_validator_peer_height_with_support(required_sync_support);
                            let local_height = latest_block.block_index;
                            if best_validator_height > local_height {
                                mesh_ready_since = None;
                                status_sync_grace_since = None;
                                drop(chain_guard);
                                drop(pool);
                                Self::sync_validator_to_network_tip(
                                    &network,
                                    local_height,
                                    best_validator_height,
                                    required_sync_support,
                                );
                                last_block_time = SystemTime::now();
                                continue;
                            }

                            match mesh_ready_since {
                                Some(ready_since)
                                    if ready_since.elapsed()
                                        >= Duration::from_secs(mesh_settle_secs) => {}
                                Some(_) => {
                                    info!(
                                        "consensus",
                                        "Validator mesh is settling before block production",
                                        "settle_secs" => mesh_settle_secs
                                    );
                                    drop(chain_guard);
                                    drop(pool);
                                    thread::sleep(Duration::from_millis(500));
                                    continue;
                                }
                                None => {
                                    mesh_ready_since = Some(Instant::now());
                                    info!(
                                        "consensus",
                                        "Validator mesh reached quorum; beginning settle window",
                                        "settle_secs" => mesh_settle_secs
                                    );
                                    drop(chain_guard);
                                    drop(pool);
                                    thread::sleep(Duration::from_millis(500));
                                    continue;
                                }
                            }
                        } else {
                            mesh_ready_since = None;
                            status_sync_grace_since = None;
                            drop(chain_guard);
                            drop(pool);
                            thread::sleep(Duration::from_secs(1));
                            continue;
                        }

                        consensus_log!(
                            "🎯 Selecting leader for block {}",
                            latest_block.block_index + 1
                        );

                        // Clone latest_block before we might need to drop the guard
                        let latest_block_clone = latest_block.clone();
                        let view_offset = Self::deterministic_view_offset(
                            latest_block_clone.block_index,
                            latest_block_clone.timestamp,
                            leader_timeout_secs,
                        );

                        // Phase 1: Leader selection using entropy beacon and synergy scores
                        // Use next block index for leader selection (current block + 1)
                        let next_block_index = latest_block_clone.block_index + 1;
                        // Rebuild leader rotation from the shared consensus validator set.
                        // The liveness gate above still decides whether block production is
                        // allowed, but every node must derive the same leader order from the
                        // same validator set for a given height/epoch.
                        let epoch_randomness = Self::deterministic_epoch_randomness(
                            &chain_guard,
                            next_block_index,
                            epoch_length,
                        );
                        let selected_validator = Self::select_leader_for_block(
                            &active_validators,
                            next_block_index,
                            &synergy_calculator,
                            &epoch_randomness,
                            epoch_length,
                            view_offset,
                        );

                        let local_validator_address = Self::resolve_local_validator_address();
                        if local_validator_address.as_deref()
                            != Some(selected_validator.address.as_str())
                        {
                            let wait_elapsed = Duration::from_secs(
                                Self::current_timestamp()
                                    .saturating_sub(latest_block_clone.timestamp),
                            );

                            if wait_elapsed >= Duration::from_secs(leader_timeout_secs) {
                                let timeout_marker = (next_block_index, view_offset);
                                if last_logged_view_timeout != Some(timeout_marker) {
                                    warn!(
                                        "consensus",
                                        "Leader proposal timeout — following shared leader rotation",
                                        "timed_out_leader" => selected_validator.address.clone(),
                                        "shared_view_offset" => view_offset,
                                        "waited_secs" => wait_elapsed.as_secs(),
                                        "block_height" => next_block_index
                                    );
                                    // Timeout penalties are intentionally skipped here.
                                    // They mutate validator-local health state, and applying
                                    // them independently on each node causes the validator
                                    // set to drift while the chain is stalled.
                                    let _ = penalization_enabled;
                                    last_logged_view_timeout = Some(timeout_marker);
                                }
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
                            thread::sleep(Duration::from_millis(250));
                            continue;
                        }

                        consensus_log!("LEADER SELECTED: {}", selected_validator.address);
                        consensus_log!("Getting transactions from pool...");

                        let confirmed_hashes = chain_guard
                            .chain
                            .iter()
                            .flat_map(|block| {
                                block
                                    .transactions
                                    .iter()
                                    .map(|transaction| transaction.hash())
                            })
                            .collect::<HashSet<_>>();
                        let transactions = if pool.is_empty() {
                            consensus_log!("Pool is empty");
                            vec![]
                        } else {
                            let pending = pool
                                .iter()
                                .filter(|transaction| {
                                    !confirmed_hashes.contains(&transaction.hash())
                                })
                                .cloned()
                                .collect::<Vec<_>>();
                            consensus_log!(
                                "Pool has {} transactions ({} eligible after confirmed-tx pruning)",
                                pool.len(),
                                pending.len()
                            );
                            pending
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

                        let dag_vertex_hash = crate::dag::create_proposal_vertex_for_transactions(
                            &processed_transactions,
                            &selected_validator.address,
                            next_block_index,
                        );
                        if let Some(hash) = &dag_vertex_hash {
                            info!(
                                "consensus",
                                "Created DAG proposal vertex",
                                "height" => next_block_index,
                                "vertex_hash" => hash.clone(),
                                "transactions" => processed_transactions.len() as u64,
                                "validator" => selected_validator.address.clone()
                            );
                        }

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

                        // The vote wait can run for multiple seconds on a public mesh.
                        // Release local chain and pool locks before that wait so the P2P
                        // path can apply parent blocks and answer vote requests in time.
                        drop(chain_guard);
                        drop(pool);

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
                                last_logged_view_timeout = None;

                                let mut block_appended_to_local_tip = false;
                                let persist_snapshot = {
                                    let mut chain_guard = chain.lock().unwrap();
                                    match chain_guard.add_block_extending_tip(new_block.clone()) {
                                        Ok(true) => {
                                            block_appended_to_local_tip = true;
                                        }
                                        Ok(false) => {
                                            info!(
                                                "consensus",
                                                "Committed block was already applied to local tip",
                                                "height" => new_block.block_index,
                                                "hash" => new_block.hash.clone()
                                            );
                                        }
                                        Err(error) => {
                                            warn!(
                                                "consensus",
                                                "Skipping stale committed block that no longer extends local tip",
                                                "height" => new_block.block_index,
                                                "hash" => new_block.hash.clone(),
                                                "error" => error
                                            );
                                        }
                                    }

                                    if !block_appended_to_local_tip {
                                        None
                                    } else {
                                        let tip_height = chain_guard
                                            .last()
                                            .map(|block| block.block_index)
                                            .unwrap_or(new_block.block_index);
                                        if Self::should_persist_consensus_chain_tip(tip_height) {
                                            let snapshot = chain_guard.clone();
                                            Self::note_consensus_chain_persist(tip_height);
                                            Some((snapshot, tip_height))
                                        } else {
                                            None
                                        }
                                    }
                                };
                                if !block_appended_to_local_tip {
                                    continue;
                                }
                                if let Some((snapshot, tip_height)) = persist_snapshot {
                                    Self::persist_consensus_chain_tip_async(snapshot, tip_height);
                                }
                                let committed_dag_vertices = crate::dag::commit_block(&new_block);
                                Self::prune_cached_block_proposals(new_block.block_index);

                                // Apply state transitions for included transactions (token transfers, staking, etc.)
                                let token_manager = TOKEN_MANAGER.clone();
                                let mut applied_txs = 0u64;
                                let mut failed_txs = 0u64;
                                let mut applied_validator_activations = 0u64;
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
                                    if is_validator_activation_transaction(tx) {
                                        match apply_validator_activation_transaction(
                                            tx,
                                            &token_manager,
                                            &validator_manager,
                                        ) {
                                            Ok(message) => {
                                                applied_validator_activations += 1;
                                                info!(
                                                    "consensus",
                                                    "Applied validator activation",
                                                    "tx_hash" => tx.hash(),
                                                    "message" => message
                                                );
                                            }
                                            Err(error) => warn!(
                                                "consensus",
                                                "Failed to apply validator activation",
                                                "tx_hash" => tx.hash(),
                                                "error" => error
                                            ),
                                        }
                                    }
                                }

                                // Persist token state for explorer continuity across restarts (best-effort).
                                if let Err(e) = token_manager.save_state("data/token_state.json") {
                                    warn!("consensus", "Failed to persist token state", "error" => e.to_string());
                                }
                                if applied_validator_activations > 0 {
                                    if let Err(e) =
                                        validator_manager.save_registry(VALIDATOR_REGISTRY_PATH)
                                    {
                                        warn!(
                                            "consensus",
                                            "Failed to persist validator registry after activation",
                                            "error" => e.to_string()
                                        );
                                    }
                                }

                                // Broadcast the committed block to peers (best-effort).
                                if let Some(p2p) = crate::p2p::get_p2p_network() {
                                    p2p.broadcast_block(&new_block);
                                }

                                // Validator health metrics and reward payouts are currently
                                // node-local bookkeeping. Mutating
                                // them here makes persisted state diverge even when every
                                // validator commits the same block hash. Keep them out of
                                // the live validator path until they are applied through a
                                // shared state transition.
                                info!("consensus", "Skipped local validator bookkeeping",
                                      "validator" => selected_validator.address.clone(),
                                      "mode" => "shared-state-only");

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

                                let confirmed_hashes = transaction_hashes(&new_block.transactions);
                                let pruned_transactions =
                                    prune_transaction_hashes_from_pool(&confirmed_hashes);

                                last_block_time =
                                    Self::system_time_from_unix_timestamp(new_block.timestamp);
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
                                    "dag_vertices_committed" => committed_dag_vertices.len() as u64,
                                    "txs_pruned_from_pool" => pruned_transactions as u64,
                                    "txs_applied" => applied_txs,
                                    "txs_failed" => failed_txs,
                                    "qc_validation_quorum_met" => qc.validation_quorum_met,
                                    "qc_cooperation_quorum_met" => qc.cooperation_quorum_met,
                                    "qc_epoch_number" => qc.epoch_number,
                                    "qc_cumulative_weight" => qc.cumulative_weight,
                                    "qc_timestamp" => qc.timestamp
                                );
                            }
                            Err(e) => {
                                warn!("consensus", "QC error - block proposal failed", "error" => e.clone());
                                use std::io::{self, Write};
                                io::stdout().flush().unwrap();
                                println!("⚠️ Block proposal failed: {}", e);
                                consecutive_failures += 1;

                                // Apply penalty to proposer for failed block
                                Self::maybe_apply_proposer_penalty(
                                    penalization_enabled,
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

    fn sync_validator_to_network_tip(
        network: &Arc<P2PNetwork>,
        local_height: u64,
        best_validator_height: u64,
        required_sync_support: usize,
    ) {
        info!(
            "consensus",
            "Starting validator catch-up sync before block production",
            "local_height" => local_height,
            "best_validator_height" => best_validator_height,
            "required_sync_support" => required_sync_support as u64
        );

        let sync_result = {
            let mut manager = SYNC_MANAGER.lock().unwrap();
            manager.attach_network(Arc::clone(network));
            manager.start_sync().map(|_| manager.local_height)
        };

        match sync_result {
            Ok(final_height) => {
                info!(
                    "consensus",
                    "Validator catch-up sync completed",
                    "starting_height" => local_height,
                    "best_validator_height" => best_validator_height,
                    "final_height" => final_height
                );
            }
            Err(error) => {
                warn!(
                    "consensus",
                    "Validator catch-up sync failed",
                    "local_height" => local_height,
                    "best_validator_height" => best_validator_height,
                    "error" => error.to_string()
                );
            }
        }
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
        let active_validator_addresses =
            consensus_membership_validators(validator_manager.get_active_validators())
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
            for validator_address in network.get_status_ready_validator_addresses() {
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

    fn system_time_from_unix_timestamp(timestamp: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(timestamp)
    }

    fn should_persist_consensus_chain_tip(tip_height: u64) -> bool {
        if tip_height <= 32 {
            return true;
        }

        let state = LAST_CONSENSUS_CHAIN_PERSIST.lock().unwrap();
        match *state {
            Some((last_height, last_at)) => {
                tip_height.saturating_sub(last_height) >= 25
                    || last_at.elapsed() >= Duration::from_secs(30)
            }
            None => tip_height % 25 == 0,
        }
    }

    fn note_consensus_chain_persist(tip_height: u64) {
        let mut state = LAST_CONSENSUS_CHAIN_PERSIST.lock().unwrap();
        *state = Some((tip_height, Instant::now()));
    }

    fn persist_consensus_chain_tip_async(snapshot: BlockChain, tip_height: u64) {
        if CONSENSUS_CHAIN_PERSIST_IN_FLIGHT
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            debug!(
                "consensus",
                "Skipping chain persistence because a previous save is still running",
                "height" => tip_height
            );
            return;
        }

        let chain_path = get_chain_path();
        thread::spawn(move || {
            snapshot.save_to_file(&chain_path);
            CONSENSUS_CHAIN_PERSIST_IN_FLIGHT.store(false, Ordering::SeqCst);
        });
    }

    fn effective_leader_timeout_secs(&self) -> u64 {
        let block_time_secs = self.block_time.max(1);
        if self.leader_timeout_secs == 0 {
            (block_time_secs * 2).max(3)
        } else {
            self.leader_timeout_secs.max(block_time_secs)
        }
    }

    // New PoSy Helper Methods

    fn epoch_for_block(block_index: u64, epoch_length: u64) -> u64 {
        block_index / epoch_length.max(1)
    }

    fn epoch_for_next_block(last_block_index: u64, epoch_length: u64) -> u64 {
        Self::epoch_for_block(last_block_index.saturating_add(1), epoch_length)
    }

    fn deterministic_view_offset(
        last_block_index: u64,
        last_block_timestamp: u64,
        leader_timeout_secs: u64,
    ) -> usize {
        Self::deterministic_view_offset_for_block_time(
            last_block_index,
            last_block_timestamp,
            leader_timeout_secs,
            Self::current_timestamp(),
        )
    }

    fn deterministic_view_offset_for_block_time(
        last_block_index: u64,
        last_block_timestamp: u64,
        leader_timeout_secs: u64,
        current_timestamp: u64,
    ) -> usize {
        // Genesis has no committed in-network clock yet. Deriving the initial view
        // offset from each node's wall clock causes different validators to rotate to
        // different leaders before block 1 exists, so keep the primary leader fixed
        // until the first block commits and provides a fresh shared timestamp anchor.
        if last_block_index == 0 {
            return 0;
        }

        Self::deterministic_view_offset_for_time(
            last_block_timestamp,
            leader_timeout_secs,
            current_timestamp,
        )
    }

    fn deterministic_view_offset_for_time(
        last_block_timestamp: u64,
        leader_timeout_secs: u64,
        current_timestamp: u64,
    ) -> usize {
        let timeout_secs = leader_timeout_secs.max(1);
        let elapsed_secs = current_timestamp.saturating_sub(last_block_timestamp);

        (elapsed_secs / timeout_secs) as usize
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
        let previous_qc =
            Self::get_previous_quorum_certificate(chain, *current_epoch, epoch_length);
        let mut beacon = entropy_beacon.lock().unwrap();
        let _epoch_randomness = beacon.generate_epoch_randomness(&previous_qc);
        drop(beacon);

        // 2. Rotate validators using new entropy
        validator_rotation.rotate_validators();

        // 3. Recalculate synergy scores
        Self::recalculate_all_synergy_scores(validator_manager, synergy_calculator);

        // 4. Rebalance validator clusters deterministically for this epoch.
        validator_manager.reorganize_clusters_for_epoch(*current_epoch);
        if let Err(error) = validator_manager.save_registry(VALIDATOR_REGISTRY_PATH) {
            warn!(
                "consensus",
                "Failed to save validator registry after epoch cluster shuffle",
                "epoch" => *current_epoch,
                "error" => error.to_string()
            );
        }

        // 5. Detect cartels and apply penalties
        let mut cartel_engine = cartel_detection.lock().unwrap();
        let cartel_penalties = cartel_engine.detect_cartels(*current_epoch);
        cartel_engine.apply_cartel_penalties(&cartel_penalties);

        // 6. Update governance proposals
        let mut governance = dao_governance.lock().unwrap();
        Self::update_governance_proposals(&mut governance, *current_epoch);

        // 7. Reset dual quorum consensus state
        let mut consensus = dual_quorum_consensus.lock().unwrap();
        consensus.current_epoch = *current_epoch;

        println!("🔄 Epoch Transition: Completed epoch {}", current_epoch);
    }

    fn get_previous_quorum_certificate(
        chain: &BlockChain,
        current_epoch: u64,
        epoch_length: u64,
    ) -> QuorumCertificate {
        let epoch_length = epoch_length.max(1);
        let boundary_height = current_epoch.saturating_mul(epoch_length).saturating_sub(1);

        // Reconstruct the epoch seed from the block immediately before the
        // epoch boundary. Falling back to the chain tip is only a safeguard for
        // truncated history; normal operation should always find the boundary block.
        if let Some(block) = chain
            .chain
            .iter()
            .find(|block| block.block_index == boundary_height)
            .or_else(|| chain.last())
        {
            QuorumCertificate {
                block_hash: block.hash.clone(),
                epoch_number: block.block_index / epoch_length,
                round_number: 1,
                aggregate_signature: block.block_signature.clone(),
                participant_bitmap: Vec::new(),
                cumulative_weight: 0.0,
                validation_quorum_met: false,
                cooperation_quorum_met: false,
                timestamp: Self::current_timestamp(),
            }
        } else {
            QuorumCertificate {
                block_hash: "genesis_block".to_string(),
                epoch_number: 0,
                round_number: 0,
                aggregate_signature: Vec::new(),
                participant_bitmap: Vec::new(),
                cumulative_weight: 0.0,
                validation_quorum_met: false,
                cooperation_quorum_met: false,
                timestamp: Self::current_timestamp(),
            }
        }
    }

    fn deterministic_epoch_randomness(
        chain: &BlockChain,
        block_height: u64,
        epoch_length: u64,
    ) -> Vec<u8> {
        let epoch_length = epoch_length.max(1);
        let current_epoch = block_height / epoch_length;
        let previous_qc = Self::get_previous_quorum_certificate(chain, current_epoch, epoch_length);
        Self::deterministic_epoch_randomness_from_qc(&previous_qc)
    }

    fn deterministic_epoch_randomness_from_qc(previous_qc: &QuorumCertificate) -> Vec<u8> {
        let next_epoch = previous_qc.epoch_number.saturating_add(1);
        let qc_hash = Self::hash_quorum_certificate(previous_qc);

        let mut input = Vec::new();
        input.extend(next_epoch.to_be_bytes());
        input.extend(qc_hash.as_bytes());

        let mut hasher = Sha3_512::new();
        hasher.update(&input);
        hasher.finalize().to_vec()
    }

    fn hash_quorum_certificate(qc: &QuorumCertificate) -> String {
        let mut hasher = Sha3_512::new();
        hasher.update(qc.block_hash.as_bytes());
        hasher.update(qc.epoch_number.to_be_bytes());
        hasher.update(qc.round_number.to_be_bytes());
        hasher.update(&qc.aggregate_signature);
        hasher.update(&qc.participant_bitmap);
        hasher.update([qc.validation_quorum_met as u8]);
        hasher.update([qc.cooperation_quorum_met as u8]);
        hex::encode(hasher.finalize())
    }

    fn select_leader_for_block(
        validators: &[Validator],
        block_height: u64,
        _synergy_calculator: &Arc<SynergyScoreCalculator>,
        epoch_randomness: &[u8],
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
        let mut candidate_addresses = validators
            .iter()
            .map(|validator| validator.address.clone())
            .collect::<Vec<_>>();
        candidate_addresses.sort();
        candidate_addresses.dedup();

        // Check if we need to recalculate leader priorities (at epoch start or if not initialized)
        let mut rotation = EPOCH_LEADER_ROTATION.lock().unwrap();
        let needs_recalculation = rotation.0 != current_epoch
            || rotation.1.is_empty()
            || rotation.3 != candidate_addresses;

        if needs_recalculation {
            consensus_log!(
                "🔄 [select_leader_for_block] Recalculating leader priorities for epoch {}",
                current_epoch
            );

            // Calculate priority for each validator using Equation 17 from PoSy spec
            let mut validator_priorities = Vec::new();

            consensus_log!(
                "🔄 [select_leader_for_block] Calculating priorities for {} validators",
                validators.len()
            );
            for validator in validators.iter() {
                // Calculate priority: H(r_e || validatorID_v) * SS_v,normalized (PoSy Equation 17)
                let mut hasher = Sha3_512::new();
                hasher.update(epoch_randomness);
                hasher.update(validator.address.as_bytes());
                let hash = hasher.finalize();
                let raw_hash = u64::from_be_bytes(hash[..8].try_into().unwrap());

                let consensus_weight = Self::stable_leader_weight(validators, validator);

                // Calculate priority value
                let priority_value = raw_hash as f64 * consensus_weight;

                validator_priorities.push((validator.clone(), priority_value, raw_hash));
            }

            // Sort deterministically. When priorities tie, fall back to the raw
            // hash and finally the validator address so every node computes the
            // same top-K order from the same epoch randomness.
            validator_priorities.sort_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| b.2.cmp(&a.2))
                    .then_with(|| a.0.address.cmp(&b.0.address))
            });

            // Select top K validators for round-robin (K = min(10, |validators|) as per PoSy)
            // Use all validators if we have 3 or fewer, otherwise use min(10, validators.len())
            let k = std::cmp::min(10, validators.len());
            let top_k_addresses: Vec<String> = validator_priorities
                .iter()
                .take(k)
                .map(|(v, _, _)| v.address.clone())
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
            rotation.3 = candidate_addresses;
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

    fn stable_leader_weight(validators: &[Validator], validator: &Validator) -> f64 {
        let total_stake = validators
            .iter()
            .map(|candidate| candidate.stake_amount)
            .sum::<u64>()
            .max(1);
        let weight = validator.stake_amount as f64 / total_stake as f64;

        if weight > 0.0 {
            weight
        } else {
            f64::EPSILON
        }
    }

    fn create_block_proposal(
        previous_block: &Block,
        leader: &Validator,
        transactions: Vec<crate::transaction::Transaction>,
        pqc_manager: &Arc<Mutex<PQCManager>>,
    ) -> Block {
        if let Some(block) = Self::load_cached_block_proposal(previous_block, leader) {
            info!(
                "consensus",
                "Reusing cached block proposal for retry",
                "height" => block.block_index,
                "hash" => block.hash.clone(),
                "validator" => leader.address.clone()
            );
            return block;
        }

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
            Self::get_or_create_leader_keypair(&leader.address, &mut pqc).unwrap_or_else(|error| {
                panic!("Aegis PQC leader signing key unavailable: {error}")
            });

        if let Ok(signature) = pqc.sign(&leader_private_key, block.hash.as_bytes()) {
            block.proposer_public_key = leader_public_key.key_data;
            block.block_signature = signature.signature_data;
            block.block_signature_algorithm = "fndsa".to_string();
        }

        if let Err(error) = Self::persist_cached_block_proposal(&block) {
            warn!(
                "consensus",
                "Failed to persist block proposal for retry",
                "height" => block.block_index,
                "hash" => block.hash.clone(),
                "error" => error.to_string()
            );
        }

        block
    }

    fn proposal_cache_dir() -> PathBuf {
        #[cfg(test)]
        if let Some(path) = TEST_PROPOSAL_CACHE_DIR
            .lock()
            .expect("test proposal cache lock should succeed")
            .clone()
        {
            return path;
        }

        crate::utils::resolve_data_path("data/consensus_proposals")
    }

    fn proposal_cache_key(block_index: u64, previous_hash: &str, leader_address: &str) -> String {
        let input = format!("{block_index}:{previous_hash}:{leader_address}");
        blake3::hash(input.as_bytes()).to_hex().to_string()
    }

    fn proposal_cache_path(block_index: u64, previous_hash: &str, leader_address: &str) -> PathBuf {
        Self::proposal_cache_dir().join(format!(
            "{}.json",
            Self::proposal_cache_key(block_index, previous_hash, leader_address)
        ))
    }

    fn load_cached_block_proposal(previous_block: &Block, leader: &Validator) -> Option<Block> {
        let _guard = PROPOSAL_CACHE_LOCK
            .lock()
            .expect("proposal cache lock should succeed");
        let path = Self::proposal_cache_path(
            previous_block.block_index + 1,
            &previous_block.hash,
            &leader.address,
        );
        let contents = fs::read_to_string(path).ok()?;
        let block = serde_json::from_str::<Block>(&contents).ok()?;
        Self::block_matches_proposal_context(&block, previous_block, leader).then_some(block)
    }

    fn persist_cached_block_proposal(block: &Block) -> Result<(), std::io::Error> {
        let _guard = PROPOSAL_CACHE_LOCK
            .lock()
            .expect("proposal cache lock should succeed");
        let dir = Self::proposal_cache_dir();
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!(
            "{}.json",
            Self::proposal_cache_key(block.block_index, &block.previous_hash, &block.validator_id)
        ));
        let tmp_path = path.with_extension("json.tmp");
        let payload = serde_json::to_vec_pretty(block)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        fs::write(&tmp_path, payload)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600))?;
        }
        fs::rename(tmp_path, path)?;
        Ok(())
    }

    fn prune_cached_block_proposals(committed_height: u64) {
        let _guard = PROPOSAL_CACHE_LOCK
            .lock()
            .expect("proposal cache lock should succeed");
        let dir = Self::proposal_cache_dir();
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let should_remove = fs::read_to_string(&path)
                .ok()
                .and_then(|contents| serde_json::from_str::<Block>(&contents).ok())
                .map(|block| block.block_index <= committed_height)
                .unwrap_or(false);
            if should_remove {
                let _ = fs::remove_file(path);
            }
        }
    }

    fn block_matches_proposal_context(
        block: &Block,
        previous_block: &Block,
        leader: &Validator,
    ) -> bool {
        if block.block_index != previous_block.block_index + 1
            || block.previous_hash != previous_block.hash
            || block.validator_id != leader.address
        {
            return false;
        }

        let recalculated = Block::new_with_timestamp(
            block.block_index,
            block.transactions.clone(),
            block.previous_hash.clone(),
            block.validator_id.clone(),
            block.nonce,
            block.timestamp,
        );
        recalculated.hash == block.hash && recalculated.transactions_root == block.transactions_root
    }

    #[cfg(test)]
    fn set_test_proposal_cache_dir(path: Option<PathBuf>) {
        *TEST_PROPOSAL_CACHE_DIR
            .lock()
            .expect("test proposal cache lock should succeed") = path;
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
        if !crate::address::is_valid_address(&tx.sender) {
            warn!(
                "consensus",
                "Rejecting transaction with invalid sender address",
                "tx_hash" => tx.hash(),
                "sender" => tx.sender.clone()
            );
            return false;
        }

        if !tx.receiver.trim().is_empty()
            && !tx.receiver.starts_with("contract_")
            && !crate::address::is_valid_address(&tx.receiver)
        {
            warn!(
                "consensus",
                "Rejecting transaction with invalid receiver address",
                "tx_hash" => tx.hash(),
                "receiver" => tx.receiver.clone()
            );
            return false;
        }

        if let Some(validator) = staking_validator_address(tx) {
            if !crate::address::is_valid_address(&validator) {
                warn!(
                    "consensus",
                    "Rejecting staking transaction with invalid validator address",
                    "tx_hash" => tx.hash(),
                    "validator" => validator
                );
                return false;
            }
        }

        // 1. Verify transaction signature. Missing sender public keys fail closed:
        // consensus cannot admit unsigned or unverifiable transactions.
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
                "Rejecting transaction because sender public key is unavailable",
                "sender" => tx.sender.clone()
            );
            return false;
        }

        // 2. Verify sender balance via token manager to reflect on-chain state
        let token_manager = TOKEN_MANAGER.clone();
        let required = snrg_balance_required_for_transaction(tx);
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
        _synergy_calculator: &Arc<SynergyScoreCalculator>,
        validator_address: &str,
    ) {
        let _ = validator_manager;
        let _ = validator_address;
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

    fn maybe_apply_proposer_penalty(
        penalization_enabled: bool,
        validator_manager: &Arc<ValidatorManager>,
        validator_address: &str,
    ) {
        if !penalization_enabled {
            return;
        }
        Self::apply_proposer_penalty(validator_manager, validator_address);
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
    fn maybe_apply_leader_timeout_penalty(
        penalization_enabled: bool,
        validator_manager: &Arc<ValidatorManager>,
        validator_address: &str,
        block_height: u64,
        view_offset: usize,
    ) {
        if !penalization_enabled {
            return;
        }
        Self::apply_leader_timeout_penalty(
            validator_manager,
            validator_address,
            block_height,
            view_offset,
        );
    }

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
        _synergy_calculator: &Arc<SynergyScoreCalculator>,
    ) {
        if let Ok(mut registry) = validator_manager.registry.lock() {
            for validator in registry.validators.values_mut() {
                // Keep the persisted validator health score aligned with the
                // intrinsic validator metrics. The synergy calculator's
                // normalized score is a comparative ranking for leader
                // selection, and persisting it here can wrongly evict healthy
                // validators at epoch boundaries when one proposer has more
                // proposal history than the rest of the set.
                validator.calculate_synergy_score();
            }
        }

        println!("📊 Recalculated validator health scores for all validators");
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
    ) -> Result<(PQCPublicKey, PQCPrivateKey), String> {
        if let Ok(cache) = EPHEMERAL_LEADER_KEYS.lock() {
            if let Some((public_key, private_key)) = cache.get(validator_address) {
                return Ok((public_key.clone(), private_key.clone()));
            }
        }

        let generated = pqc_manager
            .generate_keypair(PQCAlgorithm::FNDSA)
            .map_err(|error| format!("aegis-pqvm FN-DSA leader key generation failed: {error}"))?;

        if let Ok(mut cache) = EPHEMERAL_LEADER_KEYS.lock() {
            cache.insert(
                validator_address.to_string(),
                (generated.0.clone(), generated.1.clone()),
            );
        }

        Ok(generated)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::{Block, BlockChain};
    use crate::transaction::Transaction;
    use crate::validator::ValidatorStatus;
    use std::sync::OnceLock;

    fn proposal_cache_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn unique_proposal_cache_dir(test_name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "synergy-{test_name}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("test proposal cache dir should be created");
        dir
    }

    fn active_validator_manager(address: &str) -> Arc<ValidatorManager> {
        let manager = Arc::new(ValidatorManager::new());
        let mut validator = Validator::new(
            address.to_string(),
            format!("{address}-pubkey"),
            "Validator".to_string(),
            1_000,
        );
        validator.status = ValidatorStatus::Active;
        manager
            .registry
            .lock()
            .expect("registry lock should succeed")
            .validators
            .insert(address.to_string(), validator);
        manager
    }

    #[test]
    fn proposer_penalty_is_skipped_when_penalization_is_disabled() {
        let validator_address = "synv1proposer";
        let manager = active_validator_manager(validator_address);
        let before = manager
            .get_validator(validator_address)
            .expect("validator should exist");

        ProofOfSynergy::maybe_apply_proposer_penalty(false, &manager, validator_address);

        let after = manager
            .get_validator(validator_address)
            .expect("validator should exist");
        assert_eq!(after.reputation_score, before.reputation_score);
        assert_eq!(after.synergy_score, before.synergy_score);
    }

    #[test]
    fn leader_timeout_penalty_is_skipped_when_penalization_is_disabled() {
        let validator_address = "synv1leader";
        let manager = active_validator_manager(validator_address);
        let before = manager
            .get_validator(validator_address)
            .expect("validator should exist");

        ProofOfSynergy::maybe_apply_leader_timeout_penalty(
            false,
            &manager,
            validator_address,
            7,
            1,
        );

        let after = manager
            .get_validator(validator_address)
            .expect("validator should exist");
        assert_eq!(after.missed_blocks, before.missed_blocks);
        assert_eq!(after.missed_vote_window, before.missed_vote_window);
        assert_eq!(after.uptime_percentage, before.uptime_percentage);
    }

    #[test]
    fn next_block_epoch_transitions_at_boundary_only_once() {
        assert_eq!(ProofOfSynergy::epoch_for_next_block(998, 1000), 0);
        assert_eq!(ProofOfSynergy::epoch_for_next_block(999, 1000), 1);
        assert_eq!(ProofOfSynergy::epoch_for_next_block(1000, 1000), 1);

        let mut current_epoch = 0;
        let target_epoch = ProofOfSynergy::epoch_for_next_block(999, 1000);
        while current_epoch < target_epoch {
            current_epoch += 1;
        }
        assert_eq!(current_epoch, 1);

        let same_boundary_epoch = ProofOfSynergy::epoch_for_next_block(1000, 1000);
        while current_epoch < same_boundary_epoch {
            current_epoch += 1;
        }
        assert_eq!(current_epoch, 1);
    }

    #[test]
    fn deterministic_view_offset_advances_after_leader_timeout() {
        assert_eq!(
            ProofOfSynergy::deterministic_view_offset_for_time(4_983, 20, 4_983),
            0
        );
        assert_eq!(
            ProofOfSynergy::deterministic_view_offset_for_time(4_983, 20, 5_002),
            0
        );
        assert_eq!(
            ProofOfSynergy::deterministic_view_offset_for_time(4_983, 20, 5_003),
            1
        );
        assert_eq!(
            ProofOfSynergy::deterministic_view_offset_for_time(4_983, 20, 5_044),
            3
        );
    }

    #[test]
    fn deterministic_view_offset_keeps_genesis_on_primary_leader() {
        assert_eq!(
            ProofOfSynergy::deterministic_view_offset_for_block_time(0, 4_983, 20, 4_983),
            0
        );
        assert_eq!(
            ProofOfSynergy::deterministic_view_offset_for_block_time(0, 4_983, 20, 5_500),
            0
        );
        assert_eq!(
            ProofOfSynergy::deterministic_view_offset_for_block_time(1, 4_983, 20, 5_500),
            ProofOfSynergy::deterministic_view_offset_for_time(4_983, 20, 5_500)
        );
    }

    #[test]
    fn previous_qc_uses_epoch_boundary_block_on_mid_epoch_restart() {
        let mut chain = BlockChain::new();
        chain.add_block(Block {
            block_index: 999,
            timestamp: 999,
            transactions: Vec::new(),
            previous_hash: "998".to_string(),
            validator_id: "validator-a".to_string(),
            nonce: 999,
            hash: "boundary-999".to_string(),
            transactions_root: String::new(),
            proposer_public_key: Vec::new(),
            block_signature: vec![9, 9, 9],
            block_signature_algorithm: "fndsa".to_string(),
        });
        chain.add_block(Block {
            block_index: 1026,
            timestamp: 1026,
            transactions: Vec::new(),
            previous_hash: "1025".to_string(),
            validator_id: "validator-b".to_string(),
            nonce: 1026,
            hash: "mid-epoch-1026".to_string(),
            transactions_root: String::new(),
            proposer_public_key: Vec::new(),
            block_signature: vec![1, 2, 6],
            block_signature_algorithm: "fndsa".to_string(),
        });

        let previous_qc = ProofOfSynergy::get_previous_quorum_certificate(&chain, 1, 1000);

        assert_eq!(previous_qc.block_hash, "boundary-999");
        assert_eq!(previous_qc.epoch_number, 0);
        assert_eq!(previous_qc.aggregate_signature, vec![9, 9, 9]);
    }

    #[test]
    fn deterministic_epoch_randomness_uses_boundary_qc_only() {
        let mut chain = BlockChain::new();
        chain.add_block(Block {
            block_index: 999,
            timestamp: 999,
            transactions: Vec::new(),
            previous_hash: "998".to_string(),
            validator_id: "validator-a".to_string(),
            nonce: 999,
            hash: "boundary-999".to_string(),
            transactions_root: String::new(),
            proposer_public_key: Vec::new(),
            block_signature: vec![9, 9, 9],
            block_signature_algorithm: "fndsa".to_string(),
        });
        chain.add_block(Block {
            block_index: 1026,
            timestamp: 1026,
            transactions: Vec::new(),
            previous_hash: "1025".to_string(),
            validator_id: "validator-b".to_string(),
            nonce: 1026,
            hash: "mid-epoch-1026".to_string(),
            transactions_root: String::new(),
            proposer_public_key: Vec::new(),
            block_signature: vec![1, 2, 6],
            block_signature_algorithm: "fndsa".to_string(),
        });

        let direct_qc = ProofOfSynergy::get_previous_quorum_certificate(&chain, 1, 1000);
        let expected = ProofOfSynergy::deterministic_epoch_randomness_from_qc(&direct_qc);
        let actual = ProofOfSynergy::deterministic_epoch_randomness(&chain, 1_026, 1_000);

        assert_eq!(actual, expected);
    }

    #[test]
    fn leader_selection_ignores_local_performance_metrics() {
        let _guard = proposal_cache_test_lock()
            .lock()
            .expect("leader rotation test lock should succeed");
        let manager = Arc::new(ValidatorManager::new());
        let pqc_manager = Arc::new(Mutex::new(PQCManager::new()));
        let synergy_calculator = Arc::new(SynergyScoreCalculator::new(
            Arc::clone(&manager),
            Arc::clone(&pqc_manager),
        ));
        let epoch_randomness = vec![42; 32];

        let build_validator = |address: &str, stake_amount: u64| {
            let mut validator = Validator::new(
                address.to_string(),
                format!("{address}-pubkey"),
                address.to_string(),
                stake_amount,
            );
            validator.status = ValidatorStatus::Active;
            validator
        };

        let validators_a = vec![
            build_validator("synv1a", 3_000),
            build_validator("synv1b", 2_000),
            build_validator("synv1c", 1_000),
        ];

        let mut validators_b = validators_a.clone();
        validators_b[0].total_blocks_produced = 10_000;
        validators_b[0].total_transactions_validated = 10_000;
        validators_b[0].collaboration_score = 500.0;
        validators_b[0].average_block_time = 1.0;
        validators_b[0].reputation_score = 15.0;
        validators_b[0].slashing_penalty = 0.75;
        validators_b[0].calculate_synergy_score();
        validators_b[1].missed_blocks = 250;
        validators_b[1].reputation_score = 1.0;
        validators_b[1].calculate_synergy_score();

        *EPOCH_LEADER_ROTATION
            .lock()
            .expect("rotation lock should succeed") = (0, Vec::new(), 0, Vec::new());
        let leader_a = ProofOfSynergy::select_leader_for_block(
            &validators_a,
            1_000,
            &synergy_calculator,
            &epoch_randomness,
            1_000,
            0,
        );

        *EPOCH_LEADER_ROTATION
            .lock()
            .expect("rotation lock should succeed") = (0, Vec::new(), 0, Vec::new());
        let leader_b = ProofOfSynergy::select_leader_for_block(
            &validators_b,
            1_000,
            &synergy_calculator,
            &epoch_randomness,
            1_000,
            0,
        );

        assert_eq!(leader_a.address, leader_b.address);
    }

    #[test]
    fn epoch_recalculation_keeps_healthy_validators_eligible() {
        let manager = Arc::new(ValidatorManager::new());
        let pqc_manager = Arc::new(Mutex::new(PQCManager::new()));
        let synergy_calculator = Arc::new(SynergyScoreCalculator::new(
            Arc::clone(&manager),
            pqc_manager,
        ));

        {
            let mut registry = manager
                .registry
                .lock()
                .expect("registry lock should succeed");

            for index in 0..5 {
                let address = format!("synv1epoch{index}");
                let mut validator = Validator::new(
                    address.clone(),
                    format!("{address}-pubkey"),
                    format!("Validator {index}"),
                    1_000,
                );
                validator.status = ValidatorStatus::Active;
                validator.total_blocks_produced = u64::from(index == 0) * 1_000;
                validator.total_transactions_validated = u64::from(index == 0) * 1_000;
                registry.validators.insert(address, validator);
            }
        }

        ProofOfSynergy::recalculate_all_synergy_scores(&manager, &synergy_calculator);

        let active_validators = manager.get_active_validators();
        assert_eq!(active_validators.len(), 5);
        assert!(active_validators
            .iter()
            .all(|validator| validator.synergy_score >= 50.0));
    }

    #[test]
    fn leader_rotation_recalculates_when_candidate_set_changes_mid_epoch() {
        let _guard = proposal_cache_test_lock()
            .lock()
            .expect("leader rotation test lock should succeed");
        let pqc_manager = Arc::new(Mutex::new(PQCManager::new()));
        let synergy_calculator = Arc::new(SynergyScoreCalculator::new(
            Arc::new(ValidatorManager::new()),
            pqc_manager,
        ));
        let epoch_randomness = vec![9u8; 64];
        let build_validator = |address: &str, stake_amount: u64| {
            let mut validator = Validator::new(
                address.to_string(),
                format!("{address}-pubkey"),
                address.to_string(),
                stake_amount,
            );
            validator.status = ValidatorStatus::Active;
            validator
        };
        let validators_full = vec![
            build_validator("synv1a", 5_000),
            build_validator("synv1b", 4_000),
            build_validator("synv1c", 3_000),
            build_validator("synv1d", 2_000),
        ];
        let validators_reduced = vec![
            validators_full[0].clone(),
            validators_full[1].clone(),
            validators_full[3].clone(),
        ];

        *EPOCH_LEADER_ROTATION
            .lock()
            .expect("rotation lock should succeed") = (0, Vec::new(), 0, Vec::new());

        let _ = ProofOfSynergy::select_leader_for_block(
            &validators_full,
            1_005,
            &synergy_calculator,
            &epoch_randomness,
            1_000,
            0,
        );
        let cached_full = EPOCH_LEADER_ROTATION
            .lock()
            .expect("rotation lock should succeed")
            .clone();
        assert_eq!(cached_full.3.len(), 4);
        assert!(cached_full.1.iter().any(|address| address == "synv1c"));

        let _ = ProofOfSynergy::select_leader_for_block(
            &validators_reduced,
            1_006,
            &synergy_calculator,
            &epoch_randomness,
            1_000,
            0,
        );
        let cached_reduced = EPOCH_LEADER_ROTATION
            .lock()
            .expect("rotation lock should succeed")
            .clone();
        assert_eq!(
            cached_reduced.3,
            vec![
                "synv1a".to_string(),
                "synv1b".to_string(),
                "synv1d".to_string(),
            ]
        );
        assert!(!cached_reduced.1.iter().any(|address| address == "synv1c"));
    }

    #[test]
    fn leader_reuses_cached_proposal_for_same_height_retry() {
        let _guard = proposal_cache_test_lock()
            .lock()
            .expect("proposal cache test lock should succeed");
        let cache_dir = unique_proposal_cache_dir("leader-retry");
        ProofOfSynergy::set_test_proposal_cache_dir(Some(cache_dir.clone()));

        let previous = Block::new_with_timestamp(
            772,
            vec![],
            "previous-parent".to_string(),
            "synv1previous".to_string(),
            772,
            1_777_426_405,
        );
        let mut leader = Validator::new(
            "synv1leader-retry".to_string(),
            "leader-pubkey".to_string(),
            "Leader Retry".to_string(),
            1_000,
        );
        leader.status = ValidatorStatus::Active;
        let pqc_manager = Arc::new(Mutex::new(PQCManager::new()));

        let first = ProofOfSynergy::create_block_proposal(&previous, &leader, vec![], &pqc_manager);
        let late_transaction = Transaction::new(
            "synw1sender".to_string(),
            "synw1receiver".to_string(),
            1,
            0,
            vec![1, 2, 3],
            1,
            21_000,
            Some("late-mempool-transaction".to_string()),
            "test".to_string(),
        );
        let retry = ProofOfSynergy::create_block_proposal(
            &previous,
            &leader,
            vec![late_transaction],
            &pqc_manager,
        );

        assert_eq!(retry.hash, first.hash);
        assert_eq!(retry.transactions.len(), first.transactions.len());
        assert!(retry.transactions.is_empty());

        ProofOfSynergy::prune_cached_block_proposals(first.block_index);
        assert!(fs::read_dir(&cache_dir)
            .expect("cache dir should remain readable")
            .next()
            .is_none());

        ProofOfSynergy::set_test_proposal_cache_dir(None);
        let _ = fs::remove_dir_all(cache_dir);
    }
}
