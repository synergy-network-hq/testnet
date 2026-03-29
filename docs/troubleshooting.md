# Synergy Network Troubleshooting Guide

## 🚨 Quick Reference

### Common Issues by Category

| Issue | Symptoms | Quick Fix |
|-------|----------|-----------|
| **Node won't start** | Process exits immediately | Check logs, ports, permissions |
| **Sync issues** | Stuck at same block height | Restart sync, check peers |
| **High resource usage** | CPU/memory spikes | Adjust cache sizes, check for leaks |
| **Network issues** | Cannot connect to peers | Check firewall, ports, bootnodes |
| **Transaction failures** | Txs rejected or stuck | Check validation, gas, nonce |
| **Performance issues** | Slow block production | Tune configuration, check hardware |

---

## 🔍 Diagnostic Commands

### System Health Check
```bash
# Check if node is running
sudo systemctl status synergy-validator
ps aux | grep synergy-testbeta

# Check resource usage
htop
df -h
free -h

# Check network connectivity
ping 8.8.8.8
netstat -tlnp | grep 30303

# Check logs
sudo journalctl -u synergy-validator -f --since "1 hour ago"
tail -f /var/log/synergy/node.log
```

### Network Diagnostics
```bash
# Test RPC connectivity
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"synergy_nodeInfo","id":1}' \
  http://localhost:8545

# Check peer connections
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"synergy_peerCount","id":1}' \
  http://localhost:8545

# Test WebSocket connection
websocat ws://localhost:8546
```

### Blockchain State Check
```bash
# Check latest block
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"synergy_getLatestBlock","id":1}' \
  http://localhost:8545

# Check transaction pool
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"synergy_getTransactionPool","id":1}' \
  http://localhost:8545

# Check sync status
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"synergy_syncing","id":1}' \
  http://localhost:8545
```

---

## 🛠️ Node Startup Issues

### Node Won't Start

**Symptoms**: Process exits immediately or fails to start

**Common Causes and Solutions**:

1. **Port Conflicts**
   ```bash
   # Check what's using the ports
   sudo netstat -tlnp | grep 30303
   sudo lsof -i :30303
   sudo lsof -i :8545

   # Kill conflicting processes
   sudo kill -9 <PID>
   ```

2. **Permission Issues**
   ```bash
   # Check data directory permissions
   ls -la /var/lib/synergy/
   sudo chown -R synergy:synergy /var/lib/synergy/
   sudo chmod -R 755 /var/lib/synergy/

   # Check log directory
   sudo mkdir -p /var/log/synergy
   sudo chown synergy:synergy /var/log/synergy
   ```

3. **Missing Dependencies**
   ```bash
   # Check Rust installation
   rustc --version
   cargo --version

   # Check system dependencies
   sudo apt install -y build-essential libssl-dev pkg-config
   ```

4. **Configuration Errors**
   ```bash
   # Validate configuration files
   python3 -c "import toml, json; toml.load(open('config/network-config.toml'))"
   python3 -c "import json; json.load(open('config/genesis.json'))"

   # Check for syntax errors
   cargo run --release -- config validate
   ```

5. **Resource Limits**
   ```bash
   # Check system limits
   ulimit -n  # Should be at least 65536
   ulimit -u  # Should be at least 4096

   # Increase limits if needed
   sudo tee /etc/security/limits.d/synergy.conf > /dev/null <<EOF
   synergy soft nofile 65536
   synergy hard nofile 65536
   synergy soft nproc 4096
   synergy hard nproc 4096
   EOF
   ```

### Node Crashes

**Symptoms**: Node starts but crashes after some time

**Debug Steps**:

1. **Enable Debug Logging**
   ```bash
   export SYNERGY_LOG_LEVEL=debug
   cargo run --release -- start
   ```

2. **Check System Resources**
   ```bash
   # Monitor memory usage
   watch -n 1 'ps aux | grep synergy-testbeta'

   # Check for memory leaks
   valgrind --tool=memcheck ./target/release/synergy-testbeta start
   ```

3. **Database Corruption**
   ```bash
   # Stop the node
   sudo systemctl stop synergy-validator

   # Backup current data
   cp -r /var/lib/synergy/data /var/lib/synergy/data.backup

   # Remove potentially corrupted files
   rm -rf /var/lib/synergy/data/chain/*
   rm /var/lib/synergy/data/chain.json

   # Restart
   sudo systemctl start synergy-validator
   ```

