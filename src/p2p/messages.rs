use serde::{Deserialize, Serialize};

use crate::block::{Block, BlockHeader};
use crate::consensus::dual_quorum::Vote;
use crate::transaction::Transaction;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    Handshake {
        node_id: String,
        version: String,
        capabilities: Vec<String>,
        #[serde(default)]
        public_address: Option<String>,
        #[serde(default)]
        validator_address: Option<String>,
    },
    Block {
        block_data: Block,
    },
    VoteRequest {
        block_data: Block,
        epoch_number: u64,
        round_number: u64,
    },
    Vote {
        vote: Vote,
    },
    Transaction {
        transaction_data: Transaction,
    },
    GetBlocks {
        from_height: u64,
        count: u32,
    },
    Blocks {
        blocks: Vec<Block>,
    },
    GetPeers,
    Peers {
        peer_addresses: Vec<String>,
    },
    Ping,
    Pong,
    Error {
        message: String,
    },
    GetStatus,
    Status {
        block_height: u64,
        best_block_hash: String,
        genesis_hash: String,
    },
    GetBlockHeaders {
        start_height: u64,
        count: u64,
    },
    BlockHeaders {
        headers: Vec<BlockHeader>,
    },
    GetBlockBodies {
        hashes: Vec<String>,
    },
    BlockBodies {
        blocks: Vec<Block>,
    },
}
