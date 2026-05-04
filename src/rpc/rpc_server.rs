use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::address::generate_cluster_address;
use crate::block::BlockChain;
use crate::consensus::synergy_score::SynergyScoreCalculator;
use crate::crypto::pqc::PQCManager;
use crate::genesis::canonical_genesis;
use crate::role_profiles::{resolve_configured_role, AuthorityPlane, RoleProfile};
use crate::sxcp;
use crate::sync::{SyncManager, SyncState};
use crate::token::TOKEN_MANAGER;
use crate::transaction::Transaction;
use crate::validator::{
    Validator, ValidatorManager, ValidatorStatus, INITIAL_VALIDATOR_SYNERGY_SCORE,
    TESTNET_BETA_VALIDATOR_CLUSTER_SIZE, VALIDATOR_MANAGER,
};
use crate::wallet::WALLET_MANAGER;
// Temporarily disabled for quick compile
// use crate::aivm::AIVMRuntime;
// use crate::aivm::runtime::{ContractType, AIVMExecutionContext};
use hex;
use lazy_static::lazy_static;
use serde_json::{json, Value};
use tungstenite::handshake::server::{Request as WsRequest, Response as WsResponse};
use tungstenite::{accept_hdr, Error as WsError, Message as WsMessage};

lazy_static! {
    pub static ref TX_POOL: Arc<Mutex<Vec<Transaction>>> = Arc::new(Mutex::new(Vec::new()));
}

lazy_static! {
    static ref NODE_START_TIME: Arc<Mutex<Option<u64>>> = Arc::new(Mutex::new(None));
}

lazy_static! {
    static ref SIMULATION_CACHE: Mutex<HashMap<String, CachedSimulation>> =
        Mutex::new(HashMap::new());
}

static SUBSCRIPTION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
struct RpcError {
    code: i64,
    message: String,
    data: Option<Value>,
}

impl RpcError {
    fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    fn with_data(code: i64, message: impl Into<String>, data: Value) -> Self {
        Self {
            code,
            message: message.into(),
            data: Some(data),
        }
    }
}

#[derive(Debug, Clone)]
struct CachedSimulation {
    simulation_hash: String,
    created_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RpcTransport {
    Http,
    WebSocket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RpcMethodExposure {
    PublicRead,
    PublicClient,
    AuthorityPlane,
    NonPublicWrite,
    Operator,
}

impl RpcMethodExposure {
    fn label(self) -> &'static str {
        match self {
            Self::PublicRead => "public-read",
            Self::PublicClient => "public-client",
            Self::AuthorityPlane => "authority-plane",
            Self::NonPublicWrite => "non-public-write",
            Self::Operator => "operator",
        }
    }
}

#[derive(Debug, Clone)]
struct RpcRequestContext {
    transport: RpcTransport,
    peer_addr: Option<SocketAddr>,
    headers: HashMap<String, String>,
    role_profile: Option<&'static RoleProfile>,
}

impl RpcRequestContext {
    fn new(
        transport: RpcTransport,
        peer_addr: Option<SocketAddr>,
        headers: HashMap<String, String>,
    ) -> Self {
        Self {
            transport,
            peer_addr,
            headers,
            role_profile: current_rpc_role_profile(),
        }
    }

    fn effective_client_ip(&self) -> Option<IpAddr> {
        parse_forwarded_ip(self.forwarded_client_ip_header().or_else(|| {
            self.headers
                .get("x-forwarded-for")
                .map(String::as_str)
                .or_else(|| self.headers.get("x-real-ip").map(String::as_str))
        }))
        .or_else(|| self.peer_addr.map(|addr| addr.ip()))
    }

    fn forwarded_client_ip_header(&self) -> Option<&str> {
        self.headers
            .get("cf-connecting-ip")
            .map(String::as_str)
            .or_else(|| self.headers.get("true-client-ip").map(String::as_str))
            .or_else(|| self.headers.get("x-forwarded-for").map(String::as_str))
            .or_else(|| self.headers.get("x-real-ip").map(String::as_str))
    }

    fn is_public_request(&self) -> bool {
        self.effective_client_ip()
            .map(|ip| !ip.is_loopback())
            .unwrap_or(false)
    }

    fn transport_label(&self) -> &'static str {
        match self.transport {
            RpcTransport::Http => "http",
            RpcTransport::WebSocket => "ws",
        }
    }
}

#[derive(Debug, Clone)]
enum SubscriptionCursor {
    NewHeads {
        last_block: u64,
    },
    Logs {
        last_block: u64,
        address: Option<String>,
        topics: Vec<String>,
    },
    PendingTransactions {
        seen_hashes: HashSet<String>,
    },
    ValidatorEvents {
        last_block: u64,
    },
}

#[derive(Debug, Clone, serde::Deserialize)]
struct RpcTransactionEnvelope {
    #[serde(default)]
    from: Option<String>,
    #[serde(default)]
    sender: Option<String>,
    #[serde(default)]
    to: Option<String>,
    #[serde(default)]
    receiver: Option<String>,
    #[serde(default)]
    value: Option<Value>,
    #[serde(default)]
    amount: Option<Value>,
    #[serde(default)]
    nonce: Option<u64>,
    #[serde(default)]
    signature: Option<Value>,
    #[serde(default)]
    timestamp: Option<u64>,
    #[serde(default)]
    gas_price: Option<Value>,
    #[serde(rename = "gasPrice", default)]
    gas_price_alias: Option<Value>,
    #[serde(rename = "maxFee", default)]
    max_fee: Option<Value>,
    #[serde(default)]
    gas_limit: Option<Value>,
    #[serde(rename = "gasLimit", default)]
    gas_limit_alias: Option<Value>,
    #[serde(rename = "maxPriorityFeePerGas", default)]
    max_priority_fee_per_gas: Option<Value>,
    #[serde(default)]
    data: Option<String>,
    #[serde(default)]
    signature_algorithm: Option<String>,
    #[serde(rename = "signatureAlgorithm", default)]
    signature_algorithm_alias: Option<String>,
    #[serde(rename = "chainId", default)]
    chain_id: Option<Value>,
    #[serde(default)]
    tx_type: Option<String>,
    #[serde(rename = "type", default)]
    envelope_type: Option<String>,
    #[serde(default)]
    delegation: Option<Value>,
    #[serde(default)]
    delegations: Option<Value>,
    #[serde(rename = "authorizationList", default)]
    authorization_list: Option<Value>,
}

#[derive(Debug, Clone)]
struct NormalizedRpcTransaction {
    sender: String,
    receiver: String,
    amount: u64,
    nonce: u64,
    signature: Vec<u8>,
    timestamp: u64,
    gas_price: u64,
    gas_limit: u64,
    data: Option<String>,
    signature_algorithm: String,
    chain_id: Option<u64>,
}

#[derive(Debug, Clone)]
struct NormalizedEnvelopeResult {
    transaction: Transaction,
    warnings: Vec<String>,
    chain_id: Option<u64>,
}

// Global shared blockchain instance - will be used by both RPC and consensus
lazy_static! {
    pub static ref SHARED_CHAIN: Arc<Mutex<BlockChain>> = {
        let chain_path = crate::utils::resolve_data_path("data/chain.json");
        let canonical_genesis = canonical_genesis()
            .unwrap_or_else(|error| panic!("failed to load canonical genesis: {error}"));
        Arc::new(Mutex::new(
            match BlockChain::load_from_file(chain_path.to_str().unwrap_or("data/chain.json")) {
                Some(chain) => {
                    chain
                        .ensure_expected_genesis_hash(canonical_genesis.hash())
                        .unwrap_or_else(|error| {
                            panic!(
                                "existing chain state at {} does not match canonical genesis {}: {}",
                                chain_path.display(),
                                canonical_genesis.hash(),
                                error
                            )
                        });
                    chain
                }
                None => {
                    let mut chain = BlockChain::new();
                    chain.genesis().unwrap_or_else(|error| {
                        panic!("failed to bootstrap genesis block: {error}")
                    });
                    chain.save_to_file(chain_path.to_str().unwrap_or("data/chain.json"));
                    chain
                }
            },
        ))
    };
}

lazy_static! {
    pub static ref SYNC_MANAGER: Arc<Mutex<SyncManager>> =
        Arc::new(Mutex::new(SyncManager::new(Arc::clone(&SHARED_CHAIN))));
}

// For backward compatibility
pub use self::SHARED_CHAIN as CHAIN;

// Temporarily disabled for quick compile
// lazy_static! {
//     pub static ref AIVM_RUNTIME: Arc<AIVMRuntime> = Arc::new(AIVMRuntime::new());
// }

pub fn start_rpc_server(
    bind_address: &str,
    ws_bind_address: Option<String>,
    cors_enabled: bool,
    cors_origins: Vec<String>,
) {
    println!("📡 RPC server running on {}", bind_address);
    {
        let mut start_time = NODE_START_TIME.lock().unwrap();
        if start_time.is_none() {
            *start_time = Some(current_timestamp());
        }
    }

    // Load validator registry from disk if it exists
    let validator_registry_path = "data/validator_registry.json";
    if let Err(e) = VALIDATOR_MANAGER.load_registry(validator_registry_path) {
        println!("ℹ️ No validator registry found at startup: {}", e);
    } else {
        let validators = VALIDATOR_MANAGER.get_active_validators();
        println!(
            "✅ Loaded {} validators from registry at startup",
            validators.len()
        );
    }

    if let Some(ws_bind_address) = ws_bind_address {
        let tx_pool = Arc::clone(&TX_POOL);
        let chain = Arc::clone(&CHAIN);
        let validator_manager = Arc::clone(&VALIDATOR_MANAGER);
        thread::spawn(move || {
            start_ws_rpc_server(&ws_bind_address, &tx_pool, &chain, &validator_manager);
        });
    }

    for stream in TcpListener::bind(bind_address)
        .expect("Failed to bind RPC server")
        .incoming()
    {
        let tx_pool = Arc::clone(&TX_POOL);
        let chain = Arc::clone(&CHAIN);
        let validator_manager = Arc::clone(&VALIDATOR_MANAGER);
        let cors_enabled_for_conn = cors_enabled;
        let cors_origins_for_conn = cors_origins.clone();
        thread::spawn(move || {
            if let Ok(mut stream) = stream {
                let mut buffer = [0; 16384];
                if let Ok(bytes_read) = stream.read(&mut buffer) {
                    let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);
                    let request_line = request_str.lines().next().unwrap_or_default();
                    let mut request_line_parts = request_line.split_whitespace();
                    let http_method = request_line_parts.next().unwrap_or_default();
                    let request_path = request_line_parts.next().unwrap_or("/");

                    // Handle CORS preflight
                    if http_method == "OPTIONS" {
                        let response_str = format_cors_preflight_response(
                            cors_enabled_for_conn,
                            &cors_origins_for_conn,
                        );
                        let _ = stream.write(response_str.as_bytes());
                        let _ = stream.flush();
                        return;
                    }

                    if http_method == "GET" {
                        let response_str = match request_path {
                            "/" | "/healthz" => format_text_response(
                                "ok\n",
                                cors_enabled_for_conn,
                                &cors_origins_for_conn,
                            ),
                            "/readyz" => format_text_response(
                                "ready\n",
                                cors_enabled_for_conn,
                                &cors_origins_for_conn,
                            ),
                            _ => format_not_found_response(
                                cors_enabled_for_conn,
                                &cors_origins_for_conn,
                            ),
                        };
                        let _ = stream.write(response_str.as_bytes());
                        let _ = stream.flush();
                        return;
                    }

                    // Split headers and body
                    let parts: Vec<&str> = request_str.splitn(2, "\r\n\r\n").collect();
                    if parts.len() < 2 {
                        send_json_rpc_error(
                            &mut stream,
                            None,
                            &RpcError::new(-32700, "Malformed HTTP request"),
                            cors_enabled_for_conn,
                            &cors_origins_for_conn,
                        );
                        return;
                    }

                    let headers = parse_http_headers(parts[0]);
                    let request_context = RpcRequestContext::new(
                        RpcTransport::Http,
                        stream.peer_addr().ok(),
                        headers.clone(),
                    );
                    let body = parts[1];

                    if http_method == "POST" {
                        if !request_is_json(&headers) {
                            send_json_rpc_error(
                                &mut stream,
                                None,
                                &RpcError::new(-32700, "Content-Type must be application/json"),
                                cors_enabled_for_conn,
                                &cors_origins_for_conn,
                            );
                            return;
                        }

                        match serde_json::from_str::<Value>(body) {
                            Ok(parsed) => match process_json_rpc_payload(
                                &parsed,
                                &tx_pool,
                                &chain,
                                &validator_manager,
                                None,
                                &request_context,
                            ) {
                                Ok(Some(response)) => {
                                    let response_str = format_response(
                                        &response.to_string(),
                                        cors_enabled_for_conn,
                                        &cors_origins_for_conn,
                                    );
                                    let _ = stream.write(response_str.as_bytes());
                                    let _ = stream.flush();
                                }
                                Ok(None) => {
                                    let response_str = format_http_response(
                                        "204 No Content",
                                        "application/json",
                                        "",
                                        cors_enabled_for_conn,
                                        &cors_origins_for_conn,
                                    );
                                    let _ = stream.write(response_str.as_bytes());
                                    let _ = stream.flush();
                                }
                                Err(error) => send_json_rpc_error(
                                    &mut stream,
                                    None,
                                    &error,
                                    cors_enabled_for_conn,
                                    &cors_origins_for_conn,
                                ),
                            },
                            Err(_) => send_json_rpc_error(
                                &mut stream,
                                None,
                                &RpcError::new(-32700, "Malformed JSON in body"),
                                cors_enabled_for_conn,
                                &cors_origins_for_conn,
                            ),
                        }
                    } else {
                        send_json_rpc_error(
                            &mut stream,
                            None,
                            &RpcError::new(-32600, "Unsupported HTTP method"),
                            cors_enabled_for_conn,
                            &cors_origins_for_conn,
                        );
                    }
                }
            }
        });
    }
}

pub fn transaction_hashes(transactions: &[Transaction]) -> HashSet<String> {
    transactions
        .iter()
        .map(|transaction| transaction.hash())
        .collect()
}

pub fn prune_transaction_hashes_from_pool(confirmed_hashes: &HashSet<String>) -> usize {
    if confirmed_hashes.is_empty() {
        return 0;
    }

    let mut pool = TX_POOL.lock().unwrap();
    let before = pool.len();
    pool.retain(|transaction| !confirmed_hashes.contains(&transaction.hash()));
    before.saturating_sub(pool.len())
}

fn default_cluster_id(index: usize) -> Option<u64> {
    Some((index / TESTNET_BETA_VALIDATOR_CLUSTER_SIZE.max(1)) as u64)
}