---

## 🌐 Network and Connectivity Issues

### Cannot Connect to Peers

**Symptoms**: Node shows 0 peers or cannot sync

**Troubleshooting Steps**:

1. **Check Firewall**
   ```bash
   # Check UFW status
   sudo ufw status

   # Allow required ports
   sudo ufw allow 30303/tcp
   sudo ufw allow 8545/tcp
   sudo ufw allow 8546/tcp

   # Check if ports are open
   sudo netstat -tlnp | grep 30303
   ```

2. **Test Network Connectivity**
   ```bash
   # Test external connectivity
   ping 8.8.8.8
   curl -I httpbin.org

   # Test P2P port
   telnet localhost 30303
   ```

3. **Check Bootnodes**
   ```bash
   # Verify bootnode configuration
   grep bootnodes config/network-config.toml

   # Test bootnode connectivity
   nmap -p 30303 <bootnode-ip>
   ```

4. **Network Configuration**
   ```bash
   # Check network interfaces
   ip addr show
   ip route show

   # Test NAT/port forwarding
   curl ifconfig.me
   ```

### Control panel RPC timeout

**Symptoms**: The validator control panel reports `Connection to https://testbeta-core-rpc.synergy-network.io/rpc timed out` when you try to register a validator. The bundled `.env` (see `archive/ENV_FILE_REVIEW.md` under the `SYNERGY_RPC_ENDPOINT` block) points at the upstream RPC proxy, so the control panel immediately tries to speak to that host.

**Diagnosis**: The upstream gateway is currently unresponsive. Run the same RPC request manually and watch it hang:

```bash
curl -s -X POST https://testbeta-core-rpc.synergy-network.io/rpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"synergy_blockNumber","params":[],"id":1}'
```

If this command hangs for more than a few seconds (our test timed out after 120s), the remote endpoint cannot service requests, which is why the control panel wraps the error as a timeout.

**Workaround**:

1. Start a local RPC/bootnode before launching the control panel so that `/rpc` is available on `localhost:5730`. The repo already provides `scripts/reset-testbeta.sh` (runs `archive/start-bootnodes.sh` and prepares `data/chain`) or `./archive/start-bootnodes.sh start`.
2. Point the control panel at your node instead of the remote proxy. Set the environment overrides before starting the UI (or paste them into your `.env`):

```bash
export SYNERGY_RPC_ENDPOINT=http://localhost:5730/rpc
export SYNERGY_WS_ENDPOINT=ws://localhost:5830
```

3. Restart the control panel after updating the variables so it reconnects to the local RPC.

When the upstream proxy is healthy again, you can switch the variables back to `https://testbeta-core-rpc.synergy-network.io` and `wss://testbeta-core-ws.synergy-network.io`.

4. If the control panel runs on remote operator devices, make sure they resolve `testbeta-core-rpc.synergy-network.io` and `testbeta-core-ws.synergy-network.io` to this machine and can reach the proxied public surfaces. Do not change the control panel to `localhost` from those devices—the RPC endpoint must point to this host over the network (proxy or firewall rules permitting) so that registrations come through the real testnet-beta node you're running locally.

### Sync Issues

**Symptoms**: Node stuck at same block height or slow sync

**Solutions**:

1. **Fast Sync Issues**
   ```bash
   # Restart with fresh sync
   sudo systemctl stop synergy-validator
   rm -rf /var/lib/synergy/data/chain/*
   sudo systemctl start synergy-validator
   ```

2. **Peer Discovery Problems**
   ```bash
   # Check peer count
   curl -X POST -H "Content-Type: application/json" \
     --data '{"jsonrpc":"2.0","method":"synergy_peerCount","id":1}' \
     http://localhost:8545

   # Add more bootnodes
   # Edit config/network-config.toml
   ```

3. **Network Congestion**
   ```bash
   # Limit peer connections
   export SYNERGY_MAX_PEERS=5

   # Increase timeouts
   export SYNERGY_SYNC_TIMEOUT=60
   ```

---

## 💰 Transaction Issues

### Transactions Rejected

**Symptoms**: Transactions fail with validation errors

**Common Causes**:

