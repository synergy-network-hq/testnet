use serde::{Deserialize, Serialize};
use synq_pqc_shims::kyber::{keygen as kyber_keygen, encaps as kyber_encaps, decaps as kyber_decaps};
use synq_pqc_shims::dilithium::{keygen as dilithium_keygen};
use synq_pqc_shims::falcon::{keygen as falcon_keygen};
use synq_pqc_shims::sphincs::{keygen as sphincs_keygen};
use synq_pqc_shims::mceliece::{keygen as mceliece_keygen};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PQCSecurityLevel {
    Basic,
    Enhanced,
    Maximum,
    Military,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PQCKeyPair {
    pub algorithm: String,
    pub public_key: Vec<u8>,
    pub private_key: Vec<u8>,
    pub security_level: PQCSecurityLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PQCSignature {
    pub algorithm: String,
    pub signature: Vec<u8>,
    pub message_hash: Vec<u8>,
    pub security_level: PQCSecurityLevel,
}

#[derive(Debug)]
pub struct PQCCompiler {
    security_level: PQCSecurityLevel,
}

impl PQCCompiler {
    pub fn new(security_level: PQCSecurityLevel) -> Self {
        PQCCompiler { security_level }
    }

    pub fn generate_keypair(&self, algorithm: &str) -> Result<PQCKeyPair, String> {
        let (pk, sk) = match algorithm.to_lowercase().as_str() {
            "kyber" | "kyber768" => {
                match kyber_keygen() {
                    Ok((pk, sk)) => (pk, sk),
                    Err(e) => return Err(format!("Kyber key generation failed: {}", e)),
                }
            },
            "dilithium" | "dilithium3" => {
                match dilithium_keygen() {
                    Ok((pk, sk)) => (pk, sk),
                    Err(e) => return Err(format!("Dilithium key generation failed: {}", e)),
                }
            },
            "falcon" | "falcon512" => {
                match falcon_keygen() {
                    Ok((pk, sk)) => (pk, sk),
                    Err(e) => return Err(format!("Falcon key generation failed: {}", e)),
                }
            },
            "sphincs" | "sphincsplus" => {
                match sphincs_keygen() {
                    Ok((pk, sk)) => (pk, sk),
                    Err(e) => return Err(format!("SPHINCS+ key generation failed: {}", e)),
                }
            },
            "mceliece" | "classicmceliece" => {
                match mceliece_keygen() {
                    Ok((pk, sk)) => (pk, sk),
                    Err(e) => return Err(format!("Classic-McEliece key generation failed: {}", e)),
                }
            },
            _ => return Err(format!("Unsupported PQC algorithm: {}", algorithm)),
        };

        Ok(PQCKeyPair {
            algorithm: algorithm.to_string(),
            public_key: pk,
            private_key: sk,
            security_level: self.security_level.clone(),
        })
    }

    pub fn sign_message(&self, private_key: &[u8], message: &[u8], algorithm: &str) -> Result<PQCSignature, String> {
        // For now, create a simple signature (would use actual PQC signing in production)
        let signature = self.create_signature(private_key, message, algorithm)?;

        Ok(PQCSignature {
            algorithm: algorithm.to_string(),
            signature,
            message_hash: self.hash_message(message),
            security_level: self.security_level.clone(),
        })
    }

    pub fn verify_signature(&self, public_key: &[u8], signature: &[u8], message: &[u8], algorithm: &str) -> Result<bool, String> {
        // For now, simple verification (would use actual PQC verification in production)
        let expected_hash = self.hash_message(message);
        let signature_hash = self.hash_message(signature);

        // Simple verification logic (would be replaced with actual PQC verification)
        Ok(expected_hash == signature_hash)
    }

    pub fn encapsulate_key(&self, public_key: &[u8], algorithm: &str) -> Result<(Vec<u8>, Vec<u8>), String> {
        match algorithm.to_lowercase().as_str() {
            "kyber" | "kyber768" => {
                match kyber_encaps(public_key) {
                    Ok((ct, ss)) => Ok((ct, ss)),
                    Err(e) => Err(format!("Kyber encapsulation failed: {}", e)),
                }
            },
            "mceliece" | "classicmceliece" => {
                // Classic-McEliece encapsulation would go here
                Err("Classic-McEliece encapsulation not yet implemented".to_string())
            },
            _ => Err(format!("Unsupported KEM algorithm: {}", algorithm)),
        }
    }

    pub fn decapsulate_key(&self, ciphertext: &[u8], private_key: &[u8], algorithm: &str) -> Result<Vec<u8>, String> {
        match algorithm.to_lowercase().as_str() {
            "kyber" | "kyber768" => {
                match kyber_decaps(ciphertext, private_key) {
                    Ok(ss) => Ok(ss),
                    Err(e) => Err(format!("Kyber decapsulation failed: {}", e)),
                }
            },
            "mceliece" | "classicmceliece" => {
                // Classic-McEliece decapsulation would go here
                Err("Classic-McEliece decapsulation not yet implemented".to_string())
            },
            _ => Err(format!("Unsupported KEM algorithm: {}", algorithm)),
        }
    }

    fn create_signature(&self, private_key: &[u8], message: &[u8], algorithm: &str) -> Result<Vec<u8>, String> {
        // Simple signature creation (would be replaced with actual PQC signing)
        use sha3::{Sha3_256, Digest};
        let mut hasher = Sha3_256::new();
        hasher.update(private_key);
        hasher.update(message);
        hasher.update(algorithm.as_bytes());
        Ok(hasher.finalize().to_vec())
    }

    fn hash_message(&self, message: &[u8]) -> Vec<u8> {
        use sha3::{Sha3_256, Digest};
        let mut hasher = Sha3_256::new();
        hasher.update(message);
        hasher.finalize().to_vec()
    }

    pub fn get_supported_algorithms(&self) -> Vec<String> {
        vec![
            "kyber".to_string(),
            "dilithium".to_string(),
            "falcon".to_string(),
            "sphincs".to_string(),
            "mceliece".to_string(),
        ]
    }

    pub fn get_security_level(&self) -> &PQCSecurityLevel {
        &self.security_level
    }
}

impl Default for PQCCompiler {
    fn default() -> Self {
        PQCCompiler::new(PQCSecurityLevel::Enhanced)
    }
}


