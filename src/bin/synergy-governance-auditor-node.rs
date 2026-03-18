use synergy_testbeta::role_profiles::NodeRole;

fn main() {
    synergy_testbeta::role_runtime::run(
        "synergy-governance-auditor-node",
        Some(NodeRole::GovernanceAuditor.profile()),
    );
}
