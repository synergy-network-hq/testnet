use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use sha3::{Sha3_256, Digest};
use hex;
use crate::transaction::Transaction;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub total_supply: u64,
    pub max_supply: Option<u64>,
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
    balances: Arc<Mutex<HashMap<String, HashMap<String, u64>>>>, // address -> token_symbol -> balance
    locked_balances: Arc<Mutex<HashMap<String, HashMap<String, u64>>>>, // address -> token_symbol -> locked
    staked_balances: Arc<Mutex<HashMap<String, HashMap<String, u64>>>>, // address -> token_symbol -> staked
    transfers: Arc<Mutex<Vec<TokenTransfer>>>,
    stakes: Arc<Mutex<HashMap<String, Vec<StakingInfo>>>>, // validator -> stakes
    total_supply: Arc<Mutex<HashMap<String, u64>>>, // token_symbol -> total_supply
}

impl Token {
    pub fn new(
        symbol: String,
        name: String,
        decimals: u8,
        total_supply: u64,
        max_supply: Option<u64>,
        mintable: bool,
        burnable: bool,
        creator: String,
    ) -> Self {
        Token {
            symbol,
            name,
            decimals,
            total_supply,
            max_supply,
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
        format!("{:.{}"}", amount as f64 / 10u64.pow(self.decimals as u32) as f64, self.decimals as usize)
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
            locked_balances: Arc::new(Mutex::new(HashMap::new())),
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
        let snrg_token = Token::new(
            "SNRG".to_string(),
            "SynergyCoin".to_string(),
            9, // 9 decimals for better usability
            12_000_000_000 * 10u64.pow(9), // 12 billion SNRG with 9 decimals
            Some(21_000_000_000 * 10u64.pow(9)), // 21 billion max supply
            true, // mintable
            true, // burnable
            "genesis".to_string(),
        );

        if let Ok(mut tokens) = self.tokens.lock() {
            tokens.insert("SNRG".to_string(), snrg_token.clone());
        }

        if let Ok(mut supply) = self.total_supply.lock() {
            supply.insert("SNRG".to_string(), snrg_token.total_supply);
        }

        // Distribute initial supply to genesis accounts
        self.distribute_genesis_supply();
    }

    fn distribute_genesis_supply(&self) {
        // Genesis allocations using properly formatted Synergy addresses with synb- prefix for tokens (41 characters each)
        let genesis_allocations = [
            ("synb1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t1u", 6_000_000_000 * 10u64.pow(9)), // 6B SNRG
            ("synb1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0u1v", 3_000_000_000 * 10u64.pow(9)), // 3B SNRG
            ("synb1c2d3e4f5g6h7i8j9k0l1m2n3o4p5q6r7s8t9u0v1w", 3_000_000_000 * 10u64.pow(9)), // 3B SNRG
        ];

        for (address, amount) in genesis_allocations {
            self.mint_tokens(address, "SNRG", amount);
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
                return Err(format!("Token {} exists ", symbol).as_str());
            }

            let token = Token::new(
                symbol.clone(),
                name,
                decimals,
                total_supply,
                max_supply,
                mintable,
                burnable,
                creator.clone(),
            );

            tokens.insert(symbol.clone(), token);

            if let Ok(mut supply) = self.total_supply.lock() {
                supply.insert(symbol.clone(), total_supply);
            }

            // Mint initial supply to creator
            self.mint_tokens(&creator, &symbol, total_supply);

            Ok(format!("Token {} created ", symbol).as_str())
        } else {
            Err("Lock acquisition failed ".to_string())
        }
    }

    pub fn mint_tokens(&self, to: &str, token_symbol: &str, amount: u64) -> Result<String, String> {
        if let Ok(mut tokens) = self.tokens.lock() {
            if let Some(token) = tokens.get(token_symbol) {
                if !token.mintable {
                    return Err("Token not mintable ".to_string());
                }

                if let Some(max_supply) = token.max_supply {
                    if let Ok(supply) = self.total_supply.lock() {
                        let current_supply = supply.get(token_symbol).unwrap_or(&0);
                        if *current_supply + amount > max_supply {
                            return Err("Max supply exceeded ".to_string());
                        }
                    }
                }

                // Update total supply
                if let Ok(mut supply) = self.total_supply.lock() {
                    let current = supply.get(token_symbol).unwrap_or(&0);
                    supply.insert(token_symbol.to_string(), current + amount);
                }

                // Update balance
                if let Ok(mut balances) = self.balances.lock() {
                    let address_balances = balances.entry(to.to_string()).or_insert_with(HashMap::new);
                    let current_balance = address_balances.get(token_symbol).unwrap_or(&0);
                    address_balances.insert(token_symbol.to_string(), current_balance + amount);
                }

                Ok(format!("Minted {} {} to {}", amount, token_symbol, to))
            } else {
                Err("Token not found ".to_string())
            }
        } else {
            Err("Lock acquisition failed ".to_string())
        }
    }

