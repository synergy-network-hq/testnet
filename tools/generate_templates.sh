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
    local log_file=$6
    local enable_pruning=${7:-false}
    local vrf_enabled=${8:-true}

    cat > "$TEMPLATES_DIR/$node_type.toml" <<EOF
[network]
id = 338639
name = "synergy-testbeta"
p2p_port = $p2p_port
rpc_port = $rpc_port
ws_port = $ws_port
max_peers = 50
bootnodes = ["enode://d18491c5a94ef758b6a15478818a1903054c830afdc2cc6b8d04d30d7c8e94b5bcd9c98f33c7ff5a02f7e4c7a5394fc5a4a41d1552d9e43c0e4745a3127c93d4@testnet-beta.synergy.network:30303"]

[blockchain]
block_time = 5
max_gas_limit = "0x2fefd8"
chain_id = 338639

[consensus]
algorithm = "Proof of Synergy"
block_time_secs = 5
epoch_length = 30000
validator_cluster_size = 7
max_validators = 21
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
grpc_port = 50051
cors_enabled = true
cors_origins = ["*"]

[p2p]
listen_address = "0.0.0.0:$p2p_port"
public_address = "127.0.0.1:$p2p_port"
node_name = "$node_name"
enable_discovery = true
discovery_port = $((p2p_port - 2))
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
generate_template "validator" "validator-node-01" 30303 8545 8546 "data/logs/validator.log" false true
generate_template "archive-validator" "archive-validator-01" 30304 8546 8547 "data/logs/archive-validator.log" false true
generate_template "audit-validator" "audit-validator-01" 30305 8547 8548 "data/logs/audit-validator.log" false true
generate_template "committee" "committee-node-01" 30306 8548 8549 "data/logs/committee.log" false true
generate_template "governance-auditor" "governance-auditor-01" 30307 8549 8550 "data/logs/governance-auditor.log" false true
generate_template "security-council" "security-council-01" 30308 8550 8551 "data/logs/security-council.log" false true
generate_template "treasury-controller" "treasury-controller-01" 30309 8551 8552 "data/logs/treasury-controller.log" false true

# Class II - Data & Infrastructure
generate_template "oracle" "oracle-node-01" 30310 8552 8553 "data/logs/oracle.log" true false
generate_template "observer" "observer-node-01" 30311 8553 8554 "data/logs/observer.log" true false
generate_template "indexer" "indexer-node-01" 30312 8554 8555 "data/logs/indexer.log" false false
generate_template "data-availability" "data-availability-01" 30313 8555 8556 "data/logs/data-availability.log" false false
generate_template "cross-chain-verifier" "cross-chain-verifier-01" 30314 8556 8557 "data/logs/cross-chain-verifier.log" true false
generate_template "relayer" "relayer-node-01" 39638 49638 59638 "data/logs/relayer.log" true false
generate_template "rpc" "rpc-node-01" 30316 8558 8559 "data/logs/rpc.log" true false
generate_template "rpc-gateway" "rpc-gateway-01" 30317 8559 8560 "data/logs/rpc-gateway.log" true false
generate_template "witness" "witness-node-01" 30318 8560 8561 "data/logs/witness.log" true false

# Class III - Compute & AI
generate_template "ai-inference" "ai-inference-01" 30319 8561 8562 "data/logs/ai-inference.log" true false
generate_template "compute" "compute-node-01" 30320 8562 8563 "data/logs/compute.log" true false
generate_template "pqc-crypto" "pqc-crypto-01" 30321 8563 8564 "data/logs/pqc-crypto.log" true false
generate_template "uma-coordinator" "uma-coordinator-01" 30322 8564 8565 "data/logs/uma-coordinator.log" true false

echo
echo "✓ All 20 node templates generated successfully!"
echo "Templates location: $TEMPLATES_DIR/"
