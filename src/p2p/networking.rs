use crate::block::{Block, BlockChain};
use crate::config::NodeConfig;
use crate::p2p::messages::NetworkMessage;
use crate::rpc::rpc_server::TX_POOL;
use crate::token::TOKEN_MANAGER;
use crate::transaction::Transaction;
use crate::validator::{ValidatorRegistration, VALIDATOR_MANAGER};
use crate::{debug, error, info, warn};
use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::Resolver;
use serde::Deserialize;
use serde_json;
use std::collections::{HashMap, HashSet};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// Type aliases to avoid nested generics parsing issues
type PeerMap = HashMap<String, PeerConnection>;
type BlockchainArc = Arc<Mutex<BlockChain>>;
type PeersArc = Arc<Mutex<PeerMap>>;
type DialTargetsArc = Arc<Mutex<Vec<String>>>;

pub struct P2PNetwork {
    blockchain: BlockchainArc,
    config: NodeConfig,
    connected_peers: PeersArc,
    discovered_dial_targets: DialTargetsArc,
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

#[derive(Debug, Deserialize)]
struct SeedPeerListResponse {
    #[serde(default)]
    bootnodes: Vec<SeedBootnodeRecord>,
    #[serde(default)]
    dnsaddr_bootstrap: Vec<String>,
    #[serde(default)]
    peers: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SeedBootnodeRecord {
    hostname: String,
    port: u16,
    #[serde(default)]
    reachable: Option<bool>,
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

fn announced_validator_address(config: &NodeConfig) -> Option<String> {
    if config.node.bootstrap_only || !config.node.auto_register_validator {
        return None;
    }

    let resolved = resolve_local_validator_address(config);
    let trimmed = resolved.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
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

fn resolve_bootstrap_dial_targets(config: &NodeConfig) -> Vec<String> {
    let mut targets = HashSet::<String>::new();

    for bootnode in &config.network.bootnodes {
        if let Some(dial) = parse_bootnode_dial_address(bootnode) {
            targets.insert(dial);
        }
    }

    for dial in resolve_dns_bootstrap_targets(&config.network.bootstrap_dns_records) {
        targets.insert(dial);
    }

    for dial in resolve_seed_server_targets(&config.network.seed_servers) {
        targets.insert(dial);
    }

    for dial in &config.network.additional_dial_targets {
        if let Some(parsed) = parse_bootnode_dial_address(dial) {
            targets.insert(parsed);
        }
    }

    let mut ordered = targets.into_iter().collect::<Vec<_>>();
    ordered.sort();
    ordered
}

fn resolve_dns_bootstrap_targets(record_names: &[String]) -> Vec<String> {
    if record_names.is_empty() {
        return Vec::new();
    }

    let resolver = match build_dns_resolver() {
        Ok(resolver) => resolver,
        Err(error) => {
            warn!("p2p", "Failed to initialize DNS resolver for bootstrap discovery", "error" => error);
            return Vec::new();
        }
    };

    let mut visited = HashSet::<String>::new();
    let mut out = HashSet::<String>::new();

    for record_name in record_names {
        collect_dnsaddr_record_targets(&resolver, record_name, 0, &mut visited, &mut out);
    }

    let mut ordered = out.into_iter().collect::<Vec<_>>();
    ordered.sort();
    ordered
}

fn build_dns_resolver() -> Result<Resolver, String> {
    Resolver::from_system_conf()
        .or_else(|_| Resolver::new(ResolverConfig::default(), ResolverOpts::default()))
        .map_err(|error| error.to_string())
}

fn collect_dnsaddr_record_targets(
    resolver: &Resolver,
    record_name: &str,
    depth: usize,
    visited: &mut HashSet<String>,
    out: &mut HashSet<String>,
) {
    let record_name = record_name.trim();
    if record_name.is_empty() || depth > 4 {
        return;
    }

    let canonical = record_name.trim_end_matches('.').to_string();
    if !visited.insert(canonical.clone()) {
        return;
    }

    match resolver.txt_lookup(canonical.as_str()) {
        Ok(records) => {
            for record in records.iter() {
                for txt in record.txt_data() {
                    let Ok(value) = std::str::from_utf8(txt) else {
                        continue;
                    };
                    collect_dnsaddr_txt_target(resolver, value, depth, visited, out);
                }
            }
        }
        Err(error) => {
            debug!(
                "p2p",
                "Bootstrap DNS TXT lookup failed",
                "record" => canonical,
                "error" => error.to_string()
            );
        }
    }
}

fn collect_dnsaddr_txt_target(
    resolver: &Resolver,
    value: &str,
    depth: usize,
    visited: &mut HashSet<String>,
    out: &mut HashSet<String>,
) {
    let value = value
        .trim()
        .trim_matches('"')
        .strip_prefix("dnsaddr=")
        .unwrap_or(value.trim())
        .trim();
    if value.is_empty() {
        return;
    }

    if let Some(next_record) = parse_dnsaddr_reference_record(value) {
        collect_dnsaddr_record_targets(resolver, &next_record, depth + 1, visited, out);
        return;
    }

    if let Some(dial) = parse_dnsaddr_multiaddr_to_dial_address(value) {
        out.insert(dial);
    }
}

fn parse_dnsaddr_reference_record(value: &str) -> Option<String> {
    let referenced = value.strip_prefix("/dnsaddr/")?;
    let referenced = referenced.split('/').next()?.trim().trim_end_matches('.');
    if referenced.is_empty() {
        None
    } else {
        Some(format!("_dnsaddr.{}", referenced))
    }
}

fn parse_dnsaddr_multiaddr_to_dial_address(value: &str) -> Option<String> {
    let segments = value
        .split('/')
        .filter(|segment| !segment.trim().is_empty())
        .collect::<Vec<_>>();

    let mut host: Option<String> = None;
    let mut port: Option<u16> = None;
    let mut transport: Option<&str> = None;
    let mut index = 0usize;

    while index + 1 < segments.len() {
        let key = segments[index];
        let val = segments[index + 1];
        match key {
            "dns" | "dns4" | "dns6" | "ip4" | "ip6" if host.is_none() => {
                host = Some(val.to_string());
            }
            "tcp" => {
                if let Ok(parsed) = val.parse::<u16>() {
                    port = Some(parsed);
                    transport = Some("tcp");
                }
            }
            "udp" => {
                transport = Some("udp");
            }
            _ => {}
        }
        index += 2;
    }

    match (host, port, transport) {
        (Some(host), Some(port), Some("tcp")) => Some(format!("{host}:{port}")),
        _ => None,
    }
}

fn resolve_seed_server_targets(seed_servers: &[String]) -> Vec<String> {
    if seed_servers.is_empty() {
        return Vec::new();
    }

    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            warn!(
                "p2p",
                "Failed to build HTTP client for seed discovery",
                "error" => error.to_string()
            );
            return Vec::new();
        }
    };

    let mut out = HashSet::<String>::new();
    for seed_server in seed_servers {
        fetch_seed_server_targets(&client, seed_server, &mut out);
    }

    let mut ordered = out.into_iter().collect::<Vec<_>>();
    ordered.sort();
    ordered
}

fn fetch_seed_server_targets(
    client: &reqwest::blocking::Client,
    seed_server: &str,
    out: &mut HashSet<String>,
) {
    let json_url = normalize_seed_server_url(seed_server, "/peer-list.json");
    if !json_url.is_empty() {
        match client.get(&json_url).send() {
            Ok(response) if response.status().is_success() => {
                match response.json::<SeedPeerListResponse>() {
                    Ok(payload) => {
                        for bootnode in payload.bootnodes {
                            if bootnode.reachable.unwrap_or(true) {
                                out.insert(format!("{}:{}", bootnode.hostname, bootnode.port));
                            }
                        }
                        for value in payload.dnsaddr_bootstrap {
                            if let Some(dial) = parse_dnsaddr_multiaddr_to_dial_address(&value) {
                                out.insert(dial);
                            }
                        }
                        for peer in payload.peers {
                            if let Some(dial) = parse_bootnode_dial_address(&peer) {
                                out.insert(dial);
                            }
                        }
                        return;
                    }
                    Err(error) => {
                        debug!(
                            "p2p",
                            "Failed to parse seed peer list JSON",
                            "seed_server" => seed_server.to_string(),
                            "error" => error.to_string()
                        );
                    }
                }
            }
            Ok(response) => {
                debug!(
                    "p2p",
                    "Seed peer list request returned non-success status",
                    "seed_server" => seed_server.to_string(),
                    "status" => response.status().as_u16()
                );
            }
            Err(error) => {
                debug!(
                    "p2p",
                    "Seed peer list request failed",
                    "seed_server" => seed_server.to_string(),
                    "error" => error.to_string()
                );
            }
        }
    }

    let text_url = normalize_seed_server_url(seed_server, "/dns/bootstrap.txt");
    if text_url.is_empty() {
        return;
    }

    match client.get(&text_url).send() {
        Ok(response) if response.status().is_success() => {
            if let Ok(body) = response.text() {
                for line in body.lines() {
                    let value = line.trim();
                    if value.is_empty() {
                        continue;
                    }
                    if let Some(dial) = parse_dnsaddr_multiaddr_to_dial_address(
                        value.strip_prefix("dnsaddr=").unwrap_or(value),
                    ) {
                        out.insert(dial);
                    }
                }
            }
        }
        Ok(_) | Err(_) => {}
    }
}

fn normalize_seed_server_url(seed_server: &str, default_path: &str) -> String {
    let trimmed = seed_server.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return String::new();
    }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        let remainder = trimmed
            .split_once("://")
            .map(|(_, rest)| rest)
            .unwrap_or(trimmed);
        if remainder.contains('/') {
            trimmed.to_string()
        } else {
            format!("{trimmed}{default_path}")
        }
    } else {
        format!("http://{trimmed}{default_path}")
    }
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
            discovered_dial_targets: Arc::new(Mutex::new(Vec::new())),
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
        let discovered_targets_handler = Arc::clone(&self.discovered_dial_targets);
        let receiver = Arc::clone(&self.message_receiver);
        let handler_config = self.config.clone();
        let handler_sender = self.message_sender.clone();

