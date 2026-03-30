use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

use crate::utils::resolve_data_path;

const ZERO_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone)]
pub struct GenesisBalance {
    pub address: String,
    pub balance_nwei: u64,
}

#[derive(Debug, Clone)]
pub struct GenesisValidator {
    pub validator_id: String,
    pub operator_address: String,
    pub consensus_public_key: String,
    pub moniker: String,
    pub stake_nwei: u64,
}

#[derive(Debug, Clone)]
pub struct GenesisTokenConfig {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply_cap_nwei: u128,
    pub initial_circulating_nwei: u128,
}

#[derive(Debug, Clone)]
pub struct GenesisDocument {
    value: Value,
    path: PathBuf,
    genesis_hash: String,
    timestamp: u64,
    balances: Vec<GenesisBalance>,
    validators: Vec<GenesisValidator>,
    token: GenesisTokenConfig,
}

lazy_static! {
    static ref CANONICAL_GENESIS: Result<GenesisDocument, String> =
        load_canonical_genesis_from_disk();
}

pub fn canonical_genesis() -> Result<&'static GenesisDocument, String> {
    match &*CANONICAL_GENESIS {
        Ok(document) => Ok(document),
        Err(error) => Err(error.clone()),
    }
}

impl GenesisDocument {
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn hash(&self) -> &str {
        &self.genesis_hash
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn balances(&self) -> &[GenesisBalance] {
        &self.balances
    }

    pub fn validators(&self) -> &[GenesisValidator] {
        &self.validators
    }

    pub fn token(&self) -> &GenesisTokenConfig {
        &self.token
    }
}

fn load_canonical_genesis_from_disk() -> Result<GenesisDocument, String> {
    let path = genesis_path();
    let bytes = fs::read(&path)
        .map_err(|error| format!("read canonical genesis {}: {error}", path.display()))?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse canonical genesis {}: {error}", path.display()))?;

    validate_no_placeholders(&value)?;

    let timestamp = parse_timestamp(required(&value, &["header", "timestamp"])?)
        .map_err(|error| format!("header.timestamp: {error}"))?;
    let balances = parse_balances(&value)?;
    let validators = parse_validators(&value)?;
    let token = parse_token_config(&value)?;

    validate_integrity_hashes(&value)?;

    let genesis_hash = required_string(&value, &["integrity", "genesis_hash"])?;
    if genesis_hash.is_empty() {
        return Err("integrity.genesis_hash must not be empty".to_string());
    }

    Ok(GenesisDocument {
        value,
        path,
        genesis_hash,
        timestamp,
        balances,
        validators,
        token,
    })
}

fn genesis_path() -> PathBuf {
    let configured = std::env::var("SYNERGY_GENESIS_FILE")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "config/genesis.json".to_string());
    resolve_data_path(&configured)
}

fn parse_balances(value: &Value) -> Result<Vec<GenesisBalance>, String> {
    let balances = required_array(value, &["balances"])?;
    balances
        .iter()
        .map(|entry| {
            Ok(GenesisBalance {
                address: required_string(entry, &["address"])?,
                balance_nwei: parse_u64(&required_string(entry, &["balance_nwei"])?)?,
            })
        })
        .collect()
}

fn parse_validators(value: &Value) -> Result<Vec<GenesisValidator>, String> {
    let validators = required_array(value, &["validators"])?;
    if validators.is_empty() {
        return Err("validators must not be empty".to_string());
    }

    validators
        .iter()
        .map(|entry| {
            Ok(GenesisValidator {
                validator_id: required_string(entry, &["validator_id"])?,
                operator_address: required_string(entry, &["operator_address"])?,
                consensus_public_key: required_string(entry, &["consensus_public_key"])?,
                moniker: required_string(entry, &["moniker"])?,
                stake_nwei: parse_u64(&required_string(entry, &["stake_nwei"])?)?,
            })
        })
        .collect()
}

fn parse_token_config(value: &Value) -> Result<GenesisTokenConfig, String> {
    Ok(GenesisTokenConfig {
        name: required_string(value, &["token", "name"])?,
        symbol: required_string(value, &["token", "symbol"])?,
        decimals: required_u64(value, &["token", "decimals"])? as u8,
        total_supply_cap_nwei: parse_u128(&required_string(
            value,
            &["token", "total_supply_cap_nwei"],
        )?)?,
        initial_circulating_nwei: parse_u128(&required_string(
            value,
            &["token", "initial_circulating_nwei"],
        )?)?,
    })
}

