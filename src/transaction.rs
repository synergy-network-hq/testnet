use bincode::config::standard;
use bincode::{decode_from_slice, encode_to_vec};
use bincode::{Decode, Encode};
use blake3::Hasher;
use serde::{Deserialize, Serialize};
// Removed unused sha3 imports
use crate::crypto::pqc::{PQCAlgorithm, PQCManager, PQCPrivateKey, PQCPublicKey};
use hex;

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Transaction {
    pub sender: String,
    pub receiver: String,
    pub amount: u64,
    pub nonce: u64,
    pub signature: Vec<u8>, // Changed from String to Vec<u8> for binary signature data
    pub timestamp: u64,
    pub gas_price: u64,
    pub gas_limit: u64,
    pub data: Option<String>,
    pub signature_algorithm: String, // Track which PQC algorithm was used
}

#[derive(Debug, Clone)]
pub struct TransactionValidationResult {
    pub is_valid: bool,
    pub error_message: Option<String>,
}

impl Transaction {
    pub fn new(
        sender: String,
        receiver: String,
        amount: u64,
        nonce: u64,
        signature: Vec<u8>,
        gas_price: u64,
        gas_limit: u64,
        data: Option<String>,
        signature_algorithm: String,
    ) -> Self {
        Transaction {
            sender,
            receiver,
            amount,
            nonce,
            signature,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            gas_price,
            gas_limit,
            data,
            signature_algorithm,
        }
    }

    /// Returns the raw hash (hex string) for internal use (signing, verification)
    pub fn raw_hash(&self) -> String {
        let mut hasher = Hasher::new();
        hasher.update(self.sender.as_bytes());
        hasher.update(self.receiver.as_bytes());
        hasher.update(&self.amount.to_le_bytes());
        hasher.update(&self.nonce.to_le_bytes());
        hasher.update(&self.timestamp.to_le_bytes());
        hasher.update(&self.gas_price.to_le_bytes());
        hasher.update(&self.gas_limit.to_le_bytes());

        if let Some(ref data) = self.data {
            hasher.update(data.as_bytes());
        }

        hex::encode(hasher.finalize().as_bytes())
    }

    /// Returns Synergy-formatted transaction hash with appropriate prefix
    /// Regular transactions: syntxn-<hash>
    /// Cross-chain transactions: synxxn-<hash>
    pub fn hash(&self) -> String {
        let raw_hash = self.raw_hash();

        // Check if this is a cross-chain transaction
        let is_cross_chain = self
            .data
            .as_ref()
            .map(|d| d.starts_with("bridge_transfer:") || d.starts_with("cross_chain:"))
            .unwrap_or(false);

        if is_cross_chain {
            format!("synxxn-{}", raw_hash)
        } else {
            format!("syntxn-{}", raw_hash)
        }
    }

    pub fn sign(
        &mut self,
        private_key: &PQCPrivateKey,
        pqc_manager: &mut PQCManager,
    ) -> Result<(), String> {
        // Get the raw transaction hash (without prefix) for signing
        let message = self.raw_hash();
        let message_bytes =
            hex::decode(&message).map_err(|e| format!("Failed to decode hash: {}", e))?;

        // Sign using the appropriate PQC algorithm
        let signature = match private_key.algorithm {
            PQCAlgorithm::MLDSA => pqc_manager.sign(private_key, &message_bytes)?,
            PQCAlgorithm::FNDSA => pqc_manager.sign(private_key, &message_bytes)?,
            PQCAlgorithm::SLHDSA => pqc_manager.sign(private_key, &message_bytes)?,
            _ => return Err("Unsupported signature algorithm".to_string()),
        };

        self.signature = signature.signature_data;
        self.signature_algorithm = algorithm_name(&private_key.algorithm).to_string();

        Ok(())
    }

