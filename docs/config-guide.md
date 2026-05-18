# Synergy Network Configuration Guide

## 📋 Overview

The Synergy Network supports flexible configuration through multiple sources with the following priority order:

1. **Environment Variables** (highest priority)
2. **TOML Configuration Files**
3. **Default Values** (lowest priority)

This guide explains all configuration options and how to customize your node setup.

---

## 🗂️ Configuration Files

### Primary Configuration Files

| File | Purpose | Location |
|------|---------|----------|
| `config/network-config.toml` | Network and P2P settings | Required |
| `config/node_config.toml` | Node-specific configuration | Optional |
| `config/genesis.json` | Genesis block and network parameters | Required |
| `config/validator/config.toml` | Validator-specific settings | Optional |

### Example Configuration Structure

```
config/
├── network-config.toml      # Network settings
├── node_config.toml         # Node configuration
├── genesis.json            # Genesis block
└── validator/
    └── config.toml         # Validator settings
```

---

## 🌐 Network Configuration

### config/network-config.toml

```toml
[network]
# Network identification
id = 1262
name = "Synergy Testnet"
description = "Synergy Network Testnet"

# Port configuration
p2p_port = 30303
rpc_port = 8545
ws_port = 8546
max_peers = 50

# Bootstrap nodes
bootnodes = [
  "enode://d18491c5a94ef758b6a15478818a1903054c830afdc2cc6b8d04d30d7c8e94b5bcd9c98f33c7ff5a02f7e4c7a5394fc5a4a41d1552d9e43c0e4745a3127c93d4@testnet.synergy.network:30303"
]

[network.listen]
# Bind addresses for different services
p2p = "0.0.0.0:30303"
rpc = "127.0.0.1:8545"
ws  = "127.0.0.1:8546"

[blockchain]
# Blockchain parameters
block_time = 5
max_gas_limit = "0x2fefd8"
chain_id = 1262

[storage]
# Storage configuration
database = "rocksdb"
path = "/var/lib/synergy/data"
enable_pruning = true
pruning_interval = 86400

[api]
# API configuration
enable_http = true
http_port = 8545
enable_ws = true
ws_port = 8546
enable_grpc = true
grpc_port = 50051

[performance]
# Performance tuning
max_connections = 100
max_block_size = 2097152
max_tx_pool_size = 2000
enable_compression = true
compression_level = 6

[security]
# Security settings
enable_firewall = true
allowed_rpc_ips = ["127.0.0.1", "::1"]
rate_limit_per_minute = 1000
```

### Network Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `id` | integer | 1262 | Network identifier |
| `name` | string | "Synergy Testnet" | Human-readable network name |
| `p2p_port` | integer | 30303 | P2P communication port |
| `rpc_port` | integer | 8545 | RPC API port |
| `ws_port` | integer | 8546 | WebSocket port |
| `max_peers` | integer | 50 | Maximum peer connections |
| `bootnodes` | array | [] | Bootstrap node ENR addresses |

### Environment Variable Overrides

```bash
# Network overrides
export SYNERGY_NETWORK_ID=synergy-testnet
export SYNERGY_P2P_PORT=30303
export SYNERGY_RPC_PORT=8545
export SYNERGY_WS_PORT=8546
export SYNERGY_BOOTNODES="enode://node1@ip1:port,enode://node2@ip2:port"
```

---

## ⚙️ Node Configuration

### config/node_config.toml

```toml
[node]
# Node identification
name = "my-synergy-node"
version = "1.0.0"
data_dir = "/var/lib/synergy"

# Logging configuration
[logging]
level = "info"
file = "/var/log/synergy/node.log"
max_size = 10485760
max_files = 5
enable_console = true

# Metrics and monitoring
[metrics]
enabled = true
port = 6060
path = "/metrics"

# Health check
[health]
enabled = true
port = 8080
path = "/health"

# Backup configuration
[backup]
enabled = true
interval_hours = 24
retention_days = 30
path = "/var/lib/synergy/backups"

# Sync configuration
[sync]
fast_sync = true
max_peers = 5
timeout_seconds = 30

# Cache configuration
[cache]
block_cache_size = 134217728
tx_cache_size = 67108864
state_cache_size = 268435456

# Memory pool
[mempool]
max_size = 1000
min_fee_rate = 1
timeout_minutes = 60
```

### Node Configuration Options

#### Core Settings
| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `name` | string | "synergy-node" | Node identifier |
| `version` | string | "1.0.0" | Node version |
| `data_dir` | string | "./data" | Data directory path |

