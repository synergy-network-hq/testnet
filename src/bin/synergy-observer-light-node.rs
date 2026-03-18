use synergy_testbeta::role_profiles::NodeRole;

fn main() {
    synergy_testbeta::role_runtime::run(
        "synergy-observer-light-node",
        Some(NodeRole::ObserverLight.profile()),
    );
}