    pub fn verify_signature(&self, public_key: &PQCPublicKey, pqc_manager: &PQCManager) -> bool {
        // Get the raw transaction hash (without prefix) that was signed
        let message = self.raw_hash();
        let message_bytes = hex::decode(&message).unwrap_or_else(|_| Vec::new());

        if message_bytes.is_empty() {
            return false;
        }

        // Create a signature object for verification
        let signature = crate::crypto::pqc::PQCSignature {
            algorithm: public_key.algorithm.clone(),
            signature_data: self.signature.clone(),
            message_hash: message_bytes.clone(),
            public_key_id: public_key.key_id.clone(),
            created_at: self.timestamp,
        };

        // Verify using the appropriate PQC algorithm
        match pqc_manager.verify(public_key, &signature, &message_bytes) {
            Ok(is_valid) => is_valid,
            Err(_) => false,
        }
    }

    pub fn validate(&self) -> TransactionValidationResult {
        // Basic validation checks
        if self.sender.is_empty() {
            return TransactionValidationResult {
                is_valid: false,
                error_message: Some("Sender address cannot be empty".to_string()),
            };
        }

        if self.receiver.is_empty() {
            return TransactionValidationResult {
                is_valid: false,
                error_message: Some("Receiver address cannot be empty".to_string()),
            };
        }

        if self.amount == 0 {
            return TransactionValidationResult {
                is_valid: false,
                error_message: Some("Transaction amount must be greater than 0".to_string()),
            };
        }

        if self.gas_price == 0 {
            return TransactionValidationResult {
                is_valid: false,
                error_message: Some("Gas price must be greater than 0".to_string()),
            };
        }

        if self.gas_limit == 0 {
            return TransactionValidationResult {
                is_valid: false,
                error_message: Some("Gas limit must be greater than 0".to_string()),
            };
        }

        if self.signature.is_empty() {
            return TransactionValidationResult {
                is_valid: false,
                error_message: Some("Transaction must be signed".to_string()),
            };
        }

        // Check if signature algorithm is supported
        match self.signature_algorithm.as_str() {
            "mldsa" | "fndsa" | "slhdsa" => {}
            _ => {
                return TransactionValidationResult {
                    is_valid: false,
                    error_message: Some(format!(
                        "Unsupported signature algorithm: {}",
                        self.signature_algorithm
                    )),
                };
            }
        }

        // Check timestamp is not too old (within 1 hour)
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if current_time.saturating_sub(self.timestamp) > 3600 {
            return TransactionValidationResult {
                is_valid: false,
                error_message: Some("Transaction timestamp is too old".to_string()),
            };
        }

        TransactionValidationResult {
            is_valid: true,
            error_message: None,
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, String> {
        encode_to_vec(self, standard())
            .map_err(|e| format!("Failed to serialize transaction: {}", e))
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        decode_from_slice(data, standard())
            .map(|(transaction, _)| transaction)
            .map_err(|e| format!("Failed to deserialize transaction: {}", e))
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| format!("Failed to serialize to JSON: {}", e))
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("Failed to deserialize from JSON: {}", e))
    }

    pub fn get_fee(&self) -> u64 {
        self.gas_price * self.gas_limit
    }

    pub fn get_total_value(&self) -> u64 {
        self.amount + self.get_fee()
    }

    pub fn is_contract_call(&self) -> bool {
        self.data.is_some()
    }

    pub fn get_contract_data(&self) -> Option<&String> {
        self.data.as_ref()
    }

    pub fn get_signature_hex(&self) -> String {
        hex::encode(&self.signature)
    }

    pub fn get_signature_algorithm(&self) -> &str {
        &self.signature_algorithm
    }

    pub fn get_sender(&self) -> &str {
        &self.sender
    }

    pub fn get_receiver(&self) -> &str {
        &self.receiver
    }

    pub fn get_amount(&self) -> u64 {
        self.amount
    }

    pub fn get_nonce(&self) -> u64 {
        self.nonce
    }

    pub fn get_timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn get_gas_price(&self) -> u64 {
        self.gas_price
    }

    pub fn get_gas_limit(&self) -> u64 {
        self.gas_limit
    }

    /// Calculate gas fee using the gas module (SNTS-04 compliant)
    /// Returns fee in nWei
    pub fn calculate_gas_fee(&self) -> u128 {
        (self.gas_limit as u128) * (self.gas_price as u128)
    }

    /// Get gas fee as NWei type
    pub fn get_gas_fee_nwei(&self) -> crate::gas::NWei {
        crate::gas::NWei::from_nwei(self.calculate_gas_fee())
    }

