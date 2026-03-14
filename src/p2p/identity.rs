use k256::{ecdsa::SigningKey, EncodedPoint};
use sha3::{Digest, Keccak256};
use std::net::IpAddr;

pub fn generate_enode(ip: IpAddr, port: u16) -> String {
    // Generate private key
    let signing_key = SigningKey::random(rand::rngs::OsRng);
    let verify_key = signing_key.verifying_key();

    // Compress public key to get node ID
    let pub_key_bytes = EncodedPoint::from(verify_key).as_bytes();
    let node_id = Keccak256::digest(&pub_key_bytes[1..]); // skip 0x04 prefix

    format!(
        "enode://{}@{}:{}",
        hex::encode(node_id),
        ip,
        port
    )
}