fn validate_integrity_hashes(value: &Value) -> Result<(), String> {
    let empty_hash = hash_bytes(&[]);
    let allocation_hash = hash_json(required(value, &["allocations"])?);
    let validator_hash = hash_json(required(value, &["validators"])?);
    let contract_hash = hash_json(required(value, &["contracts"])?);
    let state_root = hash_json(&json!({
        "accounts": required(value, &["accounts"])?,
        "balances": required(value, &["balances"])?,
        "allocations": required(value, &["allocations"])?,
        "validators": required(value, &["validators"])?,
        "contracts": required(value, &["contracts"])?,
        "modules": required(value, &["modules"])?,
    }));
    let data_root = hash_json(&json!({
        "contracts": required(value, &["contracts"])?,
        "modules": required(value, &["modules"])?,
        "precompiles": required(value, &["precompiles"])?,
    }));

    compare_hash(
        value,
        &["header", "parent_hash"],
        ZERO_HASH,
        "header.parent_hash",
    )?;
    compare_hash(
        value,
        &["header", "transactions_root"],
        &empty_hash,
        "header.transactions_root",
    )?;
    compare_hash(
        value,
        &["header", "receipts_root"],
        &empty_hash,
        "header.receipts_root",
    )?;
    compare_hash(
        value,
        &["header", "state_root"],
        &state_root,
        "header.state_root",
    )?;
    compare_hash(
        value,
        &["header", "data_root"],
        &data_root,
        "header.data_root",
    )?;
    compare_hash(
        value,
        &["integrity", "allocation_hash"],
        &allocation_hash,
        "integrity.allocation_hash",
    )?;
    compare_hash(
        value,
        &["integrity", "validator_hash"],
        &validator_hash,
        "integrity.validator_hash",
    )?;
    compare_hash(
        value,
        &["integrity", "contract_hash"],
        &contract_hash,
        "integrity.contract_hash",
    )?;
    compare_hash(
        value,
        &["integrity", "state_root"],
        &state_root,
        "integrity.state_root",
    )?;

    let mut genesis_for_hash = value.clone();
    if let Some(integrity) = genesis_for_hash
        .get_mut("integrity")
        .and_then(Value::as_object_mut)
    {
        integrity.insert("genesis_hash".to_string(), Value::String(String::new()));
    }
    let expected_genesis_hash = hash_json(&genesis_for_hash);
    compare_hash(
        value,
        &["integrity", "genesis_hash"],
        &expected_genesis_hash,
        "integrity.genesis_hash",
    )?;

    Ok(())
}

fn compare_hash(value: &Value, path: &[&str], expected: &str, label: &str) -> Result<(), String> {
    let actual = required_string(value, path)?;
    if actual != expected {
        return Err(format!(
            "{label} mismatch: expected {expected}, found {actual}"
        ));
    }
    Ok(())
}

fn validate_no_placeholders(value: &Value) -> Result<(), String> {
    if let Some(path) = find_placeholder_path(value, "$") {
        return Err(format!("placeholder value found at {path}"));
    }
    Ok(())
}

fn find_placeholder_path(value: &Value, path: &str) -> Option<String> {
    match value {
        Value::String(entry) => {
            if entry.contains('<') && entry.contains('>') {
                Some(path.to_string())
            } else {
                None
            }
        }
        Value::Array(entries) => entries
            .iter()
            .enumerate()
            .find_map(|(index, entry)| find_placeholder_path(entry, &format!("{path}[{index}]"))),
        Value::Object(entries) => entries
            .iter()
            .find_map(|(key, entry)| find_placeholder_path(entry, &format!("{path}.{key}"))),
        _ => None,
    }
}

fn parse_timestamp(value: &Value) -> Result<u64, String> {
    match value {
        Value::Number(number) => number
            .as_u64()
            .ok_or_else(|| "timestamp must be an unsigned integer".to_string()),
        Value::String(raw) => DateTime::parse_from_rfc3339(raw)
            .map(|timestamp| timestamp.with_timezone(&Utc).timestamp().max(0) as u64)
            .map_err(|error| format!("invalid RFC3339 timestamp: {error}")),
        _ => Err("timestamp must be an integer or RFC3339 string".to_string()),
    }
}

fn parse_u64(raw: &str) -> Result<u64, String> {
    raw.parse::<u64>()
        .map_err(|error| format!("invalid u64 value '{raw}': {error}"))
}

fn parse_u128(raw: &str) -> Result<u128, String> {
    raw.parse::<u128>()
        .map_err(|error| format!("invalid u128 value '{raw}': {error}"))
}

fn required<'a>(value: &'a Value, path: &[&str]) -> Result<&'a Value, String> {
    let mut current = value;
    for segment in path {
        current = current
            .get(*segment)
            .ok_or_else(|| format!("missing path {}", path.join(".")))?;
    }
    Ok(current)
}

fn required_array<'a>(value: &'a Value, path: &[&str]) -> Result<&'a Vec<Value>, String> {
    required(value, path)?
        .as_array()
        .ok_or_else(|| format!("path {} is not an array", path.join(".")))
}

fn required_string(value: &Value, path: &[&str]) -> Result<String, String> {
    required(value, path)?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("path {} is not a string", path.join(".")))
}

fn required_u64(value: &Value, path: &[&str]) -> Result<u64, String> {
    required(value, path)?
        .as_u64()
        .ok_or_else(|| format!("path {} is not a u64", path.join(".")))
}

fn hash_json(value: &Value) -> String {
    hash_bytes(canonical_json(value).as_bytes())
}

fn hash_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(entry) => entry.to_string(),
        Value::Number(entry) => entry.to_string(),
        Value::String(entry) => serde_json::to_string(entry).unwrap_or_else(|_| "\"\"".to_string()),
        Value::Array(entries) => {
            let rendered = entries
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{rendered}]")
        }
        Value::Object(entries) => {
            let mut keys = entries.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            let rendered = keys
                .iter()
                .map(|key| {
                    let key_json =
                        serde_json::to_string(key).unwrap_or_else(|_| "\"\"".to_string());
                    let value_json = canonical_json(&entries[key]);
                    format!("{key_json}:{value_json}")
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{rendered}}}")
        }
    }
}
