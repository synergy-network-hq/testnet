use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::block::{Block, BlockChain};
use crate::p2p::networking::{P2PNetwork, PeerSnapshot};
use crate::sync::fast_sync;
use crate::sync::validation;

/// Represents where the sync engine currently is in the lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    Idle,
    Discovering,
    Downloading,
    Validating,
    Applying,
    Synced,
}

/// Snapshot of sync progress for reporting and RPC.
#[derive(Debug, Clone)]
pub struct SyncProgress {
    pub starting_block: u64,
    pub current_block: u64,
    pub highest_block: u64,
}

impl SyncProgress {
    fn new(starting_block: u64, highest_block: u64) -> Self {
        SyncProgress {
            starting_block,
            current_block: starting_block,
            highest_block,
        }
    }

    fn percentage(&self) -> f64 {
        if self.highest_block == self.starting_block {
            return 100.0;
        }
        let range = self.highest_block.saturating_sub(self.starting_block) as f64;
        let completed = self.current_block.saturating_sub(self.starting_block) as f64;
        if range == 0.0 {
            100.0
        } else {
            (completed / range * 100.0).min(100.0)
        }
    }
}

/// Sync manager errors represent recoverable conditions that should be surfaced via logs.
#[derive(Debug)]
pub enum SyncError {
    NetworkUnavailable,
    NoPeers,
    Timeout(String),
    MissingBlock(u64),
    InvalidParentHash {
        height: u64,
        expected: String,
        got: String,
    },
    InvalidTransactionsRoot,
    BlockValidationFailed(String),
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncError::NetworkUnavailable => write!(f, "P2P network unavailable"),
            SyncError::NoPeers => write!(f, "No peers available for sync"),
            SyncError::Timeout(reason) => write!(f, "Timeout waiting for {}", reason),
            SyncError::MissingBlock(height) => write!(f, "Missing block at height {}", height),
            SyncError::InvalidParentHash {
                height,
                expected,
                got,
            } => write!(
                f,
                "Header at {} points to {}, expected {}",
                height, got, expected
            ),
            SyncError::InvalidTransactionsRoot => write!(f, "Computed transaction root mismatched"),
            SyncError::BlockValidationFailed(reason) => {
                write!(f, "Block validation failed: {}", reason)
            }
        }
    }
}

/// Lightweight peer information derived from snapshots exposed by the network layer.
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub address: String,
    pub block_height: u64,
    pub best_block_hash: String,
    pub genesis_hash: String,
}

/// Represents a requested range that should be downloaded/applied.
#[derive(Debug, Clone)]
pub struct BlockRange {
    pub start: u64,
    pub end: u64,
}

/// Sync manager responsible for bootstrapping from genesis and keeping the node current.
pub struct SyncManager {
    pub state: SyncState,
    pub local_height: u64,
    pub network_height: u64,
    pub sync_start_height: u64,
    pub pending_blocks: BTreeMap<u64, Block>,
    pub download_queue: VecDeque<BlockRange>,
    pub peers: Vec<PeerInfo>,
    blockchain: Arc<Mutex<BlockChain>>,
    p2p_network: Option<Arc<P2PNetwork>>,
    progress: SyncProgress,
}

impl SyncManager {
    pub fn new(blockchain: Arc<Mutex<BlockChain>>) -> Self {
        let tip_height = blockchain
            .lock()
            .ok()
            .and_then(|chain| chain.last().map(|block| block.block_index))
            .unwrap_or(0);
        SyncManager {
            state: SyncState::Idle,
            local_height: tip_height,
            network_height: tip_height,
            sync_start_height: tip_height,
            pending_blocks: BTreeMap::new(),
            download_queue: VecDeque::new(),
            peers: Vec::new(),
            blockchain,
            p2p_network: None,
            progress: SyncProgress::new(tip_height, tip_height),
        }
    }

    pub fn attach_network(&mut self, network: Arc<P2PNetwork>) {
        self.p2p_network = Some(network);
    }

    fn refresh_local_height(&mut self) {
        if let Ok(chain) = self.blockchain.lock() {
            self.local_height = chain.last().map(|b| b.block_index).unwrap_or(0);
            self.progress.current_block = self.local_height;
        }
    }

    fn collect_peer_snapshots(&self) -> Vec<PeerSnapshot> {
        if let Some(network) = &self.p2p_network {
            network.collect_peer_snapshots()
        } else {
            Vec::new()
        }
    }

    fn refresh_peers_from_snapshots(&mut self, snapshots: Vec<PeerSnapshot>) {
        self.peers = snapshots
            .into_iter()
            .map(|snap| PeerInfo {
                address: snap.address,
                block_height: snap.block_height,
                best_block_hash: snap.best_block_hash,
                genesis_hash: snap.genesis_hash,
            })
            .collect();
    }

    fn await_peer_snapshots(&self, timeout: Duration) -> Vec<PeerSnapshot> {
        let start = Instant::now();
        let mut last = Vec::new();

        if let Some(network) = &self.p2p_network {
            network.request_peer_statuses();
        }

        loop {
            let snapshots = self.collect_peer_snapshots();
            if !snapshots.is_empty() {
                last = snapshots;
                let has_height = last.iter().any(|peer| peer.block_height > 0);
                if has_height {
                    return last;
                }
            }

            if start.elapsed() >= timeout {
                return last;
            }

            thread::sleep(Duration::from_millis(500));
            if let Some(network) = &self.p2p_network {
                network.request_peer_statuses();
            }
        }
    }

