use crate::transaction::Transaction;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::genesis::canonical_genesis;

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

    pub fn block_at_height(&self, height: u64) -> Option<&Block> {
        self.chain.iter().find(|block| block.block_index == height)
    }

    pub fn truncate_to_height(&mut self, height: u64) {
        if let Some(position) = self
            .chain
            .iter()
            .rposition(|block| block.block_index <= height)
        {
            self.chain.truncate(position + 1);
        } else {
            self.chain.clear();
        }
    }

    pub fn genesis(&mut self) -> Result<(), String> {
        let genesis = canonical_genesis()?;
        let genesis_block = Block {
            block_index: 0,
            timestamp: genesis.timestamp(),
            transactions: Vec::new(),
            previous_hash: genesis
                .value()
                .get("header")
                .and_then(|header| header.get("parent_hash"))
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string(),
            validator_id: "genesis".to_string(),
            nonce: 0,
            hash: genesis.hash().to_string(),
            transactions_root: compute_merkle_root(&[]),
            proposer_public_key: Vec::new(),
            block_signature: Vec::new(),
            block_signature_algorithm: String::new(),
        };
        self.chain.clear();
        self.chain.push(genesis_block);
        Ok(())
    }

    pub fn get_genesis_hash(&self) -> Option<String> {
        self.chain.first().map(|b| b.hash.clone())
    }

    pub fn ensure_expected_genesis_hash(&self, expected: &str) -> Result<(), String> {
        let actual = self
            .get_genesis_hash()
            .ok_or_else(|| "blockchain has no genesis block".to_string())?;
        if actual != expected {
            return Err(format!(
                "genesis hash mismatch: expected {expected}, found {actual}"
            ));
        }
        Ok(())
    }

    pub fn save_to_file(&self, path: &str) {
        if let Err(error) = self.save_to_file_atomic(path) {
            eprintln!("failed to save blockchain state to {path}: {error}");
        }
    }

    fn save_to_file_atomic(&self, path: &str) -> Result<(), String> {
        let json =
            serde_json::to_vec(&self.chain).map_err(|error| format!("serialize chain: {error}"))?;
        let target = Path::new(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!("create chain state directory {}: {error}", parent.display())
            })?;
        }

        let file_name = target
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("invalid chain state path: {}", target.display()))?;
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let temp_path =
            target.with_file_name(format!("{file_name}.tmp-{}-{suffix}", std::process::id()));

        {
            let mut file = File::create(&temp_path).map_err(|error| {
                format!("create temp chain state {}: {error}", temp_path.display())
            })?;
            file.write_all(&json).map_err(|error| {
                format!("write temp chain state {}: {error}", temp_path.display())
            })?;
            file.sync_all().map_err(|error| {
                format!("sync temp chain state {}: {error}", temp_path.display())
            })?;
        }

        fs::rename(&temp_path, target).map_err(|error| {
            let _ = fs::remove_file(&temp_path);
            format!(
                "replace chain state {} with {}: {error}",
                target.display(),
                temp_path.display()
            )
        })?;
        Ok(())
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
