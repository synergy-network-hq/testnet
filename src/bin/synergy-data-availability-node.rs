use synergy_testbeta::role_profiles::NodeRole;

fn main() {
    synergy_testbeta::role_runtime::run(
        "synergy-data-availability-node",
        Some(NodeRole::DataAvailability.profile()),
    );
}
