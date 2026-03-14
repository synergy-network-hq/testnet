// Synergy Network Gas & Fee System
// Implements SNTS-04: SNRG Denomination & Precision Specification
//
// Key principles:
// - 9 decimal precision (1 SNRG = 1,000,000,000 nWei)
// - All amounts stored as integer nWei
// - Gas prices in nWei
// - No floating-point arithmetic

use serde::{Deserialize, Serialize};
use std::fmt;

/// SNRG denomination constants per SNTS-04
pub mod constants {
    /// Number of decimal places for SNRG
    pub const SNRG_DECIMALS: u32 = 9;

    /// nWei per SNRG (1 billion)
    pub const NWEI_PER_SNRG: u128 = 1_000_000_000;

    /// mWei per SNRG (1 million)
    pub const MWEI_PER_SNRG: u128 = 1_000_000;

    /// µWei per SNRG (1 thousand)
    pub const UWEI_PER_SNRG: u128 = 1_000;

    /// Default gas price (40 nWei per gas unit)
    pub const DEFAULT_GAS_PRICE: u64 = 40;

    /// Minimum gas price (1 nWei per gas unit)
    pub const MIN_GAS_PRICE: u64 = 1;

    /// Maximum gas price (1000 nWei per gas unit = 0.000001 SNRG per gas)
    pub const MAX_GAS_PRICE: u64 = 1000;

    /// Gas limit for simple transfers
    pub const GAS_LIMIT_TRANSFER: u64 = 21_000;

    /// Gas limit for contract deployment
    pub const GAS_LIMIT_CONTRACT_DEPLOY: u64 = 500_000;

    /// Gas limit for contract call
    pub const GAS_LIMIT_CONTRACT_CALL: u64 = 100_000;

    /// Maximum gas limit per transaction
    pub const MAX_GAS_LIMIT: u64 = 10_000_000;

    /// Block gas limit (maximum gas for all transactions in a block)
    pub const BLOCK_GAS_LIMIT: u64 = 30_000_000;

    /// EVM decimal precision (for cross-chain compatibility)
    pub const EVM_DECIMALS: u32 = 18;

    /// Bitcoin decimal precision (for cross-chain compatibility)
    pub const BTC_DECIMALS: u32 = 8;
}

/// Represents an amount in nWei (nano-wei)
/// All monetary values must use this type to ensure precision
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NWei(pub u128);

impl NWei {
    /// Create from SNRG amount (will be converted to nWei)
    pub fn from_snrg(snrg: f64) -> Self {
        let nwei = (snrg * constants::NWEI_PER_SNRG as f64) as u128;
        NWei(nwei)
    }

    /// Convert to SNRG (for display purposes)
    pub fn to_snrg(&self) -> f64 {
        self.0 as f64 / constants::NWEI_PER_SNRG as f64
    }

    /// Create from raw nWei value
    pub fn from_nwei(nwei: u128) -> Self {
        NWei(nwei)
    }

    /// Get raw nWei value
    pub fn as_nwei(&self) -> u128 {
        self.0
    }

    /// Add two nWei amounts
    pub fn checked_add(&self, other: &NWei) -> Option<NWei> {
        self.0.checked_add(other.0).map(NWei)
    }

    /// Subtract two nWei amounts
    pub fn checked_sub(&self, other: &NWei) -> Option<NWei> {
        self.0.checked_sub(other.0).map(NWei)
    }

    /// Multiply nWei by a scalar
    pub fn checked_mul(&self, scalar: u128) -> Option<NWei> {
        self.0.checked_mul(scalar).map(NWei)
    }

    /// Divide nWei by a scalar (floored)
    pub fn checked_div(&self, scalar: u128) -> Option<NWei> {
        if scalar == 0 {
            return None;
        }
        Some(NWei(self.0 / scalar))
    }

    /// Convert to EVM wei (18 decimals)
    /// SNRG (9 decimals) → EVM (18 decimals) = multiply by 10^9
    pub fn to_evm_wei(&self) -> u128 {
        self.0 * 1_000_000_000 // 10^9
    }

