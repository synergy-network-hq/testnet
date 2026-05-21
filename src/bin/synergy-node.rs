use synergy_testnet::aegis_tx_tool::{
    build_fixture_report, sign_with_new_aegis_transaction_key, AegisTxBuildOptions,
};
use synergy_testnet::synergy_types::{ChainId, NetworkId};

fn main() {
    if let Err(error) = run() {
        eprintln!("synergy-node failed closed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let command = args.first().map(String::as_str).unwrap_or("help");
    match command {
        "tx" => run_tx_command(&args)?,
        "dag" => run_dag_command(&args)?,
        "diagnose-sync-target" => {
            require_testnet_args(&args)?;
            let rpc_url = arg_value(&args, "--rpc-url")
                .unwrap_or_else(|| "https://testnet-core-rpc.synergy-network.io".to_string());
            let expected_genesis_hash = arg_value(&args, "--expected-genesis-hash");
            let report = diagnose_sync_target(&rpc_url, expected_genesis_hash.as_deref())?;
            println!("{report}");
        }
        "sync-from-archive" | "self-heal-from-archive" => {
            require_testnet_args(&args)?;
            let archive_url = arg_value(&args, "--archive-url")
                .ok_or_else(|| format!("{command} requires --archive-url <url>"))?;
            let expected_genesis_hash = arg_value(&args, "--expected-genesis-hash")
                .ok_or_else(|| format!("{command} requires --expected-genesis-hash <hash>"))?;
            if command == "self-heal-from-archive" {
                arg_value(&args, "--divergence-height").ok_or_else(|| {
                    "self-heal-from-archive requires --divergence-height <height>".to_string()
                })?;
            }
            return Err(format!(
                "{command} is not yet wired to install archive state. Refusing to mutate local chain data from {archive_url} with expected_genesis_hash={expected_genesis_hash} until catalog, manifest, content root, state root, chunks, and every QC are verified through aegis-pqvm."
            ));
        }
        _ => {
            println!("Commands:");
            println!("  synergy-node tx create-aegis --chain-id 1264 --network-id synergy-testnet-v2 [tx options]");
            println!("  synergy-node tx sign-aegis --chain-id 1264 --network-id synergy-testnet-v2 [tx options]");
            println!("  synergy-node tx submit-aegis --chain-id 1264 --network-id synergy-testnet-v2 [tx options]");
            println!("  synergy-node dag submit-test-fixture --real-aegis-pqvm --chain-id 1264 --network-id synergy-testnet-v2");
            println!("  synergy-node diagnose-sync-target --rpc-url <url> --chain-id 1264 --network-id synergy-testnet-v2 [--expected-genesis-hash <hash>]");
            println!("  synergy-node sync-from-archive --archive-url <url> --chain-id 1264 --network-id synergy-testnet-v2 --expected-genesis-hash <hash>");
            println!("  synergy-node self-heal-from-archive --archive-url <url> --divergence-height <height> --chain-id 1264 --network-id synergy-testnet-v2 --expected-genesis-hash <hash>");
        }
    }
    Ok(())
}

fn run_tx_command(args: &[String]) -> Result<(), String> {
    require_testnet_args(args)?;
    let subcommand = args.get(1).map(String::as_str).unwrap_or("help");
    match subcommand {
        "create-aegis" | "sign-aegis" => {
            let report = sign_with_new_aegis_transaction_key(tx_options_from_args(args)?)?;
            let mut output = signed_tx_summary(subcommand, &report);
            if args.iter().any(|arg| arg == "--include-signed-transaction") {
                output["signed_transaction"] = serde_json::to_value(&report.transaction)
                    .map_err(|error| format!("failed to serialize signed transaction: {error}"))?;
                output["canonical_tx_bytes_hex"] =
                    serde_json::Value::String(report.canonical_tx_bytes_hex);
            }
            print_json(output)?;
        }
        "submit-aegis" => {
            let report = sign_with_new_aegis_transaction_key(tx_options_from_args(args)?)?;
            let mut output = signed_tx_summary(subcommand, &report);
            if let Some(rpc_url) = arg_value(args, "--rpc-url") {
                let response = submit_aegis_transaction(
                    &rpc_url,
                    "synergy_submitAegisDagTransaction",
                    &report.submission_envelope,
                )?;
                output["live_submission_status"] =
                    serde_json::Value::String("submitted_to_rpc".to_string());
                output["rpc_url"] = serde_json::Value::String(rpc_url);
                output["rpc_response"] = response;
            } else {
                output["live_submission_status"] = serde_json::Value::String(
                    "not_attempted: pass --rpc-url to submit through synergy_submitAegisTransaction"
                        .to_string(),
                );
            }
            print_json(output)?;
        }
        _ => {
            println!("Commands:");
            println!("  synergy-node tx create-aegis --chain-id 1264 --network-id synergy-testnet-v2 [--sender <uma>] [--receiver <uma>] [--nonce <n>] [--amount-nwei <n>] [--gas-limit <n>] [--max-fee-nwei <n>] [--ttl-height <h>] [--read <key>] [--write <key>] [--dependency <tx_id>] [--payload <text>]");
            println!("  synergy-node tx sign-aegis --chain-id 1264 --network-id synergy-testnet-v2 [same options]");
            println!("  synergy-node tx submit-aegis --chain-id 1264 --network-id synergy-testnet-v2 [same options]");
        }
    }
    Ok(())
}

fn run_dag_command(args: &[String]) -> Result<(), String> {
    require_testnet_args(args)?;
    let subcommand = args.get(1).map(String::as_str).unwrap_or("help");
    match subcommand {
        "submit-test-fixture" => {
            if !args.iter().any(|arg| arg == "--real-aegis-pqvm") {
                return Err(
                    "dag submit-test-fixture requires --real-aegis-pqvm; wallet CLI and demo data paths are refused"
                        .to_string(),
                );
            }
            let report = build_fixture_report()?;
            let rpc_url = arg_value(args, "--rpc-url");
            let rpc_submissions = if let Some(rpc_url) = rpc_url.as_deref() {
                report
                    .transactions
                    .iter()
                    .map(|tx| {
                        submit_aegis_transaction(
                            rpc_url,
                            "synergy_submitAegisDagTransaction",
                            &tx.submission_envelope,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                Vec::new()
            };
            print_json(serde_json::json!({
                "command": subcommand,
                "aegis_pqvm_path": "synergy_testnet::crypto::aegis_pqvm::AegisPqvmSigner",
                "wallet_cli_used": false,
                "demo_data_used": false,
                "chain_id": report.chain_id,
                "network_id": report.network_id,
                "key_id": report.key_id,
                "key_role": report.key_role,
                "transactions": report.transactions.iter().map(|tx| {
                    serde_json::json!({
                        "tx_id": tx.tx_id,
                        "key_id": tx.key_id,
                        "key_role": tx.key_role,
                        "signature_verification_result": tx.signature_verification_result,
                        "dag_node_id": tx.dag_node_id,
                        "admission_result": tx.admission_result,
                        "signature_bytes_len": tx.transaction.aegis_pq_signature.signature_bytes.len(),
                    })
                }).collect::<Vec<_>>(),
                "ready_frontier": report.ready_frontier,
                "selected_ancestor_closed_set": report.selected_ancestor_closed_set,
                "tx_order_root": report.tx_order_root,
                "dag_frontier_root": report.dag_frontier_root,
                "live_submission_status": if rpc_url.is_some() { "submitted_to_rpc" } else { "not_attempted: pass --rpc-url to submit through synergy_submitAegisDagTransaction" },
                "rpc_url": rpc_url,
                "rpc_submissions": rpc_submissions,
                "atlas_ingestion_status": if rpc_submissions.is_empty() { report.atlas_ingestion_status } else { "submitted_to_rpc: verify finalized block inclusion and Atlas DAG API from canonical chain data".to_string() },
            }))?;
        }
        _ => {
            println!("Commands:");
            println!("  synergy-node dag submit-test-fixture --real-aegis-pqvm --chain-id 1264 --network-id synergy-testnet-v2 [--rpc-url <url>]");
        }
    }
    Ok(())
}

fn signed_tx_summary(
    command: &str,
    report: &synergy_testnet::aegis_tx_tool::AegisSignedTxReport,
) -> serde_json::Value {
    serde_json::json!({
        "command": command,
        "aegis_pqvm_path": "synergy_testnet::crypto::aegis_pqvm::AegisPqvmSigner",
        "wallet_cli_used": false,
        "tx_id": report.tx_id,
        "key_id": report.key_id,
        "key_role": report.key_role,
        "signature_verification_result": report.signature_verification_result,
        "dag_node_id": report.dag_node_id,
        "admission_result": report.admission_result,
        "signature_bytes_len": report.transaction.aegis_pq_signature.signature_bytes.len(),
        "chain_id": report.transaction.chain_id.0,
        "network_id": report.transaction.network_id.0,
        "sender": report.transaction.sender_uma_or_account,
        "receiver": report.transaction.receiver_uma_or_account,
        "aegis_public_key": report.public_key,
        "key_lifecycle_record": report.lifecycle_record,
        "rpc_transaction": report.rpc_transaction,
    })
}

fn print_json(value: serde_json::Value) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(&value)
            .map_err(|error| format!("failed to serialize JSON report: {error}"))?
    );
    Ok(())
}

fn submit_aegis_transaction(
    rpc_url: &str,
    method: &str,
    envelope: &synergy_testnet::aegis_tx_tool::AegisTxSubmissionEnvelope,
) -> Result<serde_json::Value, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|error| format!("failed to initialize RPC client: {error}"))?;
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": [envelope],
    });
    let response = client
        .post(rpc_url)
        .json(&request)
        .send()
        .map_err(|error| format!("failed to submit Aegis transaction to {rpc_url}: {error}"))?;
    let status = response.status();
    let value = response
        .json::<serde_json::Value>()
        .map_err(|error| format!("failed to parse RPC response: {error}"))?;
    if !status.is_success() {
        return Err(format!("RPC returned HTTP {status}: {value}"));
    }
    Ok(value)
}