1. **Invalid Transaction Format**
   ```bash
   # Check transaction structure
   curl -X POST -H "Content-Type: application/json" \
     --data '{
       "jsonrpc":"2.0",
       "method":"synergy_sendTransaction",
       "params":[{
         "sender":"synq1...",
         "receiver":"synu1...",
         "amount":1000,
         "nonce":1,
         "signature":"sig",
         "gas_price":1,
         "gas_limit":21000
       }],
       "id":1
     }' \
     http://localhost:8545
   ```

2. **Insufficient Balance**
   ```bash
   # Check sender balance (if implemented)
   # This would require additional RPC methods
   ```

3. **Nonce Issues**
   ```bash
   # Check current nonce for address
   # This would require additional RPC methods

   # Fix nonce in transaction
   # Ensure nonce matches account state
   ```

4. **Gas Issues**
   ```bash
   # Estimate gas for transaction
   curl -X POST -H "Content-Type: application/json" \
     --data '{"jsonrpc":"2.0","method":"synergy_estimateGas","params":[{...}],"id":1}' \
     http://localhost:8545

   # Adjust gas price/limit
   ```

### Transaction Pool Issues

**Symptoms**: Transactions stuck in pool or not processing

**Debug Steps**:

1. **Check Pool Status**
   ```bash
   # Get pool contents
   curl -X POST -H "Content-Type: application/json" \
     --data '{"jsonrpc":"2.0","method":"synergy_getTransactionPool","id":1}' \
     http://localhost:8545
   ```

2. **Pool Configuration**
   ```bash
   # Check pool settings
   grep -A 10 "mempool" config/node_config.toml

   # Adjust pool size if needed
   export SYNERGY_MAX_TX_POOL_SIZE=2000
   ```

3. **Validator Issues**
   ```bash
   # Check if validator is active
   curl -X POST -H "Content-Type: application/json" \
     --data '{"jsonrpc":"2.0","method":"synergy_getValidators","id":1}' \
     http://localhost:8545
   ```

---

## ⚡ Performance Issues

### High CPU Usage

**Symptoms**: Node consuming excessive CPU resources

**Solutions**:

1. **Check for Infinite Loops**
   ```bash
   # Enable debug logging
   export SYNERGY_LOG_LEVEL=debug

   # Monitor specific threads
   top -H -p $(pgrep synergy-testbeta)
   ```

2. **Tune Cache Sizes**
   ```toml
   # Reduce cache sizes in config
   [cache]
   block_cache_size = 67108864    # 64MB
   tx_cache_size = 33554432       # 32MB
   state_cache_size = 134217728   # 128MB
   ```

3. **Limit Peer Connections**
   ```bash
   export SYNERGY_MAX_PEERS=10
   export SYNERGY_MAX_CONNECTIONS=20
   ```

### High Memory Usage

**Symptoms**: Node using excessive RAM

**Debug Steps**:

1. **Memory Profiling**
   ```bash
   # Check memory usage
   ps aux | grep synergy-testbeta

   # Use heap profiling (if enabled)
   export HEAP_PROFILE=true
   ```

2. **Database Memory**
   ```bash
   # Check RocksDB memory usage
   ls -lh /var/lib/synergy/data/chain/

   # Reduce memory maps
   export SYNERGY_DB_MAX_OPEN_FILES=1000
   ```

3. **Cache Optimization**
   ```toml
   # Reduce cache sizes
   [cache]
   block_cache_size = 33554432
   tx_cache_size = 16777216
   state_cache_size = 67108864
   ```

### Slow Block Production

**Symptoms**: Blocks taking longer than expected to produce

**Solutions**:

1. **Check System Resources**
   ```bash
   # Monitor CPU and I/O
   iostat -x 1
   vmstat 1
   ```

2. **Validator Performance**
   ```bash
   # Check validator status
   curl -X POST -H "Content-Type: application/json" \
     --data '{"jsonrpc":"2.0","method":"synergy_getValidators","id":1}' \
     http://localhost:8545
   ```

3. **Network Latency**
   ```bash
   # Test peer latency
   ping <peer-ip>
   traceroute <peer-ip>
   ```

---

## 🔒 Security Issues

### Unauthorized Access

**Symptoms**: Suspicious connection attempts or unauthorized RPC calls

