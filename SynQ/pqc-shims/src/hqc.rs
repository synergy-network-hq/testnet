//! # HQC (HQC) Shim
//!
//! **WARNING:** This is a placeholder/stub. Do NOT use for real cryptography
//! or production deployment until this module is replaced with the final pure
//! Rust implementation of HQC.

// Based on HQC-128
pub const HQC_PUBLIC_KEY_BYTES: usize = 2249;
pub const HQC_SECRET_KEY_BYTES: usize = 2289;
pub const HQC_CIPHERTEXT_BYTES: usize = 4481;
pub const HQC_SHARED_SECRET_BYTES: usize = 32;

/// A placeholder for HQC key generation.
/// Returns a tuple of (public_key, secret_key) with fixed-size zeroed vectors.
pub fn keygen() -> (Vec<u8>, Vec<u8>) {
    // TODO: Replace with real rusty-hqc keygen when ready
    (
        vec![0u8; HQC_PUBLIC_KEY_BYTES],
        vec![0u8; HQC_SECRET_KEY_BYTES],
    )
}

/// A placeholder for HQC encapsulation.
/// Returns a tuple of (ciphertext, shared_secret) with fixed-size zeroed vectors.
pub fn encaps(_pk: &[u8]) -> (Vec<u8>, Vec<u8>) {
    // TODO: Replace with real rusty-hqc encaps when ready
    (
        vec![0u8; HQC_CIPHERTEXT_BYTES],
        vec![0u8; HQC_SHARED_SECRET_BYTES],
    )
}

/// A placeholder for HQC decapsulation.
/// Returns a fixed-size zeroed vector for the shared_secret.
pub fn decaps(_ct: &[u8], _sk: &[u8]) -> Vec<u8> {
    // TODO: Replace with real rusty-hqc decaps when ready
    vec![0u8; HQC_SHARED_SECRET_BYTES]
}
