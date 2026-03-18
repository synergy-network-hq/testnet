use crate::crypto::pqc::PQCManager;
use crate::validator::{Validator, ValidatorManager};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

const VERBOSE_SYNERGY_LOGS: bool = false;

macro_rules! synergy_log {
    ($($arg:tt)*) => {
        if VERBOSE_SYNERGY_LOGS {
            println!($($arg)*);
        }
    };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynergyScoreComponents {
    pub stake_weight: f64,
    pub reputation: f64,
    pub contribution_index: f64,
    pub cartelization_penalty: f64,
    pub normalized_score: f64,
    pub last_updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorMetrics {
    pub stake_amount: u64,
    pub blocks_participated: u64,
    pub blocks_eligible: u64,
    pub correct_votes: u64,
    pub total_votes: u64,
    pub successful_proposals: u64,
    pub relay_assists: u64,
    pub average_latency: f64,
    pub slashing_penalty: f64,
    pub last_update_block: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochSnapshot {
    pub epoch_number: u64,
    pub total_stake: u64,
    pub active_validator_count: usize,
    pub individual_synergy_scores: HashMap<String, f64>,
    pub merkle_root: String,
    pub timestamp: u64,
}

#[derive(Debug)]
pub struct SynergyScoreCalculator {
    pub validator_manager: Arc<ValidatorManager>,
    pub pqc_manager: Arc<Mutex<PQCManager>>,
    pub stake_cap: f64,
    pub decay_rate: f64,
    pub contribution_coefficients: (f64, f64, f64),
    pub correlation_threshold: f64,
    pub timing_similarity_threshold: f64,
    pub cartel_size_threshold: usize,
    pub epoch_length: u64,
}

impl SynergyScoreCalculator {
    pub fn new(
        validator_manager: Arc<ValidatorManager>,
        pqc_manager: Arc<Mutex<PQCManager>>,
    ) -> Self {
        SynergyScoreCalculator {
            validator_manager,
            pqc_manager,
            stake_cap: 0.05,                            // 5% cap
            decay_rate: 0.0001,                         // per block
            contribution_coefficients: (0.5, 0.3, 0.2), // proposals, relay_assists, network_score
            correlation_threshold: 0.85,
            timing_similarity_threshold: 0.9,
            cartel_size_threshold: 10,
            epoch_length: 1000,
        }
    }

    pub fn calculate_synergy_score(&self, validator: &Validator) -> SynergyScoreComponents {
        synergy_log!(
            "    🔍 [calculate_synergy_score] START for validator: {}",
            validator.address
        );

        synergy_log!("    📏 [calculate_synergy_score] Calculating stake_weight...");
        let stake_weight = self.calculate_stake_weight(validator);
        synergy_log!(
            "    ✅ [calculate_synergy_score] stake_weight: {}",
            stake_weight
        );

        synergy_log!("    📈 [calculate_synergy_score] Calculating reputation...");
        let reputation = self.calculate_reputation(validator);
        synergy_log!(
            "    ✅ [calculate_synergy_score] reputation: {}",
            reputation
        );

        synergy_log!("    🎯 [calculate_synergy_score] Calculating contribution_index...");
        let contribution_index = self.calculate_contribution_index(validator);
        synergy_log!(
            "    ✅ [calculate_synergy_score] contribution_index: {}",
            contribution_index
        );

        synergy_log!("    🚫 [calculate_synergy_score] Calculating cartelization_penalty...");
        let cartelization_penalty = self.calculate_cartelization_penalty(validator);
        synergy_log!(
            "    ✅ [calculate_synergy_score] cartelization_penalty: {}",
            cartelization_penalty
        );

        let raw_score = Self::raw_score_from_components(
            stake_weight,
            reputation,
            contribution_index,
            cartelization_penalty,
        );
        synergy_log!("    🧮 [calculate_synergy_score] raw_score: {}", raw_score);

        let normalized_score = self.normalize_score(raw_score);
        synergy_log!(
            "    ✅ [calculate_synergy_score] normalized_score: {}",
            normalized_score
        );

        SynergyScoreComponents {
            stake_weight,
            reputation,
            contribution_index,
            cartelization_penalty,
            normalized_score,
            last_updated: Self::current_timestamp(),
        }
    }

    fn calculate_stake_weight(&self, validator: &Validator) -> f64 {
        let total_stake = self.get_total_stake();
        let stake_fraction = validator.stake_amount as f64 / total_stake as f64;
        stake_fraction.min(self.stake_cap)
    }

    fn calculate_reputation(&self, validator: &Validator) -> f64 {
        let uptime_factor = self.calculate_uptime_factor(validator);
        let accuracy_factor = self.calculate_accuracy_factor(validator);
        let slashing_penalty = self.calculate_decayed_penalty(validator);

        uptime_factor * accuracy_factor * (1.0 - slashing_penalty)
    }

    fn calculate_uptime_factor(&self, validator: &Validator) -> f64 {
        let blocks_participated = validator.total_blocks_produced;
        let blocks_eligible = validator.total_blocks_produced + validator.missed_blocks;
        if blocks_eligible == 0 {
            1.0
        } else {
            blocks_participated as f64 / blocks_eligible as f64
        }
    }

    fn calculate_accuracy_factor(&self, validator: &Validator) -> f64 {
        let correct_votes = validator.total_transactions_validated;
        let total_votes = correct_votes + validator.missed_blocks;
        if total_votes == 0 {
            1.0
        } else {
            correct_votes as f64 / total_votes as f64
        }
    }

    fn calculate_decayed_penalty(&self, validator: &Validator) -> f64 {
        // Apply exponential decay to the slashing penalty as per Equation 5 in PoSy.txt
        let decayed_penalty = validator.slashing_penalty * (-self.decay_rate).exp();
        decayed_penalty.min(1.0) // Ensure penalty doesn't exceed 1.0
    }

    fn calculate_contribution_index(&self, validator: &Validator) -> f64 {
        let proposals = validator.total_blocks_produced as f64;
        let relay_assists = validator.collaboration_score;
        let network_score = 1.0 / validator.average_block_time.max(0.1);

        let (alpha, beta, gamma) = self.contribution_coefficients;
        alpha * proposals + beta * relay_assists + gamma * network_score
    }

    pub fn calculate_pairwise_synergy(
        &self,
        validator1: &Validator,
        validator2: &Validator,
    ) -> f64 {
        // Calculate pairwise synergy between two validators
        let components1 = self.calculate_synergy_score(validator1);
        let components2 = self.calculate_synergy_score(validator2);

        // Use geometric mean of normalized scores as pairwise synergy
        (components1.normalized_score * components2.normalized_score).sqrt()
    }

    pub fn normalize_scores(&self, scores: &[f64]) -> Vec<f64> {
        // Normalize a set of scores to sum to 1.0
        let total: f64 = scores.iter().sum();
        if total == 0.0 {
            vec![0.0; scores.len()]
        } else {
            scores.iter().map(|&score| score / total).collect()
        }
    }

    pub fn apply_decay_factor(&self, score: f64, blocks_since_last_update: u64) -> f64 {
        // Apply exponential decay to a score based on time since last update
        let decay_factor = (-self.decay_rate * blocks_since_last_update as f64).exp();
        score * decay_factor
    }

    fn calculate_cartelization_penalty(&self, validator: &Validator) -> f64 {
        let correlation_factor = self.detect_cartel_correlation(validator);
        let cartel_size = self.detect_cartel_size(validator);

        if cartel_size >= self.cartel_size_threshold
            && correlation_factor > self.correlation_threshold
        {
            1.0 + correlation_factor * cartel_size as f64 * 0.1
        } else {
            1.0
        }
    }

    fn detect_cartel_correlation(&self, _validator: &Validator) -> f64 {
        // Simplified cartel detection
        // In full implementation, construct_vote_vector and calculate_pairwise_correlations methods would be needed
        // For now, return a baseline value indicating no cartel behavior detected
        0.0
    }

    fn detect_cartel_size(&self, validator: &Validator) -> usize {
        // Simplified cartel detection
        // Full implementation would require historical analysis
        if validator.double_signs > 0 {
            5 // Assume small cartel if double signing detected
        } else {
            1
        }
    }

    fn normalize_score(&self, raw_score: f64) -> f64 {
        let max_raw = self.calculate_max_raw_score();

        if max_raw == 0.0 {
            return raw_score.min(100.0);
        }

        ((raw_score / max_raw) * 100.0).min(100.0)
    }

    fn get_total_stake(&self) -> u64 {
        let validators = self.validator_manager.get_active_validators();
        validators.iter().map(|v| v.stake_amount).sum()
    }

    fn calculate_raw_score(&self, validator: &Validator) -> f64 {
        let stake_weight = self.calculate_stake_weight(validator);
        let reputation = self.calculate_reputation(validator);
        let contribution_index = self.calculate_contribution_index(validator);
        let cartelization_penalty = self.calculate_cartelization_penalty(validator);

        Self::raw_score_from_components(
            stake_weight,
            reputation,
            contribution_index,
            cartelization_penalty,
        )
    }

    fn calculate_raw_scores(&self) -> Vec<f64> {
        self.validator_manager
            .get_all_validators()
            .into_iter()
            .map(|validator| self.calculate_raw_score(&validator))
            .collect()
    }

    fn calculate_max_raw_score(&self) -> f64 {
        self.calculate_raw_scores().into_iter().fold(0.0, f64::max)
    }

    fn raw_score_from_components(
        stake_weight: f64,
        reputation: f64,
        contribution_index: f64,
        cartelization_penalty: f64,
    ) -> f64 {
        if cartelization_penalty == 0.0 {
            0.0
        } else {
            (stake_weight * reputation * contribution_index) / cartelization_penalty
        }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

#[derive(Debug, Clone)]
pub struct CartelDetection {
    pub vote_history: HashMap<String, Vec<bool>>, // validator_address -> vote vector
    pub timing_data: HashMap<String, Vec<u64>>,   // validator_address -> timestamps
    pub correlation_matrix: HashMap<(String, String), f64>,
}

impl CartelDetection {
    pub fn new() -> Self {
        CartelDetection {
            vote_history: HashMap::new(),
            timing_data: HashMap::new(),
            correlation_matrix: HashMap::new(),
        }
    }

    pub fn record_vote(&mut self, validator_address: &str, voted: bool, timestamp: u64) {
        self.vote_history
            .entry(validator_address.to_string())
            .or_insert_with(Vec::new)
            .push(voted);

        self.timing_data
            .entry(validator_address.to_string())
            .or_insert_with(Vec::new)
            .push(timestamp);
    }

    pub fn detect_cartels(&mut self) -> HashMap<String, f64> {
        let mut cartel_penalties = HashMap::new();
        let validators: Vec<String> = self.vote_history.keys().cloned().collect();

        // Calculate pairwise correlations
        for i in 0..validators.len() {
            for j in i + 1..validators.len() {
                let v1 = &validators[i];
                let v2 = &validators[j];

                if let (Some(votes1), Some(votes2)) =
                    (self.vote_history.get(v1), self.vote_history.get(v2))
                {
                    let correlation = self.calculate_pearson_correlation(votes1, votes2);
                    self.correlation_matrix
                        .insert((v1.clone(), v2.clone()), correlation);
                }
            }
        }

        // Identify cartels based on correlation and timing
        for validator in &validators {
            let penalty = self.calculate_cartel_penalty(validator);
            if penalty > 1.0 {
                cartel_penalties.insert(validator.clone(), penalty);
            }
        }

        cartel_penalties
    }

    fn calculate_pearson_correlation(&self, votes1: &[bool], votes2: &[bool]) -> f64 {
        let n = votes1.len().min(votes2.len());
        if n == 0 {
            return 0.0;
        }

        let mut sum1 = 0.0;
        let mut sum2 = 0.0;
        let mut sum1_sq = 0.0;
        let mut sum2_sq = 0.0;
        let mut p_sum = 0.0;

        for i in 0..n {
            let x = if votes1[i] { 1.0 } else { 0.0 };
            let y = if votes2[i] { 1.0 } else { 0.0 };

            sum1 += x;
            sum2 += y;
            sum1_sq += x * x;
            sum2_sq += y * y;
            p_sum += x * y;
        }

        let num = p_sum - (sum1 * sum2 / n as f64);
        let den1 = (sum1_sq - (sum1 * sum1 / n as f64)).sqrt();
        let den2 = (sum2_sq - (sum2 * sum2 / n as f64)).sqrt();

        if den1 == 0.0 || den2 == 0.0 {
            0.0
        } else {
            num / (den1 * den2)
        }
    }

    fn calculate_cartel_penalty(&self, validator: &str) -> f64 {
        let mut total_correlation = 0.0;
        let mut cartel_size = 0;

        if let Some(timestamps) = self.timing_data.get(validator) {
            for (other_validator, correlation) in &self.correlation_matrix {
                if other_validator.0 == *validator || other_validator.1 == *validator {
                    let other = if other_validator.0 == *validator {
                        &other_validator.1
                    } else {
                        &other_validator.0
                    };

                    if *correlation > 0.85 {
                        if let Some(other_timestamps) = self.timing_data.get(other) {
                            let timing_similarity =
                                self.calculate_timing_similarity(timestamps, other_timestamps);
                            if timing_similarity > 0.9 {
                                total_correlation += *correlation;
                                cartel_size += 1;
                            }
                        }
                    }
                }
            }
        }

        if cartel_size > 0 {
            let avg_correlation = total_correlation / cartel_size as f64;
            1.0 + avg_correlation * cartel_size as f64 * 0.1
        } else {
            1.0
        }
    }

    fn calculate_timing_similarity(&self, timestamps1: &[u64], timestamps2: &[u64]) -> f64 {
        let n = timestamps1.len().min(timestamps2.len());
        if n == 0 {
            return 0.0;
        }

        let median1 = self.calculate_median(timestamps1);
        let median2 = self.calculate_median(timestamps2);
        let block_time = 5.0; // average block time in seconds

        1.0 - (median1 as f64 - median2 as f64).abs() / block_time
    }

    fn calculate_median(&self, values: &[u64]) -> u64 {
        let mut sorted = values.to_vec();
        sorted.sort();
        let len = sorted.len();
        if len == 0 {
            0
        } else if len % 2 == 0 {
            (sorted[len / 2 - 1] + sorted[len / 2]) / 2
        } else {
            sorted[len / 2]
        }
    }
}
