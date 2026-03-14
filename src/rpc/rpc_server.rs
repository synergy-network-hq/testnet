use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::block::BlockChain;
use crate::consensus::synergy_score::SynergyScoreCalculator;
use crate::crypto::pqc::PQCManager;
use crate::sxcp;
use crate::sync::{SyncManager, SyncState};
use crate::token::TOKEN_MANAGER;
use crate::transaction::Transaction;
use crate::validator::{ValidatorManager, VALIDATOR_MANAGER};
use crate::wallet::WALLET_MANAGER;
// Temporarily disabled for quick compile
// use crate::aivm::AIVMRuntime;
// use crate::aivm::runtime::{ContractType, AIVMExecutionContext};
use hex;
use lazy_static::lazy_static;
use serde_json::{json, Value};

lazy_static! {
    pub static ref TX_POOL: Arc<Mutex<Vec<Transaction>>> = Arc::new(Mutex::new(Vec::new()));
}

lazy_static! {
    static ref NODE_START_TIME: Arc<Mutex<Option<u64>>> = Arc::new(Mutex::new(None));
}

// Global shared blockchain instance - will be used by both RPC and consensus
lazy_static! {
    pub static ref SHARED_CHAIN: Arc<Mutex<BlockChain>> = {
        // Use absolute path based on project root
        let chain_path = crate::utils::resolve_data_path("data/chain.json");
        Arc::new(Mutex::new(
            BlockChain::load_from_file(chain_path.to_str().unwrap_or("data/chain.json")).unwrap_or_else(|| {
                let mut chain = BlockChain::new();
                chain.genesis();
                chain
            })
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

pub fn start_rpc_server(bind_address: &str, cors_enabled: bool, cors_origins: Vec<String>) {
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

    for stream in TcpListener::bind(bind_address)
        .expect("Failed to bind RPC server")
        .incoming()
    {
        let tx_pool = Arc::clone(&TX_POOL);
        let chain = Arc::clone(&CHAIN);
        let validator_manager = Arc::clone(&VALIDATOR_MANAGER);
        let cors_enabled_for_conn = cors_enabled;
        let cors_origins_for_conn = cors_origins.clone();
        // Temporarily disabled AIVM for quick compile
        // let aivm_runtime = Arc::clone(&AIVM_RUNTIME);

        thread::spawn(move || {
            if let Ok(mut stream) = stream {
                let mut buffer = [0; 16384];
                if let Ok(bytes_read) = stream.read(&mut buffer) {
                    let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);

                    // Handle CORS preflight
                    if request_str.starts_with("OPTIONS") {
                        let response_str = format_cors_preflight_response(
                            cors_enabled_for_conn,
                            &cors_origins_for_conn,
                        );
                        let _ = stream.write(response_str.as_bytes());
                        let _ = stream.flush();
                        return;
                    }

                    // Split headers and body
                    let parts: Vec<&str> = request_str.split("\r\n\r\n").collect();
                    if parts.len() < 2 {
                        send_error(
                            &mut stream,
                            "Malformed HTTP request",
                            cors_enabled_for_conn,
                            &cors_origins_for_conn,
                        );
                        return;
                    }

                    let body = parts[1];

                    if request_str.starts_with("POST") {
                        match serde_json::from_str::<Value>(body) {
                            Ok(parsed) => {
                                let method =
                                    parsed.get("method").and_then(|m| m.as_str()).unwrap_or("");
                                let params = parsed.get("params").cloned().unwrap_or(json!([]));
                                let id = parsed.get("id").cloned().unwrap_or(json!(null));

                                let result = handle_json_rpc(
                                    method,
                                    params,
                                    &tx_pool,
                                    &chain,
                                    &validator_manager,
                                );

                                let response = json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": result
                                });

                                let response_str = format_response(
                                    &response.to_string(),
                                    cors_enabled_for_conn,
                                    &cors_origins_for_conn,
                                );
                                let _ = stream.write(response_str.as_bytes());
                                let _ = stream.flush();
                            }
                            Err(_) => send_error(
                                &mut stream,
                                "Malformed JSON in body",
                                cors_enabled_for_conn,
                                &cors_origins_for_conn,
                            ),
                        }
                    }
                }
            }
        });
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
                match serde_json::from_value::<Transaction>(tx_data.clone()) {
                    Ok(tx) => {
                        match tx.validate() {
                            crate::transaction::TransactionValidationResult {
                                is_valid: true,
                                ..
                            } => {
                                let mut pool = tx_pool.lock().unwrap();
                                let tx_hash = tx.hash();
                                pool.push(tx.clone());

                                // Best-effort gossip to peers.
                                if let Some(p2p) = crate::p2p::get_p2p_network() {
                                    p2p.broadcast_transaction(&tx);
                                }

                                json!({"success": true, "tx_hash": tx_hash, "message": "Transaction submitted"})
                            }
                            crate::transaction::TransactionValidationResult {
                                error_message: Some(msg),
                                ..
                            } => {
                                json!({"error": msg})
                            }
                            _ => {
                                json!("Invalid transaction")
                            }
                        }
                    }
                    Err(_) => json!("Invalid transaction format"),
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
        // SXCP (Synergy Cross-Chain Protocol) – Devnet RPC surface
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
                .map(|token| token == "DEVNET_RESET_SXCP_STATE")
                .unwrap_or(false)
            {
                sxcp::reset_state()
            } else {
                json!({
                    "success": false,
                    "error": "Confirmation token required as first parameter: DEVNET_RESET_SXCP_STATE"
                })
            }
        }

        // Node status
        "synergy_nodeInfo" => {
            let chain = chain.lock().unwrap();
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
                "currentBlock": chain.last().map_or(0, |b| b.block_index),
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
            let validators = validator_manager.get_active_validators();
            println!(
                "🔍 [RPC] synergy_getValidators called, returning {} validators",
                validators.len()
            );
            json!(validators)
        }

        "synergy_getValidator" => {
            if let Some(address) = params.get(0).and_then(|v| v.as_str()) {
                match validator_manager.get_validator(address) {
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
                    if let Ok(mut wallet_manager) = WALLET_MANAGER.lock() {
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
                            json!({"success": true, "transaction": transaction, "message": "Staking transaction created successfully"})
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
            json!(validator_manager.get_top_validators(count))
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
            let active_validators = validator_manager.get_active_validators();
            let top_validators = validator_manager.get_top_validators(20);

            json!({
                "total_validators": active_validators.len(),
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
                token_stats.push(json!({
                    "symbol": token.symbol,
                    "name": token.name,
                    "total_supply": token.total_supply,
                    "total_staked": total_staked,
                    "holders": token_manager.balances.lock().unwrap().keys()
                        .filter(|addr| token_manager.get_balance(addr, &token.symbol) > 0)
                        .count()
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

            let total_supply = token_manager
                .get_all_tokens()
                .iter()
                .map(|token| token.total_supply)
                .sum::<u64>();

            json!({
                "block_height": chain.last().map_or(0, |b| b.block_index),
                "total_transactions": chain.chain.iter().map(|b| b.transactions.len()).sum::<usize>(),
                "active_validators": validator_manager.get_active_validators().len(),
                "total_supply": total_supply,
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
            let chain = chain.lock().unwrap();
            let avg_block_time = calculate_average_block_time(&chain);
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
                "last_block": chain.last().map_or(0, |b| b.block_index),
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
                "active_validators": validator_manager.get_active_validators().len(),
                "total_validators": validator_manager.get_validator_count(),
                "cluster_info": {
                    "active_clusters": validator_manager.get_cluster_count(),
                    "total_stake": validator_manager.get_total_stake()
                }
            })
        }

        "synergy_getValidatorActivity" => {
            let active_validators = validator_manager.get_active_validators();
            let mut validator_activity = Vec::new();

            for validator in active_validators {
                validator_activity.push(json!({
                    "address": validator.address,
                    "name": validator.name,
                    "synergy_score": validator.synergy_score,
                    "blocks_produced": validator.total_blocks_produced,
                    "uptime": format!("{:.1}%", validator.uptime_percentage),
                    "cluster_id": validator.cluster_id,
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

        // Legacy support
        "synergy_status" => {
            json!("ok")
        }

        _ => {
            json!("Unknown method")
        }
    }
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
    if cors_enabled {
        let origin = select_cors_origin(cors_origins);
        return format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: {}\r\nAccess-Control-Allow-Methods: POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nContent-Length: {}\r\n\r\n{}",
            origin,
            body.len(),
            body
        );
    }

    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
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
        "HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: {}\r\nAccess-Control-Allow-Methods: POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nContent-Length: 0\r\n\r\n",
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

fn send_error(
    stream: &mut std::net::TcpStream,
    msg: &str,
    cors_enabled: bool,
    cors_origins: &[String],
) {
    let body = format!("{{\"error\": \"{}\"}}", msg);
    let response = format_response(&body, cors_enabled, cors_origins);
    let _ = stream.write(response.as_bytes());
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