    /// Get gas fee in SNRG (for display)
    pub fn get_gas_fee_snrg(&self) -> f64 {
        self.get_gas_fee_nwei().to_snrg()
    }

    /// Get total cost (amount + gas fee) in nWei
    pub fn get_total_cost_nwei(&self) -> u128 {
        (self.amount as u128) + self.calculate_gas_fee()
    }

    /// Check if sender has sufficient balance for transaction
    /// balance should be in nWei
    pub fn has_sufficient_balance(&self, sender_balance: u128) -> bool {
        sender_balance >= self.get_total_cost_nwei()
    }

    /// Set gas price (in nWei per gas unit)
    pub fn set_gas_price(&mut self, gas_price: u64) -> Result<(), String> {
        use crate::gas::GasPrice;
        // Validate gas price
        GasPrice::from_nwei(gas_price)?;
        self.gas_price = gas_price;
        Ok(())
    }

    /// Set gas limit
    pub fn set_gas_limit(&mut self, gas_limit: u64) -> Result<(), String> {
        use crate::gas::GasLimit;
        // Validate gas limit
        GasLimit::new(gas_limit)?;
        self.gas_limit = gas_limit;
        Ok(())
    }

    /// Estimate gas for this transaction based on its type
    pub fn estimate_gas(&self) -> u64 {
        use crate::gas::GasEstimator;

        if let Some(ref data) = self.data {
            if data.starts_with("deploy:") {
                // Contract deployment
                let bytecode_size = data.len();
                GasEstimator::estimate_contract_deploy(bytecode_size).as_u64()
            } else {
                // Contract call
                let calldata_size = data.len();
                GasEstimator::estimate_contract_call(calldata_size).as_u64()
            }
        } else {
            // Simple transfer
            GasEstimator::estimate_transfer().as_u64()
        }
    }
}

// Helper function to get algorithm name
fn algorithm_name(algorithm: &PQCAlgorithm) -> &'static str {
    match algorithm {
        PQCAlgorithm::MLKEM1024 => "mlkem1024",
        PQCAlgorithm::MLDSA => "mldsa",
        PQCAlgorithm::FNDSA => "fndsa",
        PQCAlgorithm::SLHDSA => "slhdsa",
        PQCAlgorithm::HQCKEM => "hqckem",
    }
}

