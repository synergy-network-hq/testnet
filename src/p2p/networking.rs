use crate::block::{Block, BlockChain};
use crate::config::NodeConfig;
use crate::p2p::messages::NetworkMessage;
use crate::rpc::rpc_server::TX_POOL;
use crate::token::TOKEN_MANAGER;
use crate::transaction::Transaction;
use crate::validator::{ValidatorRegistration, VALIDATOR_MANAGER};
use crate::{debug, error, info, warn};
use serde_json;
use std::collections::HashMap;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

// Type aliases to avoid nested generics parsing issues
type PeerMap = HashMap<String, PeerConnection>;
type BlockchainArc = Arc<Mutex<BlockChain>>;
type PeersArc = Arc<Mutex<PeerMap>>;

pub struct P2PNetwork {
    blockchain: BlockchainArc,
    config: NodeConfig,
    connected_peers: PeersArc,
    is_running: Arc<Mutex<bool>>,
    message_sender: mpsc::Sender<(String, NetworkMessage)>,
    message_receiver: Arc<Mutex<mpsc::Receiver<(String, NetworkMessage)>>>,
}

struct PeerConnection {
    address: String,
    public_address: Option<String>,
    connected_at: u64,
    last_seen: u64,
    blocks_sent: u64,
    blocks_received: u64,
    txs_sent: u64,
    txs_received: u64,
    stream: Option<TcpStream>,
    node_id: Option<String>,
    version: Option<String>,
    capabilities: Vec<String>,
    last_known_height: u64,
    best_block_hash: String,
    genesis_hash: String,
}

fn resolve_local_validator_address(config: &NodeConfig) -> String {
    let configured = config.node.validator_address.trim();
    if !configured.is_empty() {
        return configured.to_string();
    }

    std::env::var("SYNERGY_VALIDATOR_ADDRESS")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            std::env::var("NODE_ADDRESS")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| config.p2p.node_name.clone())
}

fn is_validator_allowed(config: &NodeConfig, validator_address: &str) -> bool {
    if !config.node.strict_validator_allowlist {
        return true;
    }

    config
        .node
        .allowed_validator_addresses
        .iter()
        .any(|allowed| allowed == validator_address)
}

#[derive(Debug, Clone)]
pub struct PeerSnapshot {
    pub address: String,
    pub block_height: u64,
    pub best_block_hash: String,
    pub genesis_hash: String,
}

