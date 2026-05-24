use crate::block::BlockChain;
use crate::consensus::consensus_algorithm::ProofOfSynergy;
use crate::consensus::dual_quorum::DualQuorumConsensus;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

const EXPECTED_CHAIN_ID: u64 = 1264;
const EXPECTED_NETWORK_ID: &str = "synergy-testnet-v2";
const EXPECTED_GENESIS_HASH: &str =
    "f79011f2aaddd40b120d47ba723104fafe3c998d4a17097fae018914b95f1789";
const DIAGNOSTIC_STALE_TRANSIENT_VOTE_LOCK_SECS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VoteLockEntry {
    #[serde(default)]
    validator_address: String,
    #[serde(default)]
    block_hash: String,
    #[serde(default)]
    block_index: u64,
    #[serde(default)]
    epoch_number: u64,
    #[serde(default)]
    first_round_number: u64,
    #[serde(default)]
    latest_round_number: u64,
    #[serde(default)]
    proposer: String,
    #[serde(default)]
    created_at: u64,
    #[serde(default)]
    updated_at: u64,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn configured_chain_id() -> u64 {
    crate::config::load_node_config(None)
        .ok()
        .map(|config| config.blockchain.chain_id)
        .unwrap_or(EXPECTED_CHAIN_ID)
}

fn configured_network_id() -> String {
    crate::config::load_node_config(None)
        .ok()
        .map(|config| config.network.network_id)
        .filter(|network_id| !network_id.is_empty())
        .unwrap_or_else(|| EXPECTED_NETWORK_ID.to_string())
}

fn configured_genesis_hash() -> String {
    crate::genesis::load_canonical_genesis_for_runtime()
        .map(|genesis| genesis.hash().to_string())
        .unwrap_or_else(|_| EXPECTED_GENESIS_HASH.to_string())
}

fn chain_identity() -> Value {
    let chain_id = configured_chain_id();
    json!({
        "chain_id": chain_id,
        "chain_id_hex": format!("0x{chain_id:x}"),
        "network_id": configured_network_id(),
        "genesis_hash": configured_genesis_hash(),
    })
}

fn require_local_testnet_v2() -> Result<(), String> {
    let chain_id = configured_chain_id();
    let network_id = configured_network_id();
    let genesis_hash = crate::genesis::load_canonical_genesis_for_runtime()
        .map(|genesis| genesis.hash().to_string())
        .map_err(|error| format!("genesis unavailable; refusing mutation: {error}"))?;
    if chain_id != EXPECTED_CHAIN_ID {
        return Err(format!(
            "wrong chain_id {chain_id}; expected {EXPECTED_CHAIN_ID}"
        ));
    }
    if network_id != EXPECTED_NETWORK_ID {
        return Err(format!(
            "wrong network_id {network_id}; expected {EXPECTED_NETWORK_ID}"
        ));
    }
    if !genesis_hash.eq_ignore_ascii_case(EXPECTED_GENESIS_HASH) {
        return Err(format!(
            "wrong genesis_hash {genesis_hash}; expected {EXPECTED_GENESIS_HASH}"
        ));
    }
    Ok(())
}

fn read_json_file(path: &str) -> Option<Value> {
    let path = crate::utils::resolve_data_path(path);
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok())
}

fn latest_canonical_lock_height() -> Option<u64> {
    let map = read_json_file("data/canonical_locks.json")?;
    map.as_object()?
        .keys()
        .filter_map(|key| key.parse::<u64>().ok())
        .max()
}

fn latest_committed_qc() -> Option<Value> {
    let path = crate::utils::resolve_data_path("data/committed_qcs.jsonl");
    let content = fs::read_to_string(path).ok()?;
    content
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .and_then(|line| serde_json::from_str::<Value>(line).ok())
}

fn vote_lock_entries() -> (String, Vec<VoteLockEntry>, Option<String>) {
    let path = crate::utils::resolve_data_path("data/consensus_vote_locks.json");
    let path_string = path.to_string_lossy().to_string();
    let Ok(content) = fs::read_to_string(&path) else {
        return (path_string, Vec::new(), None);
    };
    let parsed = match serde_json::from_str::<BTreeMap<String, VoteLockEntry>>(&content) {
        Ok(parsed) => parsed,
        Err(error) => return (path_string, Vec::new(), Some(error.to_string())),
    };
    (path_string, parsed.into_values().collect(), None)
}