#### Logging Configuration
| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `level` | string | "info" | Log level (trace, debug, info, warn, error) |
| `file` | string | "./data/logs/node.log" | Log file path |
| `max_size` | integer | 10485760 | Max log file size (bytes) |
| `max_files` | integer | 5 | Max number of log files |
| `enable_console` | boolean | true | Enable console logging |

#### Environment Variable Overrides

```bash
# Node configuration overrides
export SYNERGY_NODE_NAME="my-validator-node"
export SYNERGY_LOG_LEVEL="debug"
export SYNERGY_LOG_FILE="/var/log/synergy/node.log"
export SYNERGY_DATA_PATH="/var/lib/synergy/data"
```

---

## 🎯 Validator Configuration

### config/validator/config.toml

```toml
[validator]
# Validator identity
name = "My Validator Node"
website = "https://my-validator.com"
description = "Reliable Synergy Network validator"
email = "validator@example.com"

# Validator keys
private_key_path = "config/validator/private_key.pem"
public_key = "0x..."

# Operational settings
max_block_size = 1048576
max_txs_per_block = 100
enable_metrics = true
metrics_port = 6060

# Performance tuning
batch_size = 10
flush_interval_ms = 1000
enable_parallel_validation = true

# Backup settings
backup_enabled = true
backup_interval_hours = 24
backup_retention_days = 30
backup_path = "/home/synergy/backups"

# Security settings
enable_firewall = true
allowed_ips = ["127.0.0.1", "10.0.0.0/8"]
rate_limit_per_minute = 1000
enable_cors = true
cors_origins = ["*"]

# Monitoring
alert_email = "alerts@example.com"
slack_webhook = "https://hooks.slack.com/..."
telegram_bot_token = "bot_token"
telegram_chat_id = "chat_id"

# Advanced settings
enable_vrf = true
vrf_timeout_ms = 5000
consensus_timeout_ms = 10000
```

### Validator Configuration Options

#### Identity Settings
| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `name` | string | "" | Validator name |
| `website` | string | "" | Validator website |
| `description` | string | "" | Validator description |
| `email` | string | "" | Contact email |

#### Key Management
| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `private_key_path` | string | "" | Path to private key file |
| `public_key` | string | "" | Public key (auto-generated if empty) |

#### Operational Settings
| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `max_block_size` | integer | 1048576 | Maximum block size in bytes |
| `max_txs_per_block` | integer | 100 | Maximum transactions per block |
| `enable_metrics` | boolean | true | Enable Prometheus metrics |
| `metrics_port` | integer | 6060 | Metrics server port |

---

## 🏗️ Genesis Configuration

### config/genesis.json

The genesis file defines the initial state of the blockchain:

```json
{
  "meta": {
    "network": "Synergy Testnet",
    "version": "1.0.0",
    "description": "Synergy Network Testnet Genesis Block",
    "dateGenerated": "2024-01-01T00:00:00Z"
  },
  "config": {
    "chainId": 1262,
    "synergyConsensus": {
      "algorithm": "Proof of Synergy",
      "parameters": {
        "blockTime": 5,
        "epoch": 30000,
        "validatorClusterSize": 7,
        "synergyScoreDecayRate": 0.05,
        "vrfEnabled": true,
        "maxSynergyPointsPerEpoch": 100,
        "maxTasksPerValidator": 10,
        "rewardWeighting": {
          "taskAccuracy": 0.5,
          "uptime": 0.3,
          "collaboration": 0.2
        }
      }
    }
  },
  "difficulty": "0x1",
  "gasLimit": "0x47b760",
  "alloc": {
    "synq1zxy8qhj4j59xp5lwkwpd5qws9aygz8pl9m3kmjx3": {
      "balance": "1000000000000000000000"
    }
  },
  "validators": {
    "initialValidators": [
      {
        "address": "synq1ffzcyq7l0sw7v9fhrx2wdvxxzv9q5mj3ehd6yl3e",
        "pubKey": "0x...",
        "weight": 1
      }
    ]
  }
}
```

### Genesis Parameters

#### Consensus Parameters
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `blockTime` | integer | 5 | Block time in seconds |
| `epoch` | integer | 30000 | Epoch length in blocks |
| `validatorClusterSize` | integer | 7 | Validators per cluster |
| `synergyScoreDecayRate` | float | 0.05 | Score decay per epoch |
| `vrfEnabled` | boolean | true | Enable VRF for selection |
| `maxSynergyPointsPerEpoch` | integer | 100 | Max points per epoch |
| `maxTasksPerValidator` | integer | 10 | Max tasks per validator |