impl P2PNetwork {
    pub fn new(blockchain: BlockchainArc, config: &NodeConfig) -> Self {
        let (sender, receiver) = mpsc::channel();

        P2PNetwork {
            blockchain,
            config: config.clone(),
            connected_peers: Arc::new(Mutex::new(HashMap::new())),
            is_running: Arc::new(Mutex::new(false)),
            message_sender: sender,
            message_receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    pub fn start(&mut self, listen_address: &str) {
        let is_running = Arc::clone(&self.is_running);
        let blockchain = Arc::clone(&self.blockchain);
        let connected_peers = Arc::clone(&self.connected_peers);
        let config = self.config.clone();
        let addr_string = listen_address.to_string();
        let message_sender = self.message_sender.clone();

        // Set running flag
        *is_running.lock().unwrap() = true;

        // Start listener thread
        thread::spawn(move || {
            if let Err(e) = start_listener(
                &addr_string,
                blockchain,
                connected_peers,
                config,
                message_sender,
            ) {
                error!("p2p", "P2P listener error", "error" => e.to_string());
            }
        });

        // Start message handler thread
        let blockchain_handler = Arc::clone(&self.blockchain);
        let peers_handler = Arc::clone(&self.connected_peers);
        let receiver = Arc::clone(&self.message_receiver);
        let handler_config = self.config.clone();
        let handler_sender = self.message_sender.clone();

        thread::spawn(move || {
            handle_messages(
                blockchain_handler,
                peers_handler,
                receiver,
                handler_sender,
                handler_config,
            );
        });

        info!(
            "p2p",
            "P2P network started",
            "listen_address" => listen_address.to_string(),
            "public_address" => self.config.p2p.public_address.clone(),
            "bootnodes" => self.config.network.bootnodes.len() as u64
        );
    }

    pub fn connect_to_peer(&self, address: &str) -> Result<(), Box<dyn std::error::Error>> {
        let peer_address = address.to_string();

        let blockchain = Arc::clone(&self.blockchain);
        let connected_peers = Arc::clone(&self.connected_peers);
        let message_sender = self.message_sender.clone();
        let config = self.config.clone();

        thread::spawn(move || {
            match dial_with_timeout(&peer_address, std::time::Duration::from_secs(5)) {
                Ok(stream) => {
                    if let Err(e) = handle_outgoing_connection(
                        stream,
                        peer_address,
                        blockchain,
                        connected_peers,
                        message_sender,
                        config,
                    ) {
                        error!("p2p", "Outgoing connection error", "error" => e.to_string());
                    }
                }
                Err(e) => {
                    warn!("p2p", "Failed to dial peer", "peer" => peer_address, "error" => e.to_string());
                }
            }
        });

        Ok(())
    }

    pub fn broadcast_block(&self, block: &Block) {
        let message = NetworkMessage::Block {
            block_data: block.clone(),
        };

        let mut peers = self.connected_peers.lock().unwrap();
        for (address, peer) in peers.iter_mut() {
            if let Some(ref mut stream) = peer.stream {
                if let Err(e) = send_message(stream, &message) {
                    warn!("p2p", "Failed to send block", "peer" => address.clone(), "error" => e.to_string());
                } else {
                    peer.blocks_sent += 1;
                }
            }
        }

        info!("p2p", "Block broadcast", "peers" => peers.len() as u64, "height" => block.block_index);
    }

    pub fn broadcast_transaction(&self, transaction: &Transaction) {
        let message = NetworkMessage::Transaction {
            transaction_data: transaction.clone(),
        };

        let mut peers = self.connected_peers.lock().unwrap();
        for (address, peer) in peers.iter_mut() {
            if let Some(ref mut stream) = peer.stream {
                if let Err(e) = send_message(stream, &message) {
                    warn!("p2p", "Failed to send transaction", "peer" => address.clone(), "error" => e.to_string());
                } else {
                    peer.txs_sent += 1;
                }
            }
        }

        info!("p2p", "Transaction broadcast", "peers" => peers.len() as u64, "tx_hash" => transaction.hash());
    }

    pub fn get_peer_count(&self) -> usize {
        self.connected_peers.lock().unwrap().len()
    }

    pub fn get_peer_info(&self) -> Vec<serde_json::Value> {
        let peers = self.connected_peers.lock().unwrap();
        peers
            .values()
            .map(|peer| {
                serde_json::json!({
                    "address": peer.address,
                    "connected_at": peer.connected_at,
                    "last_seen": peer.last_seen,
                    "blocks_sent": peer.blocks_sent,
                    "blocks_received": peer.blocks_received,
                    "txs_sent": peer.txs_sent,
                    "txs_received": peer.txs_received,
                    "node_id": peer.node_id,
                    "version": peer.version,
                    "capabilities": peer.capabilities
                })
            })
            .collect()
    }

    pub fn collect_peer_snapshots(&self) -> Vec<PeerSnapshot> {
        let peers = self.connected_peers.lock().unwrap();
        peers
            .values()
            .map(|peer| PeerSnapshot {
                address: peer.address.clone(),
                block_height: peer.last_known_height,
                best_block_hash: peer.best_block_hash.clone(),
                genesis_hash: peer.genesis_hash.clone(),
            })
            .collect()
    }

    pub fn request_blocks(&self, from_height: u64, count: u32) {
        let message = NetworkMessage::GetBlocks { from_height, count };

        let mut peers = self.connected_peers.lock().unwrap();
        for (address, peer) in peers.iter_mut() {
            if let Some(ref mut stream) = peer.stream {
                if let Err(e) = send_message(stream, &message) {
                    eprintln!("❌ Failed to request blocks from {}: {}", address, e);
                }
            }
        }
    }

    pub fn request_peers(&self) {
        let message = NetworkMessage::GetPeers;
        let mut peers = self.connected_peers.lock().unwrap();
        for (address, peer) in peers.iter_mut() {
            if let Some(ref mut stream) = peer.stream {
                if let Err(e) = send_message(stream, &message) {
                    warn!("p2p", "Failed to request peers", "peer" => address.clone(), "error" => e.to_string());
                }
            }
        }
    }

    pub fn ping_peers(&self) {
        let message = NetworkMessage::Ping;

        let mut peers = self.connected_peers.lock().unwrap();
        for (address, peer) in peers.iter_mut() {
            if let Some(ref mut stream) = peer.stream {
                if let Err(e) = send_message(stream, &message) {
                    eprintln!("❌ Failed to ping {}: {}", address, e);
                }
            }
        }
    }

    pub fn request_peer_statuses(&self) {
        let message = NetworkMessage::GetStatus;
        let mut peers = self.connected_peers.lock().unwrap();
        for (address, peer) in peers.iter_mut() {
            if let Some(ref mut stream) = peer.stream {
                if let Err(e) = send_message(stream, &message) {
                    warn!("p2p", "Failed to request status", "peer" => address.clone(), "error" => e.to_string());
                }
            }
        }
    }

    /// Starts a background bootstrap loop:
    /// - dials configured bootnodes
    /// - requests peers
    /// - requests missing blocks
    /// - pings peers
    pub fn start_bootstrap(self: &Arc<Self>) {
        let network = Arc::clone(self);
        thread::spawn(move || {
            let bootnode_dials: Vec<String> = network
                .config
                .network
                .bootnodes
                .iter()
                .filter_map(|b| parse_bootnode_dial_address(b))
                .collect();
            let heartbeat =
                std::time::Duration::from_secs(network.config.p2p.heartbeat_interval.max(5));

            if bootnode_dials.is_empty() && !network.config.network.bootnodes.is_empty() {
                warn!(
                    "p2p",
                    "All configured bootnodes were invalid (cannot dial)",
                    "bootnodes" => format!("{:?}", network.config.network.bootnodes)
                );
            }

            loop {
                // Keep trying bootnodes until at least one peer is connected.
                for addr in &bootnode_dials {
                    // Avoid self-dial if the config accidentally includes itself.
                    if addr == &network.config.p2p.public_address
                        || addr == &network.config.p2p.listen_address
                    {
                        continue;
                    }
                    let already_connected = {
                        let peers = network.connected_peers.lock().unwrap();
                        peers.contains_key(addr)
                            || peers
                                .values()
                                .any(|p| p.public_address.as_deref() == Some(addr.as_str()))
                    };
                    if !already_connected {
                        let _ = network.connect_to_peer(addr);
                    }
                }

                // Ask connected peers for their peer lists and status.
                if network.config.p2p.enable_discovery {
                    network.request_peers();
                }
                network.request_peer_statuses();

                // Try to sync missing blocks.
                let from_height = {
                    let chain = network.blockchain.lock().unwrap();
                    chain.last().map(|b| b.block_index + 1).unwrap_or(0)
                };
                network.request_blocks(from_height, 200);

                // Keep connections alive.
                network.ping_peers();

                thread::sleep(heartbeat);
            }
        });
    }
}

fn start_listener(
    listen_address: &str,
    blockchain: BlockchainArc,
    connected_peers: PeersArc,
    config: NodeConfig,
    message_sender: mpsc::Sender<(String, NetworkMessage)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(listen_address)?;
    info!("p2p", "P2P listener bound", "listen_address" => listen_address.to_string());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let peer_address = stream.peer_addr()?.to_string();
                info!("p2p", "Incoming peer connection", "peer" => peer_address.clone());

                let blockchain_clone = Arc::clone(&blockchain);
                let peers_clone = Arc::clone(&connected_peers);
                let sender_clone = message_sender.clone();
                let config_clone = config.clone();

                thread::spawn(move || {
                    if let Err(e) = handle_incoming_connection(
                        stream,
                        peer_address,
                        blockchain_clone,
                        peers_clone,
                        sender_clone,
                        config_clone,
                    ) {
                        error!("p2p", "Incoming connection error", "error" => e.to_string());
                    }
                });
            }
            Err(e) => {
                warn!("p2p", "Incoming connection accept error", "error" => e.to_string());
            }
        }
    }

    Ok(())
}

