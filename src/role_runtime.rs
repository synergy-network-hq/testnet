use std::any::Any;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::config::{
    list_available_templates, load_node_config, load_node_config_from_template, NodeConfig,
};
use crate::consensus::cartel_detection::{CartelDetectionEngine, WhistleblowerSystem};
use crate::consensus::consensus_algorithm::ProofOfSynergy;
use crate::consensus::dao_governance::{DAOGovernance, SynergyOracle};
use crate::consensus::dual_quorum::{EntropyBeacon, ValidatorRotation};
use crate::consensus::synergy_score::SynergyScoreCalculator;
use crate::crypto::pqc::PQCManager;
use crate::genesis::canonical_genesis;
use crate::info;
use crate::logging::{init_logger, LogLevel};
use crate::p2p;
use crate::role_profiles::{resolve_configured_role, NodeRole, RoleProfile};
use crate::rpc;
use crate::rpc::rpc_server::{SHARED_CHAIN, SYNC_MANAGER};
use crate::sxcp;
use crate::sync::SyncManager;
use crate::telemetry;
use crate::token::TOKEN_MANAGER;
use crate::utils;
use crate::validator::{ValidatorRegistration, VALIDATOR_MANAGER};
use crate::wallet;
use serde_json::json;

struct RoleProcessGuard {
    child: Mutex<Child>,
}

impl RoleProcessGuard {
    fn new(child: Child) -> Self {
        RoleProcessGuard {
            child: Mutex::new(child),
        }
    }
}

impl Drop for RoleProcessGuard {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.child.lock() {
            let _ = guard.kill();
            let _ = guard.wait();
        }
    }
}

