//! # Falcon Shim
//!
//! **WARNING:** This is a placeholder/stub. Do NOT use for real cryptography
//! or production deployment until this module is replaced with the final pure
//! Rust implementation of Falcon.

// Based on Falcon-512
pub const FALCON_PUBLIC_KEY_BYTES: usize = 897;
pub const FALCON_SECRET_KEY_BYTES: usize = 1281;
pub const FALCON_SIGNATURE_BYTES: usize = 666; // This can vary

/// A placeholder for Falcon key generation.
/// Returns a tuple of (public_key, secret_key) with fixed-size zeroed vectors.
pub fn keygen() -> (Vec<u8>, Vec<u8>) {
    // TODO: Replace with real rusty-falcon keygen when ready
    (
        vec![0u8; FALCON_PUBLIC_KEY_BYTES],
        vec![0u8; FALCON_SECRET_KEY_BYTES],
    )
}

/// A placeholder for Falcon signing.
/// Returns a fixed-size zeroed vector for the signature.
pub fn sign(_msg: &[u8], _sk: &[u8]) -> Vec<u8> {
    // TODO: Replace with real rusty-falcon sign when ready
    vec![0u8; FALCON_SIGNATURE_BYTES]
}

/// A placeholder for Falcon signature verification.
/// Always returns `true`.
pub fn verify(_msg: &[u8], _sig: &[u8], _pk: &[u8]) -> bool {
    // TODO: Replace with real rusty-falcon verify when ready
    true
}