fn handle_incoming_connection(
    stream: TcpStream,
    peer_address: String,
    blockchain: BlockchainArc,
    connected_peers: PeersArc,
    message_sender: mpsc::Sender<(String, NetworkMessage)>,
    config: NodeConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = BufWriter::new(stream);

    // Add peer to connected peers
    {
        let mut peers = connected_peers.lock().unwrap();
        peers.insert(
            peer_address.clone(),
            PeerConnection {
                address: peer_address.clone(),
                public_address: None,
                connected_at: current_timestamp(),
                last_seen: current_timestamp(),
                blocks_sent: 0,
                blocks_received: 0,
                txs_sent: 0,
                txs_received: 0,
                stream: Some(writer.get_ref().try_clone()?),
                node_id: None,
                version: None,
                capabilities: Vec::new(),
                last_known_height: 0,
                best_block_hash: String::new(),
                genesis_hash: String::new(),
            },
        );
    }

    // Send handshake
    let handshake = NetworkMessage::Handshake {
        node_id: config.p2p.node_name.clone(),
        version: "1.0.0".to_string(),
        capabilities: vec!["blocks".to_string(), "transactions".to_string()],
        public_address: Some(config.p2p.public_address.clone()),
        validator_address: Some(resolve_local_validator_address(&config)),
    };

    send_message(&mut writer, &handshake)?;
    writer.flush()?;

    // Listen for messages
    loop {
        match receive_message(&mut reader) {
            Ok(message) => {
                // Update last seen
                {
                    let mut peers = connected_peers.lock().unwrap();
                    if let Some(peer) = peers.get_mut(&peer_address) {
                        peer.last_seen = current_timestamp();
                    }
                }

                // Send message to handler
                if let Err(_) = message_sender.send((peer_address.clone(), message)) {
                    break;
                }
            }
            Err(e) => {
                if e.kind() != io::ErrorKind::UnexpectedEof {
                    eprintln!("❌ Error receiving message from {}: {}", peer_address, e);
                }
                break;
            }
        }
    }

    // Remove peer from connected peers
    {
        let mut peers = connected_peers.lock().unwrap();
        peers.remove(&peer_address);
    }

    info!("p2p", "Peer disconnected", "peer" => peer_address);
    Ok(())
}