fn synthesize_validator(
    address: String,
    public_key: String,
    name: String,
    stake_amount: u64,
    registered_at: u64,
) -> Validator {
    Validator {
        address,
        public_key,
        name,
        website: None,
        description: None,
        email: None,
        registered_at,
        last_active: 0,
        total_blocks_produced: 0,
        total_transactions_validated: 0,
        uptime_percentage: 0.0,
        average_block_time: 0.0,
        missed_blocks: 0,
        double_signs: 0,
        consecutive_missed_votes: 0,
        missed_vote_window: 0,
        last_vote_timestamp: 0,
        equivocation_evidence_count: 0,
        synergy_score: 0.0,
        task_accuracy: 0.0,
        collaboration_score: 0.0,
        reputation_score: 0.0,
        slashing_penalty: 0.0,
        stake_amount,
        min_stake_required: stake_amount.max(1),
        cluster_id: None,
        cluster_address: None,
        status: ValidatorStatus::Inactive,
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

fn assign_cluster_addresses(validators: &mut [Validator]) {
    let mut members_by_cluster = BTreeMap::<u64, Vec<String>>::new();
    let mut existing_by_cluster = HashMap::<u64, String>::new();
    for validator in validators.iter() {
        if let Some(cluster_id) = validator.cluster_id {
            members_by_cluster
                .entry(cluster_id)
                .or_default()
                .push(validator.address.clone());
            if let Some(cluster_address) = validator.cluster_address.as_deref() {
                if cluster_address.starts_with("syngrp") {
                    existing_by_cluster
                        .entry(cluster_id)
                        .or_insert_with(|| cluster_address.to_string());
                }
            }
        }
    }

    let cluster_addresses = members_by_cluster
        .into_iter()
        .map(|(cluster_id, mut members)| {
            let cluster_address = existing_by_cluster
                .get(&cluster_id)
                .cloned()
                .unwrap_or_else(|| {
                    members.sort();
                    let group = ((cluster_id % 5) + 1) as u8;
                    let seed = format!("cluster-{}-{}", cluster_id, members.join("-"));
                    generate_cluster_address(&seed, group)
                });
            (cluster_id, cluster_address)
        })
        .collect::<HashMap<_, _>>();

    for validator in validators.iter_mut() {
        validator.cluster_address = validator
            .cluster_id
            .and_then(|cluster_id| cluster_addresses.get(&cluster_id).cloned());
    }
}

fn recent_active_validator_addresses(
    chain: &BlockChain,
    total_known_validators: usize,
) -> HashSet<String> {
    let window = total_known_validators.max(1).saturating_mul(4);
    chain
        .chain
        .iter()
        .rev()
        .filter(|block| block.block_index > 0 && block.validator_id != "genesis")
        .take(window)
        .map(|block| block.validator_id.clone())
        .collect()
}

fn network_validator_snapshot(
    chain: &BlockChain,
    validator_manager: &ValidatorManager,
) -> Vec<Validator> {
    let mut validators = validator_manager
        .get_all_validators()
        .into_iter()
        .map(|validator| (validator.address.clone(), validator))
        .collect::<HashMap<_, _>>();
    let genesis_timestamp = canonical_genesis()
        .map(|genesis| genesis.timestamp())
        .unwrap_or(0);

    if let Ok(genesis) = canonical_genesis() {
        for (index, entry) in genesis.validators().iter().enumerate() {
            let address = entry.operator_address.clone();
            let validator = validators.entry(address.clone()).or_insert_with(|| {
                synthesize_validator(
                    address.clone(),
                    entry.consensus_public_key.clone(),
                    entry.moniker.clone(),
                    entry.stake_nwei,
                    genesis.timestamp(),
                )
            });
            if validator.public_key.is_empty() {
                validator.public_key = entry.consensus_public_key.clone();
            }
            if validator.name.trim().is_empty() {
                validator.name = entry.moniker.clone();
            }
            if validator.stake_amount == 0 {
                validator.stake_amount = entry.stake_nwei;
            }
            if validator.min_stake_required == 0 {
                validator.min_stake_required = entry.stake_nwei.max(1);
            }
            if validator.cluster_id.is_none() {
                validator.cluster_id = default_cluster_id(index);
            }
            if validator.registered_at == 0 {
                validator.registered_at = genesis.timestamp();
            }
        }
    }

    let total_observed_blocks = chain
        .chain
        .iter()
        .filter(|block| block.block_index > 0)
        .count() as u64;
    let recent_active = recent_active_validator_addresses(chain, validators.len());

    for block in chain.chain.iter().filter(|block| block.block_index > 0) {
        let address = block.validator_id.clone();
        let validator = validators.entry(address.clone()).or_insert_with(|| {
            synthesize_validator(
                address.clone(),
                String::new(),
                format!("Validator-{}", &address[..8.min(address.len())]),
                0,
                genesis_timestamp,
            )
        });
        validator.total_blocks_produced = validator.total_blocks_produced.saturating_add(1);
        validator.total_transactions_validated = validator
            .total_transactions_validated
            .saturating_add(block.transactions.len() as u64);
        validator.last_active = validator.last_active.max(block.timestamp);
        validator.last_vote_timestamp = validator.last_vote_timestamp.max(block.timestamp);
    }

    let mut ordered = validators.into_values().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.address.cmp(&right.address));
    for (index, validator) in ordered.iter_mut().enumerate() {
        let is_recently_active = recent_active.contains(&validator.address);
        let has_observed_activity = validator.total_blocks_produced > 0;
        if validator.cluster_id.is_none() {
            validator.cluster_id = default_cluster_id(index);
        }
        if validator.min_stake_required == 0 {
            validator.min_stake_required = validator.stake_amount.max(1);
        }
        validator.average_block_time = calculate_average_block_time(chain);
        validator.uptime_percentage = if total_observed_blocks > 0 {
            (validator.total_blocks_produced as f64 / total_observed_blocks as f64) * 100.0
        } else if is_recently_active {
            100.0
        } else {
            0.0
        };
        if matches!(
            validator.status,
            ValidatorStatus::Active | ValidatorStatus::Pending
        ) || is_recently_active
            || has_observed_activity
        {
            validator.status = ValidatorStatus::Active;
        } else if !matches!(
            validator.status,
            ValidatorStatus::Jailed | ValidatorStatus::Slashed
        ) {
            validator.status = ValidatorStatus::Inactive;
        }
        if validator.synergy_score <= 0.0 && matches!(validator.status, ValidatorStatus::Active) {
            validator.synergy_score = INITIAL_VALIDATOR_SYNERGY_SCORE;
        }
        if validator.task_accuracy <= 0.0 && matches!(validator.status, ValidatorStatus::Active) {
            validator.task_accuracy = 100.0;
        }
        if validator.reputation_score <= 0.0 && matches!(validator.status, ValidatorStatus::Active)
        {
            validator.reputation_score = 100.0;
        }
    }
    assign_cluster_addresses(&mut ordered);

    ordered
}

fn start_ws_rpc_server(
    bind_address: &str,
    tx_pool: &Arc<Mutex<Vec<Transaction>>>,
    chain: &Arc<Mutex<BlockChain>>,
    validator_manager: &Arc<ValidatorManager>,
) {
    println!("📡 RPC WebSocket server running on {}", bind_address);

    for stream in TcpListener::bind(bind_address)
        .expect("Failed to bind RPC WebSocket server")
        .incoming()
    {
        let tx_pool = Arc::clone(tx_pool);
        let chain = Arc::clone(chain);
        let validator_manager = Arc::clone(validator_manager);
        thread::spawn(move || {
            if let Ok(stream) = stream {
                handle_ws_connection(stream, &tx_pool, &chain, &validator_manager);
            }
        });
    }
}

fn handle_ws_connection(
    stream: std::net::TcpStream,
    tx_pool: &Arc<Mutex<Vec<Transaction>>>,
    chain: &Arc<Mutex<BlockChain>>,
    validator_manager: &Arc<ValidatorManager>,
) {
    let peer_addr = stream.peer_addr().ok();
    let captured_headers: Arc<Mutex<HashMap<String, String>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let header_sink = Arc::clone(&captured_headers);
    let mut websocket = match accept_hdr(stream, |request: &WsRequest, response: WsResponse| {
        if let Ok(mut headers) = header_sink.lock() {
            headers.clear();
            for (name, value) in request.headers() {
                headers.insert(
                    name.as_str().to_ascii_lowercase(),
                    value.to_str().unwrap_or_default().to_string(),
                );
            }
        }
        Ok(response)
    }) {
        Ok(websocket) => websocket,
        Err(error) => {
            eprintln!("WebSocket handshake failed: {}", error);
            return;
        }
    };

    let _ = websocket
        .get_ref()
        .set_read_timeout(Some(Duration::from_millis(250)));

    let mut subscriptions: HashMap<String, SubscriptionCursor> = HashMap::new();
    let request_context = RpcRequestContext::new(
        RpcTransport::WebSocket,
        peer_addr,
        captured_headers
            .lock()
            .map(|headers| headers.clone())
            .unwrap_or_default(),
    );

    loop {
        emit_subscription_notifications(&mut websocket, &mut subscriptions, tx_pool, chain);

        match websocket.read() {
            Ok(WsMessage::Text(body)) => {
                match serde_json::from_str::<Value>(&body)
                    .map_err(|_| RpcError::new(-32700, "Malformed JSON in WebSocket payload"))
                {
                    Ok(parsed) => match process_json_rpc_payload(
                        &parsed,
                        tx_pool,
                        chain,
                        validator_manager,
                        Some(&mut subscriptions),
                        &request_context,
                    ) {
                        Ok(Some(response)) => {
                            if websocket
                                .send(WsMessage::Text(response.to_string()))
                                .is_err()
                            {
                                break;
                            }
                        }
                        Ok(None) => {}
                        Err(error) => {
                            let response = json_rpc_error_response(None, &error);
                            if websocket
                                .send(WsMessage::Text(response.to_string()))
                                .is_err()
                            {
                                break;
                            }
                        }
                    },
                    Err(error) => {
                        let response = json_rpc_error_response(None, &error);
                        if websocket
                            .send(WsMessage::Text(response.to_string()))
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
            Ok(WsMessage::Ping(payload)) => {
                if websocket.send(WsMessage::Pong(payload)).is_err() {
                    break;
                }
            }
            Ok(WsMessage::Close(_)) => break,
            Ok(_) => {}
            Err(WsError::ConnectionClosed) | Err(WsError::AlreadyClosed) => break,
            Err(WsError::Io(error))
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(error) => {
                eprintln!("WebSocket RPC error: {}", error);
                break;
            }
        }
    }
}

fn process_json_rpc_payload(
    parsed: &Value,
    tx_pool: &Arc<Mutex<Vec<Transaction>>>,
    chain: &Arc<Mutex<BlockChain>>,
    validator_manager: &Arc<ValidatorManager>,
    subscriptions: Option<&mut HashMap<String, SubscriptionCursor>>,
    request_context: &RpcRequestContext,
) -> Result<Option<Value>, RpcError> {
    match parsed {
        Value::Array(items) => {
            if items.is_empty() {
                return Ok(Some(json_rpc_error_response(
                    None,
                    &RpcError::new(-32600, "Invalid request"),
                )));
            }

            let mut responses = Vec::new();
            let mut subscriptions = subscriptions;
            for item in items {
                if let Some(response) = process_json_rpc_request_object(
                    item,
                    tx_pool,
                    chain,
                    validator_manager,
                    subscriptions.as_deref_mut(),
                    request_context,
                )? {
                    responses.push(response);
                }
            }

            if responses.is_empty() {
                Ok(None)
            } else {
                Ok(Some(Value::Array(responses)))
            }
        }
        Value::Object(_) => process_json_rpc_request_object(
            parsed,
            tx_pool,
            chain,
            validator_manager,
            subscriptions,
            request_context,
        )
        .map(|response| response.map(|value| json!(value))),
        _ => Ok(Some(json_rpc_error_response(
            None,
            &RpcError::new(-32600, "Invalid request"),
        ))),
    }
}

fn process_json_rpc_request_object(
    request: &Value,
    tx_pool: &Arc<Mutex<Vec<Transaction>>>,
    chain: &Arc<Mutex<BlockChain>>,
    validator_manager: &Arc<ValidatorManager>,
    subscriptions: Option<&mut HashMap<String, SubscriptionCursor>>,
    request_context: &RpcRequestContext,
) -> Result<Option<Value>, RpcError> {
    let request_object = request
        .as_object()
        .ok_or_else(|| RpcError::new(-32600, "Invalid request"))?;

    let method = request_object
        .get("method")
        .and_then(|value| value.as_str())
        .ok_or_else(|| RpcError::new(-32600, "Missing method"))?;
    let params = request_object
        .get("params")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let id = request_object.get("id").cloned();

    enforce_rpc_exposure_policy(method, request_context)?;

    if id.is_none() && method != "synergy_subscribe" && method != "synergy_unsubscribe" {
        let _ = execute_rpc_method(
            method,
            params,
            tx_pool,
            chain,
            validator_manager,
            subscriptions,
            request_context,
        )?;
        return Ok(None);
    }

    let result = execute_rpc_method(
        method,
        params,
        tx_pool,
        chain,
        validator_manager,
        subscriptions,
        request_context,
    );

    match result {
        Ok(value) => Ok(Some(json!({
            "jsonrpc": "2.0",
            "id": id.clone().unwrap_or(Value::Null),
            "result": value
        }))),
        Err(error) => Ok(Some(json_rpc_error_response(id, &error))),
    }
}

fn execute_rpc_method(
    method: &str,
    params: Value,
    tx_pool: &Arc<Mutex<Vec<Transaction>>>,
    chain: &Arc<Mutex<BlockChain>>,
    validator_manager: &Arc<ValidatorManager>,
    subscriptions: Option<&mut HashMap<String, SubscriptionCursor>>,
    _request_context: &RpcRequestContext,
) -> Result<Value, RpcError> {
    match method {
        "synergy_simulateTransaction" => simulate_transaction(&params, tx_pool, chain),
        "synergy_getAccountNonce" | "synergy_getAccountAuthNonce" => {
            get_account_nonce(&params, tx_pool, chain)
        }
        "synergy_subscribe" => {
            let subscriptions = subscriptions
                .ok_or_else(|| RpcError::new(-32601, "synergy_subscribe is WebSocket-only"))?;
            register_subscription(&params, chain, tx_pool, subscriptions)
        }
        "synergy_unsubscribe" => {
            let subscriptions = subscriptions
                .ok_or_else(|| RpcError::new(-32601, "synergy_unsubscribe is WebSocket-only"))?;
            unregister_subscription(&params, subscriptions)
        }
        _ => translate_legacy_rpc_result(handle_json_rpc(
            method,
            params,
            tx_pool,
            chain,
            validator_manager,
        )),
    }
}

fn handle_json_rpc(
    method: &str,
    params: Value,
    tx_pool: &Arc<Mutex<Vec<Transaction>>>,
    chain: &Arc<Mutex<BlockChain>>,
    validator_manager: &Arc<ValidatorManager>,
    // Temporarily disabled AIVM for quick compile
    // aivm_runtime: &Arc<AIVMRuntime>,
) -> Value {
    match method {
        // Blockchain queries
        "synergy_blockNumber" => {
            let chain = chain.lock().unwrap();
            json!(chain.last().map_or(0, |b| b.block_index))
        }

        "synergy_getBlockNumber" => {
            let chain = chain.lock().unwrap();
            json!(chain.last().map_or(0, |b| b.block_index))
        }

        "synergy_getBlockByNumber" => {
            if let Some(block_num) = params.get(0).and_then(|v| v.as_u64()) {
                let chain = chain.lock().unwrap();
                if let Some(block) = chain.chain.iter().find(|b| b.block_index == block_num) {
                    block_to_explorer_json(block)
                } else {
                    json!(null)
                }
            } else {
                json!("Invalid block number")
            }
        }

        "synergy_getBlockByHash" => {
            if let Some(block_hash) = params.get(0).and_then(|v| v.as_str()) {
                let normalized = block_hash.trim().to_lowercase();
                let chain = chain.lock().unwrap();
                if let Some(block) = chain
                    .chain
                    .iter()
                    .find(|b| b.hash.trim().eq_ignore_ascii_case(&normalized))
                {
                    block_to_explorer_json(block)
                } else {
                    json!(null)
                }
            } else {
                json!("Invalid block hash")
            }
        }

        "synergy_getLatestBlock" => {
            let chain = chain.lock().unwrap();
            if let Some(block) = chain.last() {
                block_to_explorer_json(block)
            } else {
                json!(null)
            }
        }

        // Transaction methods
        "synergy_sendTransaction" => {
            if let Some(tx_data) = params.get(0) {
                match normalize_rpc_transaction(tx_data, true) {
                    Ok(normalized) => {
                        let configured_chain_id = current_chain_id();
                        if let Some(chain_id) = normalized.chain_id {
                            if chain_id != configured_chain_id {
                                return json!({
                                    "error": format!("Transaction chainId {} does not match local chain {}", chain_id, configured_chain_id)
                                });
                            }
                        }

                        if let Some(simulation_hash) =
                            params.get(2).and_then(|value| value.as_str())
                        {
                            let tx_digest = canonical_value_digest(tx_data)
                                .unwrap_or_else(|| normalized.transaction.hash());
                            let cache = SIMULATION_CACHE.lock().unwrap();
                            match cache.get(&tx_digest) {
                                Some(cached) if cached.simulation_hash == simulation_hash => {}
                                Some(_) => {
                                    return json!({"error": "simulationHash does not match the current transaction envelope"});
                                }
                                None => {
                                    return json!({"error": "simulationHash is unknown or expired"});
                                }
                            }
                        }

                        match normalized.transaction.validate() {
                            crate::transaction::TransactionValidationResult {
                                is_valid: true,
                                ..
                            } => {
                                let mut pool = tx_pool.lock().unwrap();
                                let tx_hash = normalized.transaction.hash();
                                pool.push(normalized.transaction.clone());

                                if let Some(p2p) = crate::p2p::get_p2p_network() {
                                    p2p.broadcast_transaction(&normalized.transaction);
                                }

                                json!({
                                    "success": true,
                                    "tx_hash": tx_hash,
                                    "mempool_status": "queued",
                                    "policy_warnings": normalized.warnings,
                                    "message": "Transaction submitted"
                                })
                            }
                            crate::transaction::TransactionValidationResult {
                                error_message: Some(msg),
                                ..
                            } => json!({"error": msg}),
                            _ => json!("Invalid transaction"),
                        }
                    }
                    Err(error) => {
                        json!({"error": error.message, "code": error.code, "data": error.data})
                    }
                }
            } else {
                json!("Missing transaction data")
            }
        }

        "synergy_getTransactionPool" => {
            let pool = tx_pool.lock().unwrap();
            let txs: Vec<Value> = pool
                .iter()
                .map(|tx| tx_to_explorer_json(tx, "pending", None, None))
                .collect();
            json!(txs)
        }

        // ---------------------------------------------------------------------
        // SXCP (Synergy Cross-Chain Protocol) – Testnet-Beta RPC surface
        // ---------------------------------------------------------------------
        "synergy_registerRelayer" => {
            if let (Some(address), Some(public_key)) = (
                params.get(0).and_then(|v| v.as_str()),
                params.get(1).and_then(|v| v.as_str()),
            ) {
                sxcp::register_relayer(address, public_key)
            } else {
                json!({"success": false, "error": "Missing required parameters: address, public_key"})
            }
        }

        "synergy_unregisterRelayer" => {
            if let Some(address) = params.get(0).and_then(|v| v.as_str()) {
                sxcp::unregister_relayer(address)
            } else {
                json!({"success": false, "error": "Missing required parameter: address"})
            }
        }

        "synergy_relayerHeartbeat" => {
            if let Some(address) = params.get(0).and_then(|v| v.as_str()) {
                sxcp::heartbeat_relayer(address)
            } else {
                json!({"success": false, "error": "Missing required parameter: address"})
            }
        }

        "synergy_getRelayerSet" => sxcp::get_relayer_set(),

        "synergy_getRelayerHealth" => sxcp::get_relayer_health(),

        "synergy_getSxcpStatus" => sxcp::get_sxcp_status(),

        "synergy_submitAttestation" => {
            if let (Some(submitted_by), Some(event_hash), Some(aggregate_sig)) = (
                params.get(0).and_then(|v| v.as_str()),
                params.get(1).and_then(|v| v.as_str()),
                params.get(2).and_then(|v| v.as_str()),
            ) {
                let metadata = params.get(3).cloned().unwrap_or(json!({}));
                sxcp::submit_attestation(submitted_by, event_hash, aggregate_sig, metadata)
            } else {
                json!({"success": false, "error": "Missing required parameters: submitted_by, event_hash, aggregate_sig"})
            }
        }

        "synergy_getEventAttestation" => {
            if let Some(event_hash) = params.get(0).and_then(|v| v.as_str()) {
                sxcp::get_event_attestation(event_hash)
            } else {
                json!({"success": false, "error": "Missing required parameter: event_hash"})
            }
        }

        "synergy_getAttestations" => {
            let limit = params.get(0).and_then(|v| v.as_u64()).map(|v| v as usize);
            sxcp::get_attestations(limit)
        }

        "synergy_slashRelayer" => {
            if let (Some(address), Some(reason)) = (
                params.get(0).and_then(|v| v.as_str()),
                params.get(1).and_then(|v| v.as_str()),
            ) {
                let penalty = params.get(2).and_then(|v| v.as_i64());
                sxcp::slash_relayer(address, reason, penalty)
            } else {
                json!({"success": false, "error": "Missing required parameters: address, reason"})
            }
        }

        "synergy_setSxcpHeartbeatTimeout" => {
            if let Some(timeout_secs) = params.get(0).and_then(|v| v.as_u64()) {
                sxcp::set_heartbeat_timeout(timeout_secs)
            } else {
                json!({"success": false, "error": "Missing required parameter: timeout_secs"})
            }
        }

        "synergy_resetSxcpState" => {
            if params
                .get(0)
                .and_then(|v| v.as_str())
                .map(|token| token == "TESTBETA_RESET_SXCP_STATE")
                .unwrap_or(false)
            {
                sxcp::reset_state()
            } else {
                json!({
                    "success": false,
                    "error": "Confirmation token required as first parameter: TESTBETA_RESET_SXCP_STATE"
                })
            }
        }

        // Node status
        "synergy_nodeInfo" => {
            let current_block = {
                let chain = chain.lock().unwrap();
                chain.last().map_or(0, |b| b.block_index)
            };
            let config = crate::config::load_node_config(None).ok();
            let node_name = config
                .as_ref()
                .map(|cfg| cfg.p2p.node_name.clone())
                .filter(|name| !name.is_empty())
                .or_else(|| config.as_ref().map(|cfg| cfg.network.name.clone()));
            let network_id = config.as_ref().map(|cfg| cfg.network.id);
            let chain_id = config.as_ref().map(|cfg| cfg.blockchain.chain_id);
            let consensus = config.as_ref().map(|cfg| cfg.consensus.algorithm.clone());
            let syncing = SYNC_MANAGER
                .lock()
                .ok()
                .map(|manager| !matches!(manager.get_state(), SyncState::Synced | SyncState::Idle));
            json!({
                "name": node_name,
                "version": env!("CARGO_PKG_VERSION"),
                "protocolVersion": null,
                "networkId": network_id,
                "chainId": chain_id,
                "consensus": consensus,
                "syncing": syncing,
                "currentBlock": current_block,
                "timestamp": current_timestamp()
            })
        }

        "synergy_getDeterminismDigest" => {
            let chain = chain.lock().unwrap();
            let latest_block = chain.last().cloned();
            let latest_height = latest_block.as_ref().map(|b| b.block_index).unwrap_or(0);
            let latest_hash = latest_block
                .as_ref()
                .map(|b| b.hash.clone())
                .unwrap_or_default();

            let token_state_hash = stable_json_file_digest("data/token_state.json");
            let validator_registry_hash = stable_json_file_digest("data/validator_registry.json");
            let chain_state_hash =
                canonical_value_digest(&serde_json::to_value(&chain.chain).unwrap_or(json!([])));
            let receipt_hash = compute_receipt_hash(&chain);

            let mut state_hasher = blake3::Hasher::new();
            state_hasher.update(latest_hash.as_bytes());
            if let Some(hash) = token_state_hash.as_ref() {
                state_hasher.update(hash.as_bytes());
            }
            if let Some(hash) = validator_registry_hash.as_ref() {
                state_hasher.update(hash.as_bytes());
            }
            if let Some(hash) = chain_state_hash.as_ref() {
                state_hasher.update(hash.as_bytes());
            }
            let state_root = hex::encode(state_hasher.finalize().as_bytes());

            json!({
                "block_height": latest_height,
                "block_hash": latest_hash,
                "state_root": state_root,
                "receipt_hash": receipt_hash,
                "token_state_hash": token_state_hash,
                "validator_registry_hash": validator_registry_hash,
                "chain_state_hash": chain_state_hash
            })
        }

        // Validator management
        "synergy_getValidators" => {
            let chain = chain.lock().unwrap();
            let validators = network_validator_snapshot(&chain, &validator_manager)
                .into_iter()
                .filter(|validator| validator.status == ValidatorStatus::Active)
                .collect::<Vec<_>>();
            println!(
                "🔍 [RPC] synergy_getValidators called, returning {} validators",
                validators.len()
            );
            json!(validators)
        }

        "synergy_getValidator" => {
            if let Some(address) = params.get(0).and_then(|v| v.as_str()) {
                let chain = chain.lock().unwrap();
                match network_validator_snapshot(&chain, &validator_manager)
                    .into_iter()
                    .find(|validator| validator.address.eq_ignore_ascii_case(address))
                {
                    Some(validator) => json!(validator),
                    None => json!(null),
                }
            } else {
                json!("Missing validator address")
            }
        }

        // Token methods
        "synergy_getTokenBalance" => {
            if let (Some(address), Some(token)) = (
                params.get(0).and_then(|v| v.as_str()),
                params.get(1).and_then(|v| v.as_str()),
            ) {
                let token_manager = TOKEN_MANAGER.clone();
                json!(token_manager.get_balance(address, token))
            } else {
                json!("Missing address or token symbol")
            }
        }

        "synergy_getTokens" => {
            let token_manager = TOKEN_MANAGER.clone();
            json!(token_manager.get_all_tokens())
        }

        "synergy_createWallet" => {
            if let Ok(mut wallet_manager) = WALLET_MANAGER.lock() {
                let address = wallet_manager.create_wallet();
                json!({"address": address, "message": "Wallet created successfully"})
            } else {
                json!({"error": "Failed to create wallet"})
            }
        }

        "synergy_getWallet" => {
            if let Some(address) = params.get(0).and_then(|v| v.as_str()) {
                if let Ok(wallet_manager) = WALLET_MANAGER.lock() {
                    match wallet_manager.get_wallet(address) {
                        Some(wallet) => json!(wallet),
                        None => json!(null),
                    }
                } else {
                    json!({"error": "Failed to access wallet"})
                }
            } else {
                json!("Missing address")
            }
        }

        "synergy_createWalletFromKeypair" => {
            if let (Some(public_key), Some(private_key)) = (
                params.get(0).and_then(|v| v.as_str()),
                params.get(1).and_then(|v| v.as_str()),
            ) {
                if let Ok(mut wallet_manager) = WALLET_MANAGER.lock() {
                    let address = wallet_manager.create_wallet_from_keypair(
                        public_key.to_string(),
                        private_key.to_string(),
                    );
                    json!({"success": true, "address": address, "message": "Wallet created successfully"})
                } else {
                    json!({"success": false, "error": "Failed to access wallet manager"})
                }
            } else {
                json!({"success": false, "error": "Missing required parameters: public_key, private_key"})
            }
        }

        "synergy_getAllWallets" => {
            if let Ok(wallet_manager) = WALLET_MANAGER.lock() {
                json!(wallet_manager.get_all_wallets())
            } else {
                json!({"error": "Failed to access wallet manager"})
            }
        }

        "synergy_signTransaction" => {
            if let (Some(address), Some(tx_data)) =
                (params.get(0).and_then(|v| v.as_str()), params.get(1))
            {
                if let Ok(mut transaction) = serde_json::from_value::<Transaction>(tx_data.clone())
                {
                    if let Ok(wallet_manager) = WALLET_MANAGER.lock() {
                        match wallet_manager.sign_transaction(address, &mut transaction) {
                            Ok(result) => {
                                json!({"success": true, "message": result, "transaction": transaction})
                            }
                            Err(error) => json!({"success": false, "error": error}),
                        }
                    } else {
                        json!({"success": false, "error": "Failed to access wallet manager"})
                    }
                } else {
                    json!({"success": false, "error": "Invalid transaction format"})
                }
            } else {
                json!({"success": false, "error": "Missing required parameters: address, transaction"})
            }
        }

        "synergy_sendTokens" => {
            if let (Some(from), Some(to), Some(token_symbol), Some(amount)) = (
                params.get(0).and_then(|v| v.as_str()),
                params.get(1).and_then(|v| v.as_str()),
                params.get(2).and_then(|v| v.as_str()),
                params.get(3).and_then(|v| v.as_u64()),
            ) {
                let memo = params.get(4).and_then(|v| v.as_str());
                // Convert SNRG amount to nWei (per SNTS-04: 1 SNRG = 1,000,000,000 nWei)
                // The RPC accepts amounts in SNRG for user-friendliness, but internally stores as nWei
                use crate::gas::constants::NWEI_PER_SNRG;
                let amount_nwei = amount.saturating_mul(NWEI_PER_SNRG as u64);

                if let Ok(mut wallet_manager) = WALLET_MANAGER.lock() {
                    let token_manager = TOKEN_MANAGER.clone();
                    match wallet_manager.send_tokens(
                        from,
                        to,
                        token_symbol,
                        amount_nwei,
                        memo,
                        &token_manager,
                    ) {
                        Ok(transaction) => {
                            let tx_hash = transaction.hash();
                            if let Ok(mut pool) = tx_pool.lock() {
                                pool.push(transaction.clone());
                            }

                            // Best-effort gossip to peers.
                            if let Some(p2p) = crate::p2p::get_p2p_network() {
                                p2p.broadcast_transaction(&transaction);
                            }

                            json!({"success": true, "tx_hash": tx_hash, "transaction": transaction, "message": "Transaction submitted"})
                        }
                        Err(error) => json!({"success": false, "error": error}),
                    }
                } else {
                    json!({"success": false, "error": "Failed to access wallet manager"})
                }
            } else {
                json!({"success": false, "error": "Missing required parameters: from, to, token_symbol, amount"})
            }
        }

        "synergy_stakeTokens" => {
            if let (Some(staker), Some(validator), Some(token_symbol), Some(amount)) = (
                params.get(0).and_then(|v| v.as_str()),
                params.get(1).and_then(|v| v.as_str()),
                params.get(2).and_then(|v| v.as_str()),
                params.get(3).and_then(|v| v.as_u64()),
            ) {
                // Convert SNRG amount to nWei (per SNTS-04: 1 SNRG = 1,000,000,000 nWei)
                use crate::gas::constants::NWEI_PER_SNRG;
                let amount_nwei = amount.saturating_mul(NWEI_PER_SNRG as u64);

                if let Ok(mut wallet_manager) = WALLET_MANAGER.lock() {
                    let token_manager = TOKEN_MANAGER.clone();
                    match wallet_manager.stake_tokens(
                        staker,
                        validator,
                        token_symbol,
                        amount_nwei,
                        &token_manager,
                    ) {
                        Ok(transaction) => {
                            let tx_hash = transaction.hash();
                            if let Ok(mut pool) = tx_pool.lock() {
                                pool.push(transaction.clone());
                            }

                            if let Some(p2p) = crate::p2p::get_p2p_network() {
                                p2p.broadcast_transaction(&transaction);
                            }

                            json!({"success": true, "tx_hash": tx_hash, "transaction": transaction, "message": "Staking transaction submitted"})
                        }
                        Err(error) => json!({"success": false, "error": error}),
                    }
                } else {
                    json!({"success": false, "error": "Failed to access wallet manager"})
                }
            } else {
                json!({"success": false, "error": "Missing required parameters: staker, validator, token_symbol, amount"})
            }
        }

        "synergy_stakeTokensDirect" => {
            if let (Some(staker), Some(validator), Some(token_symbol), Some(amount)) = (
                params.get(0).and_then(|v| v.as_str()),
                params.get(1).and_then(|v| v.as_str()),
                params.get(2).and_then(|v| v.as_str()),
                params.get(3).and_then(|v| v.as_u64()),
            ) {
                // Convert SNRG amount to nWei (per SNTS-04: 1 SNRG = 1,000,000,000 nWei)
                use crate::gas::constants::NWEI_PER_SNRG;
                let amount_nwei = amount.saturating_mul(NWEI_PER_SNRG as u64);

                let token_manager = TOKEN_MANAGER.clone();
                match token_manager.stake_tokens(staker, validator, token_symbol, amount_nwei) {
                    Ok(result) => json!({"success": true, "message": result}),
                    Err(error) => json!({"success": false, "error": error}),
                }
            } else {
                json!({"success": false, "error": "Missing required parameters: staker, validator, token_symbol, amount"})
            }
        }

        "synergy_unstakeTokens" => {
            if let (Some(staker), Some(validator), Some(token_symbol), Some(amount)) = (
                params.get(0).and_then(|v| v.as_str()),
                params.get(1).and_then(|v| v.as_str()),
                params.get(2).and_then(|v| v.as_str()),
                params.get(3).and_then(|v| v.as_u64()),
            ) {
                let token_manager = TOKEN_MANAGER.clone();
                match token_manager.unstake_tokens(staker, validator, token_symbol, amount) {
                    Ok(result) => json!({"success": true, "message": result}),
                    Err(error) => json!({"success": false, "error": error}),
                }
            } else {
                json!({"success": false, "error": "Missing required parameters: staker, validator, token_symbol, amount"})
            }
        }

        "synergy_getStakedBalance" => {
            if let (Some(address), Some(token_symbol)) = (
                params.get(0).and_then(|v| v.as_str()),
                params.get(1).and_then(|v| v.as_str()),
            ) {
                let token_manager = TOKEN_MANAGER.clone();
                json!({"balance": token_manager.get_staked_balance(address, token_symbol)})
            } else {
                json!("Missing address or token_symbol parameter")
            }
        }

        "synergy_getStakingInfo" => {
            if let Some(address) = params.get(0).and_then(|v| v.as_str()) {
                let token_manager = TOKEN_MANAGER.clone();
                json!(token_manager.get_staking_info(address))
            } else {
                json!("Missing address parameter")
            }
        }

        "synergy_registerValidator" => {
            if let (Some(address), Some(public_key), Some(name), Some(stake_amount)) = (
                params.get(0).and_then(|v| v.as_str()),
                params.get(1).and_then(|v| v.as_str()),
                params.get(2).and_then(|v| v.as_str()),
                params.get(3).and_then(|v| v.as_u64()),
            ) {
                let registration = crate::validator::ValidatorRegistration {
                    address: address.to_string(),
                    public_key: public_key.to_string(),
                    name: name.to_string(),
                    stake_amount,
                    submitted_at: current_timestamp(),
                    registration_tx_hash: format!("reg_{}", current_timestamp()),
                };

                match validator_manager.register_validator(registration) {
                    Ok(result) => json!({"success": true, "message": result}),
                    Err(error) => json!({"success": false, "error": error}),
                }
            } else {
                json!({"success": false, "error": "Missing required parameters: address, public_key, name, stake_amount"})
            }
        }

        "synergy_approveValidator" => {
            if let Some(address) = params.get(0).and_then(|v| v.as_str()) {
                match validator_manager.approve_validator(address) {
                    Ok(_) => json!({"success": true, "message": "Validator approved successfully"}),
                    Err(error) => json!({"success": false, "error": error}),
                }
            } else {
                json!("Missing address parameter")
            }
        }

        "synergy_getTopValidators" => {
            let count = params.get(0).and_then(|v| v.as_u64()).unwrap_or(10) as usize;
            let chain = chain.lock().unwrap();
            let mut validators = network_validator_snapshot(&chain, &validator_manager);
            validators.sort_by(|left, right| {
                right
                    .synergy_score
                    .partial_cmp(&left.synergy_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| right.stake_amount.cmp(&left.stake_amount))
                    .then_with(|| left.address.cmp(&right.address))
            });
            json!(validators.into_iter().take(count).collect::<Vec<_>>())
        }

        "synergy_slashValidator" => {
            if let (Some(address), Some(reason)) = (
                params.get(0).and_then(|v| v.as_str()),
                params.get(1).and_then(|v| v.as_str()),
            ) {
                match validator_manager.slash_validator(address, reason) {
                    Ok(_) => json!({"success": true, "message": "Validator slashed successfully"}),
                    Err(error) => json!({"success": false, "error": error}),
                }
            } else {
                json!({"success": false, "error": "Missing required parameters: address, reason"})
            }
        }

        "synergy_getBlockRange" => {
            if let (Some(start), Some(end)) = (
                params.get(0).and_then(|v| v.as_u64()),
                params.get(1).and_then(|v| v.as_u64()),
            ) {
                let chain = chain.lock().unwrap();
                let blocks: Vec<_> = chain
                    .chain
                    .iter()
                    .filter(|block| block.block_index >= start && block.block_index <= end)
                    .map(block_to_explorer_json)
                    .collect();

                json!(blocks)
            } else {
                json!("Missing start or end parameter")
            }
        }

        "synergy_getTransactionByHash" => {
            if let Some(tx_hash) = params.get(0).and_then(|v| v.as_str()) {
                // Normalize hash: handle multiple formats
                // 1. Remove "0x" prefix if present (EVM format)
                // 2. Remove Synergy prefixes (syntxn-, synxxn-) to get raw hash
                // 3. Convert to lowercase for comparison
                let normalized = tx_hash.strip_prefix("0x").unwrap_or(tx_hash).to_lowercase();

                // Extract raw hash (without prefix) for comparison
                let raw_hash_search = if normalized.starts_with("syntxn-") {
                    normalized.strip_prefix("syntxn-").unwrap_or(&normalized)
                } else if normalized.starts_with("synxxn-") {
                    normalized.strip_prefix("synxxn-").unwrap_or(&normalized)
                } else {
                    &normalized // Assume it's already a raw hash
                };

                // Helper function to check if a transaction matches
                let matches_tx = |tx: &Transaction| -> bool {
                    let tx_hash_formatted = tx.hash().to_lowercase();
                    let tx_hash_raw = tx.raw_hash().to_lowercase();

                    // Match against:
                    // 1. Full formatted hash (with prefix)
                    // 2. Raw hash (without prefix)
                    // 3. Normalized input (might have prefix or not)
                    tx_hash_formatted == normalized
                        || tx_hash_raw == normalized
                        || tx_hash_raw == raw_hash_search
                        || (tx_hash_formatted.starts_with("syntxn-")
                            && tx_hash_formatted.strip_prefix("syntxn-").unwrap_or("")
                                == raw_hash_search)
                        || (tx_hash_formatted.starts_with("synxxn-")
                            && tx_hash_formatted.strip_prefix("synxxn-").unwrap_or("")
                                == raw_hash_search)
                };

                // First, search in confirmed transactions (blocks)
                let chain = chain.lock().unwrap();
                for block in &chain.chain {
                    for (idx, tx) in block.transactions.iter().enumerate() {
                        if matches_tx(tx) {
                            return tx_to_explorer_json(
                                tx,
                                "confirmed",
                                Some(block.block_index),
                                Some(idx),
                            );
                        }
                    }
                }

                // If not found in blocks, search in transaction pool (pending transactions)
                let pool = tx_pool.lock().unwrap();
                for tx in pool.iter() {
                    if matches_tx(tx) {
                        return tx_to_explorer_json(tx, "pending", None, None);
                    }
                }

                json!(null)
            } else {
                json!("Missing transaction hash parameter")
            }
        }

        "synergy_getTransactionsInBlock" => {
            if let Some(block_number) = params.get(0).and_then(|v| v.as_u64()) {
                let chain = chain.lock().unwrap();
                if let Some(block) = chain.chain.iter().find(|b| b.block_index == block_number) {
                    let txs: Vec<Value> = block
                        .transactions
                        .iter()
                        .enumerate()
                        .map(|(idx, tx)| {
                            tx_to_explorer_json(tx, "confirmed", Some(block.block_index), Some(idx))
                        })
                        .collect();
                    json!(txs)
                } else {
                    json!([])
                }
            } else {
                json!("Missing block number parameter")
            }
        }

        "synergy_getValidatorStats" => {
            let chain = chain.lock().unwrap();
            let mut validators = network_validator_snapshot(&chain, &validator_manager);
            let active_validators = validators
                .iter()
                .filter(|validator| validator.status == ValidatorStatus::Active)
                .cloned()
                .collect::<Vec<_>>();
            let total_validators = validators.len();
            validators.sort_by(|left, right| {
                right
                    .synergy_score
                    .partial_cmp(&left.synergy_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| right.stake_amount.cmp(&left.stake_amount))
                    .then_with(|| left.address.cmp(&right.address))
            });
            let top_validators = validators.into_iter().take(20).collect::<Vec<_>>();

            json!({
                "total_validators": total_validators,
                "active_validators": active_validators,
                "top_validators": top_validators,
                "epoch_rewards": validator_manager.calculate_epoch_rewards(0)
            })
        }

        "synergy_getTokenStats" => {
            let token_manager = TOKEN_MANAGER.clone();
            let tokens = token_manager.get_all_tokens();

            let mut token_stats = Vec::new();
            for token in tokens {
                let total_staked = token_manager.get_staked_balance("*", &token.symbol);
                let holders = {
                    let balances = token_manager.balances.lock().unwrap();
                    balances
                        .values()
                        .filter(|addr_balances| {
                            addr_balances.get(&token.symbol).copied().unwrap_or(0) > 0
                        })
                        .count()
                };
                token_stats.push(json!({
                    "symbol": token.symbol,
                    "name": token.name,
                    "total_supply": token.total_supply,
                    "total_staked": total_staked,
                    "holders": holders
                }));
            }

            json!(token_stats)
        }

        // TEMPORARILY DISABLED:         // AIVM - Artificial Intelligence Virtual Machine Methods
        // TEMPORARILY DISABLED:         "synergy_deployAIVMContract" => {
        // TEMPORARILY DISABLED:             if let (Some(bytecode), Some(abi), Some(contract_type)) = (
        // TEMPORARILY DISABLED:                 params.get(0).and_then(|v| v.as_str()),
        // TEMPORARILY DISABLED:                 params.get(1).and_then(|v| v.as_str()),
        // TEMPORARILY DISABLED:                 params.get(2).and_then(|v| v.as_str()),
        // TEMPORARILY DISABLED:             ) {
        // TEMPORARILY DISABLED:                 let bytecode_vec = hex::decode(bytecode).unwrap_or_default();
        // TEMPORARILY DISABLED:                 let contract_type_enum = match contract_type {
        // TEMPORARILY DISABLED:                     "ai" => ContractType::AIEnhanced,
        // TEMPORARILY DISABLED:                     "cross_chain" => ContractType::CrossChain,
        // TEMPORARILY DISABLED:                     "oracle" => ContractType::Oracle,
        // TEMPORARILY DISABLED:                     _ => ContractType::Standard,
        // TEMPORARILY DISABLED:                 };
        // TEMPORARILY DISABLED:
        // TEMPORARILY DISABLED:                 match aivm_runtime.deploy_contract(
        // TEMPORARILY DISABLED:                     bytecode_vec,
        // TEMPORARILY DISABLED:                     abi.to_string(),
        // TEMPORARILY DISABLED:                     "system".to_string(),
        // TEMPORARILY DISABLED:                     contract_type_enum,
        // TEMPORARILY DISABLED:                 ) {
        // TEMPORARILY DISABLED:                     Ok(address) => json!({"success": true, "contract_address": address, "message": "AIVM contract deployed successfully"}),
        // TEMPORARILY DISABLED:                     Err(error) => json!({"success": false, "error": error}),
        // TEMPORARILY DISABLED:                 }
        // TEMPORARILY DISABLED:             } else {
        // TEMPORARILY DISABLED:                 json!({"success": false, "error": "Missing required parameters: bytecode, abi, contract_type"})
        // TEMPORARILY DISABLED:             }
        // TEMPORARILY DISABLED:         }
        // TEMPORARILY DISABLED:
        // TEMPORARILY DISABLED:         "synergy_executeAIVMContract" => {
        // TEMPORARILY DISABLED:             if let (Some(contract_address), Some(input_data)) = (
        // TEMPORARILY DISABLED:                 params.get(0).and_then(|v| v.as_str()),
        // TEMPORARILY DISABLED:                 params.get(1).and_then(|v| v.as_str()),
        // TEMPORARILY DISABLED:             ) {
        // TEMPORARILY DISABLED:                 let input_bytes = hex::decode(input_data).unwrap_or_default();
        // TEMPORARILY DISABLED:                 let context = AIVMExecutionContext {
        // TEMPORARILY DISABLED:                     transaction_hash: "manual_execution".to_string(),
        // TEMPORARILY DISABLED:                     block_height: 0,
        // TEMPORARILY DISABLED:                     timestamp: current_timestamp(),
        // TEMPORARILY DISABLED:                     sender: "manual".to_string(),
        // TEMPORARILY DISABLED:                     contract_address: Some(contract_address.to_string()),
        // TEMPORARILY DISABLED:                     input_data: input_bytes,
        // TEMPORARILY DISABLED:                     gas_limit: 1000000,
        // TEMPORARILY DISABLED:                     gas_price: 1000,
        // TEMPORARILY DISABLED:                 };
        // TEMPORARILY DISABLED:
        // TEMPORARILY DISABLED:                 match aivm_runtime.execute_contract(contract_address, context) {
        // TEMPORARILY DISABLED:                     Ok(result) => json!({"success": true, "result": result, "message": "AIVM contract executed successfully"}),
        // TEMPORARILY DISABLED:                     Err(error) => json!({"success": false, "error": error}),
        // TEMPORARILY DISABLED:                 }
        // TEMPORARILY DISABLED:             } else {
        // TEMPORARILY DISABLED:                 json!({"success": false, "error": "Missing required parameters: contract_address, input_data"})
        // TEMPORARILY DISABLED:             }
        // TEMPORARILY DISABLED:         }
        // TEMPORARILY DISABLED:
        // TEMPORARILY DISABLED:         "synergy_initiateDistributedAI" => {
        // TEMPORARILY DISABLED:             if let (Some(model_id), Some(input_data)) = (
        // TEMPORARILY DISABLED:                 params.get(0).and_then(|v| v.as_str()),
        // TEMPORARILY DISABLED:                 params.get(1).and_then(|v| v.as_str()),
        // TEMPORARILY DISABLED:             ) {
        // TEMPORARILY DISABLED:                 let input_bytes = hex::decode(input_data).unwrap_or_default();
        // TEMPORARILY DISABLED:                 let cluster_id = params.get(2).and_then(|v| v.as_u64());
        // TEMPORARILY DISABLED:
        // TEMPORARILY DISABLED:                 match aivm_runtime.distributed_ai.initiate_distributed_computation(
        // TEMPORARILY DISABLED:                     model_id.to_string(),
        // TEMPORARILY DISABLED:                     input_bytes,
        // TEMPORARILY DISABLED:                     cluster_id,
        // TEMPORARILY DISABLED:                 ) {
        // TEMPORARILY DISABLED:                     Ok(computation_id) => json!({"success": true, "computation_id": computation_id, "message": "Distributed AI computation initiated"}),
        // TEMPORARILY DISABLED:                     Err(error) => json!({"success": false, "error": error}),
        // TEMPORARILY DISABLED:                 }
        // TEMPORARILY DISABLED:             } else {
        // TEMPORARILY DISABLED:                 json!({"success": false, "error": "Missing required parameters: model_id, input_data"})
        // TEMPORARILY DISABLED:             }
        // TEMPORARILY DISABLED:         }
        // TEMPORARILY DISABLED:
        // TEMPORARILY DISABLED:         "synergy_getDistributedAIStatus" => {
        // TEMPORARILY DISABLED:             if let Some(computation_id) = params.get(0).and_then(|v| v.as_str()) {
        // TEMPORARILY DISABLED:                 match aivm_runtime.distributed_ai.get_computation_status(computation_id) {
        // TEMPORARILY DISABLED:                     Some(status) => json!({"status": format!("{:?}", status), "computation_id": computation_id}),
        // TEMPORARILY DISABLED:                     None => json!({"error": "Computation not found"}),
        // TEMPORARILY DISABLED:                 }
        // TEMPORARILY DISABLED:             } else {
        // TEMPORARILY DISABLED:                 json!("Missing computation_id parameter")
        // TEMPORARILY DISABLED:             }
        // TEMPORARILY DISABLED:         }
        // TEMPORARILY DISABLED:
        // TEMPORARILY DISABLED:         "synergy_getDistributedAIResult" => {
        // TEMPORARILY DISABLED:             if let Some(computation_id) = params.get(0).and_then(|v| v.as_str()) {
        // TEMPORARILY DISABLED:                 match aivm_runtime.distributed_ai.get_computation_result(computation_id) {
        // TEMPORARILY DISABLED:                     Some(result) => json!({"success": true, "result": hex::encode(result), "computation_id": computation_id}),
        // TEMPORARILY DISABLED:                     None => json!({"error": "Result not available or computation not completed"}),
        // TEMPORARILY DISABLED:                 }
        // TEMPORARILY DISABLED:             } else {
        // TEMPORARILY DISABLED:                 json!("Missing computation_id parameter")
        // TEMPORARILY DISABLED:             }
        // TEMPORARILY DISABLED:         }
        // TEMPORARILY DISABLED:
        // TEMPORARILY DISABLED:         "synergy_submitAIPartialResult" => {
        // TEMPORARILY DISABLED:             if let (Some(task_id), Some(validator_address), Some(partial_result)) = (
        // TEMPORARILY DISABLED:                 params.get(0).and_then(|v| v.as_str()),
        // TEMPORARILY DISABLED:                 params.get(1).and_then(|v| v.as_str()),
        // TEMPORARILY DISABLED:                 params.get(2).and_then(|v| v.as_str()),
        // TEMPORARILY DISABLED:             ) {
        // TEMPORARILY DISABLED:                 let result_bytes = hex::decode(partial_result).unwrap_or_default();
        // TEMPORARILY DISABLED:
        // TEMPORARILY DISABLED:                 match aivm_runtime.distributed_ai.submit_partial_result(
        // TEMPORARILY DISABLED:                     task_id,
        // TEMPORARILY DISABLED:                     validator_address,
        // TEMPORARILY DISABLED:                     result_bytes,
        // TEMPORARILY DISABLED:                 ) {
        // TEMPORARILY DISABLED:                     Ok(_) => json!({"success": true, "message": "Partial result submitted successfully"}),
        // TEMPORARILY DISABLED:                     Err(error) => json!({"success": false, "error": error}),
        // TEMPORARILY DISABLED:                 }
        // TEMPORARILY DISABLED:             } else {
        // TEMPORARILY DISABLED:                 json!({"success": false, "error": "Missing required parameters: task_id, validator_address, partial_result"})
        // TEMPORARILY DISABLED:             }
        // TEMPORARILY DISABLED:         }
        // TEMPORARILY DISABLED:
        // TEMPORARILY DISABLED:         "synergy_getValidatorAITasks" => {
        // TEMPORARILY DISABLED:             if let Some(validator_address) = params.get(0).and_then(|v| v.as_str()) {
        // TEMPORARILY DISABLED:                 let tasks = aivm_runtime.distributed_ai.get_pending_tasks_for_validator(validator_address);
        // TEMPORARILY DISABLED:                 json!(tasks)
        // TEMPORARILY DISABLED:             } else {
        // TEMPORARILY DISABLED:                 json!("Missing validator_address parameter")
        // TEMPORARILY DISABLED:             }
        // TEMPORARILY DISABLED:         }
        // TEMPORARILY DISABLED:
        // TEMPORARILY DISABLED:         "synergy_getValidatorAIRewards" => {
        // TEMPORARILY DISABLED:             if let Some(validator_address) = params.get(0).and_then(|v| v.as_str()) {
        // TEMPORARILY DISABLED:                 let rewards = aivm_runtime.distributed_ai.get_validator_ai_rewards(validator_address);
        // TEMPORARILY DISABLED:                 json!({"validator_address": validator_address, "total_rewards": rewards})
        // TEMPORARILY DISABLED:             } else {
        // TEMPORARILY DISABLED:                 json!("Missing validator_address parameter")
        // TEMPORARILY DISABLED:             }
        // TEMPORARILY DISABLED:         }
        // TEMPORARILY DISABLED:
        // TEMPORARILY DISABLED:         "synergy_getAIDistributedStats" => {
        // TEMPORARILY DISABLED:             json!(aivm_runtime.distributed_ai.get_ai_network_stats())
        // TEMPORARILY DISABLED:         }
        // TEMPORARILY DISABLED:
        // TEMPORARILY DISABLED:         "synergy_chatWithAIVM" => {
        // TEMPORARILY DISABLED:             if let Some(message) = params.get(0).and_then(|v| v.as_str()) {
        // TEMPORARILY DISABLED:                 let context = AIVMExecutionContext {
        // TEMPORARILY DISABLED:                     transaction_hash: "chat_interaction".to_string(),
        // TEMPORARILY DISABLED:                     block_height: 0,
        // TEMPORARILY DISABLED:                     timestamp: current_timestamp(),
        // TEMPORARILY DISABLED:                     sender: "user".to_string(),
        // TEMPORARILY DISABLED:                     contract_address: None,
        // TEMPORARILY DISABLED:                     input_data: message.as_bytes().to_vec(),
        // TEMPORARILY DISABLED:                     gas_limit: 10000,
        // TEMPORARILY DISABLED:                     gas_price: 100,
        // TEMPORARILY DISABLED:                 };
        // TEMPORARILY DISABLED:
        // TEMPORARILY DISABLED:                 // This would need async support in the RPC handler
        // TEMPORARILY DISABLED:                 json!({"success": true, "message": "Chat functionality requires async support - use direct AIVM runtime calls", "context": context})
        // TEMPORARILY DISABLED:             } else {
        // TEMPORARILY DISABLED:                 json!({"success": false, "error": "Missing message parameter"})
        // TEMPORARILY DISABLED:             }
        // TEMPORARILY DISABLED:         }
        // TEMPORARILY DISABLED:
        // TEMPORARILY DISABLED:         "synergy_getAIVMContracts" => {
        // TEMPORARILY DISABLED:             json!(aivm_runtime.get_all_contracts())
        // TEMPORARILY DISABLED:         }
        // TEMPORARILY DISABLED:
        // TEMPORARILY DISABLED:         "synergy_getAIVMContract" => {
        // TEMPORARILY DISABLED:             if let Some(address) = params.get(0).and_then(|v| v.as_str()) {
        // TEMPORARILY DISABLED:                 match aivm_runtime.get_contract(address) {
        // TEMPORARILY DISABLED:                     Some(contract) => json!(contract),
        // TEMPORARILY DISABLED:                     None => json!(null),
        // TEMPORARILY DISABLED:                 }
        // TEMPORARILY DISABLED:             } else {
        // TEMPORARILY DISABLED:                 json!("Missing contract address parameter")
        // TEMPORARILY DISABLED:             }
        // TEMPORARILY DISABLED:         }
        // TEMPORARILY DISABLED:
        // TEMPORARILY DISABLED:         "synergy_getAIVMStats" => {
        // TEMPORARILY DISABLED:             let distributed_stats = aivm_runtime.distributed_ai.get_ai_network_stats();
        // TEMPORARILY DISABLED:             json!({
        // TEMPORARILY DISABLED:                 "total_contracts": aivm_runtime.get_all_contracts().len(),
        // TEMPORARILY DISABLED:                 "supported_features": ["ai_enhanced", "cross_chain", "oracle", "standard", "distributed_ai"],
        // TEMPORARILY DISABLED:                 "ai_models": ["distributed_ai_model"],
        // TEMPORARILY DISABLED:                 "supported_chains": ["ethereum", "polygon", "solana"],
        // TEMPORARILY DISABLED:                 "distributed_computations": distributed_stats.get("total_computations").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0),
        // TEMPORARILY DISABLED:                 "completed_computations": distributed_stats.get("completed_computations").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0),
        // TEMPORARILY DISABLED:                 "active_validators": distributed_stats.get("active_validators").unwrap_or(&"0".to_string()).parse::<u64>().unwrap_or(0),
        // TEMPORARILY DISABLED:                 "total_ai_rewards_distributed": distributed_stats.get("total_ai_rewards_distributed").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0)
        // TEMPORARILY DISABLED:             })
        // TEMPORARILY DISABLED:         }
        // TEMPORARILY DISABLED:
        "synergy_getNetworkStats" => {
            let chain = chain.lock().unwrap();
            let token_manager = TOKEN_MANAGER.clone();
            let validators = network_validator_snapshot(&chain, &validator_manager);
            let active_validator_count = validators
                .iter()
                .filter(|validator| validator.status == ValidatorStatus::Active)
                .count();

            let total_supply = token_manager
                .get_all_tokens()
                .iter()
                .filter_map(|token| token.total_supply.parse::<u128>().ok())
                .sum::<u128>();

            json!({
                "block_height": chain.last().map_or(0, |b| b.block_index),
                "total_transactions": chain.chain.iter().map(|b| b.transactions.len()).sum::<usize>(),
                "active_validators": active_validator_count,
                "total_supply": total_supply.to_string(),
                "tokens": token_manager.get_all_tokens().len(),
                "network_uptime": "99.9%",
                "current_epoch": validator_manager.calculate_epoch_rewards(0).len(),
                "total_staked": token_manager.get_all_tokens().iter().map(|t| t.symbol.clone()).collect::<Vec<_>>()
                    .iter().map(|symbol| token_manager.get_staked_balance("*", symbol)).sum::<u64>()
            })
        }

        // Enhanced Token Operations
        "synergy_createToken" => {
            if let (Some(symbol), Some(name), Some(decimals), Some(total_supply), Some(creator)) = (
                params.get(0).and_then(|v| v.as_str()),
                params.get(1).and_then(|v| v.as_str()),
                params.get(2).and_then(|v| v.as_u64()),
                params.get(3).and_then(|v| v.as_u64()),
                params.get(4).and_then(|v| v.as_str()),
            ) {
                let token_manager = TOKEN_MANAGER.clone();
                match token_manager.create_token(
                    symbol.to_string(),
                    name.to_string(),
                    decimals as u8,
                    total_supply,
                    Some(total_supply * 2), // max_supply = 2x total_supply
                    true,                   // mintable
                    true,                   // burnable
                    creator.to_string(),
                ) {
                    Ok(result) => json!({"success": true, "message": result}),
                    Err(error) => json!({"success": false, "error": error}),
                }
            } else {
                json!({"success": false, "error": "Missing required parameters: symbol, name, decimals, total_supply, creator"})
            }
        }

        "synergy_mintTokens" => {
            if let (Some(to), Some(token_symbol), Some(amount)) = (
                params.get(0).and_then(|v| v.as_str()),
                params.get(1).and_then(|v| v.as_str()),
                params.get(2).and_then(|v| v.as_u64()),
            ) {
                let token_manager = TOKEN_MANAGER.clone();
                match token_manager.mint_tokens(to, token_symbol, amount) {
                    Ok(result) => json!({"success": true, "message": result}),
                    Err(error) => json!({"success": false, "error": error}),
                }
            } else {
                json!({"success": false, "error": "Missing required parameters: to, token_symbol, amount"})
            }
        }

        "synergy_burnTokens" => {
            if let (Some(from), Some(token_symbol), Some(amount)) = (
                params.get(0).and_then(|v| v.as_str()),
                params.get(1).and_then(|v| v.as_str()),
                params.get(2).and_then(|v| v.as_u64()),
            ) {
                let token_manager = TOKEN_MANAGER.clone();
                match token_manager.burn_tokens(from, token_symbol, amount) {
                    Ok(result) => json!({"success": true, "message": result}),
                    Err(error) => json!({"success": false, "error": error}),
                }
            } else {
                json!({"success": false, "error": "Missing required parameters: from, token_symbol, amount"})
            }
        }

        "synergy_transferTokens" => {
            if let (Some(from), Some(to), Some(token_symbol), Some(amount)) = (
                params.get(0).and_then(|v| v.as_str()),
                params.get(1).and_then(|v| v.as_str()),
                params.get(2).and_then(|v| v.as_str()),
                params.get(3).and_then(|v| v.as_u64()),
            ) {
                let token_manager = TOKEN_MANAGER.clone();
                match token_manager.transfer_tokens(from, to, token_symbol, amount, 1000) {
                    Ok(result) => json!({"success": true, "message": result}),
                    Err(error) => json!({"success": false, "error": error}),
                }
            } else {
                json!({"success": false, "error": "Missing required parameters: from, to, token_symbol, amount"})
            }
        }

        "synergy_getAllBalances" => {
            if let Some(address) = params.get(0).and_then(|v| v.as_str()) {
                let token_manager = TOKEN_MANAGER.clone();
                json!(token_manager.get_all_balances(address))
            } else {
                json!("Missing address parameter")
            }
        }

        "synergy_getTransferHistory" => {
            let address = params.get(0).and_then(|v| v.as_str());
            let limit = params.get(1).and_then(|v| v.as_u64()).unwrap_or(50);
            if let Some(address) = address {
                let token_manager = TOKEN_MANAGER.clone();
                json!(token_manager.get_transfer_history(address, limit as usize))
            } else {
                json!("Missing address parameter")
            }
        }

        // Node monitoring methods for control panel
        "synergy_getNodeStatus" => {
            let (last_block, avg_block_time) = {
                let chain = chain.lock().unwrap();
                (
                    chain.last().map_or(0, |b| b.block_index),
                    calculate_average_block_time(&chain),
                )
            };
            let peer_count = crate::p2p::get_p2p_network()
                .map(|p2p| p2p.get_peer_count() as u64)
                .unwrap_or(0);
            let config = crate::config::load_node_config(None).ok();
            let network_name = config.as_ref().map(|cfg| cfg.network.name.clone());
            let uptime_seconds = NODE_START_TIME
                .lock()
                .ok()
                .and_then(|start| start.map(|s| current_timestamp().saturating_sub(s)));
            let uptime_percentage =
                uptime_seconds.map(|secs| ((secs as f64 / 86400.0) * 100.0).min(100.0));
            let sync_status = SYNC_MANAGER
                .lock()
                .ok()
                .map(|manager| match manager.get_state() {
                    SyncState::Synced | SyncState::Idle => "synced",
                    SyncState::Discovering
                    | SyncState::Downloading
                    | SyncState::Validating
                    | SyncState::Applying => "syncing",
                })
                .unwrap_or("unknown");
            json!({
                "node_type": null,
                "status": "running",
                "uptime": uptime_percentage.map(|p| format!("{:.1}%", p)),
                "uptime_seconds": uptime_seconds,
                "version": env!("CARGO_PKG_VERSION"),
                "network": network_name,
                "sync_status": sync_status,
                "last_block": last_block,
                "avg_block_time": avg_block_time,
                "average_block_time": avg_block_time,
                "peers_connected": peer_count,
                "peer_count": peer_count,
                "peers": peer_count,
                "timestamp": current_timestamp()
            })
        }

        "synergy_getSyncStatus" => {
            let current_block = chain.lock().unwrap().last().map_or(0, |b| b.block_index);
            if let Ok(manager) = SYNC_MANAGER.lock() {
                let state = manager.get_state();
                let syncing = !matches!(state, SyncState::Synced | SyncState::Idle);
                json!({
                    "syncing": syncing,
                    "current_block": current_block,
                    "highest_block": manager.get_network_height(),
                    "starting_block": manager.get_sync_start_height(),
                    "sync_percentage": manager.get_progress_percentage(),
                    "state": format!("{:?}", state),
                })
            } else {
                json!({"error": "Sync manager unavailable"})
            }
        }

        "synergy_getBlockValidationStatus" => {
            let chain = chain.lock().unwrap();
            let validators = network_validator_snapshot(&chain, &validator_manager);
            let active_validators = validators
                .iter()
                .filter(|validator| validator.status == ValidatorStatus::Active)
                .count();
            let active_clusters = validators
                .iter()
                .filter(|validator| validator.status == ValidatorStatus::Active)
                .filter_map(|validator| validator.cluster_id)
                .collect::<HashSet<_>>()
                .len();
            let total_stake = validators
                .iter()
                .map(|validator| validator.stake_amount)
                .sum::<u64>();
            let recent_blocks: Vec<_> = chain
                .chain
                .iter()
                .rev()
                .take(10)
                .map(|block| {
                    json!({
                        "block_number": block.block_index,
                        "validator": block.validator_id,
                        "timestamp": block.timestamp,
                        "transactions": block.transactions.len(),
                        "status": "validated" // All blocks in chain are validated
                    })
                })
                .collect();

            json!({
                "current_block_height": chain.last().map_or(0, |b| b.block_index),
                "recent_blocks": recent_blocks,
                "validation_queue": [], // Add pending validation queue
                "active_validators": active_validators,
                "total_validators": validators.len(),
                "cluster_info": {
                    "active_clusters": active_clusters,
                    "total_stake": total_stake
                }
            })
        }

        "synergy_getValidatorActivity" => {
            let chain = chain.lock().unwrap();
            let active_validators = network_validator_snapshot(&chain, &validator_manager)
                .into_iter()
                .filter(|validator| validator.status == ValidatorStatus::Active)
                .collect::<Vec<_>>();
            let mut validator_activity = Vec::new();

            for validator in active_validators {
                validator_activity.push(json!({
                    "address": validator.address,
                    "name": validator.name,
                    "synergy_score": validator.synergy_score,
                    "blocks_produced": validator.total_blocks_produced,
                    "uptime": format!("{:.1}%", validator.uptime_percentage),
                    "cluster_id": validator.cluster_id,
                    "cluster_address": validator.cluster_address,
                    "stake_amount": validator.stake_amount,
                    "last_active": validator.last_active
                }));
            }

            json!({
                "validators": validator_activity,
                "total_active": validator_activity.len(),
                "average_synergy_score": if validator_activity.is_empty() { 0.0 } else {
                    validator_activity.iter()
                        .map(|v| v["synergy_score"].as_f64().unwrap_or(0.0))
                        .sum::<f64>() / validator_activity.len() as f64
                }
            })
        }

        "synergy_getSynergyScoreBreakdown" => {
            let address = params.get(0).and_then(|v| v.as_str());
            if let Some(address) = address {
                if let Some(validator) = validator_manager.get_validator(address) {
                    let pqc_manager = Arc::new(Mutex::new(PQCManager::new()));
                    let calculator =
                        SynergyScoreCalculator::new(Arc::clone(validator_manager), pqc_manager);
                    let components = calculator.calculate_synergy_score(&validator);
                    json!({
                        "address": address,
                        "total_score": validator.synergy_score,
                        "components": components
                    })
                } else {
                    json!({"error": "Validator not found"})
                }
            } else {
                json!({"error": "Missing address parameter"})
            }
        }

        "synergy_getPeerInfo" => {
            if let Some(p2p) = crate::p2p::get_p2p_network() {
                json!({
                    "peer_count": p2p.get_peer_count(),
                    "peers": p2p.get_peer_info()
                })
            } else {
                json!({
                    "peer_count": 0,
                    "peers": []
                })
            }
        }

        // =====================================================================
        // Phase 1: Core Blockchain Functionality (New RPC Methods)
        // =====================================================================

        // 1. synergy_getTransactionReceipt
        // Get a transaction receipt with execution details.
        "synergy_getTransactionReceipt" => {
            if let Some(tx_hash) = params.get(0).and_then(|v| v.as_str()) {
                let normalized = tx_hash.strip_prefix("0x").unwrap_or(tx_hash).to_lowercase();
                let raw_hash_search = if normalized.starts_with("syntxn-") {
                    normalized.strip_prefix("syntxn-").unwrap_or(&normalized)
                } else if normalized.starts_with("synxxn-") {
                    normalized.strip_prefix("synxxn-").unwrap_or(&normalized)
                } else {
                    &normalized
                };

                let matches_tx = |tx: &Transaction| -> bool {
                    let tx_hash_formatted = tx.hash().to_lowercase();
                    let tx_hash_raw = tx.raw_hash().to_lowercase();
                    tx_hash_formatted == normalized
                        || tx_hash_raw == normalized
                        || tx_hash_raw == raw_hash_search
                        || (tx_hash_formatted.starts_with("syntxn-")
                            && tx_hash_formatted.strip_prefix("syntxn-").unwrap_or("")
                                == raw_hash_search)
                        || (tx_hash_formatted.starts_with("synxxn-")
                            && tx_hash_formatted.strip_prefix("synxxn-").unwrap_or("")
                                == raw_hash_search)
                };

                // Search in confirmed transactions
                let chain = chain.lock().unwrap();
                for block in &chain.chain {
                    let mut cumulative_gas: u64 = 0;
                    for (idx, tx) in block.transactions.iter().enumerate() {
                        let gas_used = if tx.data.is_some() {
                            tx.gas_limit.min(tx.estimate_gas())
                        } else {
                            crate::gas::constants::GAS_LIMIT_TRANSFER
                        };
                        cumulative_gas = cumulative_gas.saturating_add(gas_used);

                        if matches_tx(tx) {
                            let is_contract_creation =
                                tx.receiver.is_empty() || tx.receiver == "0x0" || tx.receiver == "";
                            let contract_address = if is_contract_creation {
                                // Derive a deterministic contract address
                                let hash_input = format!("{}{}", tx.sender, tx.nonce);
                                let addr_hash =
                                    hex::encode(blake3::hash(hash_input.as_bytes()).as_bytes());
                                Some(format!("sync1{}", &addr_hash[..38]))
                            } else {
                                None
                            };

                            return json!({
                                "transactionHash": tx.hash(),
                                "transactionIndex": idx,
                                "blockHash": block.hash.clone(),
                                "blockNumber": block.block_index,
                                "from": tx.sender.clone(),
                                "to": if is_contract_creation { Value::Null } else { json!(tx.receiver.clone()) },
                                "cumulativeGasUsed": cumulative_gas,
                                "gasUsed": gas_used,
                                "effectiveGasPrice": tx.gas_price,
                                "status": "0x1",
                                "logs": [],
                                "logsBloom": "0x".to_string() + &"0".repeat(512),
                                "contractAddress": contract_address
                            });
                        }
                    }
                }

                // Check pending pool - return null (receipt only exists for mined txs)
                json!(null)
            } else {
                json!({"error": "Missing transaction hash parameter"})
            }
        }

        // 2. synergy_getTransactionCount
        // Get the transaction count (nonce) for an address.
        "synergy_getTransactionCount" => {
            if let Some(address) = params.get(0).and_then(|v| v.as_str()) {
                let _block_tag = params.get(1).and_then(|v| v.as_str()).unwrap_or("latest");

                // Count confirmed transactions sent by this address
                let chain = chain.lock().unwrap();
                let mut count: u64 = 0;
                for block in &chain.chain {
                    for tx in &block.transactions {
                        if tx.sender.eq_ignore_ascii_case(address) {
                            count += 1;
                        }
                    }
                }

                // If block_tag is "pending", also count pending txs
                if _block_tag == "pending" {
                    let pool = tx_pool.lock().unwrap();
                    for tx in pool.iter() {
                        if tx.sender.eq_ignore_ascii_case(address) {
                            count += 1;
                        }
                    }
                }

                json!(count)
            } else {
                json!({"error": "Missing address parameter"})
            }
        }

        // 3. synergy_getBalance
        // Get the SNRG balance for an address (standardized method).
        "synergy_getBalance" => {
            if let Some(address) = params.get(0).and_then(|v| v.as_str()) {
                let token_manager = TOKEN_MANAGER.clone();
                let balance = token_manager.get_balance(address, "SNRG");
                json!(balance)
            } else {
                json!({"error": "Missing address parameter"})
            }
        }

        // 4. synergy_gasPrice
        // Get the current gas price.
        "synergy_gasPrice" => {
            use crate::gas::constants::{DEFAULT_GAS_PRICE, MAX_GAS_PRICE, MIN_GAS_PRICE};

            // Calculate dynamic gas price based on recent block utilization
            let chain = chain.lock().unwrap();
            let recent_blocks: Vec<_> = chain.chain.iter().rev().take(10).collect();

            if recent_blocks.is_empty() {
                json!(DEFAULT_GAS_PRICE)
            } else {
                let mut total_gas_used: u64 = 0;
                let block_gas_limit = crate::gas::constants::BLOCK_GAS_LIMIT;

                for block in &recent_blocks {
                    let block_gas: u64 = block.transactions.iter().map(|tx| tx.get_fee()).sum();
                    total_gas_used += block_gas;
                }

                let avg_gas_per_block = total_gas_used / recent_blocks.len() as u64;
                let utilization = avg_gas_per_block as f64 / block_gas_limit as f64;

                // Scale gas price based on utilization
                let gas_price = if utilization > 0.8 {
                    (DEFAULT_GAS_PRICE as f64 * (1.0 + utilization)) as u64
                } else if utilization < 0.1 {
                    DEFAULT_GAS_PRICE
                } else {
                    DEFAULT_GAS_PRICE
                };

                let clamped = gas_price.max(MIN_GAS_PRICE).min(MAX_GAS_PRICE);
                json!(clamped)
            }
        }

        // 5. synergy_call
        // Execute a contract call locally (read-only, no state change).
        "synergy_call" => {
            if let Some(call_obj) = params.get(0) {
                let _from = call_obj.get("from").and_then(|v| v.as_str()).unwrap_or("");
                let to = call_obj.get("to").and_then(|v| v.as_str());
                let _data = call_obj
                    .get("data")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0x");
                let _value = call_obj.get("value").and_then(|v| v.as_u64()).unwrap_or(0);

                if let Some(to_addr) = to {
                    // Check if the target is a known contract
                    // For now, return empty result since AIVM is disabled
                    // When AIVM is re-enabled, this will execute the contract call
                    if to_addr.starts_with("sync1") || to_addr.starts_with("sync0") {
                        // Contract address detected - AIVM currently disabled
                        json!({
                            "result": "0x",
                            "note": "AIVM contract execution is currently disabled in testnet-beta. Contract calls will return empty results."
                        })
                    } else {
                        // Non-contract address - return the balance as a simple read
                        json!({
                            "result": "0x",
                            "note": "Target address is not a contract"
                        })
                    }
                } else {
                    json!({"error": "Missing 'to' field in call object"})
                }
            } else {
                json!({"error": "Missing call object parameter"})
            }
        }

        // 6. synergy_estimateGas
        // Estimate the gas required for a transaction.
        "synergy_estimateGas" => {
            if let Some(tx_obj) = params.get(0) {
                match normalize_rpc_transaction(tx_obj, false) {
                    Ok(normalized) => {
                        let gas = estimate_gas_for_transaction(&normalized.transaction);
                        let gas_price = current_gas_price_from_chain(chain);
                        json!({
                            "gas": gas,
                            "safeFee": gas.saturating_mul(gas_price),
                            "maxFee": gas.saturating_mul(normalized.transaction.gas_price),
                            "warnings": normalized.warnings
                        })
                    }
                    Err(error) => {
                        json!({"error": error.message, "code": error.code, "data": error.data})
                    }
                }
            } else {
                json!({"error": "Missing transaction object parameter"})
            }
        }

        // 7. synergy_getLogs
        // Get event logs matching filters.
        "synergy_getLogs" => {
            if let Some(filter) = params.get(0) {
                let from_block = filter
                    .get("fromBlock")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let to_block = filter.get("toBlock").and_then(|v| v.as_u64());
                let filter_address = filter.get("address").and_then(|v| v.as_str());
                let _topics = filter.get("topics").and_then(|v| v.as_array());
                let block_hash = filter.get("blockHash").and_then(|v| v.as_str());

                let chain = chain.lock().unwrap();
                let to_block =
                    to_block.unwrap_or_else(|| chain.last().map_or(0, |b| b.block_index));

                let mut logs: Vec<Value> = Vec::new();
                let mut log_index: u64 = 0;

                for block in &chain.chain {
                    // Filter by block hash if specified
                    if let Some(bh) = block_hash {
                        if !block.hash.eq_ignore_ascii_case(bh) {
                            continue;
                        }
                    } else if block.block_index < from_block || block.block_index > to_block {
                        continue;
                    }

                    for (tx_idx, tx) in block.transactions.iter().enumerate() {
                        // Filter by address if specified
                        if let Some(addr) = filter_address {
                            if !tx.sender.eq_ignore_ascii_case(addr)
                                && !tx.receiver.eq_ignore_ascii_case(addr)
                            {
                                continue;
                            }
                        }

                        // Generate a log entry for each transaction
                        // (full EVM-style event logs will be available when AIVM is re-enabled)
                        if tx.data.is_some() || filter_address.is_some() {
                            logs.push(json!({
                                "logIndex": log_index,
                                "transactionIndex": tx_idx,
                                "transactionHash": tx.hash(),
                                "blockHash": block.hash.clone(),
                                "blockNumber": block.block_index,
                                "address": tx.receiver.clone(),
                                "data": tx.data.clone().unwrap_or_else(|| "0x".to_string()),
                                "topics": [],
                                "removed": false
                            }));
                            log_index += 1;
                        }
                    }
                }

                json!(logs)
            } else {
                // No filter provided - return empty logs
                json!([])
            }
        }

        // 8. synergy_getCode
        // Get the code stored at an address.
        "synergy_getCode" => {
            if let Some(address) = params.get(0).and_then(|v| v.as_str()) {
                // Check if this is a contract address (sync1... prefix)
                if address.starts_with("sync1") || address.starts_with("sync0") {
                    // AIVM is currently disabled - return empty code
                    // When re-enabled, this will look up the contract bytecode
                    json!("0x")
                } else {
                    // Regular wallet address - no code
                    json!("0x")
                }
            } else {
                json!({"error": "Missing address parameter"})
            }
        }

        // 9. synergy_getStorageAt
        // Get the value from a storage position of a contract/account.
        "synergy_getStorageAt" => {
            if let (Some(address), Some(_position)) = (
                params.get(0).and_then(|v| v.as_str()),
                params.get(1).and_then(|v| v.as_str()),
            ) {
                let _block_tag = params.get(2).and_then(|v| v.as_str()).unwrap_or("latest");

                if address.starts_with("sync1") || address.starts_with("sync0") {
                    // Contract address - AIVM currently disabled
                    // When re-enabled, this will read from contract storage
                    json!("0x".to_string() + &"0".repeat(64))
                } else {
                    // Non-contract address - return zero
                    json!("0x".to_string() + &"0".repeat(64))
                }
            } else {
                json!({"error": "Missing required parameters: address, position"})
            }
        }

        // =====================================================================
        // Additional Phase 1 utility methods
        // =====================================================================

        // synergy_getBlockTransactionCount
        "synergy_getBlockTransactionCount" => {
            let chain = chain.lock().unwrap();
            if let Some(block_num) = params.get(0).and_then(|v| v.as_u64()) {
                if let Some(block) = chain.chain.iter().find(|b| b.block_index == block_num) {
                    json!(block.transactions.len())
                } else {
                    json!(null)
                }
            } else if let Some(block_hash) = params.get(0).and_then(|v| v.as_str()) {
                if let Some(block) = chain
                    .chain
                    .iter()
                    .find(|b| b.hash.eq_ignore_ascii_case(block_hash))
                {
                    json!(block.transactions.len())
                } else {
                    json!(null)
                }
            } else {
                json!({"error": "Missing block number or block hash parameter"})
            }
        }

        // synergy_getBlockReceipts
        "synergy_getBlockReceipts" => {
            let chain = chain.lock().unwrap();
            let block = if let Some(block_num) = params.get(0).and_then(|v| v.as_u64()) {
                chain.chain.iter().find(|b| b.block_index == block_num)
            } else if let Some(block_hash) = params.get(0).and_then(|v| v.as_str()) {
                chain
                    .chain
                    .iter()
                    .find(|b| b.hash.eq_ignore_ascii_case(block_hash))
            } else {
                return json!({"error": "Missing block number or block hash parameter"});
            };

            if let Some(block) = block {
                let mut cumulative_gas: u64 = 0;
                let receipts: Vec<Value> = block.transactions.iter().enumerate().map(|(idx, tx)| {
                    let gas_used = if tx.data.is_some() {
                        tx.gas_limit.min(tx.estimate_gas())
                    } else {
                        crate::gas::constants::GAS_LIMIT_TRANSFER
                    };
                    cumulative_gas = cumulative_gas.saturating_add(gas_used);

                    let is_contract_creation = tx.receiver.is_empty() || tx.receiver == "0x0";
                    let contract_address = if is_contract_creation {
                        let hash_input = format!("{}{}", tx.sender, tx.nonce);
                        let addr_hash = hex::encode(blake3::hash(hash_input.as_bytes()).as_bytes());
                        Some(format!("sync1{}", &addr_hash[..38]))
                    } else {
                        None
                    };

                    json!({
                        "transactionHash": tx.hash(),
                        "transactionIndex": idx,
                        "blockHash": block.hash.clone(),
                        "blockNumber": block.block_index,
                        "from": tx.sender.clone(),
                        "to": if is_contract_creation { Value::Null } else { json!(tx.receiver.clone()) },
                        "cumulativeGasUsed": cumulative_gas,
                        "gasUsed": gas_used,
                        "effectiveGasPrice": tx.gas_price,
                        "status": "0x1",
                        "logs": [],
                        "logsBloom": "0x".to_string() + &"0".repeat(512),
                        "contractAddress": contract_address
                    })
                }).collect();
                json!(receipts)
            } else {
                json!(null)
            }
        }

        // synergy_getPendingTransactions
        "synergy_getPendingTransactions" => {
            let limit = params.get(0).and_then(|v| v.as_u64()).unwrap_or(100) as usize;
            let sort_by = params
                .get(1)
                .and_then(|v| v.as_str())
                .unwrap_or("timestamp");

            let pool = tx_pool.lock().unwrap();
            let mut txs: Vec<&Transaction> = pool.iter().collect();

            match sort_by {
                "gasPrice" => txs.sort_by(|a, b| b.gas_price.cmp(&a.gas_price)),
                "nonce" => txs.sort_by(|a, b| a.nonce.cmp(&b.nonce)),
                _ => txs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)),
            }

            let result: Vec<Value> = txs
                .iter()
                .take(limit)
                .map(|tx| tx_to_explorer_json(tx, "pending", None, None))
                .collect();
            json!(result)
        }

        // synergy_getTransactionByBlockNumberAndIndex
        "synergy_getTransactionByBlockNumberAndIndex" => {
            if let (Some(block_num), Some(index)) = (
                params.get(0).and_then(|v| v.as_u64()),
                params.get(1).and_then(|v| v.as_u64()),
            ) {
                let chain = chain.lock().unwrap();
                if let Some(block) = chain.chain.iter().find(|b| b.block_index == block_num) {
                    if let Some(tx) = block.transactions.get(index as usize) {
                        tx_to_explorer_json(
                            tx,
                            "confirmed",
                            Some(block.block_index),
                            Some(index as usize),
                        )
                    } else {
                        json!(null)
                    }
                } else {
                    json!(null)
                }
            } else {
                json!({"error": "Missing required parameters: blockNumber, index"})
            }
        }

        // synergy_getTransactionByBlockHashAndIndex
        "synergy_getTransactionByBlockHashAndIndex" => {
            if let (Some(block_hash), Some(index)) = (
                params.get(0).and_then(|v| v.as_str()),
                params.get(1).and_then(|v| v.as_u64()),
            ) {
                let chain = chain.lock().unwrap();
                if let Some(block) = chain
                    .chain
                    .iter()
                    .find(|b| b.hash.eq_ignore_ascii_case(block_hash))
                {
                    if let Some(tx) = block.transactions.get(index as usize) {
                        tx_to_explorer_json(
                            tx,
                            "confirmed",
                            Some(block.block_index),
                            Some(index as usize),
                        )
                    } else {
                        json!(null)
                    }
                } else {
                    json!(null)
                }
            } else {
                json!({"error": "Missing required parameters: blockHash, index"})
            }
        }

        // synergy_maxFeePerGas
        "synergy_maxFeePerGas" => {
            use crate::gas::constants::DEFAULT_GAS_PRICE;
            // In Synergy's fee model, max fee = 2x current gas price
            let base = DEFAULT_GAS_PRICE;
            json!(base * 2)
        }

        // synergy_maxPriorityFeePerGas
        "synergy_maxPriorityFeePerGas" => {
            use crate::gas::constants::DEFAULT_GAS_PRICE;
            // Priority fee tip - typically a fraction of base gas price
            json!(DEFAULT_GAS_PRICE / 4)
        }

        // synergy_getFeeHistory
        "synergy_getFeeHistory" => {
            let block_count = params.get(0).and_then(|v| v.as_u64()).unwrap_or(10) as usize;
            let _newest_block = params.get(1).and_then(|v| v.as_str()).unwrap_or("latest");
            let reward_percentiles = params.get(2).and_then(|v| v.as_array());

            let chain = chain.lock().unwrap();
            let recent_blocks: Vec<_> = chain.chain.iter().rev().take(block_count).collect();

            let mut base_fees: Vec<u64> = Vec::new();
            let mut gas_used_ratios: Vec<f64> = Vec::new();
            let mut rewards: Vec<Vec<u64>> = Vec::new();
            let block_gas_limit = crate::gas::constants::BLOCK_GAS_LIMIT;

            for block in recent_blocks.iter().rev() {
                let block_gas: u64 = block.transactions.iter().map(|tx| tx.get_fee()).sum();
                let ratio = block_gas as f64 / block_gas_limit as f64;

                base_fees.push(crate::gas::constants::DEFAULT_GAS_PRICE);
                gas_used_ratios.push(ratio);

                if let Some(percentiles) = reward_percentiles {
                    let mut gas_prices: Vec<u64> =
                        block.transactions.iter().map(|tx| tx.gas_price).collect();
                    gas_prices.sort();
                    let block_rewards: Vec<u64> = percentiles
                        .iter()
                        .map(|p| {
                            let pct = p.as_f64().unwrap_or(50.0) / 100.0;
                            if gas_prices.is_empty() {
                                0
                            } else {
                                let idx = ((gas_prices.len() as f64 * pct) as usize)
                                    .min(gas_prices.len() - 1);
                                gas_prices[idx]
                            }
                        })
                        .collect();
                    rewards.push(block_rewards);
                }
            }

            json!({
                "baseFeePerGas": base_fees,
                "gasUsedRatio": gas_used_ratios,
                "reward": rewards,
                "oldestBlock": recent_blocks.last().map(|b| b.block_index).unwrap_or(0)
            })
        }

        // =====================================================================
        // Phase 2: Enhanced Validator & Staking (New RPC Methods)
        // =====================================================================

        // synergy_getChainId
        "synergy_getChainId" => {
            json!(format!("0x{:x}", current_chain_id()))
        }

        // synergy_getValidatorByCluster
        "synergy_getValidatorByCluster" => {
            if let Some(cluster_id) = params.get(0).and_then(|v| v.as_u64()) {
                let all_validators = validator_manager.get_all_validators();
                let cluster_validators: Vec<_> = all_validators
                    .into_iter()
                    .filter(|v| v.cluster_id == Some(cluster_id))
                    .collect();
                json!(cluster_validators)
            } else {
                json!({"error": "Missing cluster ID parameter"})
            }
        }

        // synergy_getValidatorRewards
        "synergy_getValidatorRewards" => {
            if let Some(address) = params.get(0).and_then(|v| v.as_str()) {
                let _from_epoch = params.get(1).and_then(|v| v.as_u64()).unwrap_or(0);
                let _to_epoch = params.get(2).and_then(|v| v.as_u64());

                // Look up the validator to get block production stats
                match validator_manager.get_validator(address) {
                    Some(validator) => {
                        // Calculate rewards from blocks produced
                        let chain = chain.lock().unwrap();
                        let mut rewards: Vec<Value> = Vec::new();
                        let blocks_by_validator: Vec<_> = chain
                            .chain
                            .iter()
                            .filter(|b| b.validator_id.eq_ignore_ascii_case(address))
                            .collect();

                        for block in &blocks_by_validator {
                            let block_reward = 10_000_000_000u64; // 10 SNRG per block in nWei
                            let tx_fees: u64 =
                                block.transactions.iter().map(|tx| tx.get_fee()).sum();
                            rewards.push(json!({
                                "blockNumber": block.block_index,
                                "amount": block_reward + tx_fees,
                                "type": "block",
                                "timestamp": block.timestamp
                            }));
                        }

                        json!({
                            "address": address,
                            "totalBlocksProduced": validator.total_blocks_produced,
                            "rewards": rewards,
                            "totalRewards": rewards.iter()
                                .filter_map(|r| r.get("amount").and_then(|a| a.as_u64()))
                                .sum::<u64>()
                        })
                    }
                    None => json!({"error": "Validator not found"}),
                }
            } else {
                json!({"error": "Missing address parameter"})
            }
        }

        // synergy_getValidatorPerformance
        "synergy_getValidatorPerformance" => {
            if let Some(address) = params.get(0).and_then(|v| v.as_str()) {
                match validator_manager.get_validator(address) {
                    Some(validator) => {
                        let chain = chain.lock().unwrap();
                        let total_blocks = chain.chain.len() as u64;
                        let blocks_produced: u64 = chain
                            .chain
                            .iter()
                            .filter(|b| b.validator_id.eq_ignore_ascii_case(address))
                            .count() as u64;

                        let total_validators =
                            validator_manager.get_validator_count().max(1) as f64;
                        let expected_blocks = total_blocks as f64 / total_validators;
                        let proposal_rate = if expected_blocks > 0.0 {
                            (blocks_produced as f64 / expected_blocks).min(1.0)
                        } else {
                            0.0
                        };

                        json!({
                            "address": address,
                            "attestationSuccessRate": validator.uptime_percentage,
                            "blockProposalSuccessRate": proposal_rate,
                            "averageInclusionDelay": validator.average_block_time,
                            "missedAttestations": validator.missed_blocks,
                            "orphanedBlocks": 0,
                            "effectiveBalance": validator.stake_amount,
                            "totalBlocksProduced": blocks_produced,
                            "synergyScore": validator.synergy_score,
                            "reputationScore": validator.reputation_score,
                            "collaborationScore": validator.collaboration_score,
                            "uptime": validator.uptime_percentage
                        })
                    }
                    None => json!({"error": "Validator not found"}),
                }
            } else {
                json!({"error": "Missing address parameter"})
            }
        }

        // synergy_getValidatorQueue
        "synergy_getValidatorQueue" => {
            if let Ok(registry) = validator_manager.registry.lock() {
                let activation_queue: Vec<Value> = registry
                    .pending_registrations
                    .values()
                    .map(|r| {
                        json!({
                            "address": r.address,
                            "name": r.name,
                            "stakeAmount": r.stake_amount,
                            "submittedAt": r.submitted_at
                        })
                    })
                    .collect();

                let exit_queue: Vec<Value> = registry
                    .jailed_validators
                    .iter()
                    .map(|addr| json!({"address": addr}))
                    .collect();

                json!({
                    "activationQueue": activation_queue,
                    "activationQueueLength": activation_queue.len(),
                    "exitQueue": exit_queue,
                    "exitQueueLength": exit_queue.len(),
                    "estimatedActivationTime": if activation_queue.is_empty() { 0 } else { current_timestamp() + 3600 },
                    "estimatedExitTime": if exit_queue.is_empty() { 0 } else { current_timestamp() + 7200 }
                })
            } else {
                json!({"error": "Failed to access validator registry"})
            }
        }

        // synergy_requestValidatorExit
        "synergy_requestValidatorExit" => {
            if let (Some(address), Some(_signature)) = (
                params.get(0).and_then(|v| v.as_str()),
                params.get(1).and_then(|v| v.as_str()),
            ) {
                match validator_manager.get_validator(address) {
                    Some(_validator) => {
                        if let Ok(registry) = validator_manager.registry.lock() {
                            let current_epoch = registry.current_epoch;
                            let exit_epoch = current_epoch + 2; // 2 epoch delay
                            let epoch_length = registry.epoch_length;
                            let withdrawal_time = current_timestamp() + (2 * epoch_length);

                            json!({
                                "success": true,
                                "message": "Validator exit requested",
                                "validatorAddress": address,
                                "currentEpoch": current_epoch,
                                "exitEpoch": exit_epoch,
                                "withdrawalAvailableAt": withdrawal_time
                            })
                        } else {
                            json!({"success": false, "error": "Failed to access registry"})
                        }
                    }
                    None => json!({"success": false, "error": "Validator not found"}),
                }
            } else {
                json!({"success": false, "error": "Missing required parameters: address, signature"})
            }
        }

        // synergy_getValidatorSlashingHistory
        "synergy_getValidatorSlashingHistory" => {
            if let Some(address) = params.get(0).and_then(|v| v.as_str()) {
                match validator_manager.get_validator(address) {
                    Some(validator) => {
                        // Build slashing history from validator state
                        let mut history: Vec<Value> = Vec::new();
                        if validator.slashing_penalty > 0.0 {
                            history.push(json!({
                                "reason": "Slashing penalty recorded",
                                "penalty": validator.slashing_penalty,
                                "doubleSignCount": validator.double_signs,
                                "missedBlocks": validator.missed_blocks,
                                "balanceAfter": validator.stake_amount
                            }));
                        }
                        json!({
                            "address": address,
                            "slashingEvents": history,
                            "totalPenalties": validator.slashing_penalty,
                            "doubleSignCount": validator.double_signs
                        })
                    }
                    None => json!({"error": "Validator not found"}),
                }
            } else {
                json!({"error": "Missing address parameter"})
            }
        }

        // synergy_getClusterInfo
        "synergy_getClusterInfo" => {
            if let Some(cluster_id) = params.get(0).and_then(|v| v.as_u64()) {
                if let Ok(registry) = validator_manager.registry.lock() {
                    if let Some(cluster) = registry.clusters.get(&cluster_id) {
                        let validator_details: Vec<Value> = cluster
                            .validators
                            .iter()
                            .map(|addr| {
                                if let Some(v) = registry.validators.get(addr) {
                                    json!({
                                        "address": v.address,
                                        "name": v.name,
                                        "stakeAmount": v.stake_amount,
                                        "synergyScore": v.synergy_score,
                                        "status": format!("{:?}", v.status)
                                    })
                                } else {
                                    json!({"address": addr})
                                }
                            })
                            .collect();

                        json!({
                            "clusterId": cluster.id,
                            "address": cluster.address,
                            "validators": validator_details,
                            "validatorCount": cluster.validators.len(),
                            "totalStake": cluster.total_stake,
                            "averageSynergyScore": cluster.average_synergy_score,
                            "createdAt": cluster.created_at,
                            "lastRotation": cluster.last_rotation,
                            "group": cluster.group
                        })
                    } else {
                        json!(null)
                    }
                } else {
                    json!({"error": "Failed to access validator registry"})
                }
            } else {
                json!({"error": "Missing cluster ID parameter"})
            }
        }

        // synergy_getClusterRewards
        "synergy_getClusterRewards" => {
            if let Some(cluster_id) = params.get(0).and_then(|v| v.as_u64()) {
                let epoch = params.get(1).and_then(|v| v.as_u64()).unwrap_or(0);
                if let Ok(registry) = validator_manager.registry.lock() {
                    if let Some(cluster) = registry.clusters.get(&cluster_id) {
                        let epoch_rewards = registry.calculate_epoch_rewards(epoch);
                        let cluster_rewards: Vec<Value> = cluster
                            .validators
                            .iter()
                            .map(|addr| {
                                let reward = epoch_rewards.get(addr).copied().unwrap_or(0);
                                json!({
                                    "validatorAddress": addr,
                                    "rewardAmount": reward
                                })
                            })
                            .collect();

                        let total: u64 = cluster_rewards
                            .iter()
                            .filter_map(|r| r.get("rewardAmount").and_then(|a| a.as_u64()))
                            .sum();

                        json!({
                            "clusterId": cluster_id,
                            "epoch": epoch,
                            "totalRewards": total,
                            "distributions": cluster_rewards
                        })
                    } else {
                        json!(null)
                    }
                } else {
                    json!({"error": "Failed to access validator registry"})
                }
            } else {
                json!({"error": "Missing cluster ID parameter"})
            }
        }

        // synergy_proposeClusterChange
        "synergy_proposeClusterChange" => {
            if let (Some(cluster_id), Some(_proposal), Some(proposer)) = (
                params.get(0).and_then(|v| v.as_u64()),
                params.get(1),
                params.get(2).and_then(|v| v.as_str()),
            ) {
                // Verify proposer is a validator
                match validator_manager.get_validator(proposer) {
                    Some(_) => {
                        let proposal_id = format!("prop_{}_{}", cluster_id, current_timestamp());
                        json!({
                            "success": true,
                            "proposalId": proposal_id,
                            "clusterId": cluster_id,
                            "proposer": proposer,
                            "votingEndsAt": current_timestamp() + 86400 // 24 hours
                        })
                    }
                    None => {
                        json!({"success": false, "error": "Proposer must be a registered validator"})
                    }
                }
            } else {
                json!({"success": false, "error": "Missing required parameters: clusterId, proposal, proposer"})
            }
        }

        // synergy_getStakingRewards
        "synergy_getStakingRewards" => {
            if let Some(address) = params.get(0).and_then(|v| v.as_str()) {
                let specific_validator = params.get(1).and_then(|v| v.as_str());
                let token_manager = TOKEN_MANAGER.clone();
                let staking_info = token_manager.get_staking_info(address);

                let rewards: Vec<Value> = staking_info
                    .iter()
                    .filter(|info| {
                        specific_validator
                            .map_or(true, |v| info.validator_address.eq_ignore_ascii_case(v))
                    })
                    .map(|info| {
                        json!({
                            "validator": info.validator_address,
                            "stakedAmount": info.amount,
                            "rewardsEarned": info.rewards_earned,
                            "stakingStart": info.stake_start,
                            "isActive": info.is_active
                        })
                    })
                    .collect();

                let total_rewards: u64 = rewards
                    .iter()
                    .filter_map(|r| r.get("rewardsEarned").and_then(|a| a.as_u64()))
                    .sum();

                json!({
                    "address": address,
                    "rewards": rewards,
                    "totalRewardsEarned": total_rewards
                })
            } else {
                json!({"error": "Missing address parameter"})
            }
        }

        // synergy_getStakingAPY
        "synergy_getStakingAPY" => {
            let specific_validator = params.get(0).and_then(|v| v.as_str());
            let token_manager = TOKEN_MANAGER.clone();
            let total_staked = token_manager.get_staked_balance("*", "SNRG");
            let total_supply = token_manager
                .get_all_tokens()
                .iter()
                .find(|t| t.symbol == "SNRG")
                .and_then(|t| t.total_supply.parse::<u128>().ok())
                .unwrap_or(0);

            let staking_rate = if total_supply > 0 {
                total_staked as f64 / total_supply as f64
            } else {
                0.0
            };

            // Base APY: 5% annual, adjusted by staking participation
            // Lower participation = higher APY to incentivize staking
            let base_apy = 0.05;
            let current_apy = if staking_rate > 0.0 && staking_rate < 1.0 {
                base_apy / staking_rate.max(0.01)
            } else {
                base_apy
            };
            let capped_apy = current_apy.min(0.20); // Cap at 20%

            let mut result = json!({
                "currentAPY": capped_apy,
                "averageAPY": capped_apy, // Simplified: same as current in testnet
                "networkStakingRate": staking_rate,
                "totalStaked": total_staked,
                "totalSupply": total_supply.to_string(),
                "baseAPY": base_apy
            });

            if let Some(validator_addr) = specific_validator {
                if let Some(validator) = validator_manager.get_validator(validator_addr) {
                    // Higher synergy score = slightly better APY
                    let validator_apy = capped_apy * (1.0 + validator.synergy_score * 0.1);
                    result
                        .as_object_mut()
                        .unwrap()
                        .insert("validatorAPY".to_string(), json!(validator_apy.min(0.25)));
                    result.as_object_mut().unwrap().insert(
                        "validatorSynergyScore".to_string(),
                        json!(validator.synergy_score),
                    );
                }
            }

            result
        }

        // synergy_getDelegatedStakes
        "synergy_getDelegatedStakes" => {
            if let Some(address) = params.get(0).and_then(|v| v.as_str()) {
                let token_manager = TOKEN_MANAGER.clone();
                let staking_info = token_manager.get_staking_info(address);

                let delegations: Vec<Value> = staking_info
                    .iter()
                    .filter(|info| info.is_active)
                    .map(|info| {
                        json!({
                            "validator": info.validator_address,
                            "amount": info.amount,
                            "rewardsEarned": info.rewards_earned,
                            "delegatedAt": info.stake_start
                        })
                    })
                    .collect();

                json!({
                    "address": address,
                    "delegations": delegations,
                    "totalDelegated": delegations.iter()
                        .filter_map(|d| d.get("amount").and_then(|a| a.as_u64()))
                        .sum::<u64>()
                })
            } else {
                json!({"error": "Missing address parameter"})
            }
        }

        // synergy_getDelegators
        "synergy_getDelegators" => {
            if let Some(validator_addr) = params.get(0).and_then(|v| v.as_str()) {
                let _limit = params.get(1).and_then(|v| v.as_u64()).unwrap_or(100) as usize;
                let token_manager = TOKEN_MANAGER.clone();

                let addresses: Vec<String> = {
                    let balances = token_manager.balances.lock().unwrap();
                    balances.keys().cloned().collect()
                };
                let mut delegators: Vec<Value> = Vec::new();

                for address in &addresses {
                    let staking_info = token_manager.get_staking_info(address);
                    for info in &staking_info {
                        if info.validator_address.eq_ignore_ascii_case(validator_addr)
                            && info.is_active
                        {
                            delegators.push(json!({
                                "address": address,
                                "amount": info.amount,
                                "rewardsEarned": info.rewards_earned,
                                "delegatedAt": info.stake_start
                            }));
                        }
                    }
                }

                delegators.sort_by(|a, b| {
                    let a_amt = a.get("amount").and_then(|v| v.as_u64()).unwrap_or(0);
                    let b_amt = b.get("amount").and_then(|v| v.as_u64()).unwrap_or(0);
                    b_amt.cmp(&a_amt)
                });
                delegators.truncate(_limit);

                json!({
                    "validator": validator_addr,
                    "delegators": delegators,
                    "totalDelegators": delegators.len()
                })
            } else {
                json!({"error": "Missing validator address parameter"})
            }
        }

        // synergy_claimRewards
        "synergy_claimRewards" => {
            if let Some(staker) = params.get(0).and_then(|v| v.as_str()) {
                let specific_validator = params.get(1).and_then(|v| v.as_str());
                let token_manager = TOKEN_MANAGER.clone();
                let staking_info = token_manager.get_staking_info(staker);

                let mut total_claimed: u64 = 0;
                for info in &staking_info {
                    if specific_validator
                        .map_or(true, |v| info.validator_address.eq_ignore_ascii_case(v))
                    {
                        total_claimed += info.rewards_earned;
                    }
                }

                if total_claimed > 0 {
                    // Credit rewards to staker's balance
                    match token_manager.mint_tokens(staker, "SNRG", total_claimed) {
                        Ok(_) => {
                            json!({
                                "success": true,
                                "claimedAmount": total_claimed,
                                "stakerAddress": staker,
                                "message": "Rewards claimed successfully"
                            })
                        }
                        Err(e) => json!({"success": false, "error": e}),
                    }
                } else {
                    json!({
                        "success": false,
                        "error": "No rewards available to claim",
                        "stakerAddress": staker
                    })
                }
            } else {
                json!({"success": false, "error": "Missing staker address parameter"})
            }
        }

        // synergy_getRewardsProjection
        "synergy_getRewardsProjection" => {
            if let (Some(address), Some(amount), Some(duration_days)) = (
                params.get(0).and_then(|v| v.as_str()),
                params.get(1).and_then(|v| v.as_u64()),
                params.get(2).and_then(|v| v.as_u64()),
            ) {
                let specific_validator = params.get(3).and_then(|v| v.as_str());

                // Calculate APY for projection
                let token_manager = TOKEN_MANAGER.clone();
                let total_staked = token_manager.get_staked_balance("*", "SNRG");
                let total_supply = token_manager
                    .get_all_tokens()
                    .iter()
                    .find(|t| t.symbol == "SNRG")
                    .and_then(|t| t.total_supply.parse::<u128>().ok())
                    .unwrap_or(1);

                let staking_rate = (total_staked as f64 / total_supply as f64).max(0.01);
                let base_apy = 0.05;
                let apy = (base_apy / staking_rate).min(0.20);

                let daily_rate = apy / 365.0;
                let projected_reward = (amount as f64 * daily_rate * duration_days as f64) as u64;

                json!({
                    "address": address,
                    "stakeAmount": amount,
                    "durationDays": duration_days,
                    "estimatedAPY": apy,
                    "projectedReward": projected_reward,
                    "projectedTotal": amount + projected_reward,
                    "validator": specific_validator
                })
            } else {
                json!({"error": "Missing required parameters: address, amount, duration"})
            }
        }

        // synergy_getUnstakingPeriod
        "synergy_getUnstakingPeriod" => {
            json!({
                "unstakingPeriodDays": 7,
                "unstakingPeriodSeconds": 604800,
                "currentQueueLength": 0,
                "estimatedWithdrawalTime": current_timestamp() + 604800
            })
        }

        // Legacy support
        "synergy_status" => {
            json!("ok")
        }

        _ => {
            json!("Unknown method")
        }
    }
}

fn parse_http_headers(headers: &str) -> HashMap<String, String> {
    headers
        .lines()
        .skip(1)
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_ascii_lowercase(), value.trim().to_string()))
        })
        .collect()
}