    fn select_sync_peer(&self) -> Option<String> {
        let local_genesis = self
            .blockchain
            .lock()
            .ok()
            .and_then(|chain| chain.get_genesis_hash())
            .unwrap_or_default();
        let mut candidates: Vec<&PeerInfo> = self
            .peers
            .iter()
            .filter(|peer| {
                if local_genesis.is_empty() {
                    true
                } else {
                    peer.genesis_hash.is_empty() || peer.genesis_hash == local_genesis
                }
            })
            .collect();
        candidates.sort_by(|a, b| b.block_height.cmp(&a.block_height));
        candidates.first().map(|peer| peer.address.clone())
    }

    pub fn discover_network_height(&mut self) -> Result<u64, SyncError> {
        let snapshots = self.await_peer_snapshots(Duration::from_secs(10));
        self.refresh_peers_from_snapshots(snapshots);

        if self.peers.is_empty() {
            return Err(SyncError::NoPeers);
        }

        let local_genesis = self
            .blockchain
            .lock()
            .ok()
            .and_then(|chain| chain.get_genesis_hash())
            .unwrap_or_default();

        let eligible = self.peers.iter().filter(|peer| {
            if local_genesis.is_empty() {
                true
            } else {
                peer.genesis_hash.is_empty() || peer.genesis_hash == local_genesis
            }
        });

        let max_height = eligible.map(|peer| peer.block_height).max().unwrap_or(0);

        Ok(max_height)
    }

    pub fn start_sync(&mut self) -> Result<(), SyncError> {
        self.refresh_local_height();
        self.state = SyncState::Discovering;
        let mut network_height = self.discover_network_height()?;
        self.network_height = network_height;

        if self.local_height >= network_height {
            self.state = SyncState::Synced;
            return Ok(());
        }

        self.sync_start_height = self.local_height;
        self.progress.starting_block = self.local_height;
        self.progress.highest_block = network_height;
        self.state = SyncState::Downloading;

        while self.local_height < self.network_height {
            self.refresh_peers_from_snapshots(self.collect_peer_snapshots());
            if let Ok(updated_height) = self.discover_network_height() {
                if updated_height > self.network_height {
                    self.network_height = updated_height;
                }
            }
            let from = self.local_height + 1;
            let remaining = self.network_height - self.local_height;
            let batch_size = if remaining > 5000 {
                1000
            } else if remaining > 2000 {
                600
            } else if remaining > 1000 {
                400
            } else {
                std::cmp::min(remaining, 200)
            };

            if let Some(network) = &self.p2p_network {
                let preferred_peer = self.select_sync_peer();
                let sent = preferred_peer
                    .as_ref()
                    .map(|peer| network.request_blocks_from_peer(peer, from, batch_size as u32))
                    .unwrap_or(false);
                if !sent {
                    network.request_blocks(from, batch_size as u32);
                }
            } else {
                return Err(SyncError::NetworkUnavailable);
            }

            let target_height = std::cmp::min(self.network_height, from + batch_size - 1);
            let mut satisfied = self.wait_for_height(target_height, Duration::from_secs(12));
            if !satisfied {
                if let Some(network) = &self.p2p_network {
                    let preferred_peer = self.select_sync_peer();
                    let sent = preferred_peer
                        .as_ref()
                        .map(|peer| network.request_blocks_from_peer(peer, from, batch_size as u32))
                        .unwrap_or(false);
                    if !sent {
                        network.request_blocks(from, batch_size as u32);
                    }
                }
                satisfied = self.wait_for_height(target_height, Duration::from_secs(12));
            }
            if !satisfied {
                return Err(SyncError::Timeout(format!(
                    "blocks up to height {}",
                    target_height
                )));
            }

            self.refresh_local_height();
            self.state = SyncState::Validating;

            let headers = fast_sync::download_headers(&self.blockchain, from, self.local_height);
            let prev_hash = if from > 0 {
                Some(self.get_block_hash(from - 1)?)
            } else {
                None
            };
            validation::validate_header_chain(&headers, prev_hash)?;

            let bodies = fast_sync::download_block_bodies(&self.blockchain, &headers);
            for block in bodies {
                validation::validate_block(&block)?;
            }

            self.progress.current_block = self.local_height;
            self.progress.highest_block = self.network_height;

            self.state = SyncState::Applying;
            self.download_queue.push_back(BlockRange {
                start: from,
                end: target_height,
            });
        }

        self.state = SyncState::Synced;
        Ok(())
    }

    fn wait_for_height(&self, target: u64, timeout: Duration) -> bool {
        let start = Instant::now();
        while Instant::now().duration_since(start) < timeout {
            if let Ok(chain) = self.blockchain.lock() {
                if let Some(last) = chain.last() {
                    if last.block_index >= target {
                        return true;
                    }
                }
            }
            thread::sleep(Duration::from_millis(250));
        }
        false
    }

    fn get_block_hash(&self, height: u64) -> Result<String, SyncError> {
        let chain = self
            .blockchain
            .lock()
            .map_err(|_| SyncError::NetworkUnavailable)?;
        chain
            .chain
            .iter()
            .find(|block| block.block_index == height)
            .map(|block| block.hash.clone())
            .ok_or(SyncError::MissingBlock(height))
    }

    pub fn get_state(&self) -> SyncState {
        self.state
    }

    pub fn get_network_height(&self) -> u64 {
        self.network_height
    }

    pub fn get_sync_start_height(&self) -> u64 {
        self.sync_start_height
    }

    pub fn get_progress_percentage(&self) -> f64 {
        self.progress.percentage()
    }
}