// Helper function to parse algorithm name
pub fn parse_algorithm_name(name: &str) -> Result<PQCAlgorithm, String> {
    match name.to_lowercase().as_str() {
        "mlkem" | "mlkem1024" => Ok(PQCAlgorithm::MLKEM1024),
        "mldsa" => Ok(PQCAlgorithm::MLDSA),
        "fndsa" => Ok(PQCAlgorithm::FNDSA),
        "slhdsa" => Ok(PQCAlgorithm::SLHDSA),
        "hqckem" => Ok(PQCAlgorithm::HQCKEM),
        _ => Err(format!("Unknown algorithm: {}", name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_creation() {
        let tx = Transaction::new(
            "sender123".to_string(),
            "receiver456".to_string(),
            1000,
            1,
            vec![0x01, 0x02, 0x03],
            100,
            21000,
            None,
            "mldsa".to_string(),
        );

        assert_eq!(tx.sender, "sender123");
        assert_eq!(tx.receiver, "receiver456");
        assert_eq!(tx.amount, 1000);
        assert_eq!(tx.nonce, 1);
        assert_eq!(tx.signature, vec![0x01, 0x02, 0x03]);
        assert_eq!(tx.gas_price, 100);
        assert_eq!(tx.gas_limit, 21000);
        assert_eq!(tx.signature_algorithm, "mldsa");
    }

    #[test]
    fn test_transaction_hash() {
        let tx1 = Transaction::new(
            "sender123".to_string(),
            "receiver456".to_string(),
            1000,
            1,
            vec![0x01, 0x02, 0x03],
            100,
            21000,
            None,
            "mldsa".to_string(),
        );

        let tx2 = Transaction::new(
            "sender123".to_string(),
            "receiver456".to_string(),
            1000,
            1,
            vec![0x01, 0x02, 0x03],
            100,
            21000,
            None,
            "mldsa".to_string(),
        );

        // Same transaction should have same hash
        assert_eq!(tx1.hash(), tx2.hash());

        let tx3 = Transaction::new(
            "sender123".to_string(),
            "receiver456".to_string(),
            2000, // Different amount
            1,
            vec![0x01, 0x02, 0x03],
            100,
            21000,
            None,
            "mldsa".to_string(),
        );

        // Different transaction should have different hash
        assert_ne!(tx1.hash(), tx3.hash());
    }

    #[test]
    fn test_transaction_validation() {
        let valid_tx = Transaction::new(
            "sender123".to_string(),
            "receiver456".to_string(),
            1000,
            1,
            vec![0x01, 0x02, 0x03],
            100,
            21000,
            None,
            "mldsa".to_string(),
        );

        let result = valid_tx.validate();
        assert!(result.is_valid);
        assert!(result.error_message.is_none());

        let invalid_tx = Transaction::new(
            "".to_string(), // Empty sender
            "receiver456".to_string(),
            1000,
            1,
            vec![0x01, 0x02, 0x03],
            100,
            21000,
            None,
            "mldsa".to_string(),
        );

        let result = invalid_tx.validate();
        assert!(!result.is_valid);
        assert!(result.error_message.is_some());
    }

    #[test]
    fn test_transaction_serialization() {
        let tx = Transaction::new(
            "sender123".to_string(),
            "receiver456".to_string(),
            1000,
            1,
            vec![0x01, 0x02, 0x03],
            100,
            21000,
            None,
            "mldsa".to_string(),
        );

        let serialized = tx.serialize().unwrap();
        let deserialized = Transaction::deserialize(&serialized).unwrap();

        assert_eq!(tx.sender, deserialized.sender);
        assert_eq!(tx.receiver, deserialized.receiver);
        assert_eq!(tx.amount, deserialized.amount);
        assert_eq!(tx.nonce, deserialized.nonce);
        assert_eq!(tx.signature, deserialized.signature);
        assert_eq!(tx.signature_algorithm, deserialized.signature_algorithm);
    }

    #[test]
    fn test_transaction_json() {
        let tx = Transaction::new(
            "sender123".to_string(),
            "receiver456".to_string(),
            1000,
            1,
            vec![0x01, 0x02, 0x03],
            100,
            21000,
            None,
            "mldsa".to_string(),
        );

        let json = tx.to_json().unwrap();
        let deserialized = Transaction::from_json(&json).unwrap();

        assert_eq!(tx.sender, deserialized.sender);
        assert_eq!(tx.receiver, deserialized.receiver);
        assert_eq!(tx.amount, deserialized.amount);
        assert_eq!(tx.nonce, deserialized.nonce);
        assert_eq!(tx.signature, deserialized.signature);
        assert_eq!(tx.signature_algorithm, deserialized.signature_algorithm);
    }

    #[test]
    fn test_transaction_fee_calculation() {
        let tx = Transaction::new(
            "sender123".to_string(),
            "receiver456".to_string(),
            1000,
            1,
            vec![0x01, 0x02, 0x03],
            100,
            21000,
            None,
            "mldsa".to_string(),
        );

        assert_eq!(tx.get_fee(), 100 * 21000);
        assert_eq!(tx.get_total_value(), 1000 + (100 * 21000));
    }

    #[test]
    fn test_algorithm_parsing() {
        assert_eq!(parse_algorithm_name("mldsa").unwrap(), PQCAlgorithm::MLDSA);
        assert_eq!(parse_algorithm_name("fndsa").unwrap(), PQCAlgorithm::FNDSA);
        assert_eq!(
            parse_algorithm_name("slhdsa").unwrap(),
            PQCAlgorithm::SLHDSA
        );
        assert_eq!(
            parse_algorithm_name("mlkem").unwrap(),
            PQCAlgorithm::MLKEM1024
        );
        assert_eq!(
            parse_algorithm_name("mlkem1024").unwrap(),
            PQCAlgorithm::MLKEM1024
        );
        assert_eq!(
            parse_algorithm_name("hqckem").unwrap(),
            PQCAlgorithm::HQCKEM
        );

        assert!(parse_algorithm_name("unknown").is_err());
    }
}
