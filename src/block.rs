use crate::transaction::Transaction;
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub block_index: u64,
    #[serde(default)]
    pub timestamp: u64,
    pub transactions: Vec<Transaction>,
    pub previous_hash: String,
    pub validator_id: String,
    pub nonce: u64,
    pub hash: String,
    #[serde(default)]
    pub transactions_root: String,
    #[serde(default)]
    pub proposer_public_key: Vec<u8>,
    #[serde(default)]
    pub block_signature: Vec<u8>,
    #[serde(default)]
    pub block_signature_algorithm: String,
}

impl Block {
    pub fn new(
        block_index: u64,
        transactions: Vec<Transaction>,
        previous_hash: String,
        validator_id: String,
        nonce: u64,
    ) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self::new_with_timestamp(
            block_index,
            transactions,
            previous_hash,
            validator_id,
            nonce,
            timestamp,
        )
    }

    pub fn new_with_timestamp(
        block_index: u64,
        transactions: Vec<Transaction>,
        previous_hash: String,
        validator_id: String,
        nonce: u64,
        timestamp: u64,
    ) -> Self {
        let transactions_root = compute_merkle_root(&transactions);

        let data = format!(
            "{:?}{}{}{}{}{}",
            block_index, previous_hash, validator_id, nonce, timestamp, transactions_root
        );
        let hash = blake3::hash(data.as_bytes()).to_hex().to_string();
        Block {
            block_index,
            timestamp,
            transactions,
            previous_hash,
            validator_id,
            nonce,
            hash,
            transactions_root,
            proposer_public_key: Vec::new(),
            block_signature: Vec::new(),
            block_signature_algorithm: String::new(),
        }
    }

    pub fn validate(&self) -> bool {
        true
    }

    pub fn header(&self) -> BlockHeader {
        BlockHeader {
            number: self.block_index,
            timestamp: self.timestamp,
            parent_hash: self.previous_hash.clone(),
            hash: self.hash.clone(),
            validator_id: self.validator_id.clone(),
            transactions_root: self.transactions_root.clone(),
        }
    }
}

pub fn compute_merkle_root(transactions: &[Transaction]) -> String {
    if transactions.is_empty() {
        return blake3::hash(&[]).to_hex().to_string();
    }

    let mut hashes: Vec<String> = transactions.iter().map(|tx| tx.raw_hash()).collect();
    while hashes.len() > 1 {
        let mut next = Vec::new();
        for chunk in hashes.chunks(2) {
            if chunk.len() == 2 {
                let pair = format!("{}{}", chunk[0], chunk[1]);
                next.push(blake3::hash(pair.as_bytes()).to_hex().to_string());
            } else {
                next.push(chunk[0].clone());
            }
        }
        hashes = next;
    }

    hashes
        .first()
        .cloned()
        .unwrap_or_else(|| blake3::hash(&[]).to_hex().to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub number: u64,
    pub timestamp: u64,
    pub parent_hash: String,
    pub hash: String,
    pub validator_id: String,
    pub transactions_root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockChain {
    pub chain: Vec<Block>,
}

impl BlockChain {
    pub fn new() -> Self {
        BlockChain { chain: vec![] }
    }

    pub fn add_block(&mut self, block: Block) {
        self.chain.push(block);
    }

    pub fn last(&self) -> Option<&Block> {
        self.chain.last()
    }

    pub fn genesis(&mut self) {
        // NOTE: Token genesis allocations are handled by `TokenManager::initialize_snrg_token()`,
        // which reads `config/genesis.json` and mints initial balances.
        let genesis_timestamp = resolve_genesis_timestamp();
        let genesis_block = Block::new_with_timestamp(
            0,
            vec![],
            "0".to_string(),
            "genesis".to_string(),
            0,
            genesis_timestamp,
        );
        self.chain.push(genesis_block);
    }

    pub fn get_genesis_hash(&self) -> Option<String> {
        self.chain.first().map(|b| b.hash.clone())
    }

    pub fn save_to_file(&self, path: &str) {
        if let Ok(json) = serde_json::to_string_pretty(&self.chain) {
            if let Ok(mut file) = File::create(path) {
                let _ = file.write_all(json.as_bytes());
            }
        }
    }

    pub fn load_from_file(path: &str) -> Option<Self> {
        if Path::new(path).exists() {
            if let Ok(mut file) = File::open(path) {
                let mut contents = String::new();
                if file.read_to_string(&mut contents).is_ok() {
                    if let Ok(blocks) = serde_json::from_str::<Vec<Block>>(&contents) {
                        return Some(BlockChain { chain: blocks });
                    }
                }
            }
        }
        None
    }
}

fn resolve_genesis_timestamp() -> u64 {
    if let Ok(raw) = std::env::var("SYNERGY_GENESIS_TIMESTAMP") {
        if let Ok(value) = raw.parse::<u64>() {
            return value;
        }
    }

    if let Ok(content) = fs::read_to_string("config/genesis.json") {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(timestamp_raw) = value
                .get("metadata")
                .and_then(|m| m.get("genesis_time"))
                .and_then(|v| v.as_str())
            {
                if let Ok(parsed) = DateTime::parse_from_rfc3339(timestamp_raw) {
                    return parsed.timestamp().max(0) as u64;
                }
            }
        }
    }

    // 2026-01-01T00:00:00Z
    1767225600
}
