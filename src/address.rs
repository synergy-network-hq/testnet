//! Address generation and validation utilities for the Synergy network.
//!
//! This module centralizes logic for constructing Synergy network
//! identifiers from public keys and other inputs.  The Synergy address
//! specification mandates a 41‑character length and a four‑character
//! prefix indicating the address type.  Wallets, tokens, NFTs,
//! validators, contracts and other entities each have reserved
//! prefixes documented in `synergy-address-formatting-standards.txt`.
//!
//! The functions provided here implement a simple scheme for
//! constructing these identifiers.  They derive a SHA3‑256 hash of
//! the input public key and encode the resulting bytes into a
//! lowercase hexadecimal string.  The first 37 characters of that
//! string (representing 18½ bytes) are concatenated to the chosen
//! prefix, yielding a 41‑character identifier.  While the Synergy
//! specification calls for Bech32m encoding, a pure hexadecimal
//! representation is used here as a placeholder.  The hashing and
//! length constraints remain consistent, and switching to a Bech32m
//! encoder in the future only requires replacing the `encode_hash`
//! helper.

use sha3::{Digest, Sha3_256};

/// Encodes the provided byte slice into a lowercase hex string and
/// truncates it to the desired length.  If the resulting string is
/// shorter than `length` characters the remainder is filled with
/// zeros.  This helper hides the encoding logic used by all
/// generator functions.
fn encode_hash(bytes: &[u8], length: usize) -> String {
    let mut hex_str = hex::encode(bytes);
    // Ensure the string has at least `length` characters.  If the
    // underlying hash is shorter (which it isn't for SHA3‑256) we
    // repeatedly append zeros.  Afterwards we truncate to the
    // requested length.
    if hex_str.len() < length {
        hex_str.extend(std::iter::repeat('0').take(length - hex_str.len()));
    }
    hex_str.truncate(length);
    hex_str
}

/// Generates a 41‑character Synergy wallet address.  The prefix
/// distinguishes between standard user wallets and other wallet
/// flavours; for example `synq`, `synu`, `synx` and `synz` are all
/// valid wallet prefixes.  A deterministic prefix selection would
/// require additional network coordination.  To avoid external
/// dependencies this implementation selects a prefix based on the
/// current UNIX timestamp.  Multiple keys generated in the same
/// second will share the same prefix; this behaviour is acceptable
/// for a testnet-beta.
pub fn generate_wallet_address(public_key: &str) -> String {
    // Prefixes defined in the address specification for wallets.
    const PREFIXES: [&str; 4] = ["synq", "synu", "synx", "synz"];
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let prefix_index = (timestamp % PREFIXES.len() as u64) as usize;
    let prefix = PREFIXES[prefix_index];

    // Hash the public key using SHA3‑256.
    let mut hasher = Sha3_256::new();
    hasher.update(public_key.as_bytes());
    let hash = hasher.finalize();
    let body = encode_hash(&hash, 37);
    format!("{}{}", prefix, body)
}

/// Generates a validator or node address.  Validators use the
/// `synv` prefix followed by a numeric group designator in the 5th
/// character position.  Acceptable group numbers range from 1 to 5.
/// If the provided group is outside this range it will be clamped to
/// the nearest valid value.
pub fn generate_validator_address(public_key: &str, group: u8) -> String {
    // Clamp group between 1 and 5 inclusive.
    let group = if group < 1 {
        1
    } else if group > 5 {
        5
    } else {
        group
    };
    let prefix = format!("synv{}", group);
    let mut hasher = Sha3_256::new();
    hasher.update(public_key.as_bytes());
    let hash = hasher.finalize();
    let body = encode_hash(&hash, 36); // prefix is 5 chars, so 36 more to reach 41
    format!("{}{}", prefix, body)
}

/// Generates a class-based node address with the canonical format synv{1-5}{hash}
/// (no hyphen separator).
pub fn generate_class_based_address(public_key: &[u8], class: u8) -> String {
    // Clamp class between 1 and 5 inclusive.
    let class = if class < 1 {
        1
    } else if class > 5 {
        5
    } else {
        class
    };
    let prefix = format!("synv{}", class);

    // Hash the public key bytes using SHA3-256
    let mut hasher = Sha3_256::new();
    hasher.update(public_key);
    let hash = hasher.finalize();

    // Calculate body length: prefix is 5 chars (synv{1-5}), so we need 36 more for 41 total
    let body = encode_hash(&hash, 36);
    format!("{}{}", prefix, body)
}

/// Generates a generic Synergy address with the specified four‑character
/// prefix.  This function may be used for tokens, NFTs, clusters,
/// governance proposals, multisig wallets and other address types.
/// Callers must supply a valid prefix defined in the address
/// formatting specification (e.g. `synb`, `synn`, `synm`, etc.).
pub fn generate_generic_address(prefix: &str, public_key: &str) -> String {
    assert!(
        prefix.len() == 4,
        "Prefixes must be exactly four characters"
    );
    let mut hasher = Sha3_256::new();
    hasher.update(public_key.as_bytes());
    let hash = hasher.finalize();
    let body = encode_hash(&hash, 37);
    format!("{}{}", prefix, body)
}

/// Validates whether a given string conforms to the basic Synergy
/// address rules: it must be 41 characters long, start with `syn` and
/// contain only lowercase hex digits (`0–9` and `a–f`).  This
/// validation is intentionally permissive and does not check that
/// specific prefixes are reserved for appropriate entity types.  That
/// mapping is left to higher‑level application logic.
pub fn is_valid_address(address: &str) -> bool {
    // Canonical burn address is an exception to length rules.
    if address == "synr000000burn000000that000000coin" {
        return true;
    }

    if address.len() != 41 {
        return false;
    }
    if !address.starts_with("syn") {
        return false;
    }
    // Ensure each character after the prefix is a valid lowercase hex digit.
    address
        .chars()
        .skip(3)
        .all(|c| c.is_digit(16) || (c >= 'a' && c <= 'f'))
}

/// Generates a cluster address using the FN-DSA algorithm pattern.
/// Cluster addresses use the `syngrp{1-5}` prefix format.
/// The group parameter determines the cluster group (1-5).
/// The seed is typically the cluster ID combined with validator addresses.
pub fn generate_cluster_address(seed: &str, group: u8) -> String {
    // Clamp group between 1 and 5 inclusive
    let group = if group < 1 {
        1
    } else if group > 5 {
        5
    } else {
        group
    };

    // Cluster prefix format: syngrp{1-5}
    let prefix = format!("syngrp{}", group);

    // Hash the seed using SHA3-256 (FN-DSA compatible)
    let mut hasher = Sha3_256::new();
    hasher.update(seed.as_bytes());
    // Add additional entropy from timestamp for uniqueness
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    hasher.update(&timestamp.to_le_bytes());
    let hash = hasher.finalize();

    // Prefix is 7 chars (syngrp{1-5}), so we need 34 more for 41 total
    let body = encode_hash(&hash, 34);
    format!("{}{}", prefix, body)
}