fn current_rpc_role_profile() -> Option<&'static RoleProfile> {
    let role_id = std::env::var("SYNERGY_NODE_ROLE_ID").unwrap_or_default();
    let compiled_profile = std::env::var("SYNERGY_COMPILED_PROFILE").unwrap_or_default();
    resolve_configured_role(&role_id, &compiled_profile)
        .ok()
        .flatten()
}

fn parse_forwarded_ip(value: Option<&str>) -> Option<IpAddr> {
    value.and_then(|raw| {
        raw.split(',')
            .map(|segment| segment.trim())
            .find_map(|segment| segment.parse::<IpAddr>().ok())
    })
}

fn rpc_method_exposure(method: &str) -> Option<RpcMethodExposure> {
    match method {
        "synergy_subscribe"
        | "synergy_unsubscribe"
        | "synergy_getAccountNonce"
        | "synergy_getAccountAuthNonce"
        | "synergy_blockNumber"
        | "synergy_getBlockNumber"
        | "synergy_getBlockByNumber"
        | "synergy_getBlockByHash"
        | "synergy_getLatestBlock"
        | "synergy_getTransactionPool"
        | "synergy_getRelayerSet"
        | "synergy_getRelayerHealth"
        | "synergy_getSxcpStatus"
        | "synergy_getEventAttestation"
        | "synergy_getAttestations"
        | "synergy_nodeInfo"
        | "synergy_getDeterminismDigest"
        | "synergy_getValidators"
        | "synergy_getValidator"
        | "synergy_getTokenBalance"
        | "synergy_getTokens"
        | "synergy_getTopValidators"
        | "synergy_getBlockRange"
        | "synergy_getTransactionByHash"
        | "synergy_getTransactionsInBlock"
        | "synergy_getValidatorStats"
        | "synergy_getTokenStats"
        | "synergy_getAllBalances"
        | "synergy_getTransferHistory"
        | "synergy_getNodeStatus"
        | "synergy_getSyncStatus"
        | "synergy_getBlockValidationStatus"
        | "synergy_getValidatorActivity"
        | "synergy_getSynergyScoreBreakdown"
        | "synergy_getPeerInfo"
        | "synergy_getTransactionReceipt"
        | "synergy_getTransactionCount"
        | "synergy_getBalance"
        | "synergy_gasPrice"
        | "synergy_getLogs"
        | "synergy_getCode"
        | "synergy_getStorageAt"
        | "synergy_getBlockTransactionCount"
        | "synergy_getBlockReceipts"
        | "synergy_getPendingTransactions"
        | "synergy_getTransactionByBlockNumberAndIndex"
        | "synergy_getTransactionByBlockHashAndIndex"
        | "synergy_maxFeePerGas"
        | "synergy_maxPriorityFeePerGas"
        | "synergy_getFeeHistory"
        | "synergy_getChainId"
        | "synergy_getValidatorByCluster"
        | "synergy_getValidatorRewards"
        | "synergy_getValidatorPerformance"
        | "synergy_getValidatorQueue"
        | "synergy_getValidatorSlashingHistory"
        | "synergy_getClusterInfo"
        | "synergy_getClusterRewards"
        | "synergy_getStakedBalance"
        | "synergy_getStakingInfo"
        | "synergy_getStakingRewards"
        | "synergy_getStakingAPY"
        | "synergy_getDelegatedStakes"
        | "synergy_getDelegators"
        | "synergy_getRewardsProjection"
        | "synergy_getUnstakingPeriod"
        | "synergy_getActiveApprovals"
        | "synergy_getApprovalHistory"
        | "synergy_status" => Some(RpcMethodExposure::PublicRead),
        "synergy_simulateTransaction"
        | "synergy_sendTransaction"
        | "synergy_call"
        | "synergy_estimateGas"
        | "synergy_createApproval"
        | "synergy_revokeAllApprovals" => Some(RpcMethodExposure::PublicClient),
        "synergy_resolveSynID"
        | "synergy_reverseResolveSynID"
        | "synergy_getAddressBook"
        | "synergy_createWallet"
        | "synergy_getWallet"
        | "synergy_createWalletFromKeypair"
        | "synergy_getAllWallets"
        | "synergy_signTransaction"
        | "synergy_signMessage"
        | "synergy_verifyMessage"
        | "synergy_getEncryptionKey"
        | "synergy_rotateKeys"
        | "synergy_getActiveDelegations"
        | "synergy_revokeDelegation"
        | "synergy_initiateRecovery"
        | "synergy_confirmRecovery"
        | "synergy_getGuardians"
        | "synergy_verifyCurrentAuthKey"
        | "synergy_getPendingGuardianNotifications"
        | "synergy_getPendingTransfers"
        | "synergy_cancelPendingTransfer"
        | "synergy_freezeAccount"
        | "synergy_getSecurityAlerts" => Some(RpcMethodExposure::AuthorityPlane),
        "synergy_sendTokens"
        | "synergy_stakeTokens"
        | "synergy_stakeTokensDirect"
        | "synergy_unstakeTokens"
        | "synergy_registerValidator"
        | "synergy_approveValidator"
        | "synergy_slashValidator"
        | "synergy_requestValidatorExit"
        | "synergy_registerRelayer"
        | "synergy_unregisterRelayer"
        | "synergy_relayerHeartbeat"
        | "synergy_submitAttestation"
        | "synergy_slashRelayer"
        | "synergy_createToken"
        | "synergy_mintTokens"
        | "synergy_burnTokens"
        | "synergy_transferTokens"
        | "synergy_claimRewards"
        | "synergy_proposeClusterChange" => Some(RpcMethodExposure::NonPublicWrite),
        "synergy_setSxcpHeartbeatTimeout"
        | "synergy_resetSxcpState"
        | "synergy_mine"
        | "synergy_setAccountBalance"
        | "synergy_resetChainHead" => Some(RpcMethodExposure::Operator),
        _ => None,
    }
}

