use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use synergy_testbeta::config::{
    list_available_templates, load_node_config, load_node_config_from_template,
};
use synergy_testbeta::consensus::consensus_algorithm::ProofOfSynergy;
use synergy_testbeta::info;
use synergy_testbeta::logging::{init_logger, LogLevel};
use synergy_testbeta::p2p;
use synergy_testbeta::rpc;
use synergy_testbeta::rpc::rpc_server::{SHARED_CHAIN, SYNC_MANAGER};
use synergy_testbeta::sync::SyncManager;
use synergy_testbeta::token::TOKEN_MANAGER;
use synergy_testbeta::utils;
use synergy_testbeta::validator::{ValidatorRegistration, VALIDATOR_MANAGER};
use synergy_testbeta::wallet;

fn resolve_local_validator_address(config: &synergy_testbeta::config::NodeConfig) -> String {
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

fn is_validator_allowed(
    config: &synergy_testbeta::config::NodeConfig,
    validator_address: &str,
) -> bool {
    if !config.node.strict_validator_allowlist {
        return true;
    }

    config
        .node
        .allowed_validator_addresses
        .iter()
        .any(|allowed| allowed == validator_address)
}

fn print_usage() {
    eprintln!("Synergy Testnet Beta Node - Multi-role blockchain node");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    synergy-testbeta <SUBCOMMAND> [OPTIONS]");
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
    eprintln!("    --daemon              Run as background daemon");
    eprintln!();
    eprintln!("LOGS OPTIONS:");
    eprintln!("    --follow, -f          Follow log output");
    eprintln!("    --lines <N>           Number of lines to show (default: 50)");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    synergy-testbeta start --node-type validator");
    eprintln!("    synergy-testbeta start --node-type oracle --daemon");
    eprintln!("    synergy-testbeta start --config config/custom.toml");
    eprintln!("    synergy-testbeta stop");
    eprintln!("    synergy-testbeta logs --follow");
    eprintln!("    synergy-testbeta list-templates");
    eprintln!("    synergy-testbeta keygen --type ml-dsa-65 --output ./keys --class 1");
    eprintln!("    synergy-testbeta register --config config/node.toml --address synv1... --key ./keys/private.key");
    eprintln!("    synergy-testbeta sync --config config/node.toml --network testbeta --check-only");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
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
            // Parse start command arguments
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
                        print_usage();
                        process::exit(1);
                    }
                }
            }

            // Load configuration based on provided options
            let config = if let Some(path) = config_path {
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
                            "\nRun 'synergy-testbeta list-templates' to see available templates."
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

            // Initialize logger
            let log_level = LogLevel::from_str(&config.logging.log_level).unwrap_or(LogLevel::Info);
            init_logger(
                log_level,
                config.logging.enable_console,
                config.logging.log_file.clone(),
                config.logging.max_file_size,
                config.logging.max_files,
            );

            info!("main", "Synergy testbeta node starting...");
            info!("main", "Configuration loaded successfully", "network" => config.network.name.clone(), "consensus" => config.consensus.algorithm.clone());

            // Propagate consensus timing into env for consensus engine initialization.
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

            // Validate project root and get absolute paths
            let project_root = utils::validate_project_root()
                .unwrap_or_else(|e| {
                    eprintln!("⚠️  Warning: {}", e);
                    eprintln!("   Continuing with current directory, but paths may be incorrect if not run from project root");
                    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                });

            let data_dir = project_root.join("data");
            let logs_dir = data_dir.join("logs");
            let chain_dir = data_dir.join("chain");

            info!("main", "Project root validated", "root" => project_root.display().to_string());

            // Create data directories with absolute paths
            std::fs::create_dir_all(&data_dir).expect("Failed to create data directory");
            std::fs::create_dir_all(&logs_dir).expect("Failed to create logs directory");
            std::fs::create_dir_all(&chain_dir).expect("Failed to create chain directory");

            // Load testnet beta system identities (faucet/treasury/bootnodes) to enable
            // signing test transactions and validating known keys.
            wallet::init_testbeta_wallets();

            // Restore token state (balances/transfers) if available for explorer continuity.
            // Falls back to genesis allocations if no saved state exists.
            {
                let token_manager = synergy_testbeta::token::TOKEN_MANAGER.clone();
                if let Err(e) = token_manager.load_state("data/token_state.json") {
                    info!("main", "No saved token state found (using genesis allocations)", "error" => e.to_string());
                }

                // Ensure rewards pool is funded for validator rewards distribution
                if let Err(e) = token_manager.ensure_rewards_pool_funded() {
                    eprintln!("⚠️  Failed to initialize rewards pool: {}", e);
                }
            }

            info!("main", "Starting the node...");

            // Write PID file
            let pid = std::process::id();
            if let Err(e) = fs::write("data/synergy-testbeta.pid", pid.to_string()) {
                eprintln!("Warning: Failed to write PID file: {}", e);
            }

            // Use the shared blockchain instance (initialized with genesis in SHARED_CHAIN)
            let blockchain = Arc::clone(&SHARED_CHAIN);

            // Start P2P network
            let p2p_network = p2p::start_p2p_network(
                Arc::clone(&blockchain),
                &config.p2p.listen_address,
                &config,
            );
            info!("main", "P2P network started", "listen_address" => config.p2p.listen_address.clone());

            // Start RPC server in a separate thread
            let rpc_bind_address = if config.rpc.bind_address.trim().is_empty() {
                format!("127.0.0.1:{}", config.rpc.http_port)
            } else {
                config.rpc.bind_address.clone()
            };
            let cors_enabled = config.rpc.cors_enabled;
            let cors_origins = config.rpc.cors_origins.clone();
            let _rpc_handle = std::thread::spawn(move || {
                rpc::rpc_server::start_rpc_server(&rpc_bind_address, cors_enabled, cors_origins);
            });

            // Node initialized with core systems
            info!("main", "Node initialized with RPC, P2P, and consensus systems", "rpc_port" => config.rpc.http_port, "p2p_address" => config.p2p.listen_address.clone(), "consensus" => config.consensus.algorithm.clone());

            // Check if we just reset - if so, skip network sync to start fresh
            let reset_flag_path = "data/.reset_flag";
            let should_sync = !std::path::Path::new(reset_flag_path).exists();

            if should_sync {
                let sync_result = {
                    let mut manager = SYNC_MANAGER.lock().unwrap();
                    manager.attach_network(Arc::clone(&p2p_network));
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
                        eprintln!("⚠️  Sync failed before consensus: {}", err);
                    }
                }
            } else {
                // Remove reset flag after first run
                std::fs::remove_file(reset_flag_path).ok();
                info!("main", "Starting fresh after reset - skipping network sync", "height" => 0);
            }

            // Self-register as validator if not already registered
            if config.node.auto_register_validator {
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
                        info!("main", "Self-registering as validator", "address" => validator_address.clone());

                        // 1000 SNRG in nWei (1 SNRG = 1_000_000_000 nWei)
                        let funding_amount: u64 = 1000_000_000_000;
                        let stake_amount: u64 = 1000_000_000_000;

                        // First, mint 1000 SNRG to self
                        let token_manager = TOKEN_MANAGER.clone();
                        let current_balance = token_manager.get_balance(&validator_address, "SNRG");

                        if current_balance < funding_amount {
                            match token_manager.mint_tokens(
                                &validator_address,
                                "SNRG",
                                funding_amount,
                            ) {
                                Ok(_) => {
                                    info!("main", "Self-funded with 1000 SNRG", "address" => validator_address.clone());
                                }
                                Err(e) => {
                                    eprintln!("⚠️  Failed to self-fund: {}", e);
                                }
                            }
                        }

                        // Create and submit validator registration
                        let registration = ValidatorRegistration {
                            address: validator_address.clone(),
                            public_key: validator_address.clone(),
                            name: format!(
                                "Validator-{}",
                                &validator_address[..8.min(validator_address.len())]
                            ),
                            stake_amount,
                            submitted_at: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                            registration_tx_hash: format!("self-reg-{}", validator_address.clone()),
                        };

                        // Register the validator
                        if let Ok(_) = validator_manager.register_validator(registration) {
                            info!("main", "Validator registration submitted", "address" => validator_address.clone());

                            // Auto-approve for testnet-beta
                            if let Ok(_) = validator_manager.approve_validator(&validator_address) {
                                info!("main", "Validator self-approved and activated", "address" => validator_address.clone());

                                // Stake the tokens
                                match token_manager.stake_tokens(
                                    &validator_address,
                                    &validator_address,
                                    "SNRG",
                                    stake_amount,
                                ) {
                                    Ok(_) => {
                                        info!("main", "Self-staked 1000 SNRG", "address" => validator_address.clone());
                                    }
                                    Err(e) => {
                                        eprintln!("⚠️  Failed to self-stake: {}", e);
                                    }
                                }
                            }
                        }
                    } else if is_registered {
                        info!("main", "Already registered as validator", "address" => validator_address.clone());
                    }
                }
            } else {
                info!("main", "Auto validator registration disabled (bootstrap will rely on manual registrations)");
            }

            // Start consensus in a separate thread
            info!("main", "Starting consensus engine", "algorithm" => config.consensus.algorithm.clone());
            let consensus_handle = std::thread::spawn(|| {
                let mut consensus = ProofOfSynergy::new();
                consensus.initialize();
                consensus.execute();
            });

            info!("main", "Node is running. Press Ctrl+C to stop.");

            // Set up signal handler for graceful shutdown
            let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
            let r = running.clone();

            ctrlc::set_handler(move || {
                println!("\nReceived shutdown signal...");
                r.store(false, std::sync::atomic::Ordering::SeqCst);
            })
            .expect("Error setting Ctrl-C handler");

            // Keep the node running
            while running.load(std::sync::atomic::Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }

            info!("main", "Node shutdown gracefully");

            // Clean up PID file
            fs::remove_file("data/synergy-testbeta.pid").ok();

            // Wait for threads to finish
            consensus_handle.join().ok();
            // Note: RPC thread will be killed when main exits
        }

        "keygen" | "generate-keypair" => {
            use base64::{engine::general_purpose, Engine as _};
            use pqcrypto_falcon::falcon1024;
            use pqcrypto_traits::sign::{PublicKey as _, SecretKey as _};
            use std::fs;
            use std::path::PathBuf;
            use synergy_testbeta::address::generate_class_based_address;

            // Parse arguments
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

            // Generate FN-DSA-1024 keypair
            let (pk, sk) = falcon1024::keypair();

            // Encode keys as base64 for storage
            let public_key_b64 = general_purpose::STANDARD.encode(pk.as_bytes());
            let private_key_b64 = general_purpose::STANDARD.encode(sk.as_bytes());

            // Generate address if class is provided
            let address = if let Some(class) = node_class {
                generate_class_based_address(pk.as_bytes(), class)
            } else {
                String::new()
            };

            // Write keys to files if output directory is specified
            if let Some(ref output_path) = output_dir {
                // Create output directory if it doesn't exist
                if let Err(e) = fs::create_dir_all(output_path) {
                    eprintln!("Failed to create output directory: {}", e);
                    process::exit(1);
                }

                // Write public key
                let public_key_path = output_path.join("public.key");
                if let Err(e) = fs::write(&public_key_path, &public_key_b64) {
                    eprintln!("Failed to write public key: {}", e);
                    process::exit(1);
                }

                // Write private key
                let private_key_path = output_path.join("private.key");
                if let Err(e) = fs::write(&private_key_path, &private_key_b64) {
                    eprintln!("Failed to write private key: {}", e);
                    process::exit(1);
                }
            }

            // Output the address (this is what control panel expects)
            if !address.is_empty() {
                println!("{}", address);
            } else {
                eprintln!("Error: --class is required to generate an address");
                process::exit(1);
            }
        }

        "status" => {
            // Load configuration
            let config = match load_node_config(None) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("Failed to load configuration: {}", e);
                    process::exit(1);
                }
            };

            // Initialize logger
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
                        println!("Usage: synergy-testbeta start --node-type <template-name>");
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
                            std::thread::sleep(std::time::Duration::from_secs(2));
                        }
                    }
                }
            }

            println!("Starting node...");
            println!("Please run: synergy-testbeta start [OPTIONS]");
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
                        print_usage();
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
            use std::fs;
            use synergy_testbeta::validator::ValidatorRegistration;

            // Parse arguments
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

            // Load public key from private key file (assuming hex format)
            let private_key_hex = fs::read_to_string(&key_path.as_ref().unwrap())
                .map_err(|e| format!("Failed to read key file: {}", e))
                .unwrap_or_else(|e| {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                });

            // For now, use the private key hex as public key identifier
            // In production, this should derive the public key properly
            let public_key = private_key_hex.trim().to_string();

            let addr = address.unwrap();

            // Create registration
            let registration = ValidatorRegistration {
                address: addr.clone(),
                public_key,
                name: "Control Panel Node".to_string(),
                stake_amount: 1000, // Default stake for testnet-beta
                submitted_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                registration_tx_hash: format!(
                    "reg_{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                ),
            };

            // Register validator
            let validator_manager = synergy_testbeta::validator::VALIDATOR_MANAGER.clone();
            match validator_manager.register_validator(registration) {
                Ok(result) => {
                    println!("✅ {}", result);

                    // Auto-approve for testnet-beta
                    if let Err(e) = validator_manager.approve_validator(&addr) {
                        eprintln!(
                            "Warning: Registration succeeded but auto-approval failed: {}",
                            e
                        );
                    } else {
                        println!("✅ Validator auto-approved for testnet-beta");
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
        }

        "sync" => {
            use synergy_testbeta::config::load_node_config;

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
            println!(
                "Build: {} ({})",
                env!("CARGO_PKG_VERSION"),
                std::env::consts::OS
            );
        }

        "help" | "--help" | "-h" => {
            print_usage();
        }

        _ => {
            eprintln!("Unknown subcommand: {}", subcommand);
            eprintln!();
            print_usage();
            process::exit(1);
        }
    }
}
