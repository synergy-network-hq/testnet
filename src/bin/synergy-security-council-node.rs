use synergy_testbeta::role_profiles::NodeRole;

fn main() {
    synergy_testbeta::role_runtime::run(
        "synergy-security-council-node",
        Some(NodeRole::SecurityCouncil.profile()),
    );
}