        thread::spawn(move || {
            handle_messages(
                blockchain_handler,
                peers_handler,
                discovered_targets_handler,
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

    pub fn request_blocks_from_peer(
        &self,
        peer_address: &str,
        from_height: u64,
        count: u32,
    ) -> bool {
        let message = NetworkMessage::GetBlocks { from_height, count };
        let mut peers = self.connected_peers.lock().unwrap();
        if let Some(peer) = peers.get_mut(peer_address) {
            if let Some(ref mut stream) = peer.stream {
                if let Err(e) = send_message(stream, &message) {
                    warn!(
                        "p2p",
                        "Failed to request blocks from peer",
                        "peer" => peer_address.to_string(),
                        "error" => e.to_string()
                    );
                    return false;
                }
                return true;
            }
        }
        false
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
            let heartbeat =
                std::time::Duration::from_secs(network.config.p2p.heartbeat_interval.max(5));
            let bootstrap_refresh_interval = std::time::Duration::from_secs(120);
            let mut bootnode_dials = Vec::<String>::new();
            let mut last_refresh = Instant::now() - bootstrap_refresh_interval;

            loop {
                if last_refresh.elapsed() >= bootstrap_refresh_interval || bootnode_dials.is_empty()
                {
                    bootnode_dials = resolve_bootstrap_dial_targets(&network.config);
                    last_refresh = Instant::now();

                    if bootnode_dials.is_empty() {
                        warn!(
                            "p2p",
                            "Bootstrap resolution returned no dialable peers",
                            "bootnodes" => format!("{:?}", network.config.network.bootnodes),
                            "seed_servers" => format!("{:?}", network.config.network.seed_servers),
                            "dns_records" => format!(
                                "{:?}",
                                network.config.network.bootstrap_dns_records
                            )
                        );
                    } else {
                        if let Ok(mut discovered) = network.discovered_dial_targets.lock() {
                            *discovered = bootnode_dials.clone();
                        }
                        info!(
                            "p2p",
                            "Resolved bootstrap dial targets",
                            "targets" => format!("{:?}", bootnode_dials)
                        );
                    }
                }

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
                if !network.config.node.bootstrap_only {
                    network.request_peer_statuses();
                }

                // Try to sync missing blocks.
                if !network.config.node.bootstrap_only {
                    let from_height = {
                        let chain = network.blockchain.lock().unwrap();
                        chain.last().map(|b| b.block_index + 1).unwrap_or(0)
                    };
                    network.request_blocks(from_height, 200);
                }

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
    _blockchain: BlockchainArc,
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
        validator_address: announced_validator_address(&config),
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
    _blockchain: BlockchainArc,
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
        validator_address: announced_validator_address(&config),
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
    discovered_dial_targets: DialTargetsArc,
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
                        let announced_validator_address = validator_address
                            .as_ref()
                            .map(|value| value.trim().to_string())
                            .filter(|value| !value.is_empty());
                        let peer_identity = announced_validator_address
                            .clone()
                            .unwrap_or_else(|| node_id.clone());

                        info!(
                            "p2p",
                            "Handshake received",
                            "peer" => peer_address.clone(),
                            "node_id" => node_id.clone(),
                            "validator_address" => announced_validator_address.clone().unwrap_or_default(),
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
                            if config.node.bootstrap_only {
                                debug!(
                                    "p2p",
                                    "Bootstrap-only mode enabled; skipping validator auto-registration for peer",
                                    "peer" => peer_address.clone(),
                                    "peer_identity" => peer_identity.clone()
                                );
                                continue;
                            }

                            if config.node.auto_register_validator {
                                let Some(validator_address) = announced_validator_address.clone()
                                else {
                                    debug!(
                                        "p2p",
                                        "Peer did not advertise a validator address; skipping validator auto-registration",
                                        "peer" => peer_address.clone(),
                                        "peer_identity" => peer_identity.clone()
                                    );
                                    continue;
                                };

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
                        if config.node.bootstrap_only {
                            debug!(
                                "p2p",
                                "Bootstrap-only node ignoring block propagation",
                                "peer" => peer_address.clone(),
                                "height" => block_data.block_index
                            );
                            continue;
                        }

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
                        if config.node.bootstrap_only {
                            debug!(
                                "p2p",
                                "Bootstrap-only node ignoring transaction propagation",
                                "peer" => peer_address.clone(),
                                "tx_hash" => transaction_data.hash()
                            );
                            continue;
                        }

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
                        if config.node.bootstrap_only {
                            debug!(
                                "p2p",
                                "Bootstrap-only node returning empty block response",
                                "peer" => peer_address.clone(),
                                "from_height" => from_height,
                                "count" => count as u64
                            );
                            let response = NetworkMessage::Blocks { blocks: Vec::new() };
                            let mut peers = connected_peers.lock().unwrap();
                            if let Some(peer) = peers.get_mut(&peer_address) {
                                if let Some(ref mut stream) = peer.stream {
                                    let _ = send_message(stream, &response);
                                }
                            }
                            continue;
                        }

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
                        if config.node.bootstrap_only {
                            let status = NetworkMessage::Status {
                                block_height: 0,
                                best_block_hash: String::new(),
                                genesis_hash: String::new(),
                            };

                            let mut peers = connected_peers.lock().unwrap();
                            if let Some(peer) = peers.get_mut(&peer_address) {
                                peer.last_known_height = 0;
                                peer.best_block_hash.clear();
                                peer.genesis_hash.clear();
                                if let Some(ref mut stream) = peer.stream {
                                    let _ = send_message(stream, &status);
                                }
                            }
                            continue;
                        }

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
                        if config.node.bootstrap_only {
                            debug!(
                                "p2p",
                                "Bootstrap-only node ignoring remote chain status",
                                "peer" => peer_address.clone(),
                                "height" => block_height
                            );
                            continue;
                        }

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
                        if config.node.bootstrap_only {
                            let response = NetworkMessage::BlockHeaders {
                                headers: Vec::new(),
                            };
                            let mut peers = connected_peers.lock().unwrap();
                            if let Some(peer) = peers.get_mut(&peer_address) {
                                if let Some(ref mut stream) = peer.stream {
                                    let _ = send_message(stream, &response);
                                }
                            }
                            continue;
                        }

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
                        if config.node.bootstrap_only {
                            debug!(
                                "p2p",
                                "Bootstrap-only node ignoring block headers",
                                "peer" => peer_address.clone(),
                                "count" => headers.len()
                            );
                            continue;
                        }

                        debug!("p2p", "Received block headers", "peer" => peer_address.clone(), "count" => headers.len());
                    }
                    NetworkMessage::GetBlockBodies { hashes } => {
                        if config.node.bootstrap_only {
                            let response = NetworkMessage::BlockBodies { blocks: Vec::new() };
                            let mut peers = connected_peers.lock().unwrap();
                            if let Some(peer) = peers.get_mut(&peer_address) {
                                if let Some(ref mut stream) = peer.stream {
                                    let _ = send_message(stream, &response);
                                }
                            }
                            continue;
                        }

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
                        if config.node.bootstrap_only {
                            debug!(
                                "p2p",
                                "Bootstrap-only node ignoring block bodies",
                                "peer" => peer_address.clone(),
                                "count" => blocks.len()
                            );
                            continue;
                        }

                        debug!("p2p", "Received block bodies", "peer" => peer_address.clone(), "count" => blocks.len());
                        for block in blocks {
                            let height = block.block_index;
                            if apply_block_if_new(&blockchain, block) {
                                info!("p2p", "Body block applied", "height" => height);
                            }
                        }
                    }
                    NetworkMessage::Blocks { blocks } => {
                        if config.node.bootstrap_only {
                            debug!(
                                "p2p",
                                "Bootstrap-only node ignoring bulk blocks",
                                "peer" => peer_address.clone(),
                                "count" => blocks.len()
                            );
                            continue;
                        }

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
                            collect_known_peer_addresses(
                                &connected_peers,
                                &discovered_dial_targets,
                                &config,
                            )
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
                        let self_public_address = parse_bootnode_dial_address(&config.p2p.public_address);
                        let self_listen_address = parse_bootnode_dial_address(&config.p2p.listen_address);
                        for addr in peer_addresses {
                            let Some(addr) = parse_bootnode_dial_address(&addr) else {
                                debug!(
                                    "p2p",
                                    "Ignoring non-dialable peer discovery address",
                                    "peer" => peer_address.clone(),
                                    "address" => addr
                                );
                                continue;
                            };
                            if self_public_address.as_deref() == Some(addr.as_str())
                                || self_listen_address.as_deref() == Some(addr.as_str())
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
                                info!(
                                    "p2p",
                                    "Dialing discovered peer",
                                    "source_peer" => peer_address.clone(),
                                    "target" => addr.clone()
                                );
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

    normalize_dial_target(raw.trim())
}

fn normalize_dial_target(dial: &str) -> Option<String> {
    let dial = dial.trim();
    if dial.is_empty() {
        return None;
    }

    if let Some(stripped) = dial.strip_prefix('[') {
        let (host, port) = stripped.rsplit_once("]:")?;
        return normalize_host_port(host, port);
    }

    let (host, port) = dial.rsplit_once(':')?;
    normalize_host_port(host, port)
}

fn normalize_host_port(host: &str, port: &str) -> Option<String> {
    let host = host.trim().trim_matches('[').trim_matches(']').trim_end_matches('.');
    let port = port.trim().parse::<u16>().ok()?;
    if port == 0 || host.is_empty() || !is_plausible_dial_host(host) {
        return None;
    }

    match host.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V6(_)) => Some(format!("[{host}]:{port}")),
        Ok(std::net::IpAddr::V4(_)) => Some(format!("{host}:{port}")),
        Err(_) if host.contains(':') => Some(format!("[{host}]:{port}")),
        Err(_) => Some(format!("{host}:{port}")),
    }
}

fn is_plausible_dial_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") || host.parse::<std::net::IpAddr>().is_ok() {
        return true;
    }

    host.contains('.')
        && host
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '.')
}

fn dial_with_timeout(peer: &str, timeout: std::time::Duration) -> io::Result<TcpStream> {
    let mut last_err: Option<io::Error> = None;
    let addrs = peer.to_socket_addrs()?;
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(stream) => {
                let _ = stream.set_nodelay(true);
                // Keep the connect deadline, but leave established peer streams blocking.
                // A fixed read timeout on long-lived P2P sockets causes idle bootstrap peers
                // to be disconnected even though the link is healthy.
                let _ = stream.set_read_timeout(None);
                let _ = stream.set_write_timeout(None);
                return Ok(stream);
            }
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| io::Error::new(io::ErrorKind::Other, "No resolved addresses")))
}

fn collect_known_peer_addresses(
    connected_peers: &PeersArc,
    discovered_dial_targets: &DialTargetsArc,
    config: &NodeConfig,
) -> Vec<String> {
    let mut out = HashSet::<String>::new();

    if let Some(address) = parse_bootnode_dial_address(&config.p2p.public_address) {
        out.insert(address);
    }

    for dial in &config.network.additional_dial_targets {
        if let Some(address) = parse_bootnode_dial_address(dial) {
            out.insert(address);
        }
    }

    if let Ok(discovered) = discovered_dial_targets.lock() {
        for dial in discovered.iter() {
            if let Some(address) = parse_bootnode_dial_address(dial) {
                out.insert(address);
            }
        }
    }

    if let Ok(peers) = connected_peers.lock() {
        for peer in peers.values() {
            if let Some(pub_addr) = peer.public_address.as_ref() {
                if let Some(address) = parse_bootnode_dial_address(pub_addr) {
                    out.insert(address);
                    continue;
                }
            }
            if let Some(address) = parse_bootnode_dial_address(&peer.address) {
                out.insert(address);
            }
        }
    }

    let mut ordered = out.into_iter().collect::<Vec<_>>();
    ordered.sort();
    ordered
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

#[cfg(test)]
mod tests {
    use super::dial_with_timeout;
    use super::{collect_known_peer_addresses, parse_bootnode_dial_address, DialTargetsArc};
    use crate::config::NodeConfig;
    use std::collections::HashMap;
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn dial_with_timeout_keeps_established_peer_streams_blocking() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let accept_handle = thread::spawn(move || {
            let _ = listener.accept().unwrap();
        });

        let stream = dial_with_timeout(&addr.to_string(), Duration::from_millis(250)).unwrap();

        assert_eq!(stream.read_timeout().unwrap(), None);
        assert_eq!(stream.write_timeout().unwrap(), None);

        accept_handle.join().unwrap();
    }

    #[test]
    fn parse_bootnode_dial_address_normalizes_identity_and_ipv6() {
        assert_eq!(
            parse_bootnode_dial_address("snr://peer@24.181.87.76:38638"),
            Some("24.181.87.76:38638".to_string())
        );
        assert_eq!(
            parse_bootnode_dial_address(
                "snr://synv1156xl3ct9cxc4cl9pdn5ww9myxudavl0hxrq7zv@2a02:1812:172a:e900:1497:71dc:d720:e28e:38638",
            ),
            Some("[2a02:1812:172a:e900:1497:71dc:d720:e28e]:38638".to_string())
        );
    }

    #[test]
    fn parse_bootnode_dial_address_rejects_invalid_bare_host_targets() {
        assert_eq!(parse_bootnode_dial_address("snr://peer@test:38638"), None);
        assert_eq!(parse_bootnode_dial_address(""), None);
    }

    #[test]
    fn collect_known_peer_addresses_includes_discovered_targets() {
        let mut config = NodeConfig::default();
        config.p2p.public_address = "24.181.87.76:38638".to_string();
        config.network.additional_dial_targets = vec!["73.79.66.255:39638".to_string()];
        let connected_peers = Arc::new(Mutex::new(HashMap::new()));
        let discovered_targets: DialTargetsArc =
            Arc::new(Mutex::new(vec!["64.227.107.57:39638".to_string()]));

        let addresses =
            collect_known_peer_addresses(&connected_peers, &discovered_targets, &config);

        assert!(addresses.contains(&"24.181.87.76:38638".to_string()));
        assert!(addresses.contains(&"73.79.66.255:39638".to_string()));
        assert!(addresses.contains(&"64.227.107.57:39638".to_string()));
    }
}