pub fn diagnose_vote_locks(finalized_height: Option<u64>) -> Value {
    let finalized_height = finalized_height.or_else(latest_canonical_lock_height);
    let (path, locks, parse_error) = vote_lock_entries();
    let now = now_secs();
    let mut hashes_by_height: BTreeMap<u64, BTreeSet<String>> = BTreeMap::new();
    let mut stale_hashes_by_height: BTreeMap<u64, BTreeSet<String>> = BTreeMap::new();
    let mut above_finalized = Vec::new();
    let mut stale_above_finalized = Vec::new();
    let mut fresh_above_finalized = Vec::new();
    for lock in locks.iter().filter(|lock| {
        finalized_height
            .map(|height| lock.block_index > height)
            .unwrap_or(false)
    }) {
        let age_seconds = now.saturating_sub(lock.updated_at);
        hashes_by_height
            .entry(lock.block_index)
            .or_default()
            .insert(lock.block_hash.clone());
        let item = json!({
            "validator_address": lock.validator_address,
            "height": lock.block_index,
            "block_hash": lock.block_hash,
            "epoch": lock.epoch_number,
            "first_round": lock.first_round_number,
            "latest_round": lock.latest_round_number,
            "proposer": lock.proposer,
            "age_seconds": age_seconds,
        });
        if age_seconds >= DIAGNOSTIC_STALE_TRANSIENT_VOTE_LOCK_SECS {
            stale_hashes_by_height
                .entry(lock.block_index)
                .or_default()
                .insert(lock.block_hash.clone());
            stale_above_finalized.push(item.clone());
        } else {
            fresh_above_finalized.push(item.clone());
        }
        above_finalized.push(item);
    }
    let conflicting_heights = hashes_by_height
        .into_iter()
        .filter(|(_, hashes)| hashes.len() > 1)
        .map(|(height, hashes)| json!({"height": height, "hashes": hashes}))
        .collect::<Vec<_>>();
    let stale_conflicting_heights = stale_hashes_by_height
        .into_iter()
        .filter(|(_, hashes)| hashes.len() > 1)
        .map(|(height, hashes)| json!({"height": height, "hashes": hashes}))
        .collect::<Vec<_>>();

    json!({
        "chain": chain_identity(),
        "vote_lock_path": path,
        "parse_error": parse_error,
        "finalized_height": finalized_height,
        "total_vote_locks": locks.len(),
        "locks_above_finalized": above_finalized.len(),
        "fresh_locks_above_finalized": fresh_above_finalized.len(),
        "stale_locks_above_finalized": stale_above_finalized.len(),
        "stale_threshold_seconds": DIAGNOSTIC_STALE_TRANSIENT_VOTE_LOCK_SECS,
        "conflicting_heights_above_finalized": conflicting_heights,
        "stale_conflicting_heights_above_finalized": stale_conflicting_heights,
        "locks": above_finalized,
        "stale_locks": stale_above_finalized,
        "fresh_locks": fresh_above_finalized,
    })
}

pub fn quarantine_status() -> Value {
    let marker_paths = [
        "data/validator_quarantine.json",
        "data/validator_quarantine_peer_evidence.json",
    ]
    .into_iter()
    .filter_map(|path| {
        let resolved = crate::utils::resolve_data_path(path);
        resolved
            .exists()
            .then(|| resolved.to_string_lossy().to_string())
    })
    .collect::<Vec<_>>();

    json!({
        "chain": chain_identity(),
        "status": if marker_paths.is_empty() { "healthy" } else { "quarantined" },
        "quarantined": !marker_paths.is_empty(),
        "marker_paths": marker_paths,
    })
}

pub fn divergence_status(chain: &Arc<Mutex<BlockChain>>) -> Value {
    let latest = chain.lock().ok().and_then(|chain| chain.last().cloned());
    json!({
        "chain": chain_identity(),
        "latest_height": latest.as_ref().map(|block| block.block_index),
        "latest_hash": latest.as_ref().map(|block| block.hash.clone()),
        "latest_timestamp": latest.as_ref().map(|block| block.timestamp),
        "canonical_lock_height": latest_canonical_lock_height(),
        "quarantine": quarantine_status(),
        "local_only": true,
        "note": "quorum-peer divergence comparison requires a reconciliation source; this read-only call never chooses a branch by public RPC alone",
    })
}

pub fn diagnose_consensus_stall(chain: &Arc<Mutex<BlockChain>>) -> Value {
    let latest = chain.lock().ok().and_then(|chain| chain.last().cloned());
    let latest_timestamp = latest.as_ref().map(|block| block.timestamp);
    let timestamp_delta_seconds =
        latest_timestamp.map(|timestamp| now_secs().saturating_sub(timestamp));
    let finalized_height = latest_canonical_lock_height();
    let vote_locks = diagnose_vote_locks(finalized_height);
    let stale_locks_above = vote_locks
        .get("stale_locks_above_finalized")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let stale_conflicting_heights = vote_locks
        .get("stale_conflicting_heights_above_finalized")
        .and_then(Value::as_array)
        .map(|items| !items.is_empty())
        .unwrap_or(false);
    let qc = latest_committed_qc();
    let mut categories = Vec::new();
    if timestamp_delta_seconds.unwrap_or(0) > 30 {
        categories.push("no_finalized_block_for_timeout");
    }
    if stale_locks_above > 0 {
        categories.push("transient_vote_lock_above_finalized_height");
    }
    if stale_conflicting_heights {
        categories.push("same_height_competing_transient_vote_locks");
    }
    if quarantine_status()
        .get("quarantined")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        categories.push("local_validator_quarantined");
    }

    json!({
        "chain": chain_identity(),
        "latest_height": latest.as_ref().map(|block| block.block_index),
        "latest_hash": latest.as_ref().map(|block| block.hash.clone()),
        "latest_timestamp": latest_timestamp,
        "timestamp_delta_seconds": timestamp_delta_seconds,
        "canonical_lock_height": finalized_height,
        "latest_committed_qc": qc,
        "vote_locks": vote_locks,
        "categories": categories,
        "stalled": !categories.is_empty(),
        "fail_closed": true,
    })
}