fn tx_options_from_args(args: &[String]) -> Result<AegisTxBuildOptions, String> {
    let mut options = AegisTxBuildOptions::default();
    if let Some(sender) = arg_value(args, "--sender") {
        options.sender = sender.clone();
        options.signer_uma_id = sender;
    }
    if let Some(signer_uma_id) = arg_value(args, "--signer-uma-id") {
        options.signer_uma_id = signer_uma_id;
    }
    if let Some(receiver) = arg_value(args, "--receiver") {
        options.receiver = receiver;
    }
    if let Some(nonce) = arg_value(args, "--nonce") {
        options.nonce = nonce
            .parse::<u64>()
            .map_err(|error| format!("invalid --nonce: {error}"))?;
    }
    if let Some(amount) = arg_value(args, "--amount-nwei") {
        options.amount_nwei = amount
            .parse::<u128>()
            .map_err(|error| format!("invalid --amount-nwei: {error}"))?;
    }
    if let Some(gas_limit) = arg_value(args, "--gas-limit") {
        options.gas_limit = gas_limit
            .parse::<u64>()
            .map_err(|error| format!("invalid --gas-limit: {error}"))?;
    }
    if let Some(max_fee) = arg_value(args, "--max-fee-nwei") {
        options.max_fee_nwei = max_fee
            .parse::<u128>()
            .map_err(|error| format!("invalid --max-fee-nwei: {error}"))?;
    }
    if let Some(ttl) = arg_value(args, "--ttl-height") {
        options.ttl_height = ttl
            .parse::<u64>()
            .map_err(|error| format!("invalid --ttl-height: {error}"))?;
    }
    if let Some(epoch) = arg_value(args, "--epoch") {
        options.epoch = epoch
            .parse::<u64>()
            .map_err(|error| format!("invalid --epoch: {error}"))?;
    }
    if let Some(payload) = arg_value(args, "--payload") {
        options.payload = payload.into_bytes();
    }
    options.read_set_hint = arg_values(args, "--read");
    let writes = arg_values(args, "--write");
    if !writes.is_empty() {
        options.write_set_hint = writes;
    }
    options.explicit_dependencies = arg_values(args, "--dependency");
    Ok(options)
}

