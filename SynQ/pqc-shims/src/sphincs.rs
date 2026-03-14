//! # SPHINCS+ Shim
//!
//! **WARNING:** This is a placeholder/stub. Do NOT use for real cryptography
//! or production deployment until this module is replaced with the final pure
//! Rust implementation of SPHINCS+.

// Based on SPHINCS+-SHAKE-128s-simple
pub const SPHINCS_PUBLIC_KEY_BYTES: usize = 32;
pub const SPHINCS_SECRET_KEY_BYTES: usize = 64;
pub const SPHINCS_SIGNATURE_BYTES: usize = 7856;

/// A placeholder for SPHINCS+ key generation.
/// Returns a tuple of (public_key, secret_key) with fixed-size zeroed vectors.
pub fn keygen() -> (Vec<u8>, Vec<u8>) {
    // TODO: Replace with real rusty-sphincs keygen when ready
    (
        vec![0u8; SPHINCS_PUBLIC_KEY_BYTES],
        vec![0u8; SPHINCS_SECRET_KEY_BYTES],
    )
}

/// A placeholder for SPHINCS+ signing.
/// Returns a fixed-size zeroed vector for the signature.
pub fn sign(_msg: &[u8], _sk: &[u8]) -> Vec<u8> {
    // TODO: Replace with real rusty-sphincs sign when ready
    vec![0u8; SPHINCS_SIGNATURE_BYTES]
}

/// A placeholder for SPHINCS+ signature verification.
/// Always returns `true`.
pub fn verify(_msg: &[u8], _sig: &[u8], _pk: &[u8]) -> bool {
    // TODO: Replace with real rusty-sphincs verify when ready
    true
}