**Security Measures**:

1. **Firewall Hardening**
   ```bash
   # Restrict RPC access
   sudo ufw allow from 192.168.1.0/24 to any port 8545
   sudo ufw allow from 10.0.0.0/8 to any port 8545

   # Block everything else
   sudo ufw default deny
   ```

2. **Access Logging**
   ```bash
   # Monitor access logs
   sudo tail -f /var/log/ufw.log
   sudo tail -f /var/log/auth.log
   ```

3. **Rate Limiting**
   ```toml
   [security]
   rate_limit_per_minute = 100
   burst_limit = 10
   ```

### Key Compromise

**Symptoms**: Validator producing invalid blocks or acting suspiciously

**Recovery Steps**:

1. **Immediate Shutdown**
   ```bash
   sudo systemctl stop synergy-validator
   ```

2. **Key Rotation**
   ```bash
   # Generate new keys
   openssl ecparam -name prime256v1 -genkey -noout -out new_validator_key.pem
   openssl ec -in new_validator_key.pem -pubout -out new_validator_pub.pem

   # Update configuration
   # Replace old keys with new ones
   ```

3. **Contact Network**
   ```bash
   # Report the incident
   # Follow network governance procedures
   ```

---

## 📊 Monitoring and Alerting

### Setting Up Monitoring

```bash
# Install monitoring tools
sudo apt install -y prometheus-node-exporter grafana

# Configure Prometheus
sudo tee /etc/prometheus/prometheus.yml > /dev/null <<EOF
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'synergy-node'
    static_configs:
      - targets: ['localhost:6060']
EOF

# Start services
sudo systemctl enable prometheus
sudo systemctl start prometheus
sudo systemctl enable grafana-server
sudo systemctl start grafana-server
```

### Important Metrics to Monitor

```bash
# Block height and sync status
curl -s http://localhost:6060/metrics | grep synergy_block_height
curl -s http://localhost:6060/metrics | grep synergy_syncing

# Peer connections
curl -s http://localhost:6060/metrics | grep synergy_peer_count

# Transaction pool
curl -s http://localhost:6060/metrics | grep synergy_pending_txs

# Resource usage
curl -s http://localhost:6060/metrics | grep synergy_memory_usage
curl -s http://localhost:6060/metrics | grep synergy_cpu_usage
```

### Alerting Setup

```yaml
# Example Prometheus alerting rules
groups:
  - name: synergy
    rules:
      - alert: NodeDown
        expr: up{job="synergy-node"} == 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Synergy node is down"
          description: "Synergy node has been down for more than 5 minutes"

      - alert: HighMemoryUsage
        expr: synergy_memory_usage > 90
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High memory usage"
          description: "Memory usage is above 90% for more than 10 minutes"
```

---

## 🗄️ Database Issues

### Database Corruption

**Symptoms**: Node crashes with database errors

**Recovery**:

1. **Backup First**
   ```bash
   cp -r /var/lib/synergy/data /var/lib/synergy/data.backup.$(date +%Y%m%d_%H%M%S)
   ```

2. **Repair Database**
   ```bash
   # Stop the node
   sudo systemctl stop synergy-validator

   # Remove corrupted data
   rm -rf /var/lib/synergy/data/chain/*

   # Restart (will rebuild from genesis)
   sudo systemctl start synergy-validator
   ```

3. **Alternative: Manual Recovery**
   ```bash
   # Use RocksDB tools if available
   rocksdb_dump --db=/var/lib/synergy/data/chain/ > dump.txt
   rocksdb_repair --db=/var/lib/synergy/data/chain/
   ```

### Storage Space Issues

**Symptoms**: Disk full errors or slow performance

**Solutions**:

1. **Check Disk Usage**
   ```bash
   df -h
   du -sh /var/lib/synergy/data/
   ```

2. **Clean Old Data**
   ```bash
   # Remove old log files
   find /var/log/synergy/ -name "*.log" -type f -mtime +7 -delete

   # Enable pruning (if configured)
   export SYNERGY_ENABLE_PRUNING=true
   export SYNERGY_PRUNING_INTERVAL=3600
   ```