fn diagnose_sync_target(
    rpc_url: &str,
    expected_genesis_hash: Option<&str>,
) -> Result<String, String> {
    let chain_id_result = rpc_call(rpc_url, "synergy_getChainId", serde_json::json!([]));
    let node_info_result = rpc_call(rpc_url, "synergy_nodeInfo", serde_json::json!([]));
    let latest_block_result = rpc_call(rpc_url, "synergy_getLatestBlock", serde_json::json!([]));
    let genesis_block_result =
        rpc_call(rpc_url, "synergy_getBlockByNumber", serde_json::json!([0]));
    let height_result = rpc_call(rpc_url, "synergy_blockNumber", serde_json::json!([]))
        .or_else(|_| rpc_call(rpc_url, "synergy_getBlockNumber", serde_json::json!([])));

    let chain_id = chain_id_result
        .as_ref()
        .ok()
        .and_then(parse_u64ish)
        .or_else(|| {
            node_info_result
                .as_ref()
                .ok()
                .and_then(|value| {
                    value
                        .get("chainId")
                        .or_else(|| value.get("chain_id"))
                        .cloned()
                })
                .and_then(|value| parse_u64ish(&value))
        });
    let reported_network_id = node_info_result
        .as_ref()
        .ok()
        .and_then(|value| {
            value
                .get("networkId")
                .or_else(|| value.get("network_id"))
                .cloned()
        })
        .and_then(|value| {
            value
                .as_str()
                .map(str::to_string)
                .or_else(|| Some(value.to_string()))
        });
    let latest_height = height_result
        .as_ref()
        .ok()
        .and_then(parse_u64ish)
        .or_else(|| {
            latest_block_result
                .as_ref()
                .ok()
                .and_then(|value| {
                    value
                        .get("block_index")
                        .or_else(|| value.get("height"))
                        .cloned()
                })
                .and_then(|value| parse_u64ish(&value))
        });
    let latest_hash = latest_block_result
        .as_ref()
        .ok()
        .and_then(|value| value.get("hash").and_then(serde_json::Value::as_str))
        .map(str::to_string);
    let genesis_hash = genesis_block_result
        .as_ref()
        .ok()
        .and_then(block_hash_from_value)
        .or_else(|| {
            latest_block_result
                .as_ref()
                .ok()
                .and_then(|value| {
                    value
                        .get("genesis_hash")
                        .or_else(|| value.get("genesisHash"))
                        .and_then(serde_json::Value::as_str)
                })
                .map(str::to_string)
        });
    let genesis_verified = match (expected_genesis_hash, genesis_hash.as_deref()) {
        (Some(expected), Some(actual)) => actual.eq_ignore_ascii_case(expected),
        (Some(_), None) => false,
        (None, Some(_)) => true,
        (None, None) => false,
    };
    let canonical_network_id = genesis_verified.then(|| "synergy-testnet-v2".to_string());
    let usable = chain_id == Some(1264)
        && canonical_network_id.as_deref() == Some("synergy-testnet-v2")
        && latest_height.is_some()
        && genesis_verified;

    Ok(serde_json::json!({
        "source": "rpc",
        "source_url": rpc_url,
        "chain_id": chain_id,
        "network_id": canonical_network_id,
        "reported_network_id": reported_network_id,
        "genesis_hash": genesis_hash,
        "expected_genesis_hash": expected_genesis_hash,
        "genesis_verified": genesis_verified,
        "latest_height": latest_height,
        "latest_hash": latest_hash,
        "latest_qc_hash": latest_block_result
            .as_ref()
            .ok()
            .and_then(|value| value.get("qc_hash").or_else(|| value.get("latest_qc_hash")).cloned()),
        "verification_result": if usable { "accepted" } else { "rejected" },
        "usable_for_sync_target": usable,
        "errors": {
            "chain_id": chain_id_result.err(),
            "node_info": node_info_result.err(),
            "genesis_block": genesis_block_result.err(),
            "latest_block": latest_block_result.err(),
            "height": height_result.err()
        }
    })
    .to_string())
}

