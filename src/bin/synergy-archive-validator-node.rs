use synergy_testbeta::role_profiles::NodeRole;

fn main() {
    synergy_testbeta::role_runtime::run(
        "synergy-archive-validator-node",
        Some(NodeRole::ArchiveValidator.profile()),
    );
}