3. **Move Data Directory**
   ```bash
   # Stop node
   sudo systemctl stop synergy-validator

   # Move data to larger disk
   sudo mkdir -p /mnt/large_disk/synergy
   sudo cp -r /var/lib/synergy/data/* /mnt/large_disk/synergy/
   sudo rm -rf /var/lib/synergy/data/*

   # Update configuration
   # Change data_dir in config
   export SYNERGY_DATA_PATH=/mnt/large_disk/synergy

   # Restart
   sudo systemctl start synergy-validator
   ```

---

## 🔄 Backup and Recovery

### Creating Backups

```bash
#!/bin/bash
# backup-synergy.sh

BACKUP_DIR="/mnt/backup/synergy"
DATE=$(date +%Y%m%d_%H%M%S)

# Create backup directory
mkdir -p $BACKUP_DIR

# Stop node for consistent backup
sudo systemctl stop synergy-validator

# Backup data
cp -r /var/lib/synergy/data $BACKUP_DIR/data_$DATE
cp -r /var/lib/synergy/config $BACKUP_DIR/config_$DATE

# Restart node
sudo systemctl start synergy-validator

# Compress and cleanup
cd $BACKUP_DIR
tar -czf synergy_backup_$DATE.tar.gz data_$DATE/ config_$DATE/
rm -rf data_$DATE/ config_$DATE/

# Keep only last 7 days
find $BACKUP_DIR -name "synergy_backup_*.tar.gz" -type f -mtime +7 -delete

echo "Backup completed: synergy_backup_$DATE.tar.gz"
```

### Restoring from Backup

```bash
# Stop node
sudo systemctl stop synergy-validator

# Extract backup
cd /tmp
tar -xzf /mnt/backup/synergy/synergy_backup_20240101_120000.tar.gz

# Restore data
sudo cp -r data_20240101_120000/* /var/lib/synergy/data/
sudo cp -r config_20240101_120000/* /var/lib/synergy/config/

# Fix permissions
sudo chown -R synergy:synergy /var/lib/synergy/
sudo chmod -R 755 /var/lib/synergy/

# Start node
sudo systemctl start synergy-validator
```

---

## 📞 Getting Help

### Community Support

1. **Discord**: [Synergy Network Discord](https://discord.gg/synergy)
   - #troubleshooting channel
   - #validator-support channel
   - #dev-discussion channel

2. **GitHub Issues**: [Report bugs](https://github.com/synergy-network-hq/testnet-beta/issues)
   - Use issue templates
   - Provide detailed information
   - Include logs and configuration

3. **Documentation**: [Complete docs](docs/)
   - Check FAQ section
   - Review configuration examples
   - Follow setup guides

### Information to Include in Support Requests

When asking for help, please include:

1. **Node Version**: `synergy-testbeta --version`
2. **Operating System**: `uname -a`
3. **Configuration**: Relevant config files (without sensitive data)
4. **Logs**: Last 50-100 lines of logs
5. **Error Messages**: Exact error text
6. **Steps to Reproduce**: How to trigger the issue
7. **Hardware Specs**: CPU, RAM, disk space
8. **Network Setup**: Firewall rules, port configuration

### Professional Support

For enterprise deployments:

- **Email**: support@synergy.network
- **Priority Support**: Available for validator operators
- **On-site Consulting**: For large deployments
- **Custom Development**: For specialized requirements

---

## 📋 Checklists

### Pre-Launch Checklist

- [ ] Hardware requirements met
- [ ] All dependencies installed
- [ ] Configuration files validated
- [ ] Ports are open and accessible
- [ ] Firewall configured correctly
- [ ] Validator keys generated and secured
- [ ] Backup strategy in place
- [ ] Monitoring configured
- [ ] Test transaction submitted successfully

### Daily Operations Checklist

- [ ] Check node status and logs
- [ ] Verify peer connections
- [ ] Monitor resource usage
- [ ] Check block production
- [ ] Review security logs
- [ ] Verify backups
- [ ] Update software if needed

### Troubleshooting Checklist

- [ ] Check service status
- [ ] Review recent logs
- [ ] Test RPC connectivity
- [ ] Verify network connectivity
- [ ] Check system resources
- [ ] Validate configuration
- [ ] Test with minimal setup
- [ ] Check for known issues

---

*Remember: When in doubt, check the logs first! Most issues can be diagnosed from the log output.*

*For urgent issues affecting network operations, contact the core development team immediately.*
