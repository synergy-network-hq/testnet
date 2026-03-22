use synergy_testbeta::role_profiles::NodeRole;

fn main() {
    synergy_testbeta::role_runtime::run(
        "synergy-uma-coordinator-node",
        Some(NodeRole::UmaCoordinator.profile()),
    );
}
