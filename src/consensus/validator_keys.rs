use crate::block::Block;
use crate::crypto::pqc::{PQCAlgorithm, PQCManager, PQCPrivateKey, PQCPublicKey, PQCSignature};
use crate::validator::ValidatorManager;
use base64::{engine::general_purpose, Engine as _};
use lazy_static::lazy_static;
use serde_json::Value;
use sha3::{Digest, Sha3_256};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

lazy_static! {
    static ref LOCAL_VALIDATOR_SIGNING_KEYS: Mutex<HashMap<String, (PQCPublicKey, PQCPrivateKey)>> =
        Mutex::new(HashMap::new());
}

pub fn consensus_algorithm_label(algorithm: &PQCAlgorithm) -> &'static str {
    match algorithm {
        PQCAlgorithm::MLDSA => "ml-dsa",
        PQCAlgorithm::FNDSA => "fn-dsa",
        PQCAlgorithm::SLHDSA => "slh-dsa",
        PQCAlgorithm::MLKEM1024 => "ml-kem-1024",
        PQCAlgorithm::HQCKEM => "hqc-kem",
    }
}

pub fn expected_validator_public_key(
    validator_address: &str,
    validator_manager: &ValidatorManager,
) -> Result<PQCPublicKey, String> {
    let validator = validator_manager
        .get_validator(validator_address)
        .ok_or_else(|| format!("validator {validator_address} is not registered"))?;
    parse_validator_public_key(validator_address, &validator.public_key)
}

pub fn parse_validator_public_key(
    validator_address: &str,
    encoded: &str,
) -> Result<PQCPublicKey, String> {
    let encoded = encoded.trim();
    if encoded.is_empty() {
        return Err(format!(
            "validator {validator_address} is missing consensus public key"
        ));
    }

    let (algorithm, material) = split_algorithm_prefix(encoded);
    let key_data = decode_key_material(material).map_err(|error| {
        format!("validator {validator_address} consensus public key is invalid: {error}")
    })?;
    if key_data.is_empty() {
        return Err(format!(
            "validator {validator_address} consensus public key is empty"
        ));
    }

    Ok(PQCPublicKey {
        algorithm,
        key_data,
        key_id: format!("validator-consensus:{validator_address}"),
        created_at: 0,
    })
}

pub fn verify_signer_key_matches_validator(
    validator_address: &str,
    signer_public_key: &[u8],
    validator_manager: &ValidatorManager,
) -> Result<PQCPublicKey, String> {
    let expected = expected_validator_public_key(validator_address, validator_manager)?;
    if signer_public_key != expected.key_data.as_slice() {
        return Err(format!(
            "signer public key does not match canonical consensus key for validator {validator_address}"
        ));
    }
    Ok(expected)
}

pub fn verify_block_proposer_key_matches_validator(
    block: &Block,
    validator_manager: &ValidatorManager,
) -> Result<(), String> {
    if block.block_index == 0 {
        return Ok(());
    }

    let expected = verify_signer_key_matches_validator(
        &block.validator_id,
        &block.proposer_public_key,
        validator_manager,
    )?;
    let block_algorithm = block_signature_algorithm(&block.block_signature_algorithm)?;
    if block_algorithm != expected.algorithm {
        return Err(format!(
            "block proposer signature algorithm does not match canonical consensus key for validator {}",
            block.validator_id
        ));
    }
    Ok(())
}

pub fn sign_with_local_validator_key(
    validator_address: &str,
    message: &[u8],
    validator_manager: &ValidatorManager,
) -> Result<(PQCPublicKey, PQCSignature), String> {
    let expected = expected_validator_public_key(validator_address, validator_manager)?;
    let private_key = load_local_validator_private_key(validator_address, &expected)?;
    let mut pqc_manager = PQCManager::new();
    let signature = pqc_manager.sign(&private_key, message)?;
    Ok((expected, signature))
}

pub fn load_local_validator_keypair(
    validator_address: &str,
    validator_manager: &ValidatorManager,
) -> Result<(PQCPublicKey, PQCPrivateKey), String> {
    let expected = expected_validator_public_key(validator_address, validator_manager)?;
    let private_key = load_local_validator_private_key(validator_address, &expected)?;
    Ok((expected, private_key))
}

