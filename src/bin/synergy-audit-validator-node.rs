use synergy_testbeta::role_profiles::NodeRole;

fn main() {
    synergy_testbeta::role_runtime::run(
        "synergy-audit-validator-node",
        Some(NodeRole::AuditValidator.profile()),
    );
}
