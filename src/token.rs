use crate::genesis::canonical_genesis;
use crate::transaction::Transaction;
use crate::warn;
use hex;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub total_supply: String,
    pub max_supply: Option<String>,
    pub mintable: bool,
    pub burnable: bool,
    pub created_at: u64,
    pub creator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    pub address: String,
    pub token_symbol: String,
    pub balance: u64,
    pub locked_balance: u64,
    pub staked_balance: u64,
    pub last_updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTransfer {
    pub from: String,
    pub to: String,
    pub token_symbol: String,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: u64,
    pub tx_hash: String,
    pub block_height: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingInfo {
    pub validator_address: String,
    pub staker_address: String,
    pub amount: u64,
    pub stake_start: u64,
    pub stake_end: Option<u64>,
    pub rewards_earned: u64,
    pub is_active: bool,
}

#[derive(Debug)]
pub struct TokenManager {
    tokens: Arc<Mutex<HashMap<String, Token>>>,
    pub balances: Arc<Mutex<HashMap<String, HashMap<String, u64>>>>, // address -> token_symbol -> balance
    _locked_balances: Arc<Mutex<HashMap<String, HashMap<String, u64>>>>, // address -> token_symbol -> locked
    staked_balances: Arc<Mutex<HashMap<String, HashMap<String, u64>>>>, // address -> token_symbol -> staked
    transfers: Arc<Mutex<Vec<TokenTransfer>>>,
    stakes: Arc<Mutex<HashMap<String, Vec<StakingInfo>>>>, // validator -> stakes
    total_supply: Arc<Mutex<HashMap<String, u128>>>,       // token_symbol -> total_supply
}

impl Token {
    pub fn new(
        symbol: String,
        name: String,
        decimals: u8,
        total_supply: u128,
        max_supply: Option<u128>,
        mintable: bool,
        burnable: bool,
        creator: String,
    ) -> Self {
        Token {
            symbol,
            name,
            decimals,
            total_supply: total_supply.to_string(),
            max_supply: max_supply.map(|value| value.to_string()),
            mintable,
            burnable,
            created_at: Self::current_timestamp(),
            creator,
        }
    }

    pub fn calculate_amount(&self, raw_amount: u64) -> u64 {
        raw_amount * 10u64.pow(self.decimals as u32)
    }

    pub fn format_amount(&self, amount: u64) -> String {
        format!(
            "{:.1$}",
            amount as f64 / 10u64.pow(self.decimals as u32) as f64,
            self.decimals as usize
        )
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

impl TokenManager {
    pub fn new() -> Self {
        let mut manager = TokenManager {
            tokens: Arc::new(Mutex::new(HashMap::new())),
            balances: Arc::new(Mutex::new(HashMap::new())),
            _locked_balances: Arc::new(Mutex::new(HashMap::new())),
            staked_balances: Arc::new(Mutex::new(HashMap::new())),
            transfers: Arc::new(Mutex::new(Vec::new())),
            stakes: Arc::new(Mutex::new(HashMap::new())),
            total_supply: Arc::new(Mutex::new(HashMap::new())),
        };

        // Initialize with SNRG token
        manager.initialize_snrg_token();
        manager
    }

    fn initialize_snrg_token(&mut self) {
        let genesis_token = canonical_genesis()
            .ok()
            .map(|genesis| genesis.token().clone());
        let snrg_token = Token::new(
            "SNRG".to_string(),
            genesis_token
                .as_ref()
                .map(|token| token.name.clone())
                .unwrap_or_else(|| "Synergy Token".to_string()),
            genesis_token
                .as_ref()
                .map(|token| token.decimals)
                .unwrap_or(9),
            0,
            genesis_token
                .as_ref()
                .map(|token| token.total_supply_cap_nwei)
                .or(Some(1_150_000u128 * 10u128.pow(9))),
            true, // mintable during bootstrap
            true, // burnable
            "genesis".to_string(),
        );

        if let Ok(mut tokens) = self.tokens.lock() {
            tokens.insert("SNRG".to_string(), snrg_token.clone());
        }

        if let Ok(mut supply) = self.total_supply.lock() {
            supply.insert("SNRG".to_string(), 0); // Start with 0 supply
        }

        // Distribute initial supply to genesis accounts
        self.distribute_genesis_supply();

        // Testnet-Beta SNRG supply is fixed after genesis bootstrap.
        if let Ok(mut tokens) = self.tokens.lock() {
            if let Some(token) = tokens.get_mut("SNRG") {
                token.mintable = false;
            }
        }
    }

    fn distribute_genesis_supply(&self) {
        if let Ok(genesis) = canonical_genesis() {
            for balance in genesis.balances() {
                if let Ok(_) = self.mint_tokens(&balance.address, "SNRG", balance.balance_nwei) {
                    println!(
                        "✅ Genesis allocation: {} SNRG to {}",
                        balance.balance_nwei, balance.address
                    );
                }
            }
            return;
        }

        println!(
            "⚠️ Could not load canonical genesis for token allocations, using default allocations"
        );
        // Fallback to hardcoded allocations if genesis.json is not available.
        // These MUST match the genesis.json allocations exactly.
        let genesis_allocations = [
            (
                "synu1nd0fvzfhhj4s0te3ks06csfsnpg2hed8vsmh",
                400_000_000_000_000u64,
            ),
            (
                "synw1pckkuqdeep4qz47ww9hnnm6uru2f9r6qtumv",
                150_000_000_000_000u64,
            ),
            (
                "synw1vkn2dq8mftcn7nkdhyv5t0jrv83thf0cakkj",
                200_000_000_000_000u64,
            ),
            (
                "synw1prdr55ggjhupx0d7jycftrl2hzs3k8zuw5ad",
                100_000_000_000_000u64,
            ),
            (
                "synw1f2kpjt9flxl6y4e3uez0zp3hjanamrlew5ja",
                100_000_000_000_000u64,
            ),
            (
                "synv11cv5akg5xa86y8tc5jg84t7a5xhxenaypq36",
                50_000_000_000_000u64,
            ),
            (
                "synv11vwg95ecaryv33lrq6xptrg7vd5yrafturn4",
                50_000_000_000_000u64,
            ),
            (
                "synv113jp4578crnfnwg4d9r342euxfqf8a08s22g",
                50_000_000_000_000u64,
            ),
            (
                "synv11jlm4p4utpvj5ny0g8lnpa0ry65pkfecagnz",
                50_000_000_000_000u64,
            ),
        ];

        for (address, amount) in genesis_allocations {
            if let Ok(_) = self.mint_tokens(address, "SNRG", amount) {
                println!(
                    "✅ Fallback genesis allocation: {} SNRG to {}",
                    amount, address
                );
            }
        }
    }

    pub fn create_token(
        &self,
        symbol: String,
        name: String,
        decimals: u8,
        total_supply: u64,
        max_supply: Option<u64>,
        mintable: bool,
        burnable: bool,
        creator: String,
    ) -> Result<String, String> {
        if let Ok(mut tokens) = self.tokens.lock() {
            if tokens.contains_key(&symbol) {
                return Err(format!("Token {} already exists", symbol));
            }

            let token = Token::new(
                symbol.clone(),
                name,
                decimals,
                total_supply as u128,
                max_supply.map(u128::from),
                mintable,
                burnable,
                creator.clone(),
            );

            tokens.insert(symbol.clone(), token);

            if let Ok(mut supply) = self.total_supply.lock() {
                supply.insert(symbol.clone(), 0);
            }
            self.update_token_supply_snapshot(&symbol, 0);

            // Mint initial supply to creator
            let _ = self.mint_tokens(&creator, &symbol, total_supply);

            Ok(format!("Token {} created successfully", symbol))
        } else {
            Err("Failed to acquire lock".to_string())
        }
    }

    fn update_token_supply_snapshot(&self, token_symbol: &str, total_supply: u128) {
        if let Ok(mut tokens) = self.tokens.lock() {
            if let Some(token) = tokens.get_mut(token_symbol) {
                token.total_supply = total_supply.to_string();
            }
        }
    }

    pub fn mint_tokens(&self, to: &str, token_symbol: &str, amount: u64) -> Result<String, String> {
        if let Ok(mut tokens) = self.tokens.lock() {
            if let Some(token) = tokens.get(token_symbol) {
                if !token.mintable {
                    return Err("Token is not mintable".to_string());
                }

                if let Some(max_supply) = token
                    .max_supply
                    .as_deref()
                    .and_then(|value| value.parse::<u128>().ok())
                {
                    if let Ok(supply) = self.total_supply.lock() {
                        let current_supply = supply.get(token_symbol).unwrap_or(&0);
                        if *current_supply + amount as u128 > max_supply {
                            return Err("Maximum supply exceeded".to_string());
                        }
                    }
                }
            } else {
                return Err("Token not found".to_string());
            }

            // Update total supply and snapshot while holding tokens lock (mut)
            let new_total = if let Ok(mut supply) = self.total_supply.lock() {
                let current = *supply.get(token_symbol).unwrap_or(&0);
                let new_total = current + amount as u128;
                supply.insert(token_symbol.to_string(), new_total);
                new_total
            } else {
                return Err("Failed to acquire lock".to_string());
            };

            // Update token supply snapshot inline (tokens already locked as mut — no re-lock needed)
            if let Some(token) = tokens.get_mut(token_symbol) {
                token.total_supply = new_total.to_string();
            }

            // Update balance
            if let Ok(mut balances) = self.balances.lock() {
                let address_balances = balances.entry(to.to_string()).or_insert_with(HashMap::new);
                let current_balance = address_balances.get(token_symbol).unwrap_or(&0);
                address_balances.insert(token_symbol.to_string(), current_balance + amount);
            }

            Ok(format!("Minted {} {} to {}", amount, token_symbol, to))
        } else {
            Err("Failed to acquire lock".to_string())
        }
    }

    pub fn burn_tokens(
        &self,
        from: &str,
        token_symbol: &str,
        amount: u64,
    ) -> Result<String, String> {
        if let Ok(mut tokens) = self.tokens.lock() {
            if let Some(token) = tokens.get(token_symbol) {
                if !token.burnable {
                    return Err("Token is not burnable".to_string());
                }

                // Check balance
                let current_balance = self.get_balance(from, token_symbol);
                if current_balance < amount {
                    return Err("Insufficient balance".to_string());
                }
            } else {
                return Err("Token not found".to_string());
            }

            // Update balance
            if let Ok(mut balances) = self.balances.lock() {
                if let Some(address_balances) = balances.get_mut(from) {
                    let current = address_balances.get(token_symbol).unwrap_or(&0);
                    address_balances.insert(token_symbol.to_string(), current - amount);
                }
            }

            // Update total supply and snapshot inline (tokens already locked as mut — no re-lock needed)
            let new_total = if let Ok(mut supply) = self.total_supply.lock() {
                let current = *supply.get(token_symbol).unwrap_or(&0);
                let new_total = current.saturating_sub(amount as u128);
                supply.insert(token_symbol.to_string(), new_total);
                new_total
            } else {
                return Err("Failed to acquire lock".to_string());
            };

            if let Some(token) = tokens.get_mut(token_symbol) {
                token.total_supply = new_total.to_string();
            }

            Ok(format!("Burned {} {} from {}", amount, token_symbol, from))
        } else {
            Err("Failed to acquire lock".to_string())
        }
    }

    pub fn transfer_tokens(
        &self,
        from: &str,
        to: &str,
        token_symbol: &str,
        amount: u64,
        fee: u64,
    ) -> Result<String, String> {
        self.transfer_tokens_internal(from, to, token_symbol, amount, fee, None, 0)
    }

    /// Transfer tokens while recording the originating transaction hash and block height.
    /// This is used by consensus when a transaction is included in a block, so the explorer
    /// can attribute transfers to on-chain transactions.
    pub fn transfer_tokens_with_metadata(
        &self,
        from: &str,
        to: &str,
        token_symbol: &str,
        amount: u64,
        fee: u64,
        tx_hash: String,
        block_height: u64,
    ) -> Result<String, String> {
        self.transfer_tokens_internal(
            from,
            to,
            token_symbol,
            amount,
            fee,
            Some(tx_hash),
            block_height,
        )
    }

    fn transfer_tokens_internal(
        &self,
        from: &str,
        to: &str,
        token_symbol: &str,
        amount: u64,
        fee: u64,
        tx_hash: Option<String>,
        block_height: u64,
    ) -> Result<String, String> {
        let current_balance = self.get_balance(from, token_symbol);
        if current_balance < amount + fee {
            return Err("Insufficient balance for transfer and fee".to_string());
        }

        // Update sender balance
        if let Ok(mut balances) = self.balances.lock() {
            if let Some(from_balances) = balances.get_mut(from) {
                let current = from_balances.get(token_symbol).unwrap_or(&0);
                from_balances.insert(token_symbol.to_string(), current - amount - fee);
            }

            if let Some(to_balances) = balances.get_mut(to) {
                let current = to_balances.get(token_symbol).unwrap_or(&0);
                to_balances.insert(token_symbol.to_string(), current + amount);
            } else {
                let mut new_balances = HashMap::new();
                new_balances.insert(token_symbol.to_string(), amount);
                balances.insert(to.to_string(), new_balances);
            }
        }

        // Record transfer
        let transfer = TokenTransfer {
            from: from.to_string(),
            to: to.to_string(),
            token_symbol: token_symbol.to_string(),
            amount,
            fee,
            timestamp: Token::current_timestamp(),
            tx_hash: tx_hash
                .unwrap_or_else(|| Self::generate_tx_hash(from, to, token_symbol, amount, fee)),
            block_height,
        };

        if let Ok(mut transfers) = self.transfers.lock() {
            transfers.push(transfer);
        }

        Ok(format!(
            "Transferred {} {} from {} to {}",
            amount, token_symbol, from, to
        ))
    }

    pub fn get_balance(&self, address: &str, token_symbol: &str) -> u64 {
        if let Ok(balances) = self.balances.lock() {
            if let Some(address_balances) = balances.get(address) {
                return address_balances.get(token_symbol).unwrap_or(&0).clone();
            }
        }
        0
    }

    pub fn get_all_balances(&self, address: &str) -> HashMap<String, u64> {
        if let Ok(balances) = self.balances.lock() {
            balances.get(address).cloned().unwrap_or_default()
        } else {
            HashMap::new()
        }
    }

    pub fn stake_tokens(
        &self,
        staker: &str,
        validator: &str,
        token_symbol: &str,
        amount: u64,
    ) -> Result<String, String> {
        let current_balance = self.get_balance(staker, token_symbol);
        if current_balance < amount {
            return Err("Insufficient balance for staking".to_string());
        }

        // Move tokens from balance to staked balance
        if let Ok(mut balances) = self.balances.lock() {
            if let Some(staker_balances) = balances.get_mut(staker) {
                let current = staker_balances.get(token_symbol).unwrap_or(&0);
                staker_balances.insert(token_symbol.to_string(), current - amount);
            }
        }

        if let Ok(mut staked) = self.staked_balances.lock() {
            let staker_staked = staked
                .entry(staker.to_string())
                .or_insert_with(HashMap::new);
            let current = staker_staked.get(token_symbol).unwrap_or(&0);
            staker_staked.insert(token_symbol.to_string(), current + amount);
        }

        // Create staking info
        let stake_info = StakingInfo {
            validator_address: validator.to_string(),
            staker_address: staker.to_string(),
            amount,
            stake_start: Token::current_timestamp(),
            stake_end: None,
            rewards_earned: 0,
            is_active: true,
        };

        if let Ok(mut stakes) = self.stakes.lock() {
            let validator_stakes = stakes.entry(validator.to_string()).or_insert_with(Vec::new);
            validator_stakes.push(stake_info);
        }

        Ok(format!(
            "Staked {} {} to validator {}",
            amount, token_symbol, validator
        ))
    }

    pub fn unstake_tokens(
        &self,
        staker: &str,
        validator: &str,
        token_symbol: &str,
        amount: u64,
    ) -> Result<String, String> {
        // Check if staker has enough staked tokens
        let staked_balance = self.get_staked_balance(staker, token_symbol);
        if staked_balance < amount {
            return Err("Insufficient staked balance".to_string());
        }

        // Find and update the stake
        if let Ok(mut stakes) = self.stakes.lock() {
            if let Some(validator_stakes) = stakes.get_mut(validator) {
                for stake in validator_stakes.iter_mut() {
                    if stake.staker_address == staker && stake.is_active {
                        if stake.amount >= amount {
                            stake.amount -= amount;
                            if stake.amount == 0 {
                                stake.is_active = false;
                            }
                            break;
                        }
                    }
                }
            }
        }

        // Move tokens from staked back to balance
        if let Ok(mut balances) = self.balances.lock() {
            if let Some(staker_balances) = balances.get_mut(staker) {
                let current = staker_balances.get(token_symbol).unwrap_or(&0);
                staker_balances.insert(token_symbol.to_string(), current + amount);
            }
        }

        if let Ok(mut staked) = self.staked_balances.lock() {
            if let Some(staker_staked) = staked.get_mut(staker) {
                let current = staker_staked.get(token_symbol).unwrap_or(&0);
                staker_staked.insert(token_symbol.to_string(), current - amount);
            }
        }

        Ok(format!(
            "Unstaked {} {} from validator {}",
            amount, token_symbol, validator
        ))
    }

    pub fn get_staked_balance(&self, address: &str, token_symbol: &str) -> u64 {
        if let Ok(staked) = self.staked_balances.lock() {
            if let Some(address_staked) = staked.get(address) {
                return address_staked.get(token_symbol).unwrap_or(&0).clone();
            }
        }
        0
    }

    /// Distribute rewards to a cluster, then to validators in that cluster based on normalized synergy scores
    /// This implements the PoSy protocol where rewards are awarded to clusters first, then distributed
    /// among validators in the cluster based on their normalized Synergy Scores
    pub fn distribute_cluster_rewards(
        &self,
        cluster_validators: &[(String, f64)], // (validator_address, normalized_synergy_score)
        reward_amount: u64,
    ) -> Result<String, String> {
        const REWARDS_POOL: &str = "synw1zwy4m4mpdxyvz4nf8f7s0hk8nesc2cv09ex8pg";

        // Check rewards pool balance
        let pool_balance = self.get_balance(REWARDS_POOL, "SNRG");
        if pool_balance < reward_amount {
            return Err(format!(
                "Insufficient rewards pool balance: {} < {}",
                pool_balance, reward_amount
            ));
        }

        if cluster_validators.is_empty() {
            return Err("No validators in cluster".to_string());
        }

        // Verify normalized scores sum to approximately 1.0 (allowing for floating point precision)
        let total_score: f64 = cluster_validators.iter().map(|(_, score)| score).sum();
        if (total_score - 1.0).abs() > 0.01 {
            return Err(format!(
                "Invalid normalized scores: sum = {} (expected ~1.0)",
                total_score
            ));
        }

        // Deduct total reward from rewards pool
        if let Ok(mut balances) = self.balances.lock() {
            if let Some(pool_balances) = balances.get_mut(REWARDS_POOL) {
                let current = pool_balances.get("SNRG").unwrap_or(&0);
                if *current < reward_amount {
                    return Err(format!(
                        "Rewards pool balance check failed: {} < {}",
                        current, reward_amount
                    ));
                }
                pool_balances.insert("SNRG".to_string(), current - reward_amount);
            } else {
                return Err("Rewards pool not found".to_string());
            }
        }

        // Distribute rewards to each validator based on their normalized synergy score
        let mut distributed_count = 0;
        for (validator_address, normalized_score) in cluster_validators {
            let validator_reward = ((reward_amount as f64) * normalized_score) as u64;

            if validator_reward == 0 {
                continue; // Skip validators with zero reward
            }

            // Add reward to validator's balance
            if let Ok(mut balances) = self.balances.lock() {
                let validator_balances = balances
                    .entry(validator_address.clone())
                    .or_insert_with(HashMap::new);
                let current = validator_balances.get("SNRG").unwrap_or(&0);
                validator_balances.insert("SNRG".to_string(), current + validator_reward);
            }

            // Now distribute validator's reward to their stakers
            if let Err(e) =
                self.distribute_validator_rewards_to_stakers(validator_address, validator_reward)
            {
                warn!("token", "Failed to distribute validator reward to stakers", 
                      "validator" => validator_address.clone(), 
                      "error" => e);
                // Continue with other validators even if one fails
            } else {
                distributed_count += 1;
            }
        }

        Ok(format!(
            "Distributed {} rewards to cluster ({} validators)",
            reward_amount, distributed_count
        ))
    }

    /// Distribute a validator's reward to their stakers (proportional to stake)
    fn distribute_validator_rewards_to_stakers(
        &self,
        validator: &str,
        reward_amount: u64,
    ) -> Result<String, String> {
        // Collect stake data we need (read-only)
        let stake_data: Vec<(String, u64)> = {
            if let Ok(stakes) = self.stakes.lock() {
                if let Some(validator_stakes) = stakes.get(validator) {
                    let active_stakes: Vec<_> = validator_stakes
                        .iter()
                        .filter(|stake| stake.is_active)
                        .collect();

                    if active_stakes.is_empty() {
                        return Ok("No active stakes".to_string());
                    }

                    let total_staked: u64 = active_stakes.iter().map(|stake| stake.amount).sum();
                    if total_staked == 0 {
                        return Ok("No staked tokens".to_string());
                    }

                    // Collect (staker_address, reward_portion) pairs
                    active_stakes
                        .iter()
                        .map(|stake| {
                            let reward_portion = (stake.amount * reward_amount) / total_staked;
                            (stake.staker_address.clone(), reward_portion)
                        })
                        .collect()
                } else {
                    return Err("Validator not found or no active stakes".to_string());
                }
            } else {
                return Err("Failed to lock stakes".to_string());
            }
        }; // stakes lock is dropped here

        let staker_count = stake_data.len();

        // Now update balances and stakes separately, without holding multiple locks
        for (staker_address, reward_portion) in stake_data {
            // Add rewards to staker's balance
            if let Ok(mut balances) = self.balances.lock() {
                if let Some(staker_balances) = balances.get_mut(&staker_address) {
                    let current = staker_balances.get("SNRG").unwrap_or(&0);
                    staker_balances.insert("SNRG".to_string(), current + reward_portion);
                } else {
                    // Create balance entry if it doesn't exist
                    let mut new_balances = HashMap::new();
                    new_balances.insert("SNRG".to_string(), reward_portion);
                    balances.insert(staker_address.clone(), new_balances);
                }
            } // balances lock dropped

            // Update stake rewards
            if let Ok(mut stakes) = self.stakes.lock() {
                if let Some(validator_stakes) = stakes.get_mut(validator) {
                    for s in validator_stakes.iter_mut() {
                        if s.staker_address == staker_address && s.is_active {
                            s.rewards_earned += reward_portion;
                            break;
                        }
                    }
                }
            } // stakes lock dropped
        }

        Ok(format!(
            "Distributed {} rewards to {} stakers",
            reward_amount, staker_count
        ))
    }

    /// Legacy function - distributes rewards to a single validator's stakers
    /// This is kept for backward compatibility but should use distribute_cluster_rewards instead
    pub fn distribute_validator_rewards(
        &self,
        validator: &str,
        reward_amount: u64,
    ) -> Result<String, String> {
        // Rewards pool address from genesis.json
        const REWARDS_POOL: &str = "synw1zwy4m4mpdxyvz4nf8f7s0hk8nesc2cv09ex8pg";

        // First, check if rewards pool has sufficient balance
        let pool_balance = self.get_balance(REWARDS_POOL, "SNRG");
        if pool_balance < reward_amount {
            return Err(format!(
                "Insufficient rewards pool balance: {} < {}",
                pool_balance, reward_amount
            ));
        }

        // Collect stake data we need (read-only)
        let stake_data: Vec<(String, u64)> = {
            if let Ok(stakes) = self.stakes.lock() {
                if let Some(validator_stakes) = stakes.get(validator) {
                    let active_stakes: Vec<_> = validator_stakes
                        .iter()
                        .filter(|stake| stake.is_active)
                        .collect();

                    if active_stakes.is_empty() {
                        return Ok("No active stakes".to_string());
                    }

                    let total_staked: u64 = active_stakes.iter().map(|stake| stake.amount).sum();
                    if total_staked == 0 {
                        return Ok("No staked tokens".to_string());
                    }

                    // Collect (staker_address, reward_portion) pairs
                    active_stakes
                        .iter()
                        .map(|stake| {
                            let reward_portion = (stake.amount * reward_amount) / total_staked;
                            (stake.staker_address.clone(), reward_portion)
                        })
                        .collect()
                } else {
                    return Err("Validator not found or no active stakes".to_string());
                }
            } else {
                return Err("Failed to lock stakes".to_string());
            }
        }; // stakes lock is dropped here

        let staker_count = stake_data.len();

        // Deduct total reward amount from rewards pool
        if let Ok(mut balances) = self.balances.lock() {
            if let Some(pool_balances) = balances.get_mut(REWARDS_POOL) {
                let current = pool_balances.get("SNRG").unwrap_or(&0);
                if *current < reward_amount {
                    return Err(format!(
                        "Rewards pool balance check failed during deduction: {} < {}",
                        current, reward_amount
                    ));
                }
                pool_balances.insert("SNRG".to_string(), current - reward_amount);
            } else {
                return Err("Rewards pool not found".to_string());
            }
        } // balances lock dropped

        // Now update balances and stakes separately, without holding multiple locks
        for (staker_address, reward_portion) in stake_data {
            // Add rewards to staker's balance (transferred from rewards pool)
            if let Ok(mut balances) = self.balances.lock() {
                if let Some(staker_balances) = balances.get_mut(&staker_address) {
                    let current = staker_balances.get("SNRG").unwrap_or(&0);
                    staker_balances.insert("SNRG".to_string(), current + reward_portion);
                }
            } // balances lock dropped

            // Update stake rewards
            if let Ok(mut stakes) = self.stakes.lock() {
                if let Some(validator_stakes) = stakes.get_mut(validator) {
                    for s in validator_stakes.iter_mut() {
                        if s.staker_address == staker_address && s.is_active {
                            s.rewards_earned += reward_portion;
                            break;
                        }
                    }
                }
            } // stakes lock dropped
        }

        Ok(format!(
            "Distributed {} rewards from pool to {} stakers",
            reward_amount, staker_count
        ))
    }

    pub fn process_transaction(&self, tx: &Transaction) -> Result<String, String> {
        // Handle token transfers
        if tx
            .data
            .as_ref()
            .map_or(false, |data| data.starts_with("token_transfer:"))
        {
            if let Some(data_str) = &tx.data {
                if let Some(transfer_data) = data_str.strip_prefix("token_transfer:") {
                    if let Ok(transfer_info) =
                        serde_json::from_str::<serde_json::Value>(transfer_data)
                    {
                        if let (Some(to), Some(token_symbol), Some(amount)) = (
                            transfer_info.get("to").and_then(|v| v.as_str()),
                            transfer_info.get("token").and_then(|v| v.as_str()),
                            transfer_info.get("amount").and_then(|v| v.as_u64()),
                        ) {
                            return self.transfer_tokens(
                                &tx.sender,
                                to,
                                token_symbol,
                                amount,
                                tx.get_fee(),
                            );
                        }
                    }
                }
            }
        }

        // Handle staking transactions
        if tx
            .data
            .as_ref()
            .map_or(false, |data| data.starts_with("stake:"))
        {
            if let Some(data_str) = &tx.data {
                if let Some(stake_data) = data_str.strip_prefix("stake:") {
                    if let Ok(stake_info) = serde_json::from_str::<serde_json::Value>(stake_data) {
                        if let (Some(validator), Some(token_symbol), Some(amount)) = (
                            stake_info.get("validator").and_then(|v| v.as_str()),
                            stake_info.get("token").and_then(|v| v.as_str()),
                            stake_info.get("amount").and_then(|v| v.as_u64()),
                        ) {
                            return self.stake_tokens(&tx.sender, validator, token_symbol, amount);
                        }
                    }
                }
            }
        }

        Err("Unsupported transaction type".to_string())
    }

    /// Process a transaction that has been included in a specific block height.
    /// This records transfer metadata (tx hash + block height) for explorer queries.
    pub fn process_transaction_in_block(
        &self,
        tx: &Transaction,
        block_height: u64,
    ) -> Result<String, String> {
        // Handle token transfers
        if tx
            .data
            .as_ref()
            .map_or(false, |data| data.starts_with("token_transfer:"))
        {
            if let Some(data_str) = &tx.data {
                if let Some(transfer_data) = data_str.strip_prefix("token_transfer:") {
                    if let Ok(transfer_info) =
                        serde_json::from_str::<serde_json::Value>(transfer_data)
                    {
                        if let (Some(to), Some(token_symbol), Some(amount)) = (
                            transfer_info.get("to").and_then(|v| v.as_str()),
                            transfer_info.get("token").and_then(|v| v.as_str()),
                            transfer_info.get("amount").and_then(|v| v.as_u64()),
                        ) {
                            return self.transfer_tokens_with_metadata(
                                &tx.sender,
                                to,
                                token_symbol,
                                amount,
                                tx.get_fee(),
                                tx.hash(),
                                block_height,
                            );
                        }
                    }
                }
            }
        }

        // Handle staking transactions
        if tx
            .data
            .as_ref()
            .map_or(false, |data| data.starts_with("stake:"))
        {
            if let Some(data_str) = &tx.data {
                if let Some(stake_data) = data_str.strip_prefix("stake:") {
                    if let Ok(stake_info) = serde_json::from_str::<serde_json::Value>(stake_data) {
                        if let (Some(validator), Some(token_symbol), Some(amount)) = (
                            stake_info.get("validator").and_then(|v| v.as_str()),
                            stake_info.get("token").and_then(|v| v.as_str()),
                            stake_info.get("amount").and_then(|v| v.as_u64()),
                        ) {
                            return self.stake_tokens(&tx.sender, validator, token_symbol, amount);
                        }
                    }
                }
            }
        }

        Err("Unsupported transaction type".to_string())
    }

    pub fn get_token_info(&self, symbol: &str) -> Option<Token> {
        if let Ok(tokens) = self.tokens.lock() {
            tokens.get(symbol).cloned()
        } else {
            None
        }
    }

    pub fn get_all_tokens(&self) -> Vec<Token> {
        if let Ok(tokens) = self.tokens.lock() {
            tokens.values().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_transfer_history(&self, address: &str, limit: usize) -> Vec<TokenTransfer> {
        if let Ok(transfers) = self.transfers.lock() {
            transfers
                .iter()
                .filter(|transfer| transfer.from == address || transfer.to == address)
                .take(limit)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_staking_info(&self, address: &str) -> Vec<StakingInfo> {
        if let Ok(stakes) = self.stakes.lock() {
            stakes
                .values()
                .flatten()
                .filter(|stake| stake.staker_address == address)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Ensure the rewards pool has sufficient balance for validator rewards
    /// This should be called on startup to verify the pool is funded
    pub fn ensure_rewards_pool_funded(&self) -> Result<(), String> {
        const REWARDS_POOL: &str = "synw1zwy4m4mpdxyvz4nf8f7s0hk8nesc2cv09ex8pg";
        const MIN_POOL_BALANCE: u64 = 1_000_000_000_000_000_000u64; // 1B SNRG minimum
        const REFILL_AMOUNT: u64 = 2_000_000_000_000_000_000u64; // 2B SNRG refill

        let pool_balance = self.get_balance(REWARDS_POOL, "SNRG");

        if pool_balance < MIN_POOL_BALANCE {
            println!(
                "⚠️ Rewards pool balance low: {} nWei. Initializing...",
                pool_balance
            );

            // Mint tokens to rewards pool if it's empty or low
            match self.mint_tokens(REWARDS_POOL, "SNRG", REFILL_AMOUNT) {
                Ok(_) => {
                    let new_balance = self.get_balance(REWARDS_POOL, "SNRG");
                    println!(
                        "✅ Rewards pool initialized with {} SNRG (balance: {} nWei)",
                        REFILL_AMOUNT / 1_000_000_000,
                        new_balance
                    );
                    Ok(())
                }
                Err(e) => {
                    println!("❌ Failed to initialize rewards pool: {}", e);
                    Err(e)
                }
            }
        } else {
            println!(
                "✅ Rewards pool balance OK: {} SNRG",
                pool_balance / 1_000_000_000
            );
            Ok(())
        }
    }

    /// Get the rewards pool address
    pub fn get_rewards_pool_address() -> &'static str {
        "synw1zwy4m4mpdxyvz4nf8f7s0hk8nesc2cv09ex8pg"
    }

    /// Get rewards pool balance
    pub fn get_rewards_pool_balance(&self) -> u64 {
        self.get_balance(Self::get_rewards_pool_address(), "SNRG")
    }

    pub fn get_total_stake_for_validator(&self, validator: &str) -> u64 {
        if let Ok(stakes) = self.stakes.lock() {
            if let Some(validator_stakes) = stakes.get(validator) {
                return validator_stakes
                    .iter()
                    .filter(|stake| stake.is_active)
                    .map(|stake| stake.amount)
                    .sum();
            }
        }
        0
    }

    fn generate_tx_hash(from: &str, to: &str, token: &str, amount: u64, fee: u64) -> String {
        let mut hasher = Sha3_256::new();
        hasher.update(from.as_bytes());
        hasher.update(to.as_bytes());
        hasher.update(token.as_bytes());
        hasher.update(&amount.to_le_bytes());
        hasher.update(&fee.to_le_bytes());
        hasher.update(&Token::current_timestamp().to_le_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn save_state(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let state = TokenState {
            tokens: self.get_all_tokens(),
            balances: self.balances.lock().unwrap().clone(),
            transfers: self.transfers.lock().unwrap().clone(),
            stakes: self.stakes.lock().unwrap().clone(),
        };

        let json = serde_json::to_string_pretty(&state)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_state(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if std::path::Path::new(path).exists() {
            let content = std::fs::read_to_string(path)?;
            let state: TokenState = serde_json::from_str(&content)?;

            if let Ok(mut tokens) = self.tokens.lock() {
                for token in state.tokens {
                    tokens.insert(token.symbol.clone(), token);
                }
            }

            if let Ok(mut balances) = self.balances.lock() {
                *balances = state.balances;
            }

            if let Ok(mut transfers) = self.transfers.lock() {
                *transfers = state.transfers;
            }

            if let Ok(mut stakes) = self.stakes.lock() {
                *stakes = state.stakes;
            }
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct TokenState {
    tokens: Vec<Token>,
    balances: HashMap<String, HashMap<String, u64>>,
    transfers: Vec<TokenTransfer>,
    stakes: HashMap<String, Vec<StakingInfo>>,
}

// Global token manager instance
lazy_static::lazy_static! {
    pub static ref TOKEN_MANAGER: Arc<TokenManager> = Arc::new(TokenManager::new());
}
