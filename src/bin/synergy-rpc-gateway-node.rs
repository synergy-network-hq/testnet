use synergy_testbeta::role_profiles::NodeRole;

fn main() {
    synergy_testbeta::role_runtime::run(
        "synergy-rpc-gateway-node",
        Some(NodeRole::RpcGateway.profile()),
    );
}
