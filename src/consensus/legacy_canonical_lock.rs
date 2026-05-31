use crate::block::Block;
use crate::consensus::dual_quorum::QuorumCertificate;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

lazy_static! {
    static ref LEGACY_CANONICAL_LOCK_CACHE: Mutex<BTreeMap<PathBuf, BTreeMap<u64, LegacyCanonicalCommitRecord>>> =
        Mutex::new(BTreeMap::new());
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyCanonicalCommitRecord {
    pub height: u64,
    pub block_hash: String,
    pub parent_hash: String,
    pub validator_id: String,
    pub transactions_root: String,
    pub qc_block_hash: String,
    pub qc_hash: String,
    pub written_at_unix_secs: u64,
}

pub fn verify_legacy_canonical_lock(block: &Block) -> Result<(), String> {
    with_legacy_canonical_locks(|locks| {
        let Some(existing) = locks.get(&block.block_index) else {
            return Ok(());
        };
        if existing.block_hash == block.hash {
            Ok(())
        } else {
            Err(format!(
                "canonical lock at height {} already binds block {}; refusing conflicting block {}",
                block.block_index, existing.block_hash, block.hash
            ))
        }
    })
}

pub fn write_legacy_canonical_lock(block: &Block, qc: &QuorumCertificate) -> Result<(), String> {
    if qc.block_hash != block.hash {
        return Err("cannot write canonical lock with QC for a different block".to_string());
    }

    let record = LegacyCanonicalCommitRecord {
        height: block.block_index,
        block_hash: block.hash.clone(),
        parent_hash: block.previous_hash.clone(),
        validator_id: block.validator_id.clone(),
        transactions_root: block.transactions_root.clone(),
        qc_block_hash: qc.block_hash.clone(),
        qc_hash: legacy_qc_hash(qc)?,
        written_at_unix_secs: current_unix_secs(),
    };

    with_legacy_canonical_locks_mut(|locks| {
        if let Some(existing) = locks.get(&block.block_index) {
            if existing.block_hash == block.hash {
                return Ok(());
            }
            return Err(format!(
                "canonical lock at height {} already binds block {}; refusing conflicting block {}",
                block.block_index, existing.block_hash, block.hash
            ));
        }

        append_legacy_canonical_lock_journal(&record)?;
        locks.insert(block.block_index, record);
        Ok(())
    })
}

pub fn legacy_canonical_commit_record(
    height: u64,
) -> Result<Option<LegacyCanonicalCommitRecord>, String> {
    with_legacy_canonical_locks(|locks| Ok(locks.get(&height).cloned()))
}

pub fn latest_legacy_canonical_commit_record() -> Result<Option<LegacyCanonicalCommitRecord>, String>
{
    with_legacy_canonical_locks(|locks| {
        Ok(locks.iter().next_back().map(|(_, record)| record.clone()))
    })
}

pub fn materialize_legacy_canonical_locks_at(path: &Path) -> Result<(), String> {
    with_legacy_canonical_locks(|locks| persist_legacy_canonical_locks_at(path, locks))
}

pub fn materialize_legacy_canonical_locks_from_to(
    source_compact_path: &Path,
    target_compact_path: &Path,
) -> Result<(), String> {
    let locks = load_legacy_canonical_locks_from_disk(source_compact_path)?;
    persist_legacy_canonical_locks_at(target_compact_path, &locks)
}

fn load_legacy_canonical_locks_from_disk(
    path: &Path,
) -> Result<BTreeMap<u64, LegacyCanonicalCommitRecord>, String> {
    if !path.exists() {
        return load_legacy_canonical_lock_journal(path, BTreeMap::new());
    }

    let bytes = fs::read(path)
        .map_err(|error| format!("failed to read canonical lock store {:?}: {error}", path))?;
    let locks = if bytes.is_empty() {
        BTreeMap::new()
    } else {
        serde_json::from_slice(&bytes)
            .map_err(|error| format!("failed to parse canonical lock store {:?}: {error}", path))?
    };
    load_legacy_canonical_lock_journal(path, locks)
}

fn with_legacy_canonical_locks<T>(
    operation: impl FnOnce(&BTreeMap<u64, LegacyCanonicalCommitRecord>) -> Result<T, String>,
) -> Result<T, String> {
    let path = legacy_canonical_lock_path();
    let mut cache = LEGACY_CANONICAL_LOCK_CACHE
        .lock()
        .map_err(|_| "canonical lock cache mutex is poisoned".to_string())?;
    if !cache.contains_key(&path) {
        cache.insert(path.clone(), load_legacy_canonical_locks_from_disk(&path)?);
    }
    operation(
        cache
            .get(&path)
            .expect("canonical lock cache entry should exist"),
    )
}

fn with_legacy_canonical_locks_mut<T>(
    operation: impl FnOnce(&mut BTreeMap<u64, LegacyCanonicalCommitRecord>) -> Result<T, String>,
) -> Result<T, String> {
    let path = legacy_canonical_lock_path();
    let mut cache = LEGACY_CANONICAL_LOCK_CACHE
        .lock()
        .map_err(|_| "canonical lock cache mutex is poisoned".to_string())?;
    if !cache.contains_key(&path) {
        cache.insert(path.clone(), load_legacy_canonical_locks_from_disk(&path)?);
    }
    operation(
        cache
            .get_mut(&path)
            .expect("canonical lock cache entry should exist"),
    )
}

fn load_legacy_canonical_lock_journal(
    compact_path: &Path,
    mut locks: BTreeMap<u64, LegacyCanonicalCommitRecord>,
) -> Result<BTreeMap<u64, LegacyCanonicalCommitRecord>, String> {
    let journal_path = legacy_canonical_lock_journal_path_for(compact_path);
    if !journal_path.exists() {
        return Ok(locks);
    }
    let file = fs::File::open(&journal_path).map_err(|error| {
        format!(
            "failed to read canonical lock journal {:?}: {error}",
            journal_path
        )
    })?;
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|error| {
            format!(
                "failed to read canonical lock journal {:?} line {}: {error}",
                journal_path,
                index + 1
            )
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let record =
            serde_json::from_str::<LegacyCanonicalCommitRecord>(&line).map_err(|error| {
                format!(
                    "failed to parse canonical lock journal {:?} line {}: {error}",
                    journal_path,
                    index + 1
                )
            })?;
        if let Some(existing) = locks.get(&record.height) {
            if existing.block_hash != record.block_hash {
                return Err(format!(
                    "canonical lock journal conflicts at height {}: compact/journal hashes {} and {}",
                    record.height, existing.block_hash, record.block_hash
                ));
            }
            continue;
        }
        locks.insert(record.height, record);
    }
    Ok(locks)
}

fn append_legacy_canonical_lock_journal(
    record: &LegacyCanonicalCommitRecord,
) -> Result<(), String> {
    let path = legacy_canonical_lock_journal_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!("failed to create canonical lock journal directory: {error}")
        })?;
    }
    let bytes = serde_json::to_vec(record)
        .map_err(|error| format!("failed to encode canonical lock journal entry: {error}"))?;
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&path)
        .map_err(|error| format!("failed to open canonical lock journal {:?}: {error}", path))?;
    file.write_all(&bytes)
        .map_err(|error| format!("failed to write canonical lock journal entry: {error}"))?;
    file.write_all(b"\n")
        .map_err(|error| format!("failed to write canonical lock journal newline: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("failed to sync canonical lock journal {:?}: {error}", path))
}

fn persist_legacy_canonical_locks_at(
    path: &Path,
    locks: &BTreeMap<u64, LegacyCanonicalCommitRecord>,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create canonical lock directory: {error}"))?;
    }

    let tmp_path = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(locks)
        .map_err(|error| format!("failed to encode canonical locks: {error}"))?;
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&tmp_path)
        .map_err(|error| format!("failed to open canonical lock temp file: {error}"))?;
    file.write_all(&bytes)
        .map_err(|error| format!("failed to write canonical lock temp file: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("failed to sync canonical lock temp file: {error}"))?;
    drop(file);
    fs::rename(&tmp_path, &path)
        .map_err(|error| format!("failed to replace canonical lock store: {error}"))
}

fn legacy_canonical_lock_journal_path() -> PathBuf {
    legacy_canonical_lock_journal_path_for(&legacy_canonical_lock_path())
}

fn legacy_canonical_lock_journal_path_for(compact_path: &Path) -> PathBuf {
    compact_path.with_extension("jsonl")
}

fn legacy_qc_hash(qc: &QuorumCertificate) -> Result<String, String> {
    let bytes = serde_json::to_vec(qc).map_err(|error| format!("failed to encode QC: {error}"))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn legacy_canonical_lock_path() -> PathBuf {
    if let Ok(path) = std::env::var("SYNERGY_CANONICAL_LOCK_FILE") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    #[cfg(test)]
    {
        if std::env::var("SYNERGY_PROJECT_ROOT")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .is_some()
            || std::env::var("SYNERGY_CONFIG_PATH")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .is_some()
        {
            return crate::utils::resolve_data_path("data/canonical_locks.json");
        }
        if let Some(test_name) = std::thread::current().name() {
            let sanitized = test_name
                .chars()
                .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
                .collect::<String>();
            return std::env::temp_dir().join(format!(
                "synergy-test-canonical-locks-{}-{sanitized}.json",
                std::process::id()
            ));
        }
        return std::env::temp_dir().join(format!(
            "synergy-test-canonical-locks-{}.json",
            std::process::id()
        ));
    }

    #[cfg(not(test))]
    {
        crate::utils::resolve_data_path("data/canonical_locks.json")
    }
}

fn current_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
pub(crate) fn clear_legacy_canonical_locks_for_tests() {
    let path = legacy_canonical_lock_path();
    if let Ok(mut cache) = LEGACY_CANONICAL_LOCK_CACHE.lock() {
        cache.remove(&path);
    }
    let _ = fs::remove_file(legacy_canonical_lock_journal_path_for(&path));
    let _ = fs::remove_file(path.with_extension("json.tmp"));
    let _ = fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(height: u64, hash_suffix: &str) -> Block {
        let mut block = Block::new_with_timestamp(
            height,
            Vec::new(),
            "parent".to_string(),
            "validator".to_string(),
            height,
            1_700_000_000 + height,
        );
        block.hash = format!("block-{height}-{hash_suffix}");
        block.transactions_root = "root".to_string();
        block
    }

    fn qc(block_hash: &str) -> QuorumCertificate {
        QuorumCertificate {
            block_hash: block_hash.to_string(),
            epoch_number: 0,
            round_number: 1,
            aggregate_signature: vec![1],
            participant_bitmap: vec![1],
            cumulative_weight: 4.0,
            validation_quorum_met: true,
            cooperation_quorum_met: true,
            timestamp: 1,
            votes: Vec::new(),
        }
    }

    #[test]
    fn canonical_lock_rejects_conflicting_same_height_block() {
        clear_legacy_canonical_locks_for_tests();
        let block_a = block(7, "a");
        let block_b = block(7, "b");
        write_legacy_canonical_lock(&block_a, &qc(&block_a.hash)).unwrap();

        verify_legacy_canonical_lock(&block_a).unwrap();
        assert!(verify_legacy_canonical_lock(&block_b)
            .unwrap_err()
            .contains("already binds block"));
    }

    #[test]
    fn canonical_lock_journal_reloads_without_rewriting_compact_history() {
        clear_legacy_canonical_locks_for_tests();
        let block_a = block(7, "a");
        write_legacy_canonical_lock(&block_a, &qc(&block_a.hash)).unwrap();

        let compact_path = legacy_canonical_lock_path();
        assert!(!compact_path.exists());
        assert!(legacy_canonical_lock_journal_path_for(&compact_path).exists());

        LEGACY_CANONICAL_LOCK_CACHE
            .lock()
            .unwrap()
            .remove(&compact_path);
        assert_eq!(
            latest_legacy_canonical_commit_record()
                .unwrap()
                .unwrap()
                .block_hash,
            block_a.hash
        );
    }

    #[test]
    fn canonical_lock_materialization_merges_compact_map_and_journal() {
        clear_legacy_canonical_locks_for_tests();
        let block_a = block(7, "a");
        write_legacy_canonical_lock(&block_a, &qc(&block_a.hash)).unwrap();
        let materialized = legacy_canonical_lock_path().with_extension("snapshot.json");

        materialize_legacy_canonical_locks_at(&materialized).unwrap();
        let locks: BTreeMap<u64, LegacyCanonicalCommitRecord> =
            serde_json::from_slice(&fs::read(&materialized).unwrap()).unwrap();
        assert_eq!(locks.get(&7).unwrap().block_hash, block_a.hash);
        let _ = fs::remove_file(materialized);
    }

    #[test]
    fn canonical_lock_journal_preserves_existing_compact_history_bytes() {
        clear_legacy_canonical_locks_for_tests();
        let compact_path = legacy_canonical_lock_path();
        let block_a = block(7, "a");
        let block_b = block(8, "b");
        let record_a = LegacyCanonicalCommitRecord {
            height: block_a.block_index,
            block_hash: block_a.hash.clone(),
            parent_hash: block_a.previous_hash.clone(),
            validator_id: block_a.validator_id.clone(),
            transactions_root: block_a.transactions_root.clone(),
            qc_block_hash: block_a.hash.clone(),
            qc_hash: legacy_qc_hash(&qc(&block_a.hash)).unwrap(),
            written_at_unix_secs: current_unix_secs(),
        };
        persist_legacy_canonical_locks_at(
            &compact_path,
            &BTreeMap::from([(block_a.block_index, record_a)]),
        )
        .unwrap();
        let compact_before = fs::read(&compact_path).unwrap();

        write_legacy_canonical_lock(&block_b, &qc(&block_b.hash)).unwrap();

        assert_eq!(fs::read(&compact_path).unwrap(), compact_before);
        assert_eq!(
            latest_legacy_canonical_commit_record()
                .unwrap()
                .unwrap()
                .block_hash,
            block_b.hash
        );
    }
}
