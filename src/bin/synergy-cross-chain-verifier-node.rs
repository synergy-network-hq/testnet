use synergy_testbeta::role_profiles::NodeRole;

fn main() {
    synergy_testbeta::role_runtime::run(
        "synergy-cross-chain-verifier-node",
        Some(NodeRole::CrossChainVerifier.profile()),
    );
}