#### Reward Weighting
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `taskAccuracy` | float | 0.5 | Weight for task accuracy |
| `uptime` | float | 0.3 | Weight for uptime |
| `collaboration` | float | 0.2 | Weight for collaboration |

---

## 🔧 Runtime Configuration

### Environment Variables

The node supports extensive runtime configuration through environment variables:

#### Network Configuration
```bash
export SYNERGY_NETWORK_ID=synergy-testnet
export SYNERGY_CHAIN_ID=1262
export SYNERGY_P2P_PORT=30303
export SYNERGY_RPC_PORT=8545
export SYNERGY_WS_PORT=8546
export SYNERGY_MAX_PEERS=50
export SYNERGY_BOOTNODES="enode://node1@ip:port,enode://node2@ip:port"
```

#### Logging Configuration
```bash
export SYNERGY_LOG_LEVEL=info
export SYNERGY_LOG_FILE=/var/log/synergy/node.log
export SYNERGY_LOG_MAX_SIZE=10485760
export SYNERGY_LOG_MAX_FILES=5
```

#### Performance Configuration
```bash
export SYNERGY_MAX_BLOCK_SIZE=2097152
export SYNERGY_MAX_TX_POOL_SIZE=2000
export SYNERGY_CACHE_SIZE=134217728
export SYNERGY_ENABLE_COMPRESSION=true
```

#### Security Configuration
```bash
export SYNERGY_ALLOWED_IPS="127.0.0.1,10.0.0.0/8"
export SYNERGY_RATE_LIMIT=1000
export SYNERGY_ENABLE_CORS=true
```

#### Storage Configuration
```bash
export SYNERGY_DATA_PATH=/var/lib/synergy/data
export SYNERGY_DB_TYPE=rocksdb
export SYNERGY_ENABLE_PRUNING=true
```

#### Validator Configuration
```bash
export SYNERGY_VALIDATOR_NAME="My Validator"
export SYNERGY_VALIDATOR_WEBSITE="https://my-validator.com"
export SYNERGY_PRIVATE_KEY_PATH=/path/to/private/key
export SYNERGY_ENABLE_METRICS=true
```

### Configuration Priority

The node loads configuration in this order:

1. **Command-line arguments** (if supported)
2. **Environment variables**
3. **Configuration files** (TOML/JSON)
4. **Built-in defaults**

Later sources override earlier ones, allowing flexible customization.

---

## 📊 Advanced Configuration

### Performance Tuning

#### Memory Configuration
```toml
[performance.memory]
# Memory pool sizes
block_pool_size = 1000
transaction_pool_size = 5000
state_cache_size = 268435456

# Garbage collection
gc_interval_seconds = 300
gc_target_memory_mb = 1024
```

#### Network Tuning
```toml
[performance.network]
# Connection settings
max_inbound_connections = 50
max_outbound_connections = 25
connection_timeout_seconds = 30

# Bandwidth management
max_upload_speed_mbps = 100
max_download_speed_mbps = 100
```

#### Consensus Tuning
```toml
[consensus.advanced]
# Block production
max_block_time_drift_ms = 1000
min_validator_stake = 1000
max_missed_blocks = 10

# VRF settings
vrf_difficulty = "0x1"
vrf_timeout_ms = 5000
```

### Security Hardening

#### Firewall Configuration
```toml
[security.firewall]
enabled = true
default_policy = "drop"

[security.firewall.rules]
# Allow local access
"lo" = "accept"
"127.0.0.1" = "accept"
"::1" = "accept"

# Allow validator network
"10.0.0.0/8" = "accept"
"192.168.0.0/16" = "accept"

# Block everything else
"0.0.0.0/0" = "drop"
```

#### Access Control
```toml
[security.acl]
# RPC access control
enable_ip_whitelist = true
allowed_ips = ["127.0.0.1", "10.0.0.0/8"]
blocked_ips = ["192.168.1.100"]

# Rate limiting
requests_per_minute = 1000
burst_limit = 100

# Authentication
require_auth = false
auth_token = ""
```

### Monitoring Configuration

