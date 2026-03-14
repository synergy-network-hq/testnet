//! # Classic McEliece Shim
//!
//! **WARNING:** This is a placeholder/stub. Do NOT use for real cryptography
//! or production deployment until this module is replaced with the final pure
//! Rust implementation of Classic McEliece.

// Based on mceliece348864
pub const MCELIECE_PUBLIC_KEY_BYTES: usize = 261120;
pub const MCELIECE_SECRET_KEY_BYTES: usize = 6492;
pub const MCELIECE_CIPHERTEXT_BYTES: usize = 96;
pub const MCELIECE_SHARED_SECRET_BYTES: usize = 32;

/// A placeholder for Classic McEliece key generation.
/// Returns a tuple of (public_key, secret_key) with fixed-size zeroed vectors.
pub fn keygen() -> (Vec<u8>, Vec<u8>) {
    // TODO: Replace with real rusty-mceliece keygen when ready
    (
        vec![0u8; MCELIECE_PUBLIC_KEY_BYTES],
        vec![0u8; MCELIECE_SECRET_KEY_BYTES],
    )
}

/// A placeholder for Classic McEliece encapsulation.
/// Returns a tuple of (ciphertext, shared_secret) with fixed-size zeroed vectors.
pub fn encaps(_pk: &[u8]) -> (Vec<u8>, Vec<u8>) {
    // TODO: Replace with real rusty-mceliece encaps when ready
    (
        vec![0u8; MCELIECE_CIPHERTEXT_BYTES],
        vec![0u8; MCELIECE_SHARED_SECRET_BYTES],
    )
}

/// A placeholder for Classic McEliece decapsulation.
/// Returns a fixed-size zeroed vector for the shared_secret.
pub fn decaps(_ct: &[u8], _sk: &[u8]) -> Vec<u8> {
    // TODO: Replace with real rusty-mceliece decaps when ready
    vec![0u8; MCELIECE_SHARED_SECRET_BYTES]
}