fn handle_outgoing_connection(
    stream: TcpStream,
    peer_address: String,
    blockchain: BlockchainArc,
    connected_peers: PeersArc,
    message_sender: mpsc::Sender<(String, NetworkMessage)>,
    config: NodeConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = BufWriter::new(stream);

    // Add peer to connected peers
    {
        let mut peers = connected_peers.lock().unwrap();
        peers.insert(
            peer_address.clone(),
            PeerConnection {
                address: peer_address.clone(),
                public_address: None,
                connected_at: current_timestamp(),
                last_seen: current_timestamp(),
                blocks_sent: 0,
                blocks_received: 0,
                txs_sent: 0,
                txs_received: 0,
                stream: Some(writer.get_ref().try_clone()?),
                node_id: None,
                version: None,
                capabilities: Vec::new(),
                last_known_height: 0,
                best_block_hash: String::new(),
                genesis_hash: String::new(),
            },
        );
    }

    // Send handshake
    let handshake = NetworkMessage::Handshake {
        node_id: config.p2p.node_name.clone(),
        version: "1.0.0".to_string(),
        capabilities: vec!["blocks".to_string(), "transactions".to_string()],
        public_address: Some(config.p2p.public_address.clone()),
        validator_address: Some(resolve_local_validator_address(&config)),
    };

    send_message(&mut writer, &handshake)?;
    writer.flush()?;

    // Listen for messages
    loop {
        match receive_message(&mut reader) {
            Ok(message) => {
                // Update last seen
                {
                    let mut peers = connected_peers.lock().unwrap();
                    if let Some(peer) = peers.get_mut(&peer_address) {
                        peer.last_seen = current_timestamp();
                    }
                }

                // Send message to handler
                if let Err(_) = message_sender.send((peer_address.clone(), message)) {
                    break;
                }
            }
            Err(e) => {
                if e.kind() != io::ErrorKind::UnexpectedEof {
                    eprintln!("❌ Error receiving message from {}: {}", peer_address, e);
                }
                break;
            }
        }
    }

    // Remove peer from connected peers
    {
        let mut peers = connected_peers.lock().unwrap();
        peers.remove(&peer_address);
    }

    info!("p2p", "Peer disconnected", "peer" => peer_address);
    Ok(())
}