fn build_exposure_error(
    method: &str,
    exposure: RpcMethodExposure,
    request_context: &RpcRequestContext,
    detail: &str,
) -> RpcError {
    RpcError::with_data(
        -32003,
        format!("RPC method '{method}' is not available on this exposure profile"),
        json!({
            "method": method,
            "requiredProfile": exposure.label(),
            "transport": request_context.transport_label(),
            "clientIp": request_context.effective_client_ip().map(|ip| ip.to_string()),
            "roleId": request_context.role_profile.map(|profile| profile.role_id),
            "compiledProfile": request_context.role_profile.map(|profile| profile.compiled_profile),
            "detail": detail,
        }),
    )
}

fn enforce_rpc_exposure_policy(
    method: &str,
    request_context: &RpcRequestContext,
) -> Result<(), RpcError> {
    let Some(exposure) = rpc_method_exposure(method) else {
        return Ok(());
    };

    if !request_context.is_public_request() {
        return Ok(());
    }

    let is_service_access_role = request_context
        .role_profile
        .map(|profile| profile.authority_plane == AuthorityPlane::ServiceAccess)
        .unwrap_or(false);

    match exposure {
        RpcMethodExposure::PublicRead => Ok(()),
        RpcMethodExposure::PublicClient if is_service_access_role => Ok(()),
        RpcMethodExposure::PublicClient => Err(build_exposure_error(
            method,
            exposure,
            request_context,
            "public client methods are only exposed on service-access node roles",
        )),
        RpcMethodExposure::AuthorityPlane => Err(build_exposure_error(
            method,
            exposure,
            request_context,
            "authority-plane methods must not be exposed on unauthenticated public endpoints",
        )),
        RpcMethodExposure::NonPublicWrite => Err(build_exposure_error(
            method,
            exposure,
            request_context,
            "this state-mutating method is restricted to non-public authenticated routing",
        )),
        RpcMethodExposure::Operator => Err(build_exposure_error(
            method,
            exposure,
            request_context,
            "operator methods require non-public administrative routing and audit controls",
        )),
    }
}