fn resolve_local_validator_address(config: &NodeConfig) -> String {
    let configured = config.node.validator_address.trim();
    if !configured.is_empty() {
        return configured.to_string();
    }

    if let Ok(from_env) = env::var("SYNERGY_VALIDATOR_ADDRESS") {
        let trimmed = from_env.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    if let Ok(from_env) = env::var("NODE_ADDRESS") {
        let trimmed = from_env.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    config.p2p.node_name.clone()
}

fn normalize_socket_address(bind_address: &str, default_port: u16) -> String {
    let trimmed = bind_address.trim();
    let host = trimmed
        .strip_prefix("http://")
        .or_else(|| trimmed.strip_prefix("https://"))
        .unwrap_or(trimmed)
        .trim_end_matches('/')
        .trim();

    if host.is_empty() {
        return format!("127.0.0.1:{default_port}");
    }

    match host {
        "0.0.0.0" => format!("0.0.0.0:{default_port}"),
        "::" | "[::]" => format!("[::]:{default_port}"),
        "::1" | "[::1]" => format!("[::1]:{default_port}"),
        _ if host.starts_with('[') && host.contains("]:") => host.to_string(),
        _ if host.matches(':').count() == 0 => format!("{host}:{default_port}"),
        _ => host.to_string(),
    }
}

fn normalize_client_address(bind_address: &str, default_port: u16) -> String {
    let normalized = normalize_socket_address(bind_address, default_port);

    if let Some(port) = normalized.strip_prefix("0.0.0.0:") {
        return format!("127.0.0.1:{port}");
    }

    if let Some(port) = normalized.strip_prefix("[::]:") {
        return format!("127.0.0.1:{port}");
    }

    normalized
}

fn normalize_rpc_socket_address(bind_address: &str, default_port: u16) -> String {
    normalize_socket_address(bind_address, default_port)
}

fn normalize_rpc_client_address(bind_address: &str, default_port: u16) -> String {
    normalize_client_address(bind_address, default_port)
}

fn rebind_socket_address(bind_address: &str, port: u16) -> String {
    let trimmed = bind_address.trim();
    let host = trimmed
        .strip_prefix("http://")
        .or_else(|| trimmed.strip_prefix("https://"))
        .unwrap_or(trimmed)
        .trim_end_matches('/')
        .trim();

    if host.is_empty() {
        return format!("127.0.0.1:{port}");
    }

    match host {
        "0.0.0.0" => format!("0.0.0.0:{port}"),
        "::" | "[::]" => format!("[::]:{port}"),
        "::1" | "[::1]" => format!("[::1]:{port}"),
        _ if host.starts_with('[') => {
            if let Some((addr, _)) = host.rsplit_once("]:") {
                format!("{addr}]:{port}")
            } else {
                format!("{host}:{port}")
            }
        }
        _ if host.matches(':').count() == 1 => {
            let (candidate_host, candidate_port) = host.rsplit_once(':').unwrap();
            if candidate_port.chars().all(|ch| ch.is_ascii_digit()) {
                format!("{candidate_host}:{port}")
            } else {
                host.to_string()
            }
        }
        _ if host.matches(':').count() == 0 => format!("{host}:{port}"),
        _ => format!("[{host}]:{port}"),
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

fn role_profile_exposes_rpc(profile: &RoleProfile) -> bool {
    profile.required_ports.iter().any(|port| {
        let normalized = port.to_ascii_lowercase();
        normalized.contains(" rpc") || normalized.contains(" ws") || normalized.starts_with("rpc ")
    })
}

fn role_profile_requires_p2p(profile: &RoleProfile) -> bool {
    profile.service_surface.contains(&"p2p")
        || profile.required_ports.iter().any(|port| {
            let normalized = port.to_ascii_lowercase();
            normalized.contains("p2p")
        })
}

fn should_start_p2p(config: &NodeConfig, profile: Option<&RoleProfile>) -> bool {
    if config.node.bootstrap_only {
        return true;
    }

    match profile {
        Some(profile) => role_profile_requires_p2p(profile),
        None => true,
    }
}

fn should_start_rpc(config: &NodeConfig, profile: Option<&RoleProfile>) -> bool {
    if config.node.bootstrap_only {
        return false;
    }

    let transports_enabled =
        config.rpc.enable_http || config.rpc.enable_ws || config.rpc.enable_grpc;
    if !transports_enabled {
        return false;
    }

    match profile {
        Some(profile) => role_profile_exposes_rpc(profile),
        None => true,
    }
}

fn should_start_metrics(config: &NodeConfig) -> bool {
    config.telemetry.enabled && !config.telemetry.metrics_bind.trim().is_empty()
}

fn should_start_sync(config: &NodeConfig, profile: Option<&RoleProfile>) -> bool {
    if config.node.bootstrap_only {
        return false;
    }

    match profile {
        Some(profile) => role_profile_requires_p2p(profile),
        None => true,
    }
}

fn should_auto_register_validator(config: &NodeConfig, profile: Option<&RoleProfile>) -> bool {
    if config.node.bootstrap_only || !config.node.auto_register_validator {
        return false;
    }

    matches!(profile.map(|value| value.role), Some(NodeRole::Validator))
}

fn should_start_consensus(config: &NodeConfig, profile: Option<&RoleProfile>) -> bool {
    if config.node.bootstrap_only {
        return false;
    }

    match profile {
        Some(profile) => profile.service_surface.contains(&"consensus"),
        None => true,
    }
}

fn normalize_expected_profile(
    config: &mut NodeConfig,
    expected_profile: Option<&'static RoleProfile>,
) -> Result<Option<&'static RoleProfile>, String> {
    if let Some(expected_profile) = expected_profile {
        if config.identity.role.trim().is_empty() {
            config.identity.role = expected_profile.role_id.to_string();
        }

        if config.role.compiled_profile.trim().is_empty() {
            config.role.compiled_profile = expected_profile.compiled_profile.to_string();
        }
    }

    let resolved = resolve_configured_role(&config.identity.role, &config.role.compiled_profile)?;
    if let (Some(expected_profile), Some(actual_profile)) = (expected_profile, resolved) {
        if actual_profile.role != expected_profile.role {
            return Err(format!(
                "This binary is bound to '{}' but the configuration resolves to '{}'",
                expected_profile.compiled_profile, actual_profile.compiled_profile
            ));
        }
    }

    Ok(resolved.or(expected_profile))
}

fn print_usage(binary_name: &str, expected_profile: Option<&RoleProfile>) {
    eprintln!("Synergy Testnet Beta Node");
    if let Some(profile) = expected_profile {
        eprintln!(
            "Role-bound build: {} ({})",
            profile.display_name, profile.compiled_profile
        );
    } else {
        eprintln!("Multi-role build: dynamic role selection");
    }
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    {binary_name} <SUBCOMMAND> [OPTIONS]");
    eprintln!();
    eprintln!("SUBCOMMANDS:");
    eprintln!("    init                  Initialize configuration directory");
    eprintln!("    start                 Start the node");
    eprintln!("    stop                  Stop the running node");
    eprintln!("    restart               Restart the node");
    eprintln!("    status                Check node status");
    eprintln!("    logs                  View node logs");
    eprintln!("    keygen                Generate PQC keypair with address (for control panel)");
    eprintln!("    generate-keypair      Generate a new PQC keypair");
    eprintln!("    register              Register node as validator");
    eprintln!("    sync                  Check network connectivity or sync");
    eprintln!("    list-templates        List all available node templates");
    eprintln!("    version               Display version information");
    eprintln!();
    eprintln!("START OPTIONS:");
    eprintln!("    --node-type <TYPE>    Specify the node type (uses templates/<TYPE>.toml)");
    eprintln!("    --config <PATH>       Path to custom configuration file");
    eprintln!();
    eprintln!("LOGS OPTIONS:");
    eprintln!("    --follow, -f          Follow log output");
    eprintln!("    --lines <N>           Number of lines to show (default: 50)");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    {binary_name} start --config config/node.toml");
    eprintln!("    {binary_name} keygen --output ./keys --class 1");
    eprintln!("    {binary_name} sync --config config/node.toml --network testbeta --check-only");
}

struct ActiveRoleServices {
    service_names: Vec<String>,
    keep_alive: Vec<Box<dyn Any>>,
    worker_threads: Vec<thread::JoinHandle<()>>,
}

impl ActiveRoleServices {
    fn new(profile: &RoleProfile) -> Self {
        Self {
            service_names: profile
                .service_surface
                .iter()
                .map(|value| value.to_string())
                .collect(),
            keep_alive: Vec::new(),
            worker_threads: Vec::new(),
        }
    }

    fn retain<T: 'static>(&mut self, value: T) {
        self.keep_alive.push(Box::new(value));
    }

    fn spawn_worker<F>(&mut self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.worker_threads.push(thread::spawn(job));
    }
}

fn write_status_file(path: &Path, payload: serde_json::Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(bytes) = serde_json::to_vec_pretty(&payload) {
        let _ = fs::write(path, bytes);
    }
}

fn rpc_bind_url(config: &NodeConfig) -> String {
    format!(
        "http://{}",
        normalize_rpc_client_address(&config.rpc.bind_address, config.rpc.http_port)
    )
}

fn atlas_service_envs_with_overrides(
    synergy_env: String,
    database_url: String,
    source_rpc_url: Option<String>,
    fallback_rpc_url: Option<String>,
) -> Vec<(&'static str, String)> {
    let mut envs = vec![
        ("NODE_ENV", "production".to_string()),
        ("SYNERGY_ENV", synergy_env),
        ("DATABASE_URL", database_url),
    ];

    if let Some(value) = source_rpc_url.filter(|value| !value.trim().is_empty()) {
        envs.push(("SYNERGY_CORE_RPC_URL", value));
    }

    if let Some(value) = fallback_rpc_url.filter(|value| !value.trim().is_empty()) {
        envs.push(("SYNERGY_CORE_RPC_FALLBACK_URL", value));
    }

    envs
}

fn atlas_service_envs(synergy_env: String, database_url: String) -> Vec<(&'static str, String)> {
    atlas_service_envs_with_overrides(
        synergy_env,
        database_url,
        env::var("SYNERGY_CORE_RPC_URL").ok(),
        env::var("SYNERGY_CORE_RPC_FALLBACK_URL").ok(),
    )
}

fn ensure_logs_dir() -> PathBuf {
    let logs_dir = PathBuf::from("data").join("logs");
    let _ = fs::create_dir_all(&logs_dir);
    logs_dir
}

fn spawn_node_process(
    name: &str,
    working_dir: &Path,
    script: &Path,
    envs: &[(&str, String)],
) -> Result<RoleProcessGuard, String> {
    if !script.is_file() {
        return Err(format!("Missing script: {}", script.display()));
    }

    let logs_dir = ensure_logs_dir();
    let stdout_path = logs_dir.join(format!("{name}.out"));
    let stderr_path = logs_dir.join(format!("{name}.err"));
    let stdout = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(stdout_path)
        .map_err(|error| format!("Failed to open {name} stdout log: {error}"))?;
    let stderr = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(stderr_path)
        .map_err(|error| format!("Failed to open {name} stderr log: {error}"))?;

    let mut command = Command::new("node");
    command
        .arg(script)
        .current_dir(working_dir)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));

    for (key, value) in envs {
        command.env(key, value);
    }

    let child = command
        .spawn()
        .map_err(|error| format!("Failed to start {name}: {error}"))?;
    Ok(RoleProcessGuard::new(child))
}

fn run_node_script(
    name: &str,
    working_dir: &Path,
    script: &Path,
    envs: &[(&str, String)],
) -> Result<(), String> {
    if !script.is_file() {
        return Err(format!("Missing script: {}", script.display()));
    }

    let mut command = Command::new("node");
    command.arg(script).current_dir(working_dir);
    for (key, value) in envs {
        command.env(key, value);
    }

    let status = command
        .status()
        .map_err(|error| format!("Failed to run {name}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{name} exited with status {status}"))
    }
}

fn resolve_explorer_root(runtime_root: &Path) -> Option<PathBuf> {
    let local = runtime_root.join("explorer-app");
    if local.exists() {
        return Some(local);
    }

    runtime_root
        .parent()
        .map(|parent| parent.join("explorer-app"))
        .filter(|candidate| candidate.exists())
}