fn load_local_validator_private_key(
    validator_address: &str,
    expected_public_key: &PQCPublicKey,
) -> Result<PQCPrivateKey, String> {
    if let Ok(cache) = LOCAL_VALIDATOR_SIGNING_KEYS.lock() {
        if let Some((cached_public, cached_private)) = cache.get(validator_address) {
            if cached_public.key_data == expected_public_key.key_data
                && cached_public.algorithm == expected_public_key.algorithm
            {
                ensure_private_key_matches_public_key(
                    validator_address,
                    expected_public_key,
                    cached_private,
                )?;
                return Ok(cached_private.clone());
            }
        }
    }

    let private_key = load_private_key_from_config(validator_address, expected_public_key)?;
    ensure_private_key_matches_public_key(validator_address, expected_public_key, &private_key)?;

    if let Ok(mut cache) = LOCAL_VALIDATOR_SIGNING_KEYS.lock() {
        cache.insert(
            validator_address.to_string(),
            (expected_public_key.clone(), private_key.clone()),
        );
    }

    Ok(private_key)
}

fn load_private_key_from_config(
    validator_address: &str,
    expected_public_key: &PQCPublicKey,
) -> Result<PQCPrivateKey, String> {
    for key in [
        "SYNERGY_VALIDATOR_CONSENSUS_PRIVATE_KEY_B64",
        "SYNERGY_CONSENSUS_PRIVATE_KEY_B64",
    ] {
        if let Ok(value) = env::var(key) {
            let value = value.trim();
            if !value.is_empty() {
                return private_key_from_encoded(expected_public_key, value, format!("env:{key}"));
            }
        }
    }

    for path in candidate_private_key_paths() {
        if let Ok(encoded) = fs::read_to_string(&path) {
            let encoded = encoded.trim();
            if !encoded.is_empty() {
                return private_key_from_encoded(
                    expected_public_key,
                    encoded,
                    path.display().to_string(),
                );
            }
        }
    }

    for identity_path in candidate_identity_paths() {
        if let Ok(identity) = fs::read_to_string(&identity_path) {
            if let Ok(json) = serde_json::from_str::<Value>(&identity) {
                for encoded in consensus_private_key_candidates(&json, &identity_path) {
                    if !encoded.trim().is_empty() {
                        return private_key_from_encoded(
                            expected_public_key,
                            encoded.trim(),
                            identity_path.display().to_string(),
                        );
                    }
                }
            }
        }
    }

    Err(format!(
        "Aegis PQC consensus private key unavailable for validator {validator_address}; set SYNERGY_VALIDATOR_CONSENSUS_PRIVATE_KEY_FILE or SYNERGY_VALIDATOR_CONSENSUS_PRIVATE_KEY_B64"
    ))
}

fn private_key_from_encoded(
    expected_public_key: &PQCPublicKey,
    encoded: &str,
    source: String,
) -> Result<PQCPrivateKey, String> {
    let key_data = decode_key_material(encoded)
        .map_err(|error| format!("invalid Aegis PQC consensus private key in {source}: {error}"))?;
    if key_data.is_empty() {
        return Err(format!("empty Aegis PQC consensus private key in {source}"));
    }

    Ok(PQCPrivateKey {
        algorithm: expected_public_key.algorithm.clone(),
        key_data,
        public_key_id: expected_public_key.key_id.clone(),
        created_at: 0,
    })
}

fn ensure_private_key_matches_public_key(
    validator_address: &str,
    expected_public_key: &PQCPublicKey,
    private_key: &PQCPrivateKey,
) -> Result<(), String> {
    let challenge = local_key_binding_challenge(validator_address, expected_public_key);
    let mut pqc_manager = PQCManager::new();
    let signature = pqc_manager.sign(private_key, &challenge).map_err(|error| {
        format!("Aegis PQC consensus key self-test signing failed for {validator_address}: {error}")
    })?;
    pqc_manager
        .verify(expected_public_key, &signature, &challenge)
        .map_err(|error| {
            format!(
                "Aegis PQC consensus key self-test verification failed for {validator_address}: {error}"
            )
        })
        .and_then(|valid| {
            if valid {
                Ok(())
            } else {
                Err(format!(
                    "Aegis PQC consensus private key does not match canonical public key for validator {validator_address}"
                ))
            }
        })
}

fn local_key_binding_challenge(
    validator_address: &str,
    expected_public_key: &PQCPublicKey,
) -> Vec<u8> {
    let mut hasher = Sha3_256::new();
    hasher.update(b"SYNERGY_CONSENSUS_KEY_BINDING_V1");
    hasher.update(validator_address.as_bytes());
    hasher.update(consensus_algorithm_label(&expected_public_key.algorithm).as_bytes());
    hasher.update(&expected_public_key.key_data);
    hasher.finalize().to_vec()
}