pub fn reconciliation_plan(chain: &Arc<Mutex<BlockChain>>) -> Value {
    let diagnosis = diagnose_consensus_stall(chain);
    let vote_locks = diagnosis
        .get("vote_locks")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let locks_above = vote_locks
        .get("locks_above_finalized")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let quarantined = quarantine_status()
        .get("quarantined")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let recommended_action = if quarantined {
        "self_heal_from_verified_quorum_or_archive_source"
    } else if locks_above > 0 {
        "recover_transient_vote_locks_above_finalized_height"
    } else {
        "observe_or_compare_quorum_peers"
    };
    json!({
        "chain": chain_identity(),
        "recommended_action": recommended_action,
        "diagnosis": diagnosis,
        "mutation_requires_operator_method": true,
        "forbidden_actions": [
            "do_not_regenerate_genesis",
            "do_not_lower_quorum",
            "do_not_copy_keys",
            "do_not_copy_configs",
            "do_not_delete_canonical_locks",
            "do_not_delete_committed_qcs"
        ],
    })
}

pub fn recover_transient_vote_locks(
    finalized_height: Option<u64>,
    min_age_secs: u64,
    reason: &str,
) -> Result<Value, String> {
    require_local_testnet_v2()?;
    let finalized_height = finalized_height
        .or_else(latest_canonical_lock_height)
        .ok_or_else(|| {
            "missing finalized height and no canonical lock file is available".to_string()
        })?;
    let vote_report = DualQuorumConsensus::recover_transient_vote_locks_above_finalized_height(
        finalized_height,
        min_age_secs,
        reason,
    )?;
    let proposal_report = ProofOfSynergy::recover_cached_block_proposals_above_finalized_height(
        finalized_height,
        reason,
    )?;
    Ok(json!({
        "chain": chain_identity(),
        "finalized_height": finalized_height,
        "vote_lock_recovery": vote_report,
        "proposal_cache_recovery": proposal_report,
        "canonical_locks_mutated": false,
        "committed_qcs_mutated": false,
        "keys_or_configs_copied": false,
    }))
}

pub fn self_heal_status() -> Value {
    json!({
        "chain": chain_identity(),
        "status": "idle",
        "automatic_archive_install": "fail_closed_until_quorum_or_archive_manifest_qc_verification_is_wired",
        "quarantine": quarantine_status(),
    })
}

pub fn start_self_heal() -> Result<Value, String> {
    require_local_testnet_v2()?;
    Err("self-heal state installation is not yet enabled: refusing to mutate chain state until quorum/archive source, manifest, state root, and every QC are verified through aegis-pqvm".to_string())
}

pub fn sync_from_canonical_peer() -> Result<Value, String> {
    require_local_testnet_v2()?;
    Err("sync-from-canonical-peer is not yet enabled: refusing to copy state until a verified quorum source and QC/state-root checks are supplied".to_string())
}

pub fn self_heal_from_archive() -> Result<Value, String> {
    require_local_testnet_v2()?;
    Err("self-heal-from-archive is not yet enabled: refusing archive state install until signed catalog, manifest, content root, state root, chunks, and every QC verify through aegis-pqvm".to_string())
}

#[cfg(test)]
mod tests {
    use super::{diagnose_consensus_stall, DIAGNOSTIC_STALE_TRANSIENT_VOTE_LOCK_SECS};
    use crate::block::{Block, BlockChain};
    use serde_json::{json, Value};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    static DIAGNOSTICS_TEST_ENV_LOCK: Mutex<()> = Mutex::new(());