fn resolve_node_entrypoint(package_root: &Path) -> Option<PathBuf> {
    let primary = package_root.join("dist").join("index.js");
    if primary.exists() {
        return Some(primary);
    }

    let nested = package_root.join("dist").join("src").join("index.js");
    if nested.exists() {
        return Some(nested);
    }

    None
}

fn infer_synergy_env(config: &NodeConfig) -> &'static str {
    let name = config.network.name.to_ascii_lowercase();
    if name.contains("devnet") {
        "devnet"
    } else if name.contains("testnet") && name.contains("beta") {
        "testnet-beta"
    } else if name.contains("testnet") {
        "testnet"
    } else if name.contains("beta") {
        "beta"
    } else {
        "mainnet"
    }
}

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn start_role_local_services(
    profile: Option<&'static RoleProfile>,
    config: &NodeConfig,
    running: &Arc<AtomicBool>,
) -> ActiveRoleServices {
    let Some(profile) = profile else {
        return ActiveRoleServices {
            service_names: vec![],
            keep_alive: vec![],
            worker_threads: vec![],
        };
    };

    let mut active = ActiveRoleServices::new(profile);

    match profile.role {
        NodeRole::Committee => {
            let pqc_manager = Arc::new(Mutex::new(PQCManager::new()));
            let entropy_beacon = Arc::new(Mutex::new(EntropyBeacon::new(Arc::clone(&pqc_manager))));
            let rotation = ValidatorRotation::new(VALIDATOR_MANAGER.clone(), entropy_beacon);
            rotation.rotate_validators();
            active.retain(pqc_manager);
            active.retain(rotation);
        }
        NodeRole::AuditValidator => {
            let pqc_manager = Arc::new(Mutex::new(PQCManager::new()));
            let synergy_calculator = Arc::new(SynergyScoreCalculator::new(
                VALIDATOR_MANAGER.clone(),
                Arc::clone(&pqc_manager),
            ));
            let cartel_engine =
                CartelDetectionEngine::new(VALIDATOR_MANAGER.clone(), synergy_calculator);
            let whistleblower = WhistleblowerSystem::new(Arc::clone(&pqc_manager));
            active.retain(pqc_manager);
            active.retain(cartel_engine);
            active.retain(whistleblower);
        }
        NodeRole::Relayer => {
            let relayer_address = if config.identity.address.trim().is_empty() {
                config.identity.node_id.clone()
            } else {
                config.identity.address.clone()
            };
            let public_key = relayer_address.clone();
            let _ = sxcp::register_relayer(&relayer_address, &public_key);
            let heartbeat_address = relayer_address.clone();
            let heartbeat_running = Arc::clone(running);
            active.spawn_worker(move || {
                while heartbeat_running.load(Ordering::SeqCst) {
                    let _ = sxcp::heartbeat_relayer(&heartbeat_address);
                    thread::sleep(Duration::from_secs(30));
                }
            });
        }
        NodeRole::Oracle => {
            let pqc_manager = Arc::new(Mutex::new(PQCManager::new()));
            let synergy_calculator = Arc::new(SynergyScoreCalculator::new(
                VALIDATOR_MANAGER.clone(),
                Arc::clone(&pqc_manager),
            ));
            let oracle = SynergyOracle::new(synergy_calculator, pqc_manager);
            active.retain(oracle);
        }
        NodeRole::UmaCoordinator => {
            active.retain("uma-coordinator-service".to_string());
        }
        NodeRole::CrossChainVerifier => {
            active.retain("cross-chain-verifier-service".to_string());
        }
        NodeRole::SynqExecution => {
            active.retain("synq-execution-service".to_string());
        }
        NodeRole::AnalyticsSimulation => {
            active.retain("analytics-and-simulation-service".to_string());
        }
        NodeRole::AegisCryptography => {
            active.retain("aegis-cryptography-service".to_string());
            active.retain(PQCManager::new());
        }
        NodeRole::GovernanceAuditor | NodeRole::TreasuryController | NodeRole::SecurityCouncil => {
            let pqc_manager = Arc::new(Mutex::new(PQCManager::new()));
            let synergy_calculator = Arc::new(SynergyScoreCalculator::new(
                VALIDATOR_MANAGER.clone(),
                Arc::clone(&pqc_manager),
            ));
            let governance = DAOGovernance::new(
                VALIDATOR_MANAGER.clone(),
                synergy_calculator,
                Arc::clone(&pqc_manager),
            );
            active.retain(pqc_manager);
            active.retain(governance);
        }
        NodeRole::DataAvailability => {
            active.retain("data-availability-service".to_string());
        }
        NodeRole::RpcGateway => {
            let bind_url = rpc_bind_url(config);
            let status_path = PathBuf::from("data").join("rpc-gateway.json");
            let running = Arc::clone(running);
            active.spawn_worker(move || {
                while running.load(Ordering::SeqCst) {
                    let mut payload = json!({
                        "ok": false,
                        "timestamp": now_ts(),
                        "rpc_url": bind_url,
                        "block_number": null,
                        "sync_state": null,
                        "local_height": null,
                        "network_height": null,
                        "peer_count": null,
                        "error": null
                    });

                    if let Some(network) = p2p::get_p2p_network() {
                        let mut manager = SYNC_MANAGER.lock().unwrap();
                        manager.attach_network(Arc::clone(&network));
                        let _ = manager.discover_network_height();
                        if manager.local_height < manager.get_network_height() {
                            let _ = manager.start_sync();
                        }

                        payload["ok"] = json!(true);
                        payload["sync_state"] = json!(format!("{:?}", manager.get_state()));
                        payload["local_height"] = json!(manager.local_height);
                        payload["network_height"] = json!(manager.get_network_height());
                        payload["block_number"] = json!(manager.local_height);
                        payload["peer_count"] = json!(manager.peers.len());
                        payload["progress_pct"] = json!(manager.get_progress_percentage());
                    } else {
                        payload["error"] = json!("p2p network unavailable");
                    }

                    write_status_file(&status_path, payload);
                    thread::sleep(Duration::from_secs(10));
                }
            });
        }
        NodeRole::IndexerExplorer => {
            let runtime_root = utils::get_runtime_root();
            let Some(runtime_root) = runtime_root else {
                eprintln!(
                    "Indexer/Explorer role requires a runtime root with config/ and bundled explorer-app assets."
                );
                return active;
            };

            let Some(explorer_root) = resolve_explorer_root(&runtime_root) else {
                eprintln!(
                    "Indexer/Explorer role requires explorer-app directory near the node runtime."
                );
                return active;
            };

            if Command::new("node").arg("--version").output().is_err() {
                eprintln!("Indexer/Explorer role requires Node.js available on PATH.");
                return active;
            }

            let database_url = match env::var("DATABASE_URL") {
                Ok(value) => value,
                Err(_) => {
                    eprintln!("Indexer/Explorer role requires DATABASE_URL for Postgres.");
                    return active;
                }
            };

            let synergy_env = infer_synergy_env(config).to_string();
            let indexer_dir = explorer_root.join("indexer");
            let backend_dir = explorer_root.join("backend");
            let Some(indexer_script) = resolve_node_entrypoint(&indexer_dir) else {
                eprintln!("Indexer/Explorer role could not find an Atlas indexer entrypoint.");
                return active;
            };
            let Some(backend_script) = resolve_node_entrypoint(&backend_dir) else {
                eprintln!("Indexer/Explorer role could not find an Atlas backend entrypoint.");
                return active;
            };
            let indexer_migrate = indexer_dir.join("scripts").join("migrate.js");
            let backend_migrate = backend_dir.join("scripts").join("migrate.js");

            // Atlas defaults to the canonical public core RPC for the current
            // environment. Preserve only explicit overrides; do not force the
            // local explorer node RPC, because its synced block store does not
            // guarantee authoritative wallet/stake query state.
            let base_envs = atlas_service_envs(synergy_env, database_url);

            if let Err(error) = run_node_script(
                "atlas-indexer-migrate",
                &indexer_dir,
                &indexer_migrate,
                &base_envs,
            ) {
                eprintln!("Failed to run indexer migrations: {error}");
                return active;
            }

            if let Err(error) = run_node_script(
                "atlas-backend-migrate",
                &backend_dir,
                &backend_migrate,
                &base_envs,
            ) {
                eprintln!("Failed to run explorer backend migrations: {error}");
                return active;
            }

            match spawn_node_process("atlas-indexer", &indexer_dir, &indexer_script, &base_envs) {
                Ok(guard) => active.retain(guard),
                Err(error) => eprintln!("Failed to start indexer: {error}"),
            }

            match spawn_node_process("atlas-backend", &backend_dir, &backend_script, &base_envs) {
                Ok(guard) => active.retain(guard),
                Err(error) => eprintln!("Failed to start explorer backend: {error}"),
            }
        }
        NodeRole::ObserverLight => {
            let status_path = PathBuf::from("data").join("observer-light.json");
            let running = Arc::clone(running);
            active.spawn_worker(move || {
                while running.load(Ordering::SeqCst) {
                    let mut payload = json!({
                        "ok": false,
                        "timestamp": now_ts(),
                        "error": "p2p network unavailable"
                    });

                    if let Some(network) = p2p::get_p2p_network() {
                        let mut manager = SYNC_MANAGER.lock().unwrap();
                        manager.attach_network(Arc::clone(&network));
                        let _ = manager.discover_network_height();
                        if manager.local_height < manager.get_network_height() {
                            let _ = manager.start_sync();
                        }

                        payload = json!({
                            "ok": true,
                            "timestamp": now_ts(),
                            "state": format!("{:?}", manager.get_state()),
                            "local_height": manager.local_height,
                            "network_height": manager.get_network_height(),
                            "sync_start_height": manager.get_sync_start_height(),
                            "progress_pct": manager.get_progress_percentage(),
                            "peer_count": manager.peers.len()
                        });
                    }

                    write_status_file(&status_path, payload);
                    thread::sleep(Duration::from_secs(15));
                }
            });
        }
        _ => {}
    }

    active
}

