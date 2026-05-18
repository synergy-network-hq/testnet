//! Synergy Network Consensus Module
//!
//! This module handles initialization and coordination of the
//! consensus mechanism used to secure the Synergy Testnet blockchain.

pub mod anti_divergence;
pub mod cartel_detection;
pub mod consensus_algorithm;
pub mod dao_governance;
pub mod dual_quorum;
pub mod posy;
pub mod synergy_score;
#[cfg(test)]
pub mod tests;
pub mod vrf;

use self::consensus_algorithm::ProofOfSynergy;

/// Starts the consensus mechanism using Proof of Synergy.
pub fn start_consensus() {
    let mut engine = ProofOfSynergy::new();
    engine.initialize();
    engine.execute(); // Starts the mining loop
}