fn handle_messages(
    blockchain: BlockchainArc,
    connected_peers: PeersArc,
    receiver: Arc<Mutex<mpsc::Receiver<(String, NetworkMessage)>>>,
    message_sender: mpsc::Sender<(String, NetworkMessage)>,
    config: NodeConfig,
) {
    loop {
        let receiver = receiver.lock().unwrap();
        match receiver.recv() {
            Ok((peer_address, message)) => {
                drop(receiver); // Release lock before processing

                match message {
                    NetworkMessage::Handshake {
                        node_id,
                        version,
                        capabilities,
                        public_address,
                        validator_address,
                    } => {
                        let validator_address = validator_address
                            .as_ref()
                            .map(|value| value.trim().to_string())
                            .filter(|value| !value.is_empty())
                            .unwrap_or_else(|| node_id.clone());

                        info!(
                            "p2p",
                            "Handshake received",
                            "peer" => peer_address.clone(),
                            "node_id" => node_id.clone(),
                            "validator_address" => validator_address.clone(),
                            "version" => version.clone(),
                            "public_address" => public_address.clone().unwrap_or_default()
                        );

                        // Update peer info and deduplicate by node_id
                        {
                            let mut peers = connected_peers.lock().unwrap();

                            // Check if we already have a connection to this node_id
                            let existing_peer_key = peers
                                .iter()
                                .find(|(_, peer)| peer.node_id.as_ref() == Some(&node_id))
                                .map(|(key, _)| key.clone());

                            if let Some(existing_key) = existing_peer_key {
                                // If we already have this peer, remove the old connection and use the new one
                                if existing_key != peer_address {
                                    warn!("p2p", "Duplicate connection from same node_id, replacing old connection",
                                          "node_id" => node_id.clone(),
                                          "old_address" => existing_key.clone(),
                                          "new_address" => peer_address.clone());
                                    peers.remove(&existing_key);
                                }
                            }

                            // Update peer info
                            if let Some(peer) = peers.get_mut(&peer_address) {
                                peer.node_id = Some(node_id);
                                peer.version = Some(version);
                                peer.capabilities = capabilities;
                                peer.public_address = public_address;
                            }
                        }

                        // Auto-register new validators on testnet-beta (only if enabled in config)
                        // This automatically registers any node that connects as a validator
                        // and funds them with 1000 SNRG for staking
                        {
                            // Only auto-register if auto-registration is enabled in config
                            if config.node.auto_register_validator {
                                if !is_validator_allowed(&config, &validator_address) {
                                    warn!(
                                        "p2p",
                                        "Skipping validator auto-registration: address not in allowlist",
                                        "address" => validator_address.clone()
                                    );
                                    continue;
                                }
                                let validator_manager = VALIDATOR_MANAGER.clone();
                                let is_registered = validator_manager
                                    .get_validator(&validator_address)
                                    .is_some();
                                let is_pending = validator_manager.is_pending(&validator_address);

                                if !is_registered && !is_pending {
                                    info!(
                                        "p2p",
                                        "Auto-registering new validator",
                                        "address" => validator_address.clone()
                                    );

                                    // 1000 SNRG in nWei (1 SNRG = 1_000_000_000 nWei)
                                    let funding_amount: u64 = 1000_000_000_000;
                                    let stake_amount: u64 = 1000_000_000_000;

                                    // First, mint 1000 SNRG to the new validator
                                    let token_manager = TOKEN_MANAGER.clone();
                                    let current_balance =
                                        token_manager.get_balance(&validator_address, "SNRG");

                                    if current_balance < funding_amount {
                                        match token_manager.mint_tokens(
                                            &validator_address,
                                            "SNRG",
                                            funding_amount,
                                        ) {
                                            Ok(_) => {
                                                info!(
                                                    "p2p",
                                                    "Auto-funded new validator with 1000 SNRG",
                                                    "address" => validator_address.clone(),
                                                    "amount" => funding_amount
                                                );
                                            }
                                            Err(e) => {
                                                warn!(
                                                    "p2p",
                                                    "Failed to auto-fund validator",
                                                    "address" => validator_address.clone(),
                                                    "error" => e.clone()
                                                );
                                            }
                                        }
                                    }

                                    // Create and submit validator registration
                                    let registration = ValidatorRegistration {
                                        address: validator_address.clone(),
                                        public_key: validator_address.clone(), // Use address as public key for testnet-beta
                                        name: format!(
                                            "Validator-{}",
                                            &validator_address[..8.min(validator_address.len())]
                                        ),
                                        stake_amount,
                                        submitted_at: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap()
                                            .as_secs(),
                                        registration_tx_hash: format!(
                                            "auto-reg-{}",
                                            validator_address.clone()
                                        ),
                                    };

                                    // Register the validator
                                    if let Ok(_) =
                                        validator_manager.register_validator(registration)
                                    {
                                        info!(
                                            "p2p",
                                            "Validator registration submitted",
                                            "address" => validator_address.clone()
                                        );

                                        // Auto-approve the registration for testnet-beta
                                        if let Ok(_) =
                                            validator_manager.approve_validator(&validator_address)
                                        {
                                            info!(
                                                "p2p",
                                                "Validator auto-approved and activated",
                                                "address" => validator_address.clone()
                                            );

                                            // Stake the tokens for the validator
                                            match token_manager.stake_tokens(
                                                &validator_address,
                                                &validator_address,
                                                "SNRG",
                                                stake_amount,
                                            ) {
                                                Ok(_) => {
                                                    info!(
                                                        "p2p",
                                                        "Validator auto-staked 1000 SNRG",
                                                        "address" => validator_address.clone()
                                                    );
                                                }
                                                Err(e) => {
                                                    warn!(
                                                        "p2p",
                                                        "Failed to auto-stake for validator",
                                                        "address" => validator_address.clone(),
                                                        "error" => e.clone()
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    NetworkMessage::Block { block_data } => {
                        info!("p2p", "Received block", "peer" => peer_address.clone());

                        // Update peer stats
                        {
                            let mut peers = connected_peers.lock().unwrap();
                            if let Some(peer) = peers.get_mut(&peer_address) {
                                peer.blocks_received += 1;
                                peer.last_known_height = block_data.block_index;
                                peer.best_block_hash = block_data.hash.clone();
                            }
                        }

                        if apply_block_if_new(&blockchain, block_data.clone()) {
                            info!(
                                "p2p",
                                "Block applied",
                                "height" => block_data.block_index,
                                "hash" => block_data.hash.clone(),
                                "txs" => block_data.transactions.len() as u64
                            );
                        } else {
                            debug!(
                                "p2p",
                                "Block ignored (duplicate/out-of-order)",
                                "height" => block_data.block_index,
                                "hash" => block_data.hash.clone()
                            );
                        }
                    }
                    NetworkMessage::Transaction { transaction_data } => {
                        info!("p2p", "Received transaction", "peer" => peer_address.clone());

                        // Update peer stats
                        {
                            let mut peers = connected_peers.lock().unwrap();
                            if let Some(peer) = peers.get_mut(&peer_address) {
                                peer.txs_received += 1;
                            }
                        }

                        let tx_hash = transaction_data.hash();
                        let mut pool = TX_POOL.lock().unwrap();
                        if !pool.iter().any(|t| t.hash() == tx_hash) {
                            pool.push(transaction_data);
                            info!("p2p", "Transaction added to pool", "tx_hash" => tx_hash);
                        } else {
                            debug!("p2p", "Duplicate transaction ignored", "tx_hash" => tx_hash);
                        }
                    }
                    NetworkMessage::GetBlocks { from_height, count } => {
                        info!(
                            "p2p",
                            "Block request",
                            "peer" => peer_address.clone(),
                            "from_height" => from_height,
                            "count" => count as u64
                        );

                        // Send blocks
                        let blocks = {
                            let chain = blockchain.lock().unwrap();
                            chain
                                .chain
                                .iter()
                                .filter(|b| b.block_index >= from_height)
                                .take(count as usize)
                                .cloned()
                                .collect::<Vec<_>>()
                        };
                        let response = NetworkMessage::Blocks { blocks };

                        // Send response
                        {
                            let mut peers = connected_peers.lock().unwrap();
                            if let Some(peer) = peers.get_mut(&peer_address) {
                                if let Some(ref mut stream) = peer.stream {
                                    if let Err(e) = send_message(stream, &response) {
                                        warn!("p2p", "Failed to send blocks", "peer" => peer_address.clone(), "error" => e.to_string());
                                    } else {
                                        peer.blocks_sent += 1;
                                    }
                                }
                            }
                        }
                    }
                    NetworkMessage::GetStatus => {
                        let (block_height, best_block_hash, genesis_hash) = {
                            let chain = blockchain.lock().unwrap();
                            (
                                chain.last().map(|b| b.block_index).unwrap_or(0),
                                chain.last().map(|b| b.hash.clone()).unwrap_or_default(),
                                chain.get_genesis_hash().unwrap_or_default(),
                            )
                        };

                        let status = NetworkMessage::Status {
                            block_height,
                            best_block_hash: best_block_hash.clone(),
                            genesis_hash: genesis_hash.clone(),
                        };

                        let mut peers = connected_peers.lock().unwrap();
                        if let Some(peer) = peers.get_mut(&peer_address) {
                            peer.last_known_height = block_height;
                            peer.best_block_hash = best_block_hash;
                            peer.genesis_hash = genesis_hash;
                            if let Some(ref mut stream) = peer.stream {
                                if let Err(e) = send_message(stream, &status) {
                                    warn!("p2p", "Failed to send status", "peer" => peer_address.clone(), "error" => e.to_string());
                                }
                            }
                        }
                    }
                    NetworkMessage::Status {
                        block_height,
                        best_block_hash,
                        genesis_hash,
                    } => {
                        let mut peers = connected_peers.lock().unwrap();
                        if let Some(peer) = peers.get_mut(&peer_address) {
                            peer.last_known_height = block_height;
                            peer.best_block_hash = best_block_hash.clone();
                            peer.genesis_hash = genesis_hash.clone();
                        }
                        info!("p2p", "Received status", "peer" => peer_address.clone(), "height" => block_height);
                    }
                    NetworkMessage::GetBlockHeaders {
                        start_height,
                        count,
                    } => {
                        let headers = {
                            let chain = blockchain.lock().unwrap();
                            chain
                                .chain
                                .iter()
                                .filter(|block| block.block_index >= start_height)
                                .take(count.min(500) as usize)
                                .map(|block| block.header())
                                .collect::<Vec<_>>()
                        };
                        let response = NetworkMessage::BlockHeaders { headers };
                        let mut peers = connected_peers.lock().unwrap();
                        if let Some(peer) = peers.get_mut(&peer_address) {
                            if let Some(ref mut stream) = peer.stream {
                                let _ = send_message(stream, &response);
                            }
                        }
                    }
                    NetworkMessage::BlockHeaders { headers } => {
                        debug!("p2p", "Received block headers", "peer" => peer_address.clone(), "count" => headers.len());
                    }
                    NetworkMessage::GetBlockBodies { hashes } => {
                        let blocks = {
                            let chain = blockchain.lock().unwrap();
                            hashes
                                .iter()
                                .filter_map(|hash| {
                                    chain
                                        .chain
                                        .iter()
                                        .find(|block| &block.hash == hash)
                                        .cloned()
                                })
                                .collect::<Vec<_>>()
                        };
                        let response = NetworkMessage::BlockBodies { blocks };
                        let mut peers = connected_peers.lock().unwrap();
                        if let Some(peer) = peers.get_mut(&peer_address) {
                            if let Some(ref mut stream) = peer.stream {
                                let _ = send_message(stream, &response);
                            }
                        }
                    }
                    NetworkMessage::BlockBodies { blocks } => {
                        debug!("p2p", "Received block bodies", "peer" => peer_address.clone(), "count" => blocks.len());
                        for block in blocks {
                            let height = block.block_index;
                            if apply_block_if_new(&blockchain, block) {
                                info!("p2p", "Body block applied", "height" => height);
                            }
                        }
                    }
                    NetworkMessage::Blocks { blocks } => {
                        // Apply blocks received in bulk.
                        let mut applied = 0u64;
                        for block in blocks {
                            if apply_block_if_new(&blockchain, block) {
                                applied += 1;
                            }
                        }
                        if applied > 0 {
                            info!("p2p", "Blocks applied", "count" => applied);
                        }
                    }
                    NetworkMessage::GetPeers => {
                        // Respond with known peer dial addresses.
                        let peer_addresses = if config.p2p.enable_discovery {
                            collect_known_peer_addresses(&connected_peers, &config)
                        } else {
                            Vec::new()
                        };
                        let response = NetworkMessage::Peers { peer_addresses };

                        {
                            let mut peers = connected_peers.lock().unwrap();
                            if let Some(peer) = peers.get_mut(&peer_address) {
                                if let Some(ref mut stream) = peer.stream {
                                    if let Err(e) = send_message(stream, &response) {
                                        warn!("p2p", "Failed to send peers list", "peer" => peer_address.clone(), "error" => e.to_string());
                                    }
                                }
                            }
                        }
                    }
                    NetworkMessage::Peers { peer_addresses } => {
                        if !config.p2p.enable_discovery {
                            debug!(
                                "p2p",
                                "Ignoring peer discovery response because discovery is disabled",
                                "peer" => peer_address.clone()
                            );
                            continue;
                        }

                        // Attempt to dial new peers (best-effort).
                        let max_peers = config.network.max_peers as usize;
                        for addr in peer_addresses {
                            if addr.is_empty() {
                                continue;
                            }
                            if addr == config.p2p.public_address
                                || addr == config.p2p.listen_address
                            {
                                continue;
                            }
                            let should_dial = {
                                let peers = connected_peers.lock().unwrap();
                                if peers.len() >= max_peers {
                                    false
                                } else if peers.contains_key(&addr) {
                                    false
                                } else {
                                    // Also avoid redialing if the addr is already known as a public address.
                                    !peers
                                        .values()
                                        .any(|p| p.public_address.as_deref() == Some(addr.as_str()))
                                }
                            };
                            if should_dial {
                                let _ = dial_peer_async(
                                    addr.clone(),
                                    Arc::clone(&blockchain),
                                    Arc::clone(&connected_peers),
                                    message_sender.clone(),
                                    config.clone(),
                                );
                            }
                        }
                    }
                    NetworkMessage::Ping => {
                        debug!("p2p", "Ping received", "peer" => peer_address.clone());

                        // Send pong
                        {
                            let mut peers = connected_peers.lock().unwrap();
                            if let Some(peer) = peers.get_mut(&peer_address) {
                                if let Some(ref mut stream) = peer.stream {
                                    let pong = NetworkMessage::Pong;
                                    if let Err(e) = send_message(stream, &pong) {
                                        warn!("p2p", "Failed to send pong", "peer" => peer_address.clone(), "error" => e.to_string());
                                    }
                                }
                            }
                        }
                    }
                    NetworkMessage::Pong => {
                        debug!("p2p", "Pong received", "peer" => peer_address.clone());
                    }
                    _ => {
                        debug!("p2p", "Unhandled P2P message", "peer" => peer_address.clone(), "message" => format!("{:?}", message));
                    }
                }
            }
            Err(_) => {
                break;
            }
        }
    }
}

fn send_message(
    stream: &mut impl Write,
    message: &NetworkMessage,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string(message)?;
    let data = json.as_bytes();
    let len = data.len() as u32;

    // Send length prefix
    stream.write_all(&len.to_le_bytes())?;
    // Send message data
    stream.write_all(data)?;
    stream.flush()?;

    Ok(())
}

fn receive_message(stream: &mut impl Read) -> Result<NetworkMessage, io::Error> {
    // Read length prefix
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes)?;
    let len = u32::from_le_bytes(len_bytes) as usize;

    // Read message data
    let mut data = vec![0u8; len];
    stream.read_exact(&mut data)?;

    // Parse JSON message
    let json =
        String::from_utf8(data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let message: NetworkMessage =
        serde_json::from_str(&json).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    Ok(message)
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn parse_bootnode_dial_address(bootnode: &str) -> Option<String> {
    let raw = bootnode.trim();
    if raw.is_empty() {
        return None;
    }

    // Strip common schemes
    let raw = raw
        .strip_prefix("snr://")
        .or_else(|| raw.strip_prefix("enode://"))
        .unwrap_or(raw);

    // Use part after '@' if present.
    let raw = raw.rsplit_once('@').map(|(_, right)| right).unwrap_or(raw);

    // Strip path / query / fragment.
    let raw = raw.split('/').next().unwrap_or(raw);
    let raw = raw.split('?').next().unwrap_or(raw);
    let raw = raw.split('#').next().unwrap_or(raw);

    let dial = raw.trim();
    if dial.is_empty() || !dial.contains(':') {
        return None;
    }

    Some(dial.to_string())
}

fn dial_with_timeout(peer: &str, timeout: std::time::Duration) -> io::Result<TcpStream> {
    let mut last_err: Option<io::Error> = None;
    let addrs = peer.to_socket_addrs()?;
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(stream) => {
                let _ = stream.set_nodelay(true);
                let _ = stream.set_read_timeout(Some(timeout));
                let _ = stream.set_write_timeout(Some(timeout));
                return Ok(stream);
            }
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| io::Error::new(io::ErrorKind::Other, "No resolved addresses")))
}

fn collect_known_peer_addresses(connected_peers: &PeersArc, config: &NodeConfig) -> Vec<String> {
    use std::collections::HashSet;

    let mut out = HashSet::<String>::new();

    if !config.p2p.public_address.trim().is_empty() {
        out.insert(config.p2p.public_address.clone());
    }

    if let Ok(peers) = connected_peers.lock() {
        for peer in peers.values() {
            if let Some(pub_addr) = peer.public_address.as_ref() {
                if !pub_addr.trim().is_empty() {
                    out.insert(pub_addr.clone());
                    continue;
                }
            }
            if !peer.address.trim().is_empty() {
                out.insert(peer.address.clone());
            }
        }
    }

    out.into_iter().collect()
}

fn apply_block_if_new(blockchain: &BlockchainArc, block: Block) -> bool {
    let mut chain = blockchain.lock().unwrap();

    // Deduplicate by hash.
    if chain.chain.iter().any(|b| b.hash == block.hash) {
        return false;
    }

    // Append if it extends the tip, otherwise ignore for now (no fork handling).
    if let Some(tip) = chain.last() {
        if block.block_index != tip.block_index + 1 || block.previous_hash != tip.hash {
            return false;
        }
    }

    chain.add_block(block);
    // Use absolute path based on project root
    let chain_path = crate::utils::resolve_data_path("data/chain.json");
    chain.save_to_file(chain_path.to_str().unwrap_or("data/chain.json"));
    true
}

/// Best-effort dial for a discovered peer.
fn dial_peer_async(
    peer_address: String,
    blockchain: BlockchainArc,
    connected_peers: PeersArc,
    message_sender: mpsc::Sender<(String, NetworkMessage)>,
    config: NodeConfig,
) -> Result<(), ()> {
    thread::spawn(move || {
        match dial_with_timeout(&peer_address, std::time::Duration::from_secs(5)) {
            Ok(stream) => {
                if let Err(e) = handle_outgoing_connection(
                    stream,
                    peer_address,
                    blockchain,
                    connected_peers,
                    message_sender,
                    config,
                ) {
                    warn!("p2p", "Discovered peer dial failed", "error" => e.to_string());
                }
            }
            Err(e) => {
                debug!("p2p", "Discovered peer dial error", "error" => e.to_string());
            }
        }
    });
    Ok(())
}