    fn now_secs_for_test() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn test_runtime_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "synergy-diagnostics-{name}-{}-{}",
            std::process::id(),
            now_secs_for_test()
        ));
        fs::create_dir_all(root.join("config")).expect("test config dir should be created");
        fs::create_dir_all(root.join("data")).expect("test data dir should be created");
        root
    }

    fn write_vote_lock(root: &Path, updated_at: u64, second_hash: Option<&str>) {
        let mut locks = serde_json::Map::new();
        locks.insert(
            "synv1a:101".to_string(),
            json!({
                "validator_address": "synv1a",
                "block_hash": "hash-a",
                "block_index": 101,
                "epoch_number": 0,
                "first_round_number": 1,
                "latest_round_number": 1,
                "proposer": "synv1leader",
                "created_at": updated_at,
                "updated_at": updated_at,
            }),
        );
        if let Some(hash) = second_hash {
            locks.insert(
                "synv1b:101".to_string(),
                json!({
                    "validator_address": "synv1b",
                    "block_hash": hash,
                    "block_index": 101,
                    "epoch_number": 0,
                    "first_round_number": 1,
                    "latest_round_number": 1,
                    "proposer": "synv1leader2",
                    "created_at": updated_at,
                    "updated_at": updated_at,
                }),
            );
        }
        fs::write(
            root.join("data/consensus_vote_locks.json"),
            Value::Object(locks).to_string(),
        )
        .expect("test vote locks should be written");
    }

    fn write_canonical_lock(root: &Path) {
        fs::write(
            root.join("data/canonical_locks.json"),
            json!({
                "100": {
                    "block_hash": "finalized-hash",
                    "qc_hash": "qc-hash"
                }
            })
            .to_string(),
        )
        .expect("test canonical lock should be written");
    }

    fn advancing_chain() -> Arc<Mutex<BlockChain>> {
        let mut chain = BlockChain::new();
        chain.add_block(Block::new_with_timestamp(
            100,
            Vec::new(),
            "parent".to_string(),
            "synv1leader".to_string(),
            1,
            now_secs_for_test(),
        ));
        Arc::new(Mutex::new(chain))
    }

    fn with_runtime_root<T>(root: &Path, test: impl FnOnce() -> T) -> T {
        let previous_root = std::env::var("SYNERGY_PROJECT_ROOT").ok();
        let previous_genesis = std::env::var("SYNERGY_GENESIS_FILE").ok();
        std::env::set_var("SYNERGY_PROJECT_ROOT", root);
        std::env::remove_var("SYNERGY_GENESIS_FILE");
        let result = test();
        match previous_root {
            Some(value) => std::env::set_var("SYNERGY_PROJECT_ROOT", value),
            None => std::env::remove_var("SYNERGY_PROJECT_ROOT"),
        }
        match previous_genesis {
            Some(value) => std::env::set_var("SYNERGY_GENESIS_FILE", value),
            None => std::env::remove_var("SYNERGY_GENESIS_FILE"),
        }
        result
    }

    #[test]
    fn fresh_vote_lock_above_finalized_does_not_false_report_stall() {
        let _guard = DIAGNOSTICS_TEST_ENV_LOCK
            .lock()
            .expect("diagnostics env lock should succeed");
        let root = test_runtime_root("fresh-lock");
        write_canonical_lock(&root);
        write_vote_lock(&root, now_secs_for_test(), None);

        let diagnosis = with_runtime_root(&root, || diagnose_consensus_stall(&advancing_chain()));
        let categories = diagnosis
            .get("categories")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        assert!(!diagnosis
            .get("stalled")
            .and_then(Value::as_bool)
            .unwrap_or(true));
        assert!(!categories
            .iter()
            .any(|category| category == "transient_vote_lock_above_finalized_height"));
        assert_eq!(
            diagnosis
                .get("vote_locks")
                .and_then(|locks| locks.get("fresh_locks_above_finalized"))
                .and_then(Value::as_u64),
            Some(1)
        );
    }

    #[test]
    fn stale_conflicting_vote_locks_above_finalized_report_stall() {
        let _guard = DIAGNOSTICS_TEST_ENV_LOCK
            .lock()
            .expect("diagnostics env lock should succeed");
        let root = test_runtime_root("stale-conflict");
        write_canonical_lock(&root);
        write_vote_lock(
            &root,
            now_secs_for_test().saturating_sub(DIAGNOSTIC_STALE_TRANSIENT_VOTE_LOCK_SECS + 5),
            Some("hash-b"),
        );

        let diagnosis = with_runtime_root(&root, || diagnose_consensus_stall(&advancing_chain()));
        let categories = diagnosis
            .get("categories")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        assert!(diagnosis
            .get("stalled")
            .and_then(Value::as_bool)
            .unwrap_or(false));
        assert!(categories
            .iter()
            .any(|category| category == "transient_vote_lock_above_finalized_height"));
        assert!(categories
            .iter()
            .any(|category| category == "same_height_competing_transient_vote_locks"));
        assert_eq!(
            diagnosis
                .get("vote_locks")
                .and_then(|locks| locks.get("stale_locks_above_finalized"))
                .and_then(Value::as_u64),
            Some(2)
        );
    }
}
