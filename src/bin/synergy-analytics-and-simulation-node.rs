use synergy_testbeta::role_profiles::NodeRole;

fn main() {
    synergy_testbeta::role_runtime::run(
        "synergy-analytics-and-simulation-node",
        Some(NodeRole::AnalyticsSimulation.profile()),
    );
}