fn request_is_json(headers: &HashMap<String, String>) -> bool {
    headers
        .get("content-type")
        .map(|value| value.to_ascii_lowercase().starts_with("application/json"))
        .unwrap_or(false)
}

fn json_rpc_error_response(id: Option<Value>, error: &RpcError) -> Value {
    let mut payload = serde_json::Map::new();
    payload.insert("code".to_string(), json!(error.code));
    payload.insert("message".to_string(), json!(error.message.clone()));
    if let Some(data) = &error.data {
        payload.insert("data".to_string(), data.clone());
    }

    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "error": Value::Object(payload)
    })
}

fn send_json_rpc_error(
    stream: &mut std::net::TcpStream,
    id: Option<Value>,
    error: &RpcError,
    cors_enabled: bool,
    cors_origins: &[String],
) {
    let response = json_rpc_error_response(id, error);
    let response = format_response(&response.to_string(), cors_enabled, cors_origins);
    let _ = stream.write(response.as_bytes());
}

fn translate_legacy_rpc_result(value: Value) -> Result<Value, RpcError> {
    if let Some(message) = value.as_str() {
        if message == "Unknown method" {
            return Err(RpcError::new(-32601, "Method not found"));
        }
        if message.starts_with("Invalid") || message.starts_with("Missing") {
            return Err(RpcError::new(-32602, message));
        }
    }

    if let Some(map) = value.as_object() {
        if matches!(map.get("success"), Some(Value::Bool(false))) {
            let message = map
                .get("error")
                .and_then(|entry| entry.as_str())
                .unwrap_or("RPC request failed");
            return Err(RpcError::with_data(-32000, message, value.clone()));
        }

        if let Some(error) = map.get("error") {
            let message = error
                .as_str()
                .map(|entry| entry.to_string())
                .unwrap_or_else(|| error.to_string());
            return Err(RpcError::with_data(-32000, message, value.clone()));
        }
    }

    Ok(value)
}