    /// Create from EVM wei (18 decimals)
    /// EVM (18 decimals) → SNRG (9 decimals) = divide by 10^9
    pub fn from_evm_wei(evm_wei: u128) -> Self {
        NWei(evm_wei / 1_000_000_000) // Floored division
    }

    /// Convert to Bitcoin satoshis (8 decimals)
    /// SNRG (9 decimals) → BTC (8 decimals) = divide by 10
    pub fn to_bitcoin_sats(&self) -> u64 {
        (self.0 / 10) as u64 // Floored division
    }

    /// Create from Bitcoin satoshis (8 decimals)
    /// BTC (8 decimals) → SNRG (9 decimals) = multiply by 10
    pub fn from_bitcoin_sats(sats: u64) -> Self {
        NWei((sats as u128) * 10)
    }
}

impl fmt::Display for NWei {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} nWei ({:.9} SNRG)", self.0, self.to_snrg())
    }
}

/// Gas price in nWei per gas unit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GasPrice(pub u64);

impl GasPrice {
    /// Create gas price from nWei
    pub fn from_nwei(nwei: u64) -> Result<Self, &'static str> {
        if nwei < constants::MIN_GAS_PRICE {
            return Err("Gas price below minimum");
        }
        if nwei > constants::MAX_GAS_PRICE {
            return Err("Gas price above maximum");
        }
        Ok(GasPrice(nwei))
    }

    /// Get default gas price (40 nWei)
    pub fn default() -> Self {
        GasPrice(constants::DEFAULT_GAS_PRICE)
    }

    /// Get raw nWei value
    pub fn as_nwei(&self) -> u64 {
        self.0
    }
}

impl Default for GasPrice {
    fn default() -> Self {
        Self::default()
    }
}

/// Gas limit (maximum gas units for a transaction)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GasLimit(pub u64);

impl GasLimit {
    /// Create gas limit
    pub fn new(limit: u64) -> Result<Self, &'static str> {
        if limit == 0 {
            return Err("Gas limit cannot be zero");
        }
        if limit > constants::MAX_GAS_LIMIT {
            return Err("Gas limit exceeds maximum");
        }
        Ok(GasLimit(limit))
    }

    /// Get gas limit for simple transfer
    pub fn transfer() -> Self {
        GasLimit(constants::GAS_LIMIT_TRANSFER)
    }

    /// Get gas limit for contract deployment
    pub fn contract_deploy() -> Self {
        GasLimit(constants::GAS_LIMIT_CONTRACT_DEPLOY)
    }

    /// Get gas limit for contract call
    pub fn contract_call() -> Self {
        GasLimit(constants::GAS_LIMIT_CONTRACT_CALL)
    }

    /// Get raw gas limit value
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

/// Gas fee calculation result
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GasFee {
    /// Gas limit (max gas units)
    pub gas_limit: u64,
    /// Gas price (nWei per gas unit)
    pub gas_price: u64,
    /// Total fee in nWei (gas_limit × gas_price)
    pub fee_nwei: u128,
    /// Total fee in SNRG (for display)
    pub fee_snrg: f64,
}

impl GasFee {
    /// Calculate gas fee
    /// fee_nWei = gasLimit × gasPrice
    pub fn calculate(gas_limit: GasLimit, gas_price: GasPrice) -> Self {
        let fee_nwei = (gas_limit.0 as u128) * (gas_price.0 as u128);
        let fee_snrg = fee_nwei as f64 / constants::NWEI_PER_SNRG as f64;

        GasFee {
            gas_limit: gas_limit.0,
            gas_price: gas_price.0,
            fee_nwei,
            fee_snrg,
        }
    }

    /// Get fee as NWei type
    pub fn as_nwei(&self) -> NWei {
        NWei(self.fee_nwei)
    }
}

/// Gas estimation for different transaction types
pub struct GasEstimator;

impl GasEstimator {
    /// Estimate gas for a simple transfer
    pub fn estimate_transfer() -> GasLimit {
        GasLimit::transfer()
    }

