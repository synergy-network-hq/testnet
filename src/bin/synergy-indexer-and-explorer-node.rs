use synergy_testbeta::role_profiles::NodeRole;

fn main() {
    synergy_testbeta::role_runtime::run(
        "synergy-indexer-and-explorer-node",
        Some(NodeRole::IndexerExplorer.profile()),
    );
}