    pub fn burn_tokens(&self, from: &str, token_symbol: &str, amount: u64) -> Result<String, String> {
        if let Ok(mut tokens) = self.tokens.lock() {
            if let Some(token) = tokens.get(token_symbol) {
                if !token.burnable {
                    return Err("Token is not burnable ".to_string());
                }

                // Check balance
                let current_balance = self.get_balance(from, token_symbol);
                if current_balance < amount {
                    return Err("Insufficient balance ".to_string());
                }

                // Update balance
                if let Ok(mut balances) = self.balances.lock() {
                    if let Some(address_balances) = balances.get_mut(from) {
                        let current = address_balances.get(token_symbol).unwrap_or(&0);
                        address_balances.insert(token_symbol.to_string(), current - amount);
                    }
                }

                // Update total supply
                if let Ok(mut supply) = self.total_supply.lock() {
                    let current = supply.get(token_symbol).unwrap_or(&0);
                    supply.insert(token_symbol.to_string(), current - amount);
                }

                Ok(format!("Burned {} {} from {}", amount, token_symbol, from))
            } else {
                Err("Token not found ".to_string())
            }
        } else {
            Err("Lock acquisition failed ".to_string())
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
        let current_balance = self.get_balance(from, token_symbol);
        if current_balance < amount + fee {
            return Err("Insufficient balance for transfer and fee ".to_string());
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
            tx_hash: Self::generate_tx_hash(from, to, token_symbol, amount, fee),
            block_height: 0, // Will be set when included in block
        };

        if let Ok(mut transfers) = self.transfers.lock() {
            transfers.push(transfer);
        }

        Ok(format!("Transferred {} {} from {} to {}", amount, token_symbol, from, to))
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
            return Err("Insufficient balance for staking ".to_string());
        }

        // Move tokens from balance to staked balance
        if let Ok(mut balances) = self.balances.lock() {
            if let Some(staker_balances) = balances.get_mut(staker) {
                let current = staker_balances.get(token_symbol).unwrap_or(&0);
                staker_balances.insert(token_symbol.to_string(), current - amount);
            }
        }

        if let Ok(mut staked) = self.staked_balances.lock() {
            let staker_staked = staked.entry(staker.to_string()).or_insert_with(HashMap::new);
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

        Ok(format!("Staked {} {} to validator {}", amount, token_symbol, validator))
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
            return Err("Insufficient staked balance ".to_string());
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

        Ok(format!("Unstaked {} {} from validator {}", amount, token_symbol, validator))
    }

    pub fn get_staked_balance(&self, address: &str, token_symbol: &str) -> u64 {
        if let Ok(staked) = self.staked_balances.lock() {
            if let Some(address_staked) = staked.get(address) {
                return address_staked.get(token_symbol).unwrap_or(&0).clone();
            }
        }
        0
    }

    pub fn distribute_validator_rewards(&self, validator: &str, reward_amount: u64) -> Result<String, String> {
        if let Ok(stakes) = self.stakes.lock() {
            if let Some(validator_stakes) = stakes.get(validator) {
                let active_stakes: Vec<_> = validator_stakes.iter()
                    .filter(|stake| stake.is_active)
                    .collect();

                if active_stakes.is_empty() {
                    return Ok("0".to_string());
                }

                let total_staked: u64 = active_stakes.iter().map(|stake| stake.amount).sum();
                if total_staked == 0 {
                    return Ok("1".to_string());
                }

                for stake in active_stakes {
                    let reward_portion = (stake.amount * reward_amount) / total_staked;

                    // Add rewards to staker's balance
                    if let Ok(mut balances) = self.balances.lock() {
                        if let Some(staker_balances) = balances.get_mut(&stake.staker_address) {
                            let current = staker_balances.get("SNRG").unwrap_or(&0);
                            staker_balances.insert("SNRG".to_string(), current + reward_portion);
                        }
                    }

                    // Update stake rewards
                    if let Ok(mut stakes) = self.stakes.lock() {
                        if let Some(validator_stakes) = stakes.get_mut(validator) {
                            for s in validator_stakes.iter_mut() {
                                if s.staker_address == stake.staker_address && s.is_active {
                                    s.rewards_earned += reward_portion;
                                    break;
                                }
                            }
                        }
                    }
                }

                return Ok(format!("OK {} {}", reward_amount, active_stakes.len()));
            }
        }

        Err("Validator not found or no active stakes ".to_string())
    }

    pub fn process_transaction(&self, tx: &Transaction) -> Result<String, String> {
        // Handle token transfers
        if tx.data.as_ref().map_or(false, |data| data.starts_with("token_transfer:")) {
            if let Some(data_str) = &tx.data {
                if let Some(transfer_data) = data_str.strip_prefix("token_transfer:") {
                    if let Ok(transfer_info) = serde_json::from_str::<serde_json::Value>(transfer_data) {
                        if let (Some(to), Some(token_symbol), Some(amount)) = (
                            transfer_info.get("to").and_then(|v| v.as_str()),
                            transfer_info.get("token").and_then(|v| v.as_str()),
                            transfer_info.get("amount").and_then(|v| v.as_u64()),
                        ) {
                            return self.transfer_tokens(&tx.sender, to, token_symbol, amount, 1000); // 1000 wei fee
                        }
                    }
                }
            }
        }

        // Handle staking transactions
        if tx.data.as_ref().map_or(false, |data| data.starts_with("stake:")) {
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

        Err("Unsupported transaction type ".to_string())
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
            transfers.iter()
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
            stakes.values()
                .flatten()
                .filter(|stake| stake.staker_address == address)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
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