fn write_role_runtime_report(
    binary_name: &str,
    config: &NodeConfig,
    profile: Option<&RoleProfile>,
    p2p_enabled: bool,
    rpc_enabled: bool,
    consensus_enabled: bool,
    active_services: &ActiveRoleServices,
) {
    let report_dir = PathBuf::from("data");
    if fs::create_dir_all(&report_dir).is_err() {
        return;
    }

    let report = json!({
        "binary": binary_name,
        "generated_at": now_ts(),
        "node_id": config.identity.node_id,
        "role_id": profile.map(|value| value.role_id),
        "compiled_profile": profile.map(|value| value.compiled_profile),
        "authority_plane": profile.map(|value| format!("{:?}", value.authority_plane)),
        "service_surface": active_services.service_names,
        "p2p_enabled": p2p_enabled,
        "rpc_enabled": rpc_enabled,
        "consensus_enabled": consensus_enabled,
        "bootstrap_only": config.node.bootstrap_only,
        "ports": {
            "p2p": config.network.p2p_port,
            "rpc": config.network.rpc_port,
            "ws": config.network.ws_port,
        },
    });

    let report_path = report_dir.join("role-runtime.json");
    if let Ok(bytes) = serde_json::to_vec_pretty(&report) {
        let _ = fs::write(report_path, bytes);
    }
}