fn candidate_private_key_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for key in [
        "SYNERGY_VALIDATOR_CONSENSUS_PRIVATE_KEY_FILE",
        "SYNERGY_CONSENSUS_PRIVATE_KEY_FILE",
        "SYNERGY_VALIDATOR_PRIVATE_KEY_FILE",
        "PRIVATE_KEY_FILE",
    ] {
        if let Ok(path) = env::var(key) {
            let path = path.trim();
            if !path.is_empty() {
                paths.push(PathBuf::from(path));
            }
        }
    }
    paths.extend([
        PathBuf::from("config/validator/consensus.private.key"),
        PathBuf::from("config/validator/consensus_private.key"),
        PathBuf::from("config/validator/private_key.txt"),
    ]);
    paths
}

fn candidate_identity_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for key in [
        "SYNERGY_VALIDATOR_IDENTITY_FILE",
        "SYNERGY_NODE_IDENTITY_FILE",
        "SYNERGY_IDENTITY_FILE",
    ] {
        if let Ok(path) = env::var(key) {
            let path = path.trim();
            if !path.is_empty() {
                paths.push(PathBuf::from(path));
            }
        }
    }
    paths.extend([
        PathBuf::from("config/validator/identity.json"),
        PathBuf::from("config/identity.json"),
        PathBuf::from("keys/identity.json"),
    ]);
    paths
}

fn consensus_private_key_candidates(json: &Value, identity_path: &Path) -> Vec<String> {
    let mut values = Vec::new();
    for path in [
        &["consensus_key", "private_key"][..],
        &["consensus_private_key"][..],
        &["keys", "consensus_private_key"][..],
        &["keys", "private_key"][..],
        &["private_key"][..],
    ] {
        if let Some(value) = json_path_string(json, path) {
            values.push(value);
        }
    }

    if let Some(parent) = identity_path.parent() {
        for filename in [
            "consensus.private.key",
            "consensus_private.key",
            "consensus.key",
            "private.key",
        ] {
            if let Ok(value) = fs::read_to_string(parent.join(filename)) {
                values.push(value.trim().to_string());
            }
        }
    }

    values
}

fn json_path_string(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(str::to_string)
}

fn split_algorithm_prefix(encoded: &str) -> (PQCAlgorithm, &str) {
    let Some((prefix, material)) = encoded.split_once(':') else {
        return (PQCAlgorithm::MLDSA, encoded);
    };
    match algorithm_from_label(prefix) {
        Ok(algorithm) => (algorithm, material.trim()),
        Err(_) => (PQCAlgorithm::MLDSA, encoded),
    }
}

fn block_signature_algorithm(label: &str) -> Result<PQCAlgorithm, String> {
    algorithm_from_label(label)
        .map_err(|error| format!("unsupported block signature algorithm: {error}"))
}

fn algorithm_from_label(label: &str) -> Result<PQCAlgorithm, String> {
    match label.trim().to_ascii_lowercase().as_str() {
        "mldsa" | "ml-dsa" | "ml-dsa-65" | "ml-dsa-87" => Ok(PQCAlgorithm::MLDSA),
        "fndsa" | "fn-dsa" | "fn-dsa-1024" => Ok(PQCAlgorithm::FNDSA),
        "slhdsa" | "slh-dsa" => Ok(PQCAlgorithm::SLHDSA),
        other => Err(other.to_string()),
    }
}

fn decode_key_material(encoded: &str) -> Result<Vec<u8>, String> {
    let normalized = encoded
        .trim()
        .trim_matches('"')
        .trim_start_matches("0x")
        .trim();
    if normalized.is_empty() {
        return Err("empty key material".to_string());
    }

    if normalized.len() % 2 == 0 && normalized.chars().all(|ch| ch.is_ascii_hexdigit()) {
        if let Ok(bytes) = hex::decode(normalized) {
            return Ok(bytes);
        }
    }

    general_purpose::STANDARD
        .decode(normalized.as_bytes())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
pub(crate) fn register_test_validator_signing_key(
    validator_address: &str,
    public_key: PQCPublicKey,
    private_key: PQCPrivateKey,
) {
    LOCAL_VALIDATOR_SIGNING_KEYS
        .lock()
        .expect("test validator key cache lock")
        .insert(validator_address.to_string(), (public_key, private_key));
}