fn current_chain_id() -> u64 {
    crate::config::load_node_config(None)
        .ok()
        .map(|cfg| cfg.blockchain.chain_id)
        .unwrap_or(338639)
}

fn parse_u64ish(value: Option<&Value>) -> Result<Option<u64>, RpcError> {
    let Some(value) = value else {
        return Ok(None);
    };

    match value {
        Value::Null => Ok(None),
        Value::Number(number) => number
            .as_u64()
            .map(Some)
            .ok_or_else(|| RpcError::new(-32602, "Numeric field must be an unsigned integer")),
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            let parsed = if let Some(hex_value) = trimmed.strip_prefix("0x") {
                u64::from_str_radix(hex_value, 16)
            } else {
                trimmed.parse::<u64>()
            };
            parsed.map(Some).map_err(|_| {
                RpcError::new(-32602, format!("Unable to parse integer value '{}'", text))
            })
        }
        _ => Err(RpcError::new(
            -32602,
            "Numeric field must be a number or string",
        )),
    }
}

fn parse_signature_bytes(
    value: Option<&Value>,
    require_signature: bool,
) -> Result<Vec<u8>, RpcError> {
    let Some(value) = value else {
        return if require_signature {
            Err(RpcError::new(-32602, "Missing signature"))
        } else {
            Ok(Vec::new())
        };
    };

    match value {
        Value::String(text) => {
            let normalized = text.trim().strip_prefix("0x").unwrap_or(text.trim());
            if normalized.is_empty() {
                return if require_signature {
                    Err(RpcError::new(-32602, "Missing signature"))
                } else {
                    Ok(Vec::new())
                };
            }
            hex::decode(normalized)
                .map_err(|_| RpcError::new(-32602, "Signature must be valid hex"))
        }
        Value::Array(values) => {
            let mut bytes = Vec::with_capacity(values.len());
            for value in values {
                let byte = value
                    .as_u64()
                    .filter(|entry| *entry <= 255)
                    .ok_or_else(|| RpcError::new(-32602, "Signature array must contain bytes"))?;
                bytes.push(byte as u8);
            }
            Ok(bytes)
        }
        _ => Err(RpcError::new(
            -32602,
            "Signature must be a hex string or byte array",
        )),
    }
}

