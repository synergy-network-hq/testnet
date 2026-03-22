use synergy_testbeta::role_profiles::NodeRole;

fn main() {
    synergy_testbeta::role_runtime::run(
        "synergy-committee-node",
        Some(NodeRole::Committee.profile()),
    );
}
