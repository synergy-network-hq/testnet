use crate::address::generate_cluster_address;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

const VERBOSE_VALIDATOR_LOGS: bool = false;
pub const INITIAL_VALIDATOR_SYNERGY_SCORE: f64 = 100.0;
pub const TESTNET_BETA_VALIDATOR_CLUSTER_SIZE: usize = 5;

macro_rules! validator_log {
    ($($arg:tt)*) => {
        if VERBOSE_VALIDATOR_LOGS {
            println!($($arg)*);
        }
    };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validator {
    pub address: String,
    pub public_key: String,
    pub name: String,
    pub website: Option<String>,
    pub description: Option<String>,
    pub email: Option<String>,

    // Registration info
    pub registered_at: u64,
    pub last_active: u64,
    pub total_blocks_produced: u64,
    pub total_transactions_validated: u64,

    // Performance metrics
    pub uptime_percentage: f64,
    pub average_block_time: f64,
    pub missed_blocks: u64,
    pub double_signs: u64,

    // Synergy scores
    pub synergy_score: f64,
    pub task_accuracy: f64,
    pub collaboration_score: f64,
    pub reputation_score: f64,
    pub slashing_penalty: f64,

    // Staking info
    pub stake_amount: u64,
    pub min_stake_required: u64,

    // Network info
    pub cluster_id: Option<u64>,
    pub status: ValidatorStatus,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidatorStatus {
    Active,
    Inactive,
    Jailed,
    Slashed,
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorCluster {
    pub id: u64,
    pub address: String, // Cluster address using syngrp{1-5} format
    pub validators: Vec<String>,
    pub total_stake: u64,
    pub average_synergy_score: f64,
    pub created_at: u64,
    pub last_rotation: u64,
    pub group: u8, // Cluster group (1-5) for address prefix
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorRegistry {
    pub validators: HashMap<String, Validator>,
    pub clusters: HashMap<u64, ValidatorCluster>,
    pub pending_registrations: HashMap<String, ValidatorRegistration>,
    pub jailed_validators: HashSet<String>,

    // Registry settings
    pub min_stake_amount: u64,
    pub max_validators: usize,
    pub cluster_size: usize,
    pub epoch_length: u64,
    pub current_epoch: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorRegistration {
    pub address: String,
    pub public_key: String,
    pub name: String,
    pub stake_amount: u64,
    pub submitted_at: u64,
    pub registration_tx_hash: String,
}

#[derive(Debug)]
pub struct ValidatorManager {
    pub registry: Arc<Mutex<ValidatorRegistry>>,
}

impl Validator {
    pub fn new(address: String, public_key: String, name: String, stake_amount: u64) -> Self {
        let current_time = Self::current_timestamp();

        Validator {
            address,
            public_key,
            name,
            website: None,
            description: None,
            email: None,
            registered_at: current_time,
            last_active: current_time,
            total_blocks_produced: 0,
            total_transactions_validated: 0,
            uptime_percentage: 100.0,
            average_block_time: 5.0,
            missed_blocks: 0,
            double_signs: 0,
            synergy_score: INITIAL_VALIDATOR_SYNERGY_SCORE,
            task_accuracy: 100.0,
            collaboration_score: 0.0,
            reputation_score: 100.0,
            slashing_penalty: 0.0,
            stake_amount,
            min_stake_required: stake_amount,
            cluster_id: None,
            status: ValidatorStatus::Pending,
            version: "1.0.0".to_string(),
        }
    }

    pub fn update_activity(&mut self) {
        self.last_active = Self::current_timestamp();
    }

    pub fn record_block_production(&mut self) {
        self.total_blocks_produced += 1;
        self.update_activity();
        self.calculate_synergy_score();
    }

    pub fn record_missed_block(&mut self) {
        self.missed_blocks += 1;
        self.update_activity();
        self.calculate_synergy_score();
    }

    pub fn record_double_sign(&mut self) {
        self.double_signs += 1;
        self.status = ValidatorStatus::Jailed;
        self.update_activity();
    }

    pub fn calculate_synergy_score(&mut self) {
        // Calculate synergy score based on multiple factors
        let uptime_factor = self.uptime_percentage / 100.0;
        let accuracy_factor = self.task_accuracy / 100.0;
        let reputation_factor = self.reputation_score / 100.0;
        let stake_factor = (self.stake_amount as f64 / self.min_stake_required as f64).min(2.0);

        // Weighted average of factors
        self.synergy_score = (uptime_factor * 0.3
            + accuracy_factor * 0.3
            + reputation_factor * 0.2
            + stake_factor * 0.2)
            * 100.0;
    }

    pub fn is_eligible(&self, min_stake: u64) -> bool {
        self.status == ValidatorStatus::Active
            && self.stake_amount >= min_stake
            && self.synergy_score >= 50.0
            && self.uptime_percentage >= 95.0
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

impl ValidatorRegistry {
    pub fn new() -> Self {
        ValidatorRegistry {
            validators: HashMap::new(),
            clusters: HashMap::new(),
            pending_registrations: HashMap::new(),
            jailed_validators: HashSet::new(),
            min_stake_amount: 0, // Lowered for testnet-beta (production: 1000)
            max_validators: 100,
            cluster_size: TESTNET_BETA_VALIDATOR_CLUSTER_SIZE,
            epoch_length: 30000,
            current_epoch: 0,
        }
    }

    pub fn register_validator(
        &mut self,
        registration: ValidatorRegistration,
    ) -> Result<String, String> {
        // Check if already registered
        if self.validators.contains_key(&registration.address) {
            return Err("Validator already registered".to_string());
        }

        // Check if pending
        if self
            .pending_registrations
            .contains_key(&registration.address)
        {
            return Err("Registration already pending".to_string());
        }

        // Validate stake amount
        if registration.stake_amount < self.min_stake_amount {
            return Err(format!(
                "Insufficient stake. Minimum required: {}",
                self.min_stake_amount
            ));
        }

        // Add to pending registrations
        self.pending_registrations
            .insert(registration.address.clone(), registration);

        Ok("Validator registration submitted successfully".to_string())
    }

    pub fn approve_registration(&mut self, address: &str) -> Result<(), String> {
        if let Some(registration) = self.pending_registrations.remove(address) {
            let mut validator = Validator::new(
                registration.address.clone(),
                registration.public_key,
                registration.name,
                registration.stake_amount,
            );

            validator.status = ValidatorStatus::Active;

            // New validators start fully healthy, then consensus updates the score from activity.
            validator.synergy_score = INITIAL_VALIDATOR_SYNERGY_SCORE;
            validator.uptime_percentage = 100.0;

            // Ensure stake amount is properly set (this was the missing piece)
            validator.stake_amount = registration.stake_amount;
            validator.min_stake_required = registration.stake_amount;

            self.validators.insert(address.to_string(), validator);

            // Trigger cluster reorganization
            self.reorganize_clusters();

            Ok(())
        } else {
            Err("No pending registration found".to_string())
        }
    }

    pub fn update_validator_performance(
        &mut self,
        address: &str,
        performance_data: ValidatorPerformanceUpdate,
    ) {
        if let Some(validator) = self.validators.get_mut(address) {
            validator.update_activity();

            match performance_data.update_type.as_str() {
                "block_produced" => {
                    validator.record_block_production();
                }
                "block_missed" => {
                    validator.record_missed_block();
                }
                "double_sign" => {
                    validator.record_double_sign();
                }
                "uptime_update" => {
                    if let Some(uptime) = performance_data.value {
                        validator.uptime_percentage = uptime;
                    }
                }
                "accuracy_update" => {
                    if let Some(accuracy) = performance_data.value {
                        validator.task_accuracy = accuracy;
                    }
                }
                _ => {}
            }

            validator.calculate_synergy_score();
        }
    }

    pub fn get_active_validators(&self) -> Vec<&Validator> {
        self.validators
            .values()
            .filter(|v| v.status == ValidatorStatus::Active && v.is_eligible(self.min_stake_amount))
            .collect()
    }

    pub fn get_validator_by_address(&self, address: &str) -> Option<&Validator> {
        self.validators.get(address)
    }

    pub fn reorganize_clusters(&mut self) {
        let cluster_size = self.cluster_size.max(1);
        let mut active_validators: Vec<Validator> =
            self.get_active_validators().into_iter().cloned().collect();

        for validator in self.validators.values_mut() {
            validator.cluster_id = None;
        }
        self.clusters.clear();

        if active_validators.is_empty() {
            return;
        }

        active_validators.sort_by(|a, b| {
            b.synergy_score
                .partial_cmp(&a.synergy_score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.address.cmp(&b.address))
        });

        let cluster_count = active_validators.len().div_ceil(cluster_size);
        let base_cluster_size = active_validators.len() / cluster_count;
        let extra_members = active_validators.len() % cluster_count;
        let target_sizes: Vec<usize> = (0..cluster_count)
            .map(|index| base_cluster_size + usize::from(index < extra_members))
            .collect();
        let mut cluster_members: Vec<Vec<Validator>> = (0..cluster_count).map(|_| Vec::new()).collect();
        let mut next_cluster_index = 0usize;

        for validator in active_validators {
            while cluster_members[next_cluster_index].len() >= target_sizes[next_cluster_index] {
                next_cluster_index = (next_cluster_index + 1) % cluster_count;
            }
            cluster_members[next_cluster_index].push(validator);
            next_cluster_index = (next_cluster_index + 1) % cluster_count;
        }

        let now = Validator::current_timestamp();
        for (cluster_index, members) in cluster_members.into_iter().enumerate() {
            let cluster_id = cluster_index as u64;
            let cluster_group = ((cluster_id % 5) + 1) as u8;
            let validator_addresses: Vec<String> =
                members.iter().map(|validator| validator.address.clone()).collect();
            let cluster_seed = format!("cluster-{}-{}", cluster_id, validator_addresses.join("-"));
            let total_stake = members.iter().map(|validator| validator.stake_amount).sum();
            let average_synergy_score = members
                .iter()
                .map(|validator| validator.synergy_score)
                .sum::<f64>()
                / members.len() as f64;

            self.clusters.insert(
                cluster_id,
                ValidatorCluster {
                    id: cluster_id,
                    address: generate_cluster_address(&cluster_seed, cluster_group),
                    validators: validator_addresses.clone(),
                    total_stake,
                    average_synergy_score,
                    created_at: now,
                    last_rotation: now,
                    group: cluster_group,
                },
            );

            for address in validator_addresses {
                if let Some(validator) = self.validators.get_mut(&address) {
                    validator.cluster_id = Some(cluster_id);
                }
            }
        }
    }

    pub fn get_validator_cluster(&self, address: &str) -> Option<&ValidatorCluster> {
        if let Some(validator) = self.validators.get(address) {
            if let Some(cluster_id) = validator.cluster_id {
                return self.clusters.get(&cluster_id);
            }
        }
        None
    }

    pub fn slash_validator(&mut self, address: &str, reason: &str) -> Result<(), String> {
        if let Some(validator) = self.validators.get_mut(address) {
            match reason {
                "double_sign" => {
                    validator.record_double_sign();
                    validator.status = ValidatorStatus::Slashed;
                    self.jailed_validators.insert(address.to_string());
                }
                "inactivity" => {
                    validator.status = ValidatorStatus::Jailed;
                    self.jailed_validators.insert(address.to_string());
                }
                _ => {
                    return Err("Unknown slashing reason".to_string());
                }
            }

            // Trigger cluster reorganization
            self.reorganize_clusters();

            Ok(())
        } else {
            Err("Validator not found".to_string())
        }
    }

    pub fn unjail_validator(&mut self, address: &str) -> Result<(), String> {
        if let Some(validator) = self.validators.get_mut(address) {
            if self.jailed_validators.remove(address) {
                validator.status = ValidatorStatus::Active;
                validator.double_signs = 0;
                validator.missed_blocks = 0;
                validator.update_activity();
                self.reorganize_clusters();
                Ok(())
            } else {
                Err("Validator is not jailed".to_string())
            }
        } else {
            Err("Validator not found".to_string())
        }
    }

    pub fn get_top_validators(&self, count: usize) -> Vec<&Validator> {
        let mut validators: Vec<_> = self.validators.values().collect();
        validators.sort_by(|a, b| b.synergy_score.partial_cmp(&a.synergy_score).unwrap());
        validators.into_iter().take(count).collect()
    }

    pub fn calculate_epoch_rewards(&self, _epoch: u64) -> HashMap<String, u64> {
        let mut rewards = HashMap::new();

        // Ensure we have active validators with stakes
        let active_validators: Vec<_> = self
            .validators
            .values()
            .filter(|v| v.status == ValidatorStatus::Active && v.stake_amount > 0)
            .collect();

        if active_validators.is_empty() {
            // If no active validators with stakes, return empty rewards
            return rewards;
        }

        for validator in active_validators {
            if validator.is_eligible(self.min_stake_amount) {
                // Calculate rewards based on synergy score and stake
                let base_reward = 100; // Base reward per epoch
                let synergy_multiplier = validator.synergy_score / 100.0;
                let stake_multiplier =
                    (validator.stake_amount as f64 / self.min_stake_amount as f64).min(3.0);

                let total_reward =
                    (base_reward as f64 * synergy_multiplier * stake_multiplier) as u64;
                rewards.insert(validator.address.clone(), total_reward);
            }
        }

        rewards
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let registry: ValidatorRegistry = serde_json::from_str(&content)?;
        Ok(registry)
    }
}

#[derive(Debug, Clone)]
pub struct ValidatorPerformanceUpdate {
    pub validator_address: String,
    pub update_type: String, // "block_produced", "block_missed", "uptime_update", etc.
    pub value: Option<f64>,
    pub timestamp: u64,
}

impl ValidatorManager {
    pub fn new() -> Self {
        ValidatorManager {
            registry: Arc::new(Mutex::new(ValidatorRegistry::new())),
        }
    }

    pub fn register_validator(
        &self,
        registration: ValidatorRegistration,
    ) -> Result<String, String> {
        if let Ok(mut registry) = self.registry.lock() {
            registry.register_validator(registration)
        } else {
            Err("Failed to acquire registry lock".to_string())
        }
    }

    pub fn approve_validator(&self, address: &str) -> Result<(), String> {
        if let Ok(mut registry) = self.registry.lock() {
            // First try to approve from pending registrations
            if registry.approve_registration(address).is_ok() {
                return Ok(());
            }

            // If not in pending, check if already registered but not approved
            if let Some(validator) = registry.validators.get(address) {
                if validator.status != ValidatorStatus::Active {
                    // Create a new active validator with proper defaults
                    let mut active_validator = validator.clone();
                    active_validator.status = ValidatorStatus::Active;
                    active_validator.synergy_score = INITIAL_VALIDATOR_SYNERGY_SCORE;
                    active_validator.uptime_percentage = 100.0;
                    registry
                        .validators
                        .insert(address.to_string(), active_validator);
                    registry.reorganize_clusters();
                    return Ok(());
                }
            }

            Err("Validator not found or already active".to_string())
        } else {
            Err("Failed to acquire registry lock".to_string())
        }
    }

    pub fn update_performance(&self, update: ValidatorPerformanceUpdate) {
        if let Ok(mut registry) = self.registry.lock() {
            registry.update_validator_performance(&update.validator_address.clone(), update);
        }
    }

    pub fn update_synergy_score(&self, address: &str, score: f64) -> bool {
        if let Ok(mut registry) = self.registry.lock() {
            if let Some(validator) = registry.validators.get_mut(address) {
                validator.synergy_score = score;
                return true;
            }
        }
        false
    }

    pub fn update_validator_stake(&self, address: &str, stake_amount: u64) -> bool {
        if let Ok(mut registry) = self.registry.lock() {
            if let Some(validator) = registry.validators.get_mut(address) {
                validator.stake_amount = stake_amount;
                validator.min_stake_required = stake_amount;
                validator.calculate_synergy_score();
                return true;
            }
        }
        false
    }

    pub fn get_validator(&self, address: &str) -> Option<Validator> {
        if let Ok(registry) = self.registry.lock() {
            registry.get_validator_by_address(address).cloned()
        } else {
            None
        }
    }

    pub fn get_validator_cluster(&self, address: &str) -> Option<ValidatorCluster> {
        if let Ok(registry) = self.registry.lock() {
            registry.get_validator_cluster(address).cloned()
        } else {
            None
        }
    }

    pub fn get_active_validators(&self) -> Vec<Validator> {
        if let Ok(registry) = self.registry.lock() {
            validator_log!(
                "🔍 [get_active_validators] Total validators in registry: {}",
                registry.validators.len()
            );
            validator_log!(
                "🔍 [get_active_validators] Min stake amount: {}",
                registry.min_stake_amount
            );
            let active_validators: Vec<Validator> = registry.validators.values()
                .filter(|v| {
                    let is_active = v.status == ValidatorStatus::Active;
                    let is_eligible = v.is_eligible(registry.min_stake_amount);
                    validator_log!("🔍 [get_active_validators] Validator {}: Active={}, Eligible={}, Stake={}, Score={}, Uptime={}",
                        v.address, is_active, is_eligible, v.stake_amount, v.synergy_score, v.uptime_percentage);
                    is_active && is_eligible
                })
                .cloned()
                .collect();
            validator_log!(
                "🔍 [get_active_validators] Returning {} active validators",
                active_validators.len()
            );
            active_validators
        } else {
            validator_log!("⚠️ [get_active_validators] Failed to acquire registry lock!");
            Vec::new()
        }
    }

    pub fn get_all_validators(&self) -> Vec<Validator> {
        if let Ok(registry) = self.registry.lock() {
            registry.validators.values().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_validator_count(&self) -> usize {
        if let Ok(registry) = self.registry.lock() {
            registry.validators.len()
        } else {
            0
        }
    }

    pub fn get_cluster_count(&self) -> usize {
        if let Ok(registry) = self.registry.lock() {
            registry.clusters.len()
        } else {
            0
        }
    }

    pub fn get_total_stake(&self) -> u64 {
        if let Ok(registry) = self.registry.lock() {
            registry
                .validators
                .values()
                .map(|validator| validator.stake_amount)
                .sum()
        } else {
            0
        }
    }

    pub fn slash_validator(&self, address: &str, reason: &str) -> Result<(), String> {
        if let Ok(mut registry) = self.registry.lock() {
            registry.slash_validator(address, reason)
        } else {
            Err("Failed to acquire registry lock".to_string())
        }
    }

    pub fn get_top_validators(&self, count: usize) -> Vec<Validator> {
        if let Ok(registry) = self.registry.lock() {
            registry
                .get_top_validators(count)
                .into_iter()
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn calculate_epoch_rewards(&self, epoch: u64) -> HashMap<String, u64> {
        if let Ok(registry) = self.registry.lock() {
            registry.calculate_epoch_rewards(epoch)
        } else {
            HashMap::new()
        }
    }

    pub fn save_registry(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(registry) = self.registry.lock() {
            registry.save_to_file(path)
        } else {
            Err("Failed to acquire registry lock".into())
        }
    }

    pub fn load_registry(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let registry = ValidatorRegistry::load_from_file(path)?;
        if let Ok(mut current_registry) = self.registry.lock() {
            *current_registry = registry;
        }
        Ok(())
    }

    pub fn is_pending(&self, address: &str) -> bool {
        if let Ok(registry) = self.registry.lock() {
            registry.pending_registrations.contains_key(address)
        } else {
            false
        }
    }
}

// Global validator manager instance
lazy_static::lazy_static! {
    pub static ref VALIDATOR_MANAGER: Arc<ValidatorManager> = Arc::new(ValidatorManager::new());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending_registration(index: usize) -> ValidatorRegistration {
        ValidatorRegistration {
            address: format!("validator-{}", index),
            public_key: format!("validator-key-{}", index),
            name: format!("Validator {}", index),
            stake_amount: 1_000,
            submitted_at: 0,
            registration_tx_hash: format!("registration-{}", index),
        }
    }

    fn active_registry(count: usize) -> ValidatorRegistry {
        let mut registry = ValidatorRegistry::new();
        for index in 0..count {
            let mut validator = Validator::new(
                format!("validator-{}", index),
                format!("validator-key-{}", index),
                format!("Validator {}", index),
                1_000,
            );
            validator.status = ValidatorStatus::Active;
            validator.synergy_score = INITIAL_VALIDATOR_SYNERGY_SCORE - index as f64;
            registry.validators.insert(validator.address.clone(), validator);
        }
        registry.reorganize_clusters();
        registry
    }

    #[test]
    fn approved_validators_start_at_full_synergy_score() {
        let mut registry = ValidatorRegistry::new();
        let registration = pending_registration(1);
        let address = registration.address.clone();

        registry
            .register_validator(registration)
            .expect("validator registration should be accepted");
        registry
            .approve_registration(&address)
            .expect("pending validator should be approved");

        let validator = registry
            .get_validator_by_address(&address)
            .expect("approved validator should exist");
        assert_eq!(validator.status, ValidatorStatus::Active);
        assert_eq!(validator.synergy_score, INITIAL_VALIDATOR_SYNERGY_SCORE);
        assert_eq!(validator.uptime_percentage, 100.0);
    }

    #[test]
    fn reorganize_clusters_balances_six_validators_into_two_clusters() {
        let registry = active_registry(6);
        let mut cluster_sizes: Vec<usize> = registry
            .clusters
            .values()
            .map(|cluster| cluster.validators.len())
            .collect();
        cluster_sizes.sort_unstable();

        assert_eq!(registry.clusters.len(), 2);
        assert_eq!(cluster_sizes, vec![3, 3]);
    }

    #[test]
    fn reorganize_clusters_adds_a_new_cluster_every_five_validators() {
        let registry = active_registry(16);
        let mut cluster_sizes: Vec<usize> = registry
            .clusters
            .values()
            .map(|cluster| cluster.validators.len())
            .collect();
        cluster_sizes.sort_unstable();

        assert_eq!(registry.clusters.len(), 4);
        assert_eq!(cluster_sizes, vec![4, 4, 4, 4]);
        assert!(cluster_sizes.iter().all(|size| *size <= TESTNET_BETA_VALIDATOR_CLUSTER_SIZE));
    }
}