fn rpc_call(
    rpc_url: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|error| format!("failed to build HTTP client: {error}"))?;
    let payload = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        }))
        .send()
        .map_err(|error| format!("{method} request failed: {error}"))?
        .json::<serde_json::Value>()
        .map_err(|error| format!("{method} response parse failed: {error}"))?;
    if let Some(error) = payload.get("error") {
        return Err(format!("{method} returned error: {error}"));
    }
    payload
        .get("result")
        .cloned()
        .ok_or_else(|| format!("{method} response did not include result"))
}

fn block_hash_from_value(value: &serde_json::Value) -> Option<String> {
    value
        .get("hash")
        .or_else(|| value.get("block_hash"))
        .or_else(|| value.get("blockHash"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

fn parse_u64ish(value: &serde_json::Value) -> Option<u64> {
    if let Some(number) = value.as_u64() {
        return Some(number);
    }
    let text = value.as_str()?.trim();
    if let Some(hex) = text.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).ok()
    } else {
        text.parse::<u64>().ok()
    }
}

fn require_testnet_args(args: &[String]) -> Result<(), String> {
    let chain_id = arg_value(args, "--chain-id")
        .ok_or_else(|| "missing --chain-id 1264".to_string())?
        .parse::<u64>()
        .map_err(|error| format!("invalid --chain-id: {error}"))?;
    ChainId(chain_id).require_testnet_v2()?;
    let network_id = arg_value(args, "--network-id")
        .ok_or_else(|| "missing --network-id synergy-testnet-v2".to_string())?;
    NetworkId(network_id).require_testnet_v2()?;
    Ok(())
}

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}

fn arg_values(args: &[String], name: &str) -> Vec<String> {
    args.windows(2)
        .filter(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
        .collect()
}