fn normalize_signature_algorithm(value: Option<&str>) -> Result<String, RpcError> {
    let normalized = value.unwrap_or("fndsa").trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" | "fndsa" | "fn-dsa" | "fn-dsa-512" | "fn-dsa-1024" => Ok("fndsa".to_string()),
        "mldsa" | "ml-dsa" | "ml-dsa-44" | "ml-dsa-65" | "ml-dsa-87" => Ok("mldsa".to_string()),
        "slhdsa" | "slh-dsa" | "slh-dsa-128s" | "slh-dsa-192s" | "slh-dsa-256s" => {
            Ok("slhdsa".to_string())
        }
        _ => Err(RpcError::new(
            -32602,
            format!(
                "Unsupported signature algorithm '{}'",
                value.unwrap_or_default()
            ),
        )),
    }
}

fn normalize_rpc_transaction(
    value: &Value,
    require_signature: bool,
) -> Result<NormalizedEnvelopeResult, RpcError> {
    if let Ok(transaction) = serde_json::from_value::<Transaction>(value.clone()) {
        return Ok(NormalizedEnvelopeResult {
            chain_id: None,
            warnings: Vec::new(),
            transaction,
        });
    }

    let envelope = serde_json::from_value::<RpcTransactionEnvelope>(value.clone())
        .map_err(|_| RpcError::new(-32602, "Invalid transaction format"))?;

    if matches!(
        envelope.envelope_type.as_deref(),
        Some("0x04") | Some("4") | Some("delegation")
    ) || envelope.delegation.is_some()
        || envelope.delegations.is_some()
        || envelope.authorization_list.is_some()
        || matches!(
            envelope.tx_type.as_deref(),
            Some("0x04") | Some("4") | Some("delegation")
        )
    {
        return Err(RpcError::with_data(
            -32014,
            "Delegation-bearing transaction envelopes are not permitted on Synergy",
            json!({"reason": "type-0x04 / delegation payload rejected"}),
        ));
    }

    let sender = envelope
        .from
        .clone()
        .or(envelope.sender.clone())
        .ok_or_else(|| RpcError::new(-32602, "Missing transaction sender"))?;
    let receiver = envelope
        .to
        .clone()
        .or(envelope.receiver.clone())
        .ok_or_else(|| RpcError::new(-32602, "Missing transaction recipient"))?;
    let amount = parse_u64ish(envelope.value.as_ref().or(envelope.amount.as_ref()))?.unwrap_or(0);
    let nonce = envelope
        .nonce
        .ok_or_else(|| RpcError::new(-32602, "Missing transaction nonce"))?;
    let gas_price = parse_u64ish(
        envelope
            .max_fee
            .as_ref()
            .or(envelope.gas_price.as_ref())
            .or(envelope.gas_price_alias.as_ref()),
    )?
    .unwrap_or_else(|| current_gas_price_from_chain(&CHAIN));
    let gas_limit = parse_u64ish(
        envelope
            .gas_limit_alias
            .as_ref()
            .or(envelope.gas_limit.as_ref()),
    )?
    .unwrap_or(crate::gas::constants::GAS_LIMIT_TRANSFER);
    let signature = parse_signature_bytes(envelope.signature.as_ref(), require_signature)?;
    let signature_algorithm = normalize_signature_algorithm(
        envelope
            .signature_algorithm_alias
            .as_deref()
            .or(envelope.signature_algorithm.as_deref()),
    )?;
    let chain_id = parse_u64ish(envelope.chain_id.as_ref())?;

    let normalized = NormalizedRpcTransaction {
        sender,
        receiver,
        amount,
        nonce,
        signature,
        timestamp: envelope.timestamp.unwrap_or_else(current_timestamp),
        gas_price,
        gas_limit,
        data: envelope.data.clone(),
        signature_algorithm,
        chain_id,
    };

    let mut warnings = Vec::new();
    if envelope.max_priority_fee_per_gas.is_some() {
        warnings.push(
            "maxPriorityFeePerGas is accepted for compatibility but not used by the current fee model"
                .to_string(),
        );
    }

    if normalized.amount == 0
        && normalized
            .data
            .as_deref()
            .map(|value| !value.is_empty() && value != "0x")
            .unwrap_or(false)
    {
        warnings.push(
            "Zero-value contract calls remain subject to the current AIVM execution limitations"
                .to_string(),
        );
    }

    let transaction = Transaction {
        sender: normalized.sender,
        receiver: normalized.receiver,
        amount: normalized.amount,
        nonce: normalized.nonce,
        signature: normalized.signature,
        timestamp: normalized.timestamp,
        gas_price: normalized.gas_price,
        gas_limit: normalized.gas_limit,
        data: normalized.data,
        signature_algorithm: normalized.signature_algorithm,
    };

    Ok(NormalizedEnvelopeResult {
        transaction,
        warnings,
        chain_id: normalized.chain_id,
    })
}

fn estimate_gas_for_transaction(transaction: &Transaction) -> u64 {
    use crate::gas::GasEstimator;

    if transaction.receiver.is_empty() || transaction.receiver == "0x0" {
        let bytecode_size = transaction
            .data
            .as_deref()
            .map(|data| {
                let data = data.strip_prefix("0x").unwrap_or(data);
                data.len() / 2
            })
            .unwrap_or(0);
        GasEstimator::estimate_contract_deploy(bytecode_size).as_u64()
    } else if transaction
        .data
        .as_deref()
        .map(|data| !data.is_empty() && data != "0x")
        .unwrap_or(false)
    {
        let calldata_size = transaction
            .data
            .as_deref()
            .map(|data| {
                let data = data.strip_prefix("0x").unwrap_or(data);
                data.len() / 2
            })
            .unwrap_or(0);
        GasEstimator::estimate_contract_call(calldata_size).as_u64()
    } else {
        GasEstimator::estimate_transfer().as_u64()
    }
}

fn dynamic_gas_price(chain: &BlockChain) -> u64 {
    use crate::gas::constants::{DEFAULT_GAS_PRICE, MAX_GAS_PRICE, MIN_GAS_PRICE};

    let recent_blocks: Vec<_> = chain.chain.iter().rev().take(10).collect();
    if recent_blocks.is_empty() {
        return DEFAULT_GAS_PRICE;
    }

    let mut total_gas_used: u64 = 0;
    let block_gas_limit = crate::gas::constants::BLOCK_GAS_LIMIT;
    for block in &recent_blocks {
        let block_gas: u64 = block.transactions.iter().map(|tx| tx.get_fee()).sum();
        total_gas_used += block_gas;
    }

    let avg_gas_per_block = total_gas_used / recent_blocks.len() as u64;
    let utilization = avg_gas_per_block as f64 / block_gas_limit as f64;
    let gas_price = if utilization > 0.8 {
        (DEFAULT_GAS_PRICE as f64 * (1.0 + utilization)) as u64
    } else {
        DEFAULT_GAS_PRICE
    };

    gas_price.max(MIN_GAS_PRICE).min(MAX_GAS_PRICE)
}

fn current_gas_price_from_chain(chain: &Arc<Mutex<BlockChain>>) -> u64 {
    let chain = chain.lock().unwrap();
    dynamic_gas_price(&chain)
}

fn get_account_nonce(
    params: &Value,
    tx_pool: &Arc<Mutex<Vec<Transaction>>>,
    chain: &Arc<Mutex<BlockChain>>,
) -> Result<Value, RpcError> {
    let address = params
        .get(0)
        .and_then(|value| value.as_str())
        .ok_or_else(|| RpcError::new(-32602, "Missing address parameter"))?;

    let mut next_nonce = 0u64;

    if let Ok(wallet_manager) = WALLET_MANAGER.lock() {
        if let Some(wallet) = wallet_manager.get_wallet(address) {
            next_nonce = next_nonce.max(wallet.nonce);
        }
    }

    {
        let chain = chain.lock().unwrap();
        for block in &chain.chain {
            for tx in &block.transactions {
                if tx.sender.eq_ignore_ascii_case(address) {
                    next_nonce = next_nonce.max(tx.nonce.saturating_add(1));
                }
            }
        }
    }

    {
        let pool = tx_pool.lock().unwrap();
        for tx in pool.iter() {
            if tx.sender.eq_ignore_ascii_case(address) {
                next_nonce = next_nonce.max(tx.nonce.saturating_add(1));
            }
        }
    }

    Ok(json!(next_nonce))
}

fn simulate_transaction(
    params: &Value,
    _tx_pool: &Arc<Mutex<Vec<Transaction>>>,
    chain: &Arc<Mutex<BlockChain>>,
) -> Result<Value, RpcError> {
    let transaction_value = params
        .get(0)
        .ok_or_else(|| RpcError::new(-32602, "Missing transaction parameter"))?;
    let normalized = normalize_rpc_transaction(transaction_value, false)?;

    let configured_chain_id = current_chain_id();
    if let Some(chain_id) = normalized.chain_id {
        if chain_id != configured_chain_id {
            return Err(RpcError::with_data(
                -32015,
                "Simulation chainId does not match the local chain",
                json!({
                    "expected": format!("0x{:x}", configured_chain_id),
                    "actual": format!("0x{:x}", chain_id)
                }),
            ));
        }
    }

    let gas = estimate_gas_for_transaction(&normalized.transaction);
    let network_gas_price = current_gas_price_from_chain(chain);
    let safe_fee = gas.saturating_mul(network_gas_price);
    let max_fee = gas.saturating_mul(normalized.transaction.gas_price);
    let sender_balance = TOKEN_MANAGER
        .clone()
        .get_balance(&normalized.transaction.sender, "SNRG");
    let total_cost = normalized.transaction.amount.saturating_add(max_fee);

    let mut warnings = normalized.warnings.clone();
    let mut divergence = false;
    if normalized
        .transaction
        .data
        .as_deref()
        .map(|value| !value.is_empty() && value != "0x")
        .unwrap_or(false)
    {
        divergence = true;
        warnings.push(
            "AIVM contract execution is currently disabled, so contract-side effects could not be fully simulated"
                .to_string(),
        );
    }

    if sender_balance < total_cost {
        warnings.push(format!(
            "Sender balance {} is below the projected total cost {}",
            sender_balance, total_cost
        ));
    }

    let asset_flows = if normalized.transaction.amount > 0 {
        vec![json!({
            "asset": "SNRG",
            "from": normalized.transaction.sender,
            "to": normalized.transaction.receiver,
            "amount": normalized.transaction.amount
        })]
    } else {
        Vec::new()
    };

    let tx_digest =
        canonical_value_digest(transaction_value).unwrap_or_else(|| normalized.transaction.hash());
    let preview = json!({
        "accepted": sender_balance >= total_cost,
        "chainId": format!("0x{:x}", configured_chain_id),
        "txDigest": tx_digest,
        "gas": gas,
        "safeFee": safe_fee,
        "maxFee": max_fee,
        "assetFlows": asset_flows,
        "approvals": [],
        "delegations": [],
        "warnings": warnings,
        "divergence": divergence
    });
    let simulation_hash = canonical_value_digest(&preview)
        .unwrap_or_else(|| hex::encode(blake3::hash(preview.to_string().as_bytes()).as_bytes()));

    {
        let mut cache = SIMULATION_CACHE.lock().unwrap();
        cache.retain(|_, entry| current_timestamp().saturating_sub(entry.created_at) <= 900);
        cache.insert(
            preview
                .get("txDigest")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string(),
            CachedSimulation {
                simulation_hash: simulation_hash.clone(),
                created_at: current_timestamp(),
            },
        );
    }

    Ok(json!({
        "simulationHash": simulation_hash,
        "transactionHashPreview": normalized.transaction.hash(),
        "result": preview
    }))
}