#### Metrics Collection
```toml
[monitoring]
enabled = true
interval_seconds = 15

[monitoring.exporters]
prometheus_enabled = true
prometheus_port = 6030

influxdb_enabled = false
influxdb_url = "http://localhost:8086"
influxdb_database = "synergy_metrics"

json_file_enabled = true
json_file_path = "/var/lib/synergy/metrics.json"
```

#### Alerting Rules
```toml
[alerts]
enabled = true
check_interval_minutes = 5

[alerts.rules]
# Node health
node_down_duration = "5m"
high_memory_usage = 90
high_cpu_usage = 80

# Consensus issues
missed_blocks_threshold = 5
peer_count_low = 3

# Performance
slow_block_time = 10
high_pending_txs = 1000
```

---

## 🔧 Configuration Management

### Validation

The node validates configuration on startup:

```bash
# Validate configuration
cargo run --release -- config validate

# Check for deprecated options
cargo run --release -- config check-deprecated

# Generate example configuration
cargo run --release -- config generate-example > config/example.toml
```

### Hot Reloading

Some configuration options support hot reloading:

```toml
[hot_reload]
enabled = true
check_interval_seconds = 60

# Reloadable sections
log_level = true
metrics_enabled = true
cache_sizes = false
network_ports = false
```

### Configuration Profiles

Use different profiles for different environments:

```bash
# Development profile
export SYNERGY_PROFILE=development
# Uses config/development.toml

# Production profile
export SYNERGY_PROFILE=production
# Uses config/production.toml

# Test profile
export SYNERGY_PROFILE=test
# Uses config/test.toml
```

### Backup and Migration

```bash
# Create configuration backup
cp -r config/ config.backup.$(date +%Y%m%d_%H%M%S)

# Migrate configuration to new version
cargo run --release -- config migrate --from-version 1.0 --to-version 1.1

# Validate migration
cargo run --release -- config validate --file config/network-config.toml
```

---

## 🚨 Troubleshooting Configuration

### Common Issues

#### Port Conflicts
```bash
# Check port usage
netstat -tlnp | grep 30303
ss -tlnp | grep 8545

# Find process using port
lsof -i :30303
fuser 8545/tcp
```

#### Permission Issues
```bash
# Fix data directory permissions
sudo chown -R synergy:synergy /var/lib/synergy/
sudo chmod -R 755 /var/lib/synergy/

# Check log file permissions
ls -la /var/log/synergy/
```

#### Configuration Syntax Errors
```bash
# Validate TOML syntax
python3 -c "import toml; toml.load('config/network-config.toml')"

# Check JSON syntax
python3 -c "import json; json.load(open('config/genesis.json'))"
```

### Debug Configuration

```bash
# Enable debug logging
export SYNERGY_LOG_LEVEL=debug

# Show configuration loading
export SYNERGY_DEBUG_CONFIG=true

# Validate all configuration files
find config/ -name "*.toml" -o -name "*.json" | xargs -I {} sh -c 'echo "Validating {}"; python3 -c "import toml, json; (toml.load if _.endswith(\".toml\") else json.load)(open(_))" _ {} || echo "Error in {}"'
```

---

## 📚 Examples

### Minimal Configuration

```toml
# config/network-config.toml
[network]
id = 1262
p2p_port = 30303
rpc_port = 8545

[blockchain]
block_time = 5
chain_id = 1262
```

### Production Configuration

```toml
# config/production.toml
[node]
name = "prod-validator-01"
data_dir = "/opt/synergy/data"

[logging]
level = "warn"
file = "/var/log/synergy/node.log"
max_size = 104857600
max_files = 10

[security]
enable_firewall = true
allowed_ips = ["10.0.0.0/8", "172.16.0.0/12"]
rate_limit_per_minute = 100

[monitoring]
enabled = true
interval_seconds = 10

[alerts]
enabled = true
alert_email = "ops@company.com"
```

### Development Configuration

```toml
# config/development.toml
[node]
name = "dev-node"
data_dir = "./data"

[logging]
level = "debug"
enable_console = true

[network]
max_peers = 5

[performance]
max_connections = 10
enable_compression = false
```

---

## 🔗 Related Documentation

- [Setup Guide](setup-guide.md) - Step-by-step installation
- [Validator Guide](validator-guide.md) - Validator-specific setup
- [API Reference](api-reference.md) - RPC API documentation
- [Troubleshooting Guide](troubleshooting.md) - Common issues and solutions

---

*For configuration-related issues, check the [troubleshooting guide](troubleshooting.md) or join our [Discord community](https://discord.gg/synergy).*