pub fn run(binary_name: &'static str, expected_profile: Option<&'static RoleProfile>) {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage(binary_name, expected_profile);
        process::exit(1);
    }

    let subcommand = &args[1];

    match subcommand.as_str() {
        "init" => {
            let config_dir = PathBuf::from("config");
            if !config_dir.exists() {
                fs::create_dir_all(&config_dir).expect("Failed to create config directory");
                println!("Created config directory.");
            } else {
                println!("Config directory already exists.");
            }
        }
        "start" => {
            let mut node_type: Option<String> = None;
            let mut config_path: Option<String> = None;

            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--node-type" => {
                        if i + 1 < args.len() {
                            node_type = Some(args[i + 1].clone());
                            i += 2;
                        } else {
                            eprintln!("Error: --node-type requires a value");
                            process::exit(1);
                        }
                    }
                    "--config" => {
                        if i + 1 < args.len() {
                            config_path = Some(args[i + 1].clone());
                            i += 2;
                        } else {
                            eprintln!("Error: --config requires a value");
                            process::exit(1);
                        }
                    }
                    _ => {
                        eprintln!("Error: Unknown option '{}'", args[i]);
                        print_usage(binary_name, expected_profile);
                        process::exit(1);
                    }
                }
            }

            let mut config = if let Some(path) = config_path {
                match load_node_config(Some(&path)) {
                    Ok(config) => config,
                    Err(e) => {
                        eprintln!("Failed to load configuration from '{}': {}", path, e);
                        process::exit(1);
                    }
                }
            } else if let Some(node_type_val) = node_type {
                match load_node_config_from_template(&node_type_val) {
                    Ok(config) => {
                        println!(
                            "Loading node configuration from template: {}",
                            node_type_val
                        );
                        config
                    }
                    Err(e) => {
                        eprintln!("Failed to load template '{}': {}", node_type_val, e);
                        eprintln!(
                            "\nRun '{binary_name} list-templates' to see available templates."
                        );
                        process::exit(1);
                    }
                }
            } else {
                match load_node_config(None) {
                    Ok(config) => config,
                    Err(e) => {
                        eprintln!("Failed to load configuration: {}", e);
                        eprintln!("\nTip: Use --node-type <TYPE> to specify a node type");
                        eprintln!("     or --config <PATH> to specify a custom config file");
                        process::exit(1);
                    }
                }
            };

            let role_profile = match normalize_expected_profile(&mut config, expected_profile) {
                Ok(profile) => profile,
                Err(error) => {
                    eprintln!("Failed to validate node role/profile binding: {error}");
                    process::exit(1);
                }
            };

            let log_level = LogLevel::from_str(&config.logging.log_level).unwrap_or(LogLevel::Info);
            init_logger(
                log_level,
                config.logging.enable_console,
                config.logging.log_file.clone(),
                config.logging.max_file_size,
                config.logging.max_files,
            );

            info!("main", "Synergy testbeta node starting...");
            info!(
                "main",
                "Configuration loaded successfully",
                "network" => config.network.name.clone(),
                "consensus" => config.consensus.algorithm.clone()
            );
            if let Some(profile) = role_profile {
                info!(
                    "main",
                    "Validated role-bound runtime profile",
                    "role_id" => profile.role_id,
                    "compiled_profile" => profile.compiled_profile,
                    "authority_plane" => format!("{:?}", profile.authority_plane),
                    "binary" => binary_name
                );
            }

            env::set_var(
                "SYNERGY_CONSENSUS_BLOCK_TIME_SECS",
                config.consensus.block_time_secs.to_string(),
            );
            env::set_var(
                "SYNERGY_CONSENSUS_EPOCH_LENGTH",
                config.consensus.epoch_length.to_string(),
            );
            env::set_var(
                "SYNERGY_CONSENSUS_MIN_VALIDATORS",
                config.consensus.min_validators.to_string(),
            );
            env::set_var("SYNERGY_NODE_ROLE_ID", config.identity.role.clone());
            env::set_var(
                "SYNERGY_COMPILED_PROFILE",
                config.role.compiled_profile.clone(),
            );

            let project_root = utils::validate_project_root().unwrap_or_else(|e| {
                eprintln!("Failed to determine writable project root: {}", e);
                process::exit(1);
            });
            env::set_var("SYNERGY_PROJECT_ROOT", &project_root);

            let data_dir = project_root.join("data");
            let logs_dir = data_dir.join("logs");
            let chain_dir = data_dir.join("chain");

            info!(
                "main",
                "Project root validated",
                "root" => project_root.display().to_string()
            );

            std::fs::create_dir_all(&data_dir).expect("Failed to create data directory");
            std::fs::create_dir_all(&logs_dir).expect("Failed to create logs directory");
            std::fs::create_dir_all(&chain_dir).expect("Failed to create chain directory");

            let genesis = canonical_genesis().unwrap_or_else(|error| {
                eprintln!("Failed to load canonical genesis: {}", error);
                process::exit(1);
            });
            info!(
                "main",
                "Canonical genesis loaded",
                "path" => genesis.path().display().to_string(),
                "hash" => genesis.hash().to_string()
            );

            wallet::init_testbeta_wallets();
            {
                let token_manager = TOKEN_MANAGER.clone();
                if let Err(e) = token_manager.load_state("data/token_state.json") {
                    info!(
                        "main",
                        "No saved token state found (using genesis allocations)",
                        "error" => e.to_string()
                    );
                }
                if let Err(e) = token_manager.ensure_rewards_pool_funded() {
                    eprintln!("Warning: Failed to initialize rewards pool: {}", e);
                }
            }

            info!("main", "Starting the node...");

            let pid = std::process::id();
            if let Err(e) = fs::write("data/synergy-testbeta.pid", pid.to_string()) {
                eprintln!("Warning: Failed to write PID file: {}", e);
            }

            let process_start_time = SystemTime::now();
            let blockchain = Arc::clone(&SHARED_CHAIN);

            let p2p_enabled = should_start_p2p(&config, role_profile);
            let p2p_network = if p2p_enabled {
                let network = p2p::start_p2p_network(
                    Arc::clone(&blockchain),
                    &config.p2p.listen_address,
                    &config,
                );
                info!(
                    "main",
                    "P2P network started",
                    "listen_address" => config.p2p.listen_address.clone()
                );
                Some(network)
            } else {
                info!(
                    "main",
                    "P2P network disabled for this node profile",
                    "role" => config.identity.role.clone()
                );
                None
            };

            let rpc_enabled = should_start_rpc(&config, role_profile);
            if rpc_enabled {
                let rpc_bind_address =
                    normalize_rpc_socket_address(&config.rpc.bind_address, config.rpc.http_port);
                let ws_bind_address = if config.rpc.enable_ws {
                    Some(rebind_socket_address(&rpc_bind_address, config.rpc.ws_port))
                } else {
                    None
                };
                let cors_enabled = config.rpc.cors_enabled;
                let cors_origins = config.rpc.cors_origins.clone();
                let _rpc_handle = std::thread::spawn(move || {
                    rpc::rpc_server::start_rpc_server(
                        &rpc_bind_address,
                        ws_bind_address,
                        cors_enabled,
                        cors_origins,
                    );
                });

                // Wait until the RPC listener is actually accepting connections before
                // allowing the consensus engine to start.  This prevents a race where
                // the consensus engine (or the desktop app) tries to reach the RPC
                // endpoint before it has finished binding, producing "fetch failed" errors.
                let rpc_port = config.rpc.http_port;
                let rpc_ready_addr = format!("127.0.0.1:{}", rpc_port);
                let rpc_ready_deadline = std::time::Instant::now() + Duration::from_secs(10);
                loop {
                    if std::net::TcpStream::connect(&rpc_ready_addr).is_ok() {
                        info!("main", "RPC server ready", "port" => rpc_port);
                        break;
                    }
                    if std::time::Instant::now() >= rpc_ready_deadline {
                        eprintln!(
                            "Warning: RPC server did not become ready within 10 s on port {}; proceeding anyway",
                            rpc_port
                        );
                        break;
                    }
                    thread::sleep(Duration::from_millis(50));
                }
            } else {
                info!(
                    "main",
                    "RPC server disabled for this node profile",
                    "bootstrap_only" => config.node.bootstrap_only,
                    "enable_http" => config.rpc.enable_http,
                    "enable_ws" => config.rpc.enable_ws,
                    "enable_grpc" => config.rpc.enable_grpc
                );
            }

            let metrics_enabled = should_start_metrics(&config);
            if metrics_enabled {
                let metrics_bind_address =
                    normalize_socket_address(&config.telemetry.metrics_bind, 6030);
                let metrics_ready_addr =
                    normalize_client_address(&config.telemetry.metrics_bind, 6030);
                let metrics_config = config.clone();
                let _metrics_handle = std::thread::spawn(move || {
                    telemetry::start_metrics_server(
                        &metrics_bind_address,
                        metrics_config,
                        process_start_time,
                    );
                });

                let metrics_ready_deadline = std::time::Instant::now() + Duration::from_secs(5);
                loop {
                    if std::net::TcpStream::connect(&metrics_ready_addr).is_ok() {
                        info!("main", "Metrics server ready", "bind_address" => metrics_ready_addr.clone());
                        break;
                    }
                    if std::time::Instant::now() >= metrics_ready_deadline {
                        eprintln!(
                            "Warning: metrics server did not become ready within 5 s on {}; proceeding anyway",
                            metrics_ready_addr
                        );
                        break;
                    }
                    thread::sleep(Duration::from_millis(50));
                }
            } else {
                info!(
                    "main",
                    "Metrics server disabled",
                    "enabled" => config.telemetry.enabled,
                    "metrics_bind" => config.telemetry.metrics_bind.clone()
                );
            }

            let consensus_enabled = should_start_consensus(&config, role_profile);
            info!(
                "main",
                "Node initialized",
                "bootstrap_only" => config.node.bootstrap_only,
                "rpc_enabled" => rpc_enabled,
                "p2p_enabled" => p2p_enabled,
                "metrics_enabled" => metrics_enabled,
                "rpc_port" => config.rpc.http_port,
                "metrics_bind" => config.telemetry.metrics_bind.clone(),
                "p2p_address" => config.p2p.listen_address.clone(),
                "consensus" => config.consensus.algorithm.clone()
            );

            let reset_flag_path = "data/.reset_flag";
            let should_sync = !std::path::Path::new(reset_flag_path).exists();

            if !should_start_sync(&config, role_profile) {
                info!("main", "Chain sync disabled for this node profile");
            } else if should_sync {
                let sync_result = {
                    let mut manager = SYNC_MANAGER.lock().unwrap();
                    if let Some(network) = &p2p_network {
                        manager.attach_network(Arc::clone(network));
                    }
                    manager.start_sync()
                };
                match sync_result {
                    Ok(_) => {
                        let current_height = blockchain
                            .lock()
                            .unwrap()
                            .last()
                            .map(|b| b.block_index)
                            .unwrap_or(0);
                        info!("main", "Sync complete", "height" => current_height);
                    }
                    Err(err) => {
                        eprintln!("Warning: Sync failed before consensus: {}", err);
                    }
                }
            } else {
                std::fs::remove_file(reset_flag_path).ok();
                info!(
                    "main",
                    "Starting fresh after reset - skipping network sync",
                    "height" => 0
                );
            }

            if should_auto_register_validator(&config, role_profile) {
                let validator_address = resolve_local_validator_address(&config);
                if !is_validator_allowed(&config, &validator_address) {
                    info!(
                        "main",
                        "Skipping self-registration because validator is not in allowlist",
                        "validator_address" => validator_address.clone()
                    );
                } else {
                    let validator_manager = VALIDATOR_MANAGER.clone();
                    let is_registered = validator_manager
                        .get_validator(&validator_address)
                        .is_some();
                    let is_pending = validator_manager.is_pending(&validator_address);

                    if !is_registered && !is_pending {
                        info!(
                            "main",
                            "Self-registering as validator",
                            "address" => validator_address.clone()
                        );

                        let funding_amount: u64 = 1_000_000_000_000;
                        let stake_amount: u64 = 1_000_000_000_000;

                        let token_manager = TOKEN_MANAGER.clone();
                        let current_balance = token_manager.get_balance(&validator_address, "SNRG");

                        if current_balance < funding_amount {
                            match token_manager.mint_tokens(
                                &validator_address,
                                "SNRG",
                                funding_amount,
                            ) {
                                Ok(_) => {
                                    info!(
                                        "main",
                                        "Self-funded with 1000 SNRG",
                                        "address" => validator_address.clone()
                                    );
                                }
                                Err(e) => {
                                    eprintln!("Warning: Failed to self-fund: {}", e);
                                }
                            }
                        }

                        let registration = ValidatorRegistration {
                            address: validator_address.clone(),
                            public_key: validator_address.clone(),
                            name: format!(
                                "Validator-{}",
                                &validator_address[..8.min(validator_address.len())]
                            ),
                            stake_amount,
                            submitted_at: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                            registration_tx_hash: format!("self-reg-{validator_address}"),
                        };

                        if validator_manager.register_validator(registration).is_ok() {
                            info!(
                                "main",
                                "Validator registration submitted",
                                "address" => validator_address.clone()
                            );
                            if validator_manager
                                .approve_validator(&validator_address)
                                .is_ok()
                            {
                                info!(
                                    "main",
                                    "Validator self-approved and activated",
                                    "address" => validator_address.clone()
                                );

                                if let Err(e) = token_manager.stake_tokens(
                                    &validator_address,
                                    &validator_address,
                                    "SNRG",
                                    stake_amount,
                                ) {
                                    eprintln!("Warning: Failed to self-stake: {}", e);
                                }
                            }
                        }
                    }
                }
            } else {
                info!(
                    "main",
                    "Auto validator registration disabled for this node profile"
                );
            }

            let consensus_handle = if !consensus_enabled {
                info!(
                    "main",
                    "Consensus engine disabled for this node profile",
                    "bootstrap_only" => config.node.bootstrap_only,
                    "role" => config.identity.role.clone()
                );
                None
            } else {
                info!(
                    "main",
                    "Starting consensus engine",
                    "algorithm" => config.consensus.algorithm.clone()
                );
                Some(std::thread::spawn(|| {
                    let mut consensus = ProofOfSynergy::new();
                    consensus.initialize();
                    consensus.execute();
                }))
            };

            let running = Arc::new(AtomicBool::new(true));
            let role_services = start_role_local_services(role_profile, &config, &running);
            write_role_runtime_report(
                binary_name,
                &config,
                role_profile,
                p2p_enabled,
                rpc_enabled,
                consensus_enabled,
                &role_services,
            );

            info!("main", "Node is running. Press Ctrl+C to stop.");

            let shutdown_flag = Arc::clone(&running);
            ctrlc::set_handler(move || {
                println!("\nReceived shutdown signal...");
                shutdown_flag.store(false, Ordering::SeqCst);
            })
            .expect("Error setting Ctrl-C handler");

            while running.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_secs(1));
            }

            info!("main", "Node shutdown gracefully");
            fs::remove_file("data/synergy-testbeta.pid").ok();

            for handle in role_services.worker_threads {
                let _ = handle.join();
            }

            if let Some(consensus_handle) = consensus_handle {
                consensus_handle.join().ok();
            }
        }
        "keygen" | "generate-keypair" => {
            use crate::address::generate_class_based_address;
            use base64::{engine::general_purpose, Engine as _};
            use pqcrypto_falcon::falcon1024;
            use pqcrypto_traits::sign::{PublicKey as _, SecretKey as _};

            let mut output_dir: Option<PathBuf> = None;
            let mut node_class: Option<u8> = None;

            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--output" => {
                        if i + 1 < args.len() {
                            output_dir = Some(PathBuf::from(&args[i + 1]));
                            i += 2;
                        } else {
                            eprintln!("Error: --output requires a path");
                            process::exit(1);
                        }
                    }
                    "--class" => {
                        if i + 1 < args.len() {
                            node_class = args[i + 1].parse().ok();
                            if node_class.is_none()
                                || node_class.unwrap() < 1
                                || node_class.unwrap() > 5
                            {
                                eprintln!("Error: --class must be a number between 1 and 5");
                                process::exit(1);
                            }
                            i += 2;
                        } else {
                            eprintln!("Error: --class requires a number (1-5)");
                            process::exit(1);
                        }
                    }
                    _ => {
                        eprintln!("Error: Unknown option '{}'", args[i]);
                        process::exit(1);
                    }
                }
            }

            let (pk, sk) = falcon1024::keypair();
            let public_key_b64 = general_purpose::STANDARD.encode(pk.as_bytes());
            let private_key_b64 = general_purpose::STANDARD.encode(sk.as_bytes());

            let address = if let Some(class) = node_class {
                generate_class_based_address(pk.as_bytes(), class)
            } else {
                String::new()
            };

            if let Some(ref output_path) = output_dir {
                if let Err(e) = fs::create_dir_all(output_path) {
                    eprintln!("Failed to create output directory: {}", e);
                    process::exit(1);
                }

                let public_key_path = output_path.join("public.key");
                if let Err(e) = fs::write(&public_key_path, &public_key_b64) {
                    eprintln!("Failed to write public key: {}", e);
                    process::exit(1);
                }

                let private_key_path = output_path.join("private.key");
                if let Err(e) = fs::write(&private_key_path, &private_key_b64) {
                    eprintln!("Failed to write private key: {}", e);
                    process::exit(1);
                }
            }

            if !address.is_empty() {
                println!("{}", address);
            } else {
                eprintln!("Error: --class is required to generate an address");
                process::exit(1);
            }
        }
        "status" => {
            let config = match load_node_config(None) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("Failed to load configuration: {}", e);
                    process::exit(1);
                }
            };

            let log_level = LogLevel::from_str(&config.logging.log_level).unwrap_or(LogLevel::Info);
            init_logger(
                log_level,
                config.logging.enable_console,
                config.logging.log_file.clone(),
                config.logging.max_file_size,
                config.logging.max_files,
            );

            info!("main", "Node status: Online");
        }
        "list-templates" => {
            println!("Available Node Templates:");
            println!();
            match list_available_templates() {
                Ok(templates) => {
                    if templates.is_empty() {
                        println!("  No templates found in 'templates/' directory");
                    } else {
                        for (idx, template) in templates.iter().enumerate() {
                            println!("  {}. {}", idx + 1, template);
                        }
                        println!();
                        println!("Usage: {binary_name} start --node-type <template-name>");
                    }
                }
                Err(e) => {
                    eprintln!("Failed to list templates: {}", e);
                    process::exit(1);
                }
            }
        }
        "stop" => {
            println!("Stopping Synergy testbeta node...");

            let pid_file = "data/synergy-testbeta.pid";
            if !PathBuf::from(pid_file).exists() {
                eprintln!("Error: PID file not found. Is the node running?");
                process::exit(1);
            }

            match fs::read_to_string(pid_file) {
                Ok(pid_str) => match pid_str.trim().parse::<i32>() {
                    Ok(pid) => {
                        #[cfg(unix)]
                        {
                            use std::process::Command;
                            match Command::new("kill").arg(pid.to_string()).status() {
                                Ok(_) => {
                                    println!("Node stopped successfully (PID: {})", pid);
                                    fs::remove_file(pid_file).ok();
                                }
                                Err(e) => {
                                    eprintln!("Failed to stop node: {}", e);
                                    process::exit(1);
                                }
                            }
                        }
                        #[cfg(not(unix))]
                        {
                            eprintln!("Stop command is only supported on Unix systems");
                            process::exit(1);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error parsing PID file: {}", e);
                        process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!("Error reading PID file: {}", e);
                    process::exit(1);
                }
            }
        }
        "restart" => {
            println!("Restarting Synergy testbeta node...");

            let pid_file = "data/synergy-testbeta.pid";
            if PathBuf::from(pid_file).exists() {
                println!("Stopping running node...");
                #[cfg(unix)]
                {
                    if let Ok(pid_str) = fs::read_to_string(pid_file) {
                        if let Ok(pid) = pid_str.trim().parse::<i32>() {
                            use std::process::Command;
                            Command::new("kill").arg(pid.to_string()).status().ok();
                            fs::remove_file(pid_file).ok();
                            std::thread::sleep(Duration::from_secs(2));
                        }
                    }
                }
            }

            println!("Starting node...");
            println!("Please run: {binary_name} start [OPTIONS]");
        }
        "logs" => {
            let mut follow = false;
            let mut lines = 50;

            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--follow" | "-f" => {
                        follow = true;
                        i += 1;
                    }
                    "--lines" => {
                        if i + 1 < args.len() {
                            lines = args[i + 1].parse().unwrap_or(50);
                            i += 2;
                        } else {
                            eprintln!("Error: --lines requires a value");
                            process::exit(1);
                        }
                    }
                    _ => {
                        eprintln!("Error: Unknown option '{}'", args[i]);
                        print_usage(binary_name, expected_profile);
                        process::exit(1);
                    }
                }
            }

            let log_file = "data/logs/synergy-node.log";
            if !PathBuf::from(log_file).exists() {
                eprintln!("Error: Log file not found at {}", log_file);
                process::exit(1);
            }

            if follow {
                #[cfg(unix)]
                {
                    use std::process::Command;
                    let _ = Command::new("tail")
                        .arg("-f")
                        .arg("-n")
                        .arg(lines.to_string())
                        .arg(log_file)
                        .status();
                }
                #[cfg(not(unix))]
                {
                    eprintln!("Follow mode is only supported on Unix systems");
                    process::exit(1);
                }
            } else {
                #[cfg(unix)]
                {
                    use std::process::Command;
                    let _ = Command::new("tail")
                        .arg("-n")
                        .arg(lines.to_string())
                        .arg(log_file)
                        .status();
                }
                #[cfg(not(unix))]
                {
                    match fs::read_to_string(log_file) {
                        Ok(content) => {
                            let log_lines: Vec<&str> = content.lines().collect();
                            let start = if log_lines.len() > lines {
                                log_lines.len() - lines
                            } else {
                                0
                            };
                            for line in &log_lines[start..] {
                                println!("{}", line);
                            }
                        }
                        Err(e) => {
                            eprintln!("Error reading log file: {}", e);
                            process::exit(1);
                        }
                    }
                }
            }
        }
        "register" => {
            let mut address: Option<String> = None;
            let mut key_path: Option<String> = None;

            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--config" => {
                        if i + 1 < args.len() {
                            i += 2;
                        } else {
                            eprintln!("Error: --config requires a path");
                            process::exit(1);
                        }
                    }
                    "--address" => {
                        if i + 1 < args.len() {
                            address = Some(args[i + 1].clone());
                            i += 2;
                        } else {
                            eprintln!("Error: --address requires an address");
                            process::exit(1);
                        }
                    }
                    "--key" => {
                        if i + 1 < args.len() {
                            key_path = Some(args[i + 1].clone());
                            i += 2;
                        } else {
                            eprintln!("Error: --key requires a path");
                            process::exit(1);
                        }
                    }
                    _ => {
                        eprintln!("Error: Unknown option '{}'", args[i]);
                        process::exit(1);
                    }
                }
            }

            if address.is_none() || key_path.is_none() {
                eprintln!("Error: --address and --key are required");
                process::exit(1);
            }

            let private_key_hex = fs::read_to_string(&key_path.as_ref().unwrap())
                .map_err(|e| format!("Failed to read key file: {}", e))
                .unwrap_or_else(|e| {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                });

            let public_key = private_key_hex.trim().to_string();
            let addr = address.unwrap();

            let registration = ValidatorRegistration {
                address: addr.clone(),
                public_key,
                name: "Control Panel Node".to_string(),
                stake_amount: 1000,
                submitted_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                registration_tx_hash: format!(
                    "reg_{}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                ),
            };

            let validator_manager = VALIDATOR_MANAGER.clone();
            match validator_manager.register_validator(registration) {
                Ok(result) => {
                    println!("✅ {}", result);
                    if let Err(e) = validator_manager.approve_validator(&addr) {
                        eprintln!(
                            "Warning: Registration succeeded but auto-approval failed: {}",
                            e
                        );
                    } else {
                        println!("✅ Validator auto-approved for testbeta");
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
        }
        "sync" => {
            let mut config_path: Option<String> = None;
            let mut network = "testbeta".to_string();
            let mut check_only = false;

            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--config" => {
                        if i + 1 < args.len() {
                            config_path = Some(args[i + 1].clone());
                            i += 2;
                        } else {
                            eprintln!("Error: --config requires a path");
                            process::exit(1);
                        }
                    }
                    "--network" => {
                        if i + 1 < args.len() {
                            network = args[i + 1].clone();
                            i += 2;
                        } else {
                            eprintln!("Error: --network requires a name");
                            process::exit(1);
                        }
                    }
                    "--check-only" => {
                        check_only = true;
                        i += 1;
                    }
                    _ => {
                        eprintln!("Error: Unknown option '{}'", args[i]);
                        process::exit(1);
                    }
                }
            }

            let config = if let Some(path) = config_path {
                match load_node_config(Some(&path)) {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        eprintln!("Failed to load configuration: {}", e);
                        process::exit(1);
                    }
                }
            } else {
                match load_node_config(None) {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        eprintln!("Failed to load configuration: {}", e);
                        process::exit(1);
                    }
                }
            };

            println!("Starting sync runner for {}", network);

            let blockchain = Arc::clone(&SHARED_CHAIN);
            let p2p_network = p2p::start_p2p_network(
                Arc::clone(&blockchain),
                &config.p2p.listen_address,
                &config,
            );

            let mut cli_sync_manager = SyncManager::new(Arc::clone(&blockchain));
            cli_sync_manager.attach_network(Arc::clone(&p2p_network));

            if check_only {
                match cli_sync_manager.discover_network_height() {
                    Ok(network_height) => {
                        let local_height = blockchain
                            .lock()
                            .unwrap()
                            .last()
                            .map(|b| b.block_index)
                            .unwrap_or(0);
                        println!(
                            "Local height: {}, network height: {}",
                            local_height, network_height
                        );
                        if local_height >= network_height {
                            println!("Node is already synced.");
                        } else {
                            println!(
                                "Node is behind by {} blocks.",
                                network_height.saturating_sub(local_height)
                            );
                        }
                    }
                    Err(err) => {
                        eprintln!("Failed to determine network height: {}", err);
                        process::exit(1);
                    }
                }
            } else {
                println!("Starting fast sync to catch up...");
                if let Err(err) = cli_sync_manager.start_sync() {
                    eprintln!("Sync error: {}", err);
                    process::exit(1);
                }
                let current_height = blockchain
                    .lock()
                    .unwrap()
                    .last()
                    .map(|b| b.block_index)
                    .unwrap_or(0);
                println!("Sync complete! Current block height: {}", current_height);
            }
        }
        "version" | "--version" | "-v" => {
            println!("Synergy Testnet Beta Node v{}", env!("CARGO_PKG_VERSION"));
            println!("Binary: {}", binary_name);
            if let Some(profile) = expected_profile {
                println!(
                    "Profile: {} ({})",
                    profile.display_name, profile.compiled_profile
                );
            }
            println!("Build OS: {}", std::env::consts::OS);
        }
        "help" | "--help" | "-h" => {
            print_usage(binary_name, expected_profile);
        }
        _ => {
            eprintln!("Unknown subcommand: {}", subcommand);
            eprintln!();
            print_usage(binary_name, expected_profile);
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::NodeConfig;

    #[test]
    fn expected_profile_populates_blank_config() {
        let mut config = NodeConfig::default();
        let profile = NodeRole::Validator.profile();

        let resolved = normalize_expected_profile(&mut config, Some(profile))
            .expect("expected profile should bind")
            .expect("profile should resolve");

        assert_eq!(config.identity.role, "validator");
        assert_eq!(config.role.compiled_profile, "validator_node");
        assert_eq!(resolved.role, NodeRole::Validator);
    }

    #[test]
    fn expected_profile_rejects_mismatch() {
        let mut config = NodeConfig::default();
        config.identity.role = "oracle".to_string();
        config.role.compiled_profile = "oracle_node".to_string();

        let error = normalize_expected_profile(&mut config, Some(NodeRole::Validator.profile()))
            .expect_err("mismatched profile should fail");

        assert!(error.contains("validator_node"));
        assert!(error.contains("oracle_node"));
    }

    #[test]
    fn rpc_gateway_profile_starts_p2p() {
        assert!(role_profile_requires_p2p(NodeRole::RpcGateway.profile()));
    }

    #[test]
    fn relayer_profile_starts_p2p() {
        assert!(role_profile_requires_p2p(NodeRole::Relayer.profile()));
    }

    #[test]
    fn indexer_explorer_profile_starts_p2p_and_sync() {
        let config = NodeConfig::default();
        let profile = NodeRole::IndexerExplorer.profile();

        assert!(role_profile_requires_p2p(profile));
        assert!(should_start_p2p(&config, Some(profile)));
        assert!(should_start_sync(&config, Some(profile)));
    }

    #[test]
    fn rpc_bind_address_normalizes_host_only_socket_inputs() {
        assert_eq!(
            normalize_rpc_socket_address("0.0.0.0", 5640),
            "0.0.0.0:5640"
        );
        assert_eq!(
            normalize_rpc_socket_address("127.0.0.1", 5640),
            "127.0.0.1:5640"
        );
        assert_eq!(normalize_socket_address("0.0.0.0", 6030), "0.0.0.0:6030");
    }

    #[test]
    fn rpc_bind_url_uses_loopback_for_wildcard_bind_addresses() {
        let mut config = NodeConfig::default();
        config.rpc.bind_address = "0.0.0.0".to_string();
        config.rpc.http_port = 5647;

        assert_eq!(rpc_bind_url(&config), "http://127.0.0.1:5647");
    }

    #[test]
    fn client_address_uses_loopback_for_wildcard_metrics_bind() {
        assert_eq!(
            normalize_client_address("0.0.0.0:6030", 6030),
            "127.0.0.1:6030"
        );
    }
}
