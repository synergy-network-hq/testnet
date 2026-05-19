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
            println!("  synergy-node diagnose-sync-target --rpc-url <url> --chain-id 1264 --network-id synergy-testnet-v2 [--expected-genesis-hash <hash>]");
            println!("  synergy-node sync-from-archive --archive-url <url> --chain-id 1264 --network-id synergy-testnet-v2 --expected-genesis-hash <hash>");
            println!("  synergy-node self-heal-from-archive --archive-url <url> --divergence-height <height> --chain-id 1264 --network-id synergy-testnet-v2 --expected-genesis-hash <hash>");
        }
    }
    Ok(())
}

fn diagnose_sync_target(
    rpc_url: &str,
    expected_genesis_hash: Option<&str>,
) -> Result<String, String> {
    let chain_id_result = rpc_call(rpc_url, "synergy_getChainId", serde_json::json!([]));
    let node_info_result = rpc_call(rpc_url, "synergy_nodeInfo", serde_json::json!([]));
    let latest_block_result = rpc_call(rpc_url, "synergy_getLatestBlock", serde_json::json!([]));
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
    let network_id = node_info_result
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
    let genesis_hash = latest_block_result
        .as_ref()
        .ok()
        .and_then(|value| {
            value
                .get("genesis_hash")
                .or_else(|| value.get("genesisHash"))
                .and_then(serde_json::Value::as_str)
        })
        .map(str::to_string);
    let genesis_verified = match (expected_genesis_hash, genesis_hash.as_deref()) {
        (Some(expected), Some(actual)) => actual.eq_ignore_ascii_case(expected),
        (Some(_), None) => false,
        (None, _) => true,
    };
    let usable = chain_id == Some(1264)
        && network_id
            .as_deref()
            .map(|value| value.contains("1264") || value.contains("synergy-testnet-v2"))
            .unwrap_or(false)
        && latest_height.is_some()
        && genesis_verified;

    Ok(serde_json::json!({
        "source": "rpc",
        "source_url": rpc_url,
        "chain_id": chain_id,
        "network_id": network_id,
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