    /// Estimate gas for contract deployment
    /// Base cost + bytecode size cost
    pub fn estimate_contract_deploy(bytecode_size: usize) -> GasLimit {
        let base_cost = constants::GAS_LIMIT_CONTRACT_DEPLOY;
        let bytecode_cost = (bytecode_size as u64) * 200; // 200 gas per byte
        let total = base_cost + bytecode_cost;

        GasLimit::new(total.min(constants::MAX_GAS_LIMIT))
            .unwrap_or(GasLimit(constants::MAX_GAS_LIMIT))
    }

    /// Estimate gas for contract call
    /// Base cost + calldata size cost
    pub fn estimate_contract_call(calldata_size: usize) -> GasLimit {
        let base_cost = constants::GAS_LIMIT_CONTRACT_CALL;
        let calldata_cost = (calldata_size as u64) * 68; // 68 gas per non-zero byte
        let total = base_cost + calldata_cost;

        GasLimit::new(total.min(constants::MAX_GAS_LIMIT))
            .unwrap_or(GasLimit(constants::MAX_GAS_LIMIT))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nwei_conversions() {
        // 1 SNRG = 1,000,000,000 nWei
        let one_snrg = NWei::from_snrg(1.0);
        assert_eq!(one_snrg.as_nwei(), 1_000_000_000);

        // 2.5 SNRG = 2,500,000,000 nWei
        let two_and_half = NWei::from_snrg(2.5);
        assert_eq!(two_and_half.as_nwei(), 2_500_000_000);

        // Conversion back to SNRG
        assert_eq!(one_snrg.to_snrg(), 1.0);
    }

    #[test]
    fn test_gas_fee_calculation() {
        // Example from spec:
        // Gas Limit: 25,000
        // Gas Price: 40 nWei
        // Fee: 25,000 × 40 = 1,000,000 nWei = 0.001 SNRG
        let gas_limit = GasLimit::new(25_000).unwrap();
        let gas_price = GasPrice::from_nwei(40).unwrap();

        let fee = GasFee::calculate(gas_limit, gas_price);

        assert_eq!(fee.fee_nwei, 1_000_000);
        assert_eq!(fee.fee_snrg, 0.001);
    }

    #[test]
    fn test_evm_conversion() {
        // 2.5 SNRG = 2,500,000,000 nWei
        let snrg_amount = NWei::from_snrg(2.5);
        assert_eq!(snrg_amount.as_nwei(), 2_500_000_000);

        // Convert to EVM wei (multiply by 10^9)
        let evm_wei = snrg_amount.to_evm_wei();
        assert_eq!(evm_wei, 2_500_000_000_000_000_000);

        // Convert back from EVM wei
        let back_to_snrg = NWei::from_evm_wei(evm_wei);
        assert_eq!(back_to_snrg.as_nwei(), 2_500_000_000);
    }

    #[test]
    fn test_bitcoin_conversion() {
        // 10 SNRG = 10,000,000,000 nWei
        let snrg_amount = NWei::from_snrg(10.0);

        // Convert to Bitcoin satoshis (divide by 10)
        let sats = snrg_amount.to_bitcoin_sats();
        assert_eq!(sats, 1_000_000_000);

        // Convert back from satoshis
        let back_to_snrg = NWei::from_bitcoin_sats(sats);
        assert_eq!(back_to_snrg.as_nwei(), 10_000_000_000);
    }

    #[test]
    fn test_gas_estimation() {
        // Simple transfer
        let transfer_gas = GasEstimator::estimate_transfer();
        assert_eq!(transfer_gas.as_u64(), 21_000);

        // Contract deployment with 10KB bytecode
        let deploy_gas = GasEstimator::estimate_contract_deploy(10_000);
        assert!(deploy_gas.as_u64() > 500_000);

        // Contract call with 256 bytes calldata
        let call_gas = GasEstimator::estimate_contract_call(256);
        assert!(call_gas.as_u64() > 100_000);
    }
}
