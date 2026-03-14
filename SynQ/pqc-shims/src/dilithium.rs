//! # Dilithium Shim
//!
//! **WARNING:** This is a placeholder/stub. Do NOT use for real cryptography
//! or production deployment until this module is replaced with the final pure
//! Rust implementation of Dilithium.

// Based on Dilithium3
pub const DILITHIUM_PUBLIC_KEY_BYTES: usize = 1952;
pub const DILITHIUM_SECRET_KEY_BYTES: usize = 4016;
pub const DILITHIUM_SIGNATURE_BYTES: usize = 3293;

/// A placeholder for Dilithium key generation.
/// Returns a tuple of (public_key, secret_key) with fixed-size zeroed vectors.
pub fn keygen() -> (Vec<u8>, Vec<u8>) {
    // TODO: Replace with real rusty-dilithium keygen when ready
    (
        vec![0u8; DILITHIUM_PUBLIC_KEY_BYTES],
        vec![0u8; DILITHIUM_SECRET_KEY_BYTES],
    )
}

/// A placeholder for Dilithium signing.
/// Returns a fixed-size zeroed vector for the signature.
pub fn sign(_msg: &[u8], _sk: &[u8]) -> Vec<u8> {
    // TODO: Replace with real rusty-dilithium sign when ready
    vec![0u8; DILITHIUM_SIGNATURE_BYTES]
}

/// A placeholder for Dilithium signature verification.
/// Always returns `true`.
pub fn verify(_msg: &[u8], _sig: &[u8], _pk: &[u8]) -> bool {
    // TODO: Replace with real rusty-dilithium verify when ready
    true
}