fn register_subscription(
    params: &Value,
    chain: &Arc<Mutex<BlockChain>>,
    tx_pool: &Arc<Mutex<Vec<Transaction>>>,
    subscriptions: &mut HashMap<String, SubscriptionCursor>,
) -> Result<Value, RpcError> {
    let subscription_type = params
        .get(0)
        .and_then(|value| value.as_str())
        .ok_or_else(|| RpcError::new(-32602, "Missing subscription type"))?;
    let current_height = {
        let chain = chain.lock().unwrap();
        chain.last().map(|block| block.block_index).unwrap_or(0)
    };
    let filter = params.get(1).cloned().unwrap_or(Value::Null);

    let cursor = match subscription_type {
        "newHeads" => SubscriptionCursor::NewHeads {
            last_block: current_height,
        },
        "logs" => SubscriptionCursor::Logs {
            last_block: current_height,
            address: filter
                .get("address")
                .and_then(|value| value.as_str())
                .map(|value| value.to_ascii_lowercase()),
            topics: filter
                .get("topics")
                .and_then(|value| value.as_array())
                .map(|values| {
                    values
                        .iter()
                        .filter_map(|value| value.as_str().map(|entry| entry.to_ascii_lowercase()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        },
        "pendingTransactions" => {
            let seen_hashes = tx_pool
                .lock()
                .unwrap()
                .iter()
                .map(|transaction| transaction.hash())
                .collect();
            SubscriptionCursor::PendingTransactions { seen_hashes }
        }
        "validatorEvents" => SubscriptionCursor::ValidatorEvents {
            last_block: current_height,
        },
        _ => {
            return Err(RpcError::new(
                -32602,
                format!("Unsupported subscription type '{}'", subscription_type),
            ));
        }
    };

    let subscription_id = format!(
        "0x{:016x}",
        SUBSCRIPTION_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    subscriptions.insert(subscription_id.clone(), cursor);
    Ok(json!(subscription_id))
}

fn unregister_subscription(
    params: &Value,
    subscriptions: &mut HashMap<String, SubscriptionCursor>,
) -> Result<Value, RpcError> {
    let subscription_id = params
        .get(0)
        .and_then(|value| value.as_str())
        .ok_or_else(|| RpcError::new(-32602, "Missing subscriptionId parameter"))?;
    Ok(json!(subscriptions.remove(subscription_id).is_some()))
}

fn emit_subscription_notifications(
    websocket: &mut tungstenite::WebSocket<std::net::TcpStream>,
    subscriptions: &mut HashMap<String, SubscriptionCursor>,
    tx_pool: &Arc<Mutex<Vec<Transaction>>>,
    chain: &Arc<Mutex<BlockChain>>,
) {
    if subscriptions.is_empty() {
        return;
    }

    let subscription_ids: Vec<String> = subscriptions.keys().cloned().collect();
    for subscription_id in subscription_ids {
        let Some(cursor) = subscriptions.get_mut(&subscription_id) else {
            continue;
        };

        match cursor {
            SubscriptionCursor::NewHeads { last_block } => {
                let blocks = {
                    let chain = chain.lock().unwrap();
                    chain
                        .chain
                        .iter()
                        .filter(|block| block.block_index > *last_block)
                        .cloned()
                        .collect::<Vec<_>>()
                };

                for block in blocks {
                    *last_block = block.block_index;
                    let notification = json!({
                        "jsonrpc": "2.0",
                        "method": "synergy_subscription",
                        "params": {
                            "subscription": subscription_id,
                            "result": {
                                "block_index": block.block_index,
                                "hash": block.hash,
                                "parent_hash": block.previous_hash,
                                "timestamp": block.timestamp,
                                "validator": block.validator_id,
                                "tx_count": block.transactions.len()
                            }
                        }
                    });
                    if websocket
                        .send(WsMessage::Text(notification.to_string()))
                        .is_err()
                    {
                        return;
                    }
                }
            }
            SubscriptionCursor::Logs {
                last_block,
                address,
                topics,
            } => {
                let blocks = {
                    let chain = chain.lock().unwrap();
                    chain
                        .chain
                        .iter()
                        .filter(|block| block.block_index > *last_block)
                        .cloned()
                        .collect::<Vec<_>>()
                };

                for block in blocks {
                    *last_block = block.block_index;
                    for log in collect_logs_for_block(&block, address.as_deref(), topics) {
                        let notification = json!({
                            "jsonrpc": "2.0",
                            "method": "synergy_subscription",
                            "params": {
                                "subscription": subscription_id,
                                "result": log
                            }
                        });
                        if websocket
                            .send(WsMessage::Text(notification.to_string()))
                            .is_err()
                        {
                            return;
                        }
                    }
                }
            }
            SubscriptionCursor::PendingTransactions { seen_hashes } => {
                let pending_transactions =
                    tx_pool.lock().unwrap().iter().cloned().collect::<Vec<_>>();

                for transaction in pending_transactions {
                    let hash = transaction.hash();
                    if seen_hashes.insert(hash.clone()) {
                        let notification = json!({
                            "jsonrpc": "2.0",
                            "method": "synergy_subscription",
                            "params": {
                                "subscription": subscription_id,
                                "result": hash
                            }
                        });
                        if websocket
                            .send(WsMessage::Text(notification.to_string()))
                            .is_err()
                        {
                            return;
                        }
                    }
                }
            }
            SubscriptionCursor::ValidatorEvents { last_block } => {
                let blocks = {
                    let chain = chain.lock().unwrap();
                    chain
                        .chain
                        .iter()
                        .filter(|block| block.block_index > *last_block)
                        .cloned()
                        .collect::<Vec<_>>()
                };

                for block in blocks {
                    *last_block = block.block_index;
                    let notification = json!({
                        "jsonrpc": "2.0",
                        "method": "synergy_subscription",
                        "params": {
                            "subscription": subscription_id,
                            "result": {
                                "event": "blockAccepted",
                                "block_index": block.block_index,
                                "validator": block.validator_id,
                                "hash": block.hash
                            }
                        }
                    });
                    if websocket
                        .send(WsMessage::Text(notification.to_string()))
                        .is_err()
                    {
                        return;
                    }
                }
            }
        }
    }
}

fn collect_logs_for_block(
    block: &crate::block::Block,
    address_filter: Option<&str>,
    topics_filter: &[String],
) -> Vec<Value> {
    if !topics_filter.is_empty() {
        return Vec::new();
    }

    let mut logs = Vec::new();
    for (tx_index, transaction) in block.transactions.iter().enumerate() {
        if let Some(address_filter) = address_filter {
            if !transaction.sender.eq_ignore_ascii_case(address_filter)
                && !transaction.receiver.eq_ignore_ascii_case(address_filter)
            {
                continue;
            }
        }

        if transaction.data.is_none() && address_filter.is_none() {
            continue;
        }

        logs.push(json!({
            "logIndex": logs.len(),
            "transactionIndex": tx_index,
            "transactionHash": transaction.hash(),
            "blockHash": block.hash.clone(),
            "blockNumber": block.block_index,
            "address": transaction.receiver.clone(),
            "data": transaction.data.clone().unwrap_or_else(|| "0x".to_string()),
            "topics": [],
            "removed": false
        }));
    }

    logs
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn canonicalize_json_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<String> = map.keys().cloned().collect();
            keys.sort();

            let mut ordered = serde_json::Map::new();
            for key in keys {
                if let Some(child) = map.get(&key) {
                    ordered.insert(key, canonicalize_json_value(child));
                }
            }
            Value::Object(ordered)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(canonicalize_json_value).collect()),
        _ => value.clone(),
    }
}

fn canonical_value_digest(value: &Value) -> Option<String> {
    let canonical = canonicalize_json_value(value);
    let bytes = serde_json::to_vec(&canonical).ok()?;
    Some(hex::encode(blake3::hash(&bytes).as_bytes()))
}

fn stable_json_file_digest(path: &str) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let parsed: Value = serde_json::from_str(&content).ok()?;
    canonical_value_digest(&parsed)
}

fn compute_receipt_hash(chain: &BlockChain) -> String {
    let mut hasher = blake3::Hasher::new();
    for block in &chain.chain {
        hasher.update(block.hash.as_bytes());
        for tx in &block.transactions {
            hasher.update(tx.hash().as_bytes());
        }
    }
    hex::encode(hasher.finalize().as_bytes())
}

fn select_cors_origin(cors_origins: &[String]) -> String {
    if cors_origins.iter().any(|origin| origin == "*") {
        return "*".to_string();
    }

    cors_origins
        .iter()
        .find(|origin| !origin.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| "*".to_string())
}

fn format_response(body: &str, cors_enabled: bool, cors_origins: &[String]) -> String {
    format_http_response(
        "200 OK",
        "application/json",
        body,
        cors_enabled,
        cors_origins,
    )
}

fn format_text_response(body: &str, cors_enabled: bool, cors_origins: &[String]) -> String {
    format_http_response(
        "200 OK",
        "text/plain; charset=utf-8",
        body,
        cors_enabled,
        cors_origins,
    )
}

fn format_not_found_response(cors_enabled: bool, cors_origins: &[String]) -> String {
    format_http_response(
        "404 Not Found",
        "text/plain; charset=utf-8",
        "not found\n",
        cors_enabled,
        cors_origins,
    )
}

fn format_http_response(
    status: &str,
    content_type: &str,
    body: &str,
    cors_enabled: bool,
    cors_origins: &[String],
) -> String {
    if cors_enabled {
        let origin = select_cors_origin(cors_origins);
        return format!(
            "HTTP/1.1 {}\r\nContent-Type: {}\r\nAccess-Control-Allow-Origin: {}\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nContent-Length: {}\r\n\r\n{}",
            status,
            content_type,
            origin,
            body.len(),
            body
        );
    }

    format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}",
        status,
        content_type,
        body.len(),
        body
    )
}

fn format_cors_preflight_response(cors_enabled: bool, cors_origins: &[String]) -> String {
    if !cors_enabled {
        return "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n".to_string();
    }

    let origin = select_cors_origin(cors_origins);
    format!(
        "HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: {}\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nContent-Length: 0\r\n\r\n",
        origin
    )
}

fn calculate_average_block_time(chain: &BlockChain) -> f64 {
    let recent_blocks: Vec<_> = chain.chain.iter().rev().take(20).collect();
    if recent_blocks.len() < 2 {
        return 0.0;
    }

    let mut total_diff = 0u64;
    let mut count = 0u64;

    for window in recent_blocks.windows(2) {
        let newer = window[0];
        let older = window[1];
        if newer.timestamp > older.timestamp {
            total_diff += newer.timestamp - older.timestamp;
            count += 1;
        }
    }

    if count == 0 {
        return 0.0;
    }

    total_diff as f64 / count as f64
}

fn tx_to_explorer_json(
    tx: &Transaction,
    status: &str,
    block_number: Option<u64>,
    tx_index: Option<usize>,
) -> Value {
    // Convert amount from nWei to SNRG for display (per SNTS-04: 1 SNRG = 1,000,000,000 nWei)
    use crate::gas::constants::NWEI_PER_SNRG;
    let amount_snrg = tx.amount as f64 / NWEI_PER_SNRG as f64;

    json!({
        "hash": tx.hash(),
        "sender": tx.sender.clone(),
        "receiver": tx.receiver.clone(),
        "from": tx.sender.clone(), // explorer-friendly alias
        "to": tx.receiver.clone(), // explorer-friendly alias
        "amount": tx.amount, // amount in nWei (for compatibility)
        "amount_snrg": amount_snrg, // amount in SNRG (for explorer display)
        "nonce": tx.nonce,
        "gas_price": tx.gas_price,
        "gas_limit": tx.gas_limit,
        "fee": tx.get_fee(),
        "timestamp": tx.timestamp,
        "data": tx.data.clone(),
        "signature_algorithm": tx.signature_algorithm.clone(),
        "signature": hex::encode(&tx.signature),
        "status": status,
        "block_number": block_number,
        "transaction_index": tx_index
    })
}

fn block_to_explorer_json(block: &crate::block::Block) -> Value {
    let txs: Vec<Value> = block
        .transactions
        .iter()
        .enumerate()
        .map(|(idx, tx)| tx_to_explorer_json(tx, "confirmed", Some(block.block_index), Some(idx)))
        .collect();

    json!({
        "block_index": block.block_index,
        "timestamp": block.timestamp,
        "hash": block.hash.clone(),
        "previous_hash": block.previous_hash.clone(),
        "parent_hash": block.previous_hash.clone(), // explorer-friendly alias
        "validator_id": block.validator_id.clone(),
        "validator": block.validator_id.clone(), // explorer-friendly alias
        "nonce": block.nonce,
        "tx_count": block.transactions.len() as u64,
        "transactions": txs
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::{Block, BlockChain};

    #[test]
    fn normalize_spec_transaction_envelope_maps_to_internal_transaction() {
        let envelope = json!({
            "from": "syna1sender",
            "to": "syna1receiver",
            "value": 42,
            "nonce": 7,
            "gasLimit": 21000,
            "maxFee": 1000,
            "signature": "0x01020304",
            "signatureAlgorithm": "FN-DSA-1024",
            "chainId": "0x1234"
        });

        let normalized =
            normalize_rpc_transaction(&envelope, true).expect("envelope should normalize");
        assert_eq!(normalized.transaction.sender, "syna1sender");
        assert_eq!(normalized.transaction.receiver, "syna1receiver");
        assert_eq!(normalized.transaction.amount, 42);
        assert_eq!(normalized.transaction.nonce, 7);
        assert_eq!(normalized.transaction.gas_limit, 21000);
        assert_eq!(normalized.transaction.gas_price, 1000);
        assert_eq!(normalized.transaction.signature, vec![1, 2, 3, 4]);
        assert_eq!(normalized.transaction.signature_algorithm, "fndsa");
        assert_eq!(normalized.chain_id, Some(0x1234));
    }

    #[test]
    fn normalize_transaction_rejects_delegation_payloads() {
        let envelope = json!({
            "from": "syna1sender",
            "to": "syna1receiver",
            "value": 1,
            "nonce": 1,
            "signature": "0x01",
            "type": "0x04"
        });

        let error =
            normalize_rpc_transaction(&envelope, true).expect_err("delegations must be rejected");
        assert_eq!(error.code, -32014);
    }

    #[test]
    fn translate_legacy_result_promotes_embedded_errors() {
        let legacy = json!({
            "success": false,
            "error": "boom"
        });

        let error =
            translate_legacy_rpc_result(legacy).expect_err("legacy error should map to RpcError");
        assert_eq!(error.code, -32000);
        assert_eq!(error.message, "boom");
    }

    #[test]
    fn request_is_json_recognizes_application_json() {
        let headers = parse_http_headers(
            "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json; charset=utf-8\r\n\r\n",
        );
        assert!(request_is_json(&headers));
    }

    #[test]
    fn forwarded_ip_prefers_proxy_header() {
        let mut headers = HashMap::new();
        headers.insert(
            "x-forwarded-for".to_string(),
            "198.51.100.22, 127.0.0.1".to_string(),
        );

        let context = RpcRequestContext {
            transport: RpcTransport::Http,
            peer_addr: Some("127.0.0.1:5646".parse().unwrap()),
            headers,
            role_profile: None,
        };

        assert_eq!(
            context
                .effective_client_ip()
                .expect("forwarded ip should be present")
                .to_string(),
            "198.51.100.22"
        );
        assert!(context.is_public_request());
    }

    #[test]
    fn forwarded_ip_accepts_cloudflare_header() {
        let mut headers = HashMap::new();
        headers.insert("cf-connecting-ip".to_string(), "198.51.100.44".to_string());
        headers.insert(
            "x-forwarded-for".to_string(),
            "127.0.0.1, 127.0.0.1".to_string(),
        );

        let context = RpcRequestContext {
            transport: RpcTransport::WebSocket,
            peer_addr: Some("127.0.0.1:5666".parse().unwrap()),
            headers,
            role_profile: crate::role_profiles::profile_from_compiled_profile("rpc_gateway_node"),
        };

        assert_eq!(
            context
                .effective_client_ip()
                .expect("cloudflare header should be present")
                .to_string(),
            "198.51.100.44"
        );
        assert!(context.is_public_request());
        let error = enforce_rpc_exposure_policy("synergy_createWallet", &context)
            .expect_err("authority-plane method should be denied");
        assert_eq!(error.code, -32003);
    }

    #[test]
    fn public_proxy_denies_authority_plane_methods() {
        let mut headers = HashMap::new();
        headers.insert("x-forwarded-for".to_string(), "198.51.100.22".to_string());
        let context = RpcRequestContext {
            transport: RpcTransport::Http,
            peer_addr: Some("127.0.0.1:5646".parse().unwrap()),
            headers,
            role_profile: crate::role_profiles::profile_from_compiled_profile("rpc_gateway_node"),
        };

        let error = enforce_rpc_exposure_policy("synergy_createWallet", &context)
            .expect_err("authority-plane method should be denied");
        assert_eq!(error.code, -32003);
        assert!(error.message.contains("exposure profile"));
    }

    #[test]
    fn public_gateway_allows_canonical_client_pipeline() {
        let mut headers = HashMap::new();
        headers.insert("x-forwarded-for".to_string(), "198.51.100.22".to_string());
        let context = RpcRequestContext {
            transport: RpcTransport::Http,
            peer_addr: Some("127.0.0.1:5646".parse().unwrap()),
            headers,
            role_profile: crate::role_profiles::profile_from_compiled_profile("rpc_gateway_node"),
        };

        enforce_rpc_exposure_policy("synergy_sendTransaction", &context)
            .expect("canonical public client method should be allowed");
        enforce_rpc_exposure_policy("synergy_simulateTransaction", &context)
            .expect("simulation should be allowed on the public client surface");
    }

    #[test]
    fn loopback_allows_non_public_write_methods() {
        let context = RpcRequestContext {
            transport: RpcTransport::Http,
            peer_addr: Some("127.0.0.1:5646".parse().unwrap()),
            headers: HashMap::new(),
            role_profile: crate::role_profiles::profile_from_compiled_profile("validator_node"),
        };

        enforce_rpc_exposure_policy("synergy_sendTokens", &context)
            .expect("loopback traffic should retain access to local write methods");
        enforce_rpc_exposure_policy("synergy_resetSxcpState", &context)
            .expect("loopback traffic should retain access to operator methods");
    }

    #[test]
    fn prune_confirmed_transactions_from_pool_removes_only_matching_hashes() {
        let tx_a = Transaction::new(
            "syna1sendera".to_string(),
            "syna1receivera".to_string(),
            1,
            0,
            vec![1, 2, 3],
            1000,
            21000,
            None,
            "fndsa".to_string(),
        );
        let tx_b = Transaction::new(
            "syna1senderb".to_string(),
            "syna1receiverb".to_string(),
            2,
            1,
            vec![4, 5, 6],
            1000,
            21000,
            None,
            "fndsa".to_string(),
        );

        {
            let mut pool = TX_POOL.lock().unwrap();
            pool.clear();
            pool.push(tx_a.clone());
            pool.push(tx_b.clone());
        }

        let pruned = prune_transaction_hashes_from_pool(&transaction_hashes(&[tx_a]));
        assert_eq!(pruned, 1);

        let remaining_hashes = TX_POOL
            .lock()
            .unwrap()
            .iter()
            .map(|transaction| transaction.hash())
            .collect::<Vec<_>>();
        assert_eq!(remaining_hashes, vec![tx_b.hash()]);

        TX_POOL.lock().unwrap().clear();
    }

    #[test]
    fn network_validator_snapshot_uses_canonical_genesis_for_read_only_nodes() {
        let genesis_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../config/genesis.json")
            .canonicalize()
            .expect("repo genesis path should resolve");
        std::env::set_var("SYNERGY_GENESIS_FILE", genesis_path);
        let genesis = canonical_genesis().expect("canonical genesis must load");
        let first_validator = genesis
            .validators()
            .first()
            .expect("canonical genesis should define validators");

        let mut chain = BlockChain::new();
        chain.genesis().expect("genesis block should load");
        chain.add_block(Block::new_with_timestamp(
            1,
            Vec::new(),
            chain.last().unwrap().hash.clone(),
            first_validator.operator_address.clone(),
            1,
            genesis.timestamp().saturating_add(2),
        ));

        let validator_manager = ValidatorManager::new();
        let validators = network_validator_snapshot(&chain, &validator_manager);
        let matched = validators
            .into_iter()
            .find(|validator| validator.address == first_validator.operator_address)
            .expect("canonical validator should be present in synthesized snapshot");

        assert_eq!(matched.name, first_validator.moniker);
        assert_eq!(matched.stake_amount, first_validator.stake_nwei);
        assert_eq!(matched.status, ValidatorStatus::Active);
        assert_eq!(matched.total_blocks_produced, 1);
    }
}
