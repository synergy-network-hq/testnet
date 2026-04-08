#!/bin/bash

# Script to generate complete configuration templates for all 19 node types

TEMPLATES_DIR="templates"

# Base configuration that all templates will extend
generate_template() {
    local node_type=$1
    local node_name=$2
    local p2p_port=$3
    local rpc_port=$4
    local ws_port=$5
    local discovery_port=$6
    local log_file=$7
    local enable_pruning=${8:-false}
    local vrf_enabled=${9:-true}

    cat > "$TEMPLATES_DIR/$node_type.toml" <<EOF
[network]
id = 338639
name = "synergy-testnet-beta"
p2p_port = $p2p_port
rpc_port = $rpc_port
ws_port = $ws_port
max_peers = 50
bootnodes = ["snr://bootstrap@bootnode1.synergynode.xyz:5620", "snr://bootstrap@bootnode2.synergynode.xyz:5620", "snr://bootstrap@bootnode3.synergynode.xyz:5620"]

[blockchain]
block_time = 5
max_gas_limit = "0x2fefd8"
chain_id = 338639

[consensus]
algorithm = "Proof of Synergy"
block_time_secs = 5
epoch_length = 1000
validator_cluster_size = 5
validator_vote_threshold = 3
max_validators = 5
synergy_score_decay_rate = 0.05
vrf_enabled = $vrf_enabled
vrf_seed_epoch_interval = 1000
max_synergy_points_per_epoch = 100
max_tasks_per_validator = 10

[consensus.reward_weighting]
task_accuracy = 0.5
uptime = 0.3
collaboration = 0.2

[logging]
log_level = "info"
log_file = "$log_file"
enable_console = true
max_file_size = 10485760
max_files = 5

[rpc]
enable_http = true
http_port = $rpc_port
enable_ws = true
ws_port = $ws_port
enable_grpc = true
grpc_port = $rpc_port
cors_enabled = true
cors_origins = ["*"]

[p2p]
listen_address = "0.0.0.0:$p2p_port"
public_address = "127.0.0.1:$p2p_port"
node_name = "$node_name"
enable_discovery = true
discovery_port = $discovery_port
heartbeat_interval = 30

[storage]
database = "rocksdb"
path = "data/chain"
enable_pruning = $enable_pruning
pruning_interval = 86400
EOF

    echo "Generated: $node_type.toml"
}

mkdir -p "$TEMPLATES_DIR"

echo "Generating node configuration templates..."
echo

# Class I - Validators & Governance
generate_template "validator" "validator-node-01" 5622 5640 5660 5680 "data/logs/validator.log" false true
generate_template "archive-validator" "archive-validator-01" 5622 5640 5660 5680 "data/logs/archive-validator.log" false true
generate_template "audit-validator" "audit-validator-01" 5622 5640 5660 5680 "data/logs/audit-validator.log" false true
generate_template "committee" "committee-node-01" 5622 5640 5660 5680 "data/logs/committee.log" false true
generate_template "governance-auditor" "governance-auditor-01" 5622 5640 5660 5680 "data/logs/governance-auditor.log" false true
generate_template "security-council" "security-council-01" 5622 5640 5660 5680 "data/logs/security-council.log" false true
generate_template "treasury-controller" "treasury-controller-01" 5622 5640 5660 5680 "data/logs/treasury-controller.log" false true

# Class II - Data & Infrastructure
generate_template "oracle" "oracle-node-01" 5622 5640 5660 5680 "data/logs/oracle.log" true false
generate_template "observer" "observer-node-01" 5622 5640 5660 5680 "data/logs/observer.log" true false
generate_template "indexer" "indexer-node-01" 5622 5640 5660 5680 "data/logs/indexer.log" false false
generate_template "data-availability" "data-availability-01" 5622 5640 5660 5680 "data/logs/data-availability.log" false false
generate_template "cross-chain-verifier" "cross-chain-verifier-01" 5622 5640 5660 5680 "data/logs/cross-chain-verifier.log" true false
generate_template "relayer" "relayer-node-01" 5622 5640 5660 5680 "data/logs/relayer.log" true false
generate_template "rpc" "rpc-node-01" 5622 5640 5660 5680 "data/logs/rpc.log" true false
generate_template "rpc-gateway" "rpc-gateway-01" 5622 5640 5660 5680 "data/logs/rpc-gateway.log" true false
generate_template "witness" "witness-node-01" 5622 5640 5660 5680 "data/logs/witness.log" true false

# Class III - Compute & AI
generate_template "ai-inference" "ai-inference-01" 5622 5640 5660 5680 "data/logs/ai-inference.log" true false
generate_template "compute" "compute-node-01" 5622 5640 5660 5680 "data/logs/compute.log" true false
generate_template "pqc-crypto" "pqc-crypto-01" 5622 5640 5660 5680 "data/logs/pqc-crypto.log" true false
generate_template "uma-coordinator" "uma-coordinator-01" 5622 5640 5660 5680 "data/logs/uma-coordinator.log" true false

echo
echo "✓ All 20 node templates generated successfully!"
echo "Templates location: $TEMPLATES_DIR/"
