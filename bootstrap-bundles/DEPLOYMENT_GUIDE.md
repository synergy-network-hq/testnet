# Bootstrap Node & Seed Server Deployment Guide

This guide explains how to deploy and manage the updated Synergy Testnet-Beta bootstrap nodes and seed servers across three machines.

---

## Infrastructure Overview

| Machine | Hostname | IP Address | OS | Services |
|---------|----------|------------|-----|----------|
| **Machine 1** | bootnode1.synergynode.xyz<br>seed1.synergynode.xyz | 74.208.227.23 | Ubuntu | Bootnode + Seed Server |
| **Machine 2** | bootnode2.synergynode.xyz<br>seed2.synergynode.xyz | 73.79.66.255 | macOS (Mac Mini) | Bootnode + Seed Server |
| **Machine 3** | bootnode3.synergynode.xyz<br>seed3.synergynode.xyz | 64.227.107.57 | Ubuntu | Bootnode + Seed Server |

---

## Prerequisites

### For All Machines

1. **SSH Access**: Ensure you can SSH into each machine:
   ```bash
   # Machine 1 (Ubuntu)
   ssh root@74.208.227.23

   # Machine 2 (macOS)
   ssh synergynode@73.79.66.255

   # Machine 3 (Ubuntu)
   ssh root@64.227.107.57
   ```

2. **Python 3**: Required for seed services (usually pre-installed)
   ```bash
   python3 --version  # Should be 3.8+
   ```

3. **Directory Structure**: Each machine should have:
   - `~/bootnodeX/` - Bootstrap node files
   - `~/seedX/` - Seed server files

---

## Part 1: Deploy Updated Code to All Machines

### Step 1.1: Transfer Bootstrap Bundles

From your local machine, copy the updated bundles to all three servers:

```bash
# Set local source path
SOURCE_PATH="/Users/devpup/Desktop/Testnet-Beta/synergy-testnet-beta/bootstrap-bundles"

# Machine 1 (Ubuntu - bootnode1 + seed1)
scp -r "$SOURCE_PATH/bootnode1" root@74.208.227.23:~/bootnode1
scp -r "$SOURCE_PATH/seed1" root@74.208.227.23:~/seed1

# Machine 2 (macOS - bootnode2 + seed2)
scp -r "$SOURCE_PATH/bootnode2" synergynode@73.79.66.255:~/bootnode2
scp -r "$SOURCE_PATH/seed2" synergynode@73.79.66.255:~/seed2

# Machine 3 (Ubuntu - bootnode3 + seed3)
scp -r "$SOURCE_PATH/bootnode3" root@64.227.107.57:~/bootnode3
scp -r "$SOURCE_PATH/seed3" root@64.227.107.57:~/seed3
```

---

## Part 2: Install and Start Services on Each Machine

### Machine 1: Ubuntu (74.208.227.23)

#### Step 2.1.1: Install and Start Bootnode1

```bash
# SSH into machine
ssh root@74.208.227.23

# Navigate to bootnode directory
cd ~/bootnode1

# Make scripts executable
chmod +x install_and_start.sh nodectl.sh

# Install and start the bootnode
./install_and_start.sh

# Verify it's running
./nodectl.sh status
./nodectl.sh logs --follow
```

#### Step 2.1.2: Install and Start Seed1

```bash
# In a new terminal or background
cd ~/seed1

# Make scripts executable
chmod +x install_and_start.sh nodectl.sh

# Install and start the seed service
./install_and_start.sh

# Verify it's running
./nodectl.sh status
./nodectl.sh logs --follow
```

---

### Machine 2: macOS (73.79.66.255)

#### Step 2.2.1: Install and Start Bootnode2

```bash
# SSH into machine
ssh synergynode@73.79.66.255

# Navigate to bootnode directory
cd ~/bootnode2

# Make scripts executable
chmod +x install_and_start.sh nodectl.sh

# Clear macOS quarantine attribute (if needed)
xattr -dr com.apple.quarantine .

# Install and start the bootnode
./install_and_start.sh

# Verify it's running
./nodectl.sh status
./nodectl.sh logs --follow
```

#### Step 2.2.2: Install and Start Seed2

```bash
# In a new terminal or background
cd ~/seed2

# Make scripts executable
chmod +x install_and_start.sh nodectl.sh

# Clear macOS quarantine attribute (if needed)
xattr -dr com.apple.quarantine .

# Install and start the seed service
./install_and_start.sh

# Verify it's running
./nodectl.sh status
./nodectl.sh logs --follow
```

---

### Machine 3: Ubuntu (64.227.107.57)

#### Step 2.3.1: Install and Start Bootnode3

```bash
# SSH into machine
ssh root@64.227.107.57

# Navigate to bootnode directory
cd ~/bootnode3

# Make scripts executable
chmod +x install_and_start.sh nodectl.sh

# Install and start the bootnode
./install_and_start.sh

# Verify it's running
./nodectl.sh status
./nodectl.sh logs --follow
```

#### Step 2.3.2: Install and Start Seed3

```bash
# In a new terminal or background
cd ~/seed3

# Make scripts executable
chmod +x install_and_start.sh nodectl.sh

# Install and start the seed service
./install_and_start.sh

# Verify it's running
./nodectl.sh status
./nodectl.sh logs --follow
```

---

## Part 3: Set Up Persistent Services (Auto-Restart)

### For Ubuntu Machines (Machine 1 & Machine 3)

Create systemd service units for automatic restart on failure or system reboot.

#### Step 3.1: Create Bootnode Systemd Service

Run this on **both Ubuntu machines** (74.208.227.23 and 64.227.107.57), adjusting the `MACHINE_ID`:

```bash
# Create systemd service file for bootnode
sudo tee /etc/systemd/system/synergy-bootnode.service > /dev/null <<'EOF'
[Unit]
Description=Synergy Testnet-Beta Bootstrap Node
After=network.target
StartLimitIntervalSec=0

[Service]
Type=simple
Restart=always
RestartSec=10
User=root
WorkingDirectory=/root/bootnode1
ExecStart=/root/bootnode1/install_and_start.sh
ExecStop=/root/bootnode1/nodectl.sh stop
PIDFile=/root/bootnode1/data/node.pid
StandardOutput=append:/root/bootnode1/data/logs/systemd.out
StandardError=append:/root/bootnode1/data/logs/systemd.err

# Restart on failure
RestartSec=5
StartLimitBurst=5

# Environment
Environment="SYNERGY_BOOTSTRAP_ONLY=true"
Environment="SYNERGY_AUTO_REGISTER_VALIDATOR=false"

[Install]
WantedBy=multi-user.target
EOF
```

**Important**: On Machine 3 (64.227.107.57), change the paths:
```bash
# On Machine 3 only, use these paths instead:
# WorkingDirectory=/root/bootnode3
# ExecStart=/root/bootnode3/install_and_start.sh
# ExecStop=/root/bootnode3/nodectl.sh stop
# PIDFile=/root/bootnode3/data/node.pid
# StandardOutput=append:/root/bootnode3/data/logs/systemd.out
# StandardError=append:/root/bootnode3/data/logs/systemd.err
```

#### Step 3.2: Create Seed Service Systemd Unit

```bash
# Create systemd service file for seed service
sudo tee /etc/systemd/system/synergy-seed.service > /dev/null <<'EOF'
[Unit]
Description=Synergy Testnet-Beta Seed Server
After=network.target
StartLimitIntervalSec=0

[Service]
Type=simple
Restart=always
RestartSec=10
User=root
WorkingDirectory=/root/seed1
ExecStart=/usr/bin/python3 /root/seed1/seed_service.py
ExecStop=/root/seed1/nodectl.sh stop
PIDFile=/root/seed1/data/seed.pid
StandardOutput=append:/root/seed1/data/logs/systemd.out
StandardError=append:/root/seed1/data/logs/systemd.err

# Restart on failure
RestartSec=5
StartLimitBurst=5

# Peer TTL configuration (10 minutes default)
Environment="SEED_PEER_TTL_SECONDS=600"

[Install]
WantedBy=multi-user.target
EOF
```

**Important**: On Machine 3 (64.227.107.57), change the paths:
```bash
# On Machine 3 only, use these paths instead:
# WorkingDirectory=/root/seed3
# ExecStart=/usr/bin/python3 /root/seed3/seed_service.py
# ExecStop=/root/seed3/nodectl.sh stop
# PIDFile=/root/seed3/data/seed.pid
# StandardOutput=append:/root/seed3/data/logs/systemd.out
# StandardError=append:/root/seed3/data/logs/systemd.err
```

#### Step 3.3: Enable and Start Systemd Services

Run on **both Ubuntu machines**:

```bash
# Reload systemd to recognize new services
sudo systemctl daemon-reload

# Stop any manually running instances first
cd ~/bootnode1 && ./nodectl.sh stop 2>/dev/null || true
cd ~/seed1 && ./nodectl.sh stop 2>/dev/null || true

# Enable services to start on boot
sudo systemctl enable synergy-bootnode.service
sudo systemctl enable synergy-seed.service

# Start the services now
sudo systemctl start synergy-bootnode.service
sudo systemctl start synergy-seed.service

# Check status
sudo systemctl status synergy-bootnode.service
sudo systemctl status synergy-seed.service

# View logs
sudo journalctl -u synergy-bootnode.service -f
sudo journalctl -u synergy-seed.service -f
```

---

### For macOS Machine (Machine 2 - 73.79.66.255)

Create launchd agents for automatic restart.

#### Step 3.4: Create Bootnode Launchd Agent

```bash
# SSH into macOS machine
ssh synergynode@73.79.66.255

# Create launchd plist for bootnode
cat > ~/Library/LaunchAgents/com.synergy.bootnode.plist <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.synergy.bootnode</string>
    
    <key>ProgramArguments</key>
    <array>
        <string>/Users/synergynode/bootnode2/install_and_start.sh</string>
    </array>
    
    <key>WorkingDirectory</key>
    <string>/Users/synergynode/bootnode2</string>
    
    <key>RunAtLoad</key>
    <true/>
    
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
        <key>Crashed</key>
        <true/>
    </dict>
    
    <key>StandardOutPath</key>
    <string>/Users/synergynode/bootnode2/data/logs/launchd.out</string>
    
    <key>StandardErrorPath</key>
    <string>/Users/synergynode/bootnode2/data/logs/launchd.err</string>
    
    <key>EnvironmentVariables</key>
    <dict>
        <key>SYNERGY_BOOTSTRAP_ONLY</key>
        <string>true</string>
        <key>SYNERGY_AUTO_REGISTER_VALIDATOR</key>
        <string>false</string>
    </dict>
</dict>
</plist>
EOF

# Load and start the agent
launchctl load ~/Library/LaunchAgents/com.synergy.bootnode.plist
launchctl start com.synergy.bootnode
```

#### Step 3.5: Create Seed Service Launchd Agent

```bash
# Create launchd plist for seed service
cat > ~/Library/LaunchAgents/com.synergy.seed.plist <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.synergy.seed</string>
    
    <key>ProgramArguments</key>
    <array>
        <string>/usr/bin/python3</string>
        <string>/Users/synergynode/seed2/seed_service.py</string>
    </array>
    
    <key>WorkingDirectory</key>
    <string>/Users/synergynode/seed2</string>
    
    <key>RunAtLoad</key>
    <true/>
    
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
        <key>Crashed</key>
        <true/>
    </dict>
    
    <key>StandardOutPath</key>
    <string>/Users/synergynode/seed2/data/logs/launchd.out</string>
    
    <key>StandardErrorPath</key>
    <string>/Users/synergynode/seed2/data/logs/launchd.err</string>
    
    <key>EnvironmentVariables</key>
    <dict>
        <key>SEED_PEER_TTL_SECONDS</key>
        <string>600</string>
    </dict>
</dict>
</plist>
EOF

# Load and start the agent
launchctl load ~/Library/LaunchAgents/com.synergy.seed.plist
launchctl start com.synergy.seed
```

#### Step 3.6: Verify macOS Services

```bash
# Check if services are loaded
launchctl list | grep synergy

# View logs
tail -f ~/bootnode2/data/logs/launchd.out
tail -f ~/seed2/data/logs/launchd.out
```

---

## Part 4: Verification and Testing

### Step 4.1: Verify Bootnodes are Running

From any machine, test P2P connectivity:

```bash
# Test bootnode1 P2P port
nc -zv 74.208.227.23 38638

# Test bootnode2 P2P port
nc -zv 73.79.66.255 38638

# Test bootnode3 P2P port
nc -zv 64.227.107.57 38638
```

### Step 4.2: Verify Seed Services are Running

Test HTTP endpoints:

```bash
# Test seed1
curl http://74.208.227.23:18080/peer-list.json

# Test seed2
curl http://73.79.66.255:18080/peer-list.json

# Test seed3
curl http://64.227.107.57:18080/peer-list.json
```

Expected response should contain bootnode and peer information.

### Step 4.3: Check Service Status

#### Ubuntu (systemd):
```bash
# Check bootnode service
sudo systemctl status synergy-bootnode.service

# Check seed service
sudo systemctl status synergy-seed.service

# View recent logs
sudo journalctl -u synergy-bootnode.service --since "5 minutes ago"
sudo journalctl -u synergy-seed.service --since "5 minutes ago"
```

#### macOS (launchd):
```bash
# Check if services are running
launchctl list | grep synergy

# View logs
tail -n 50 ~/bootnode2/data/logs/launchd.out
tail -n 50 ~/seed2/data/logs/launchd.out
```

### Step 4.4: Test Auto-Restart

To verify auto-restart works:

#### Ubuntu:
```bash
# Kill the process manually
sudo systemctl stop synergy-bootnode.service

# Wait 10 seconds, then check if it restarted
sleep 10
sudo systemctl status synergy-bootnode.service
# Should show "active (running)"
```

#### macOS:
```bash
# Kill the process manually
launchctl stop com.synergy.bootnode

# Wait 10 seconds, then check if it restarted
sleep 10
launchctl list | grep synergy
# Should show the service is loaded
```

---

## Part 5: Ongoing Management

### Manual Control Commands

#### Ubuntu:
```bash
# Start/Stop/Restart bootnode
sudo systemctl start synergy-bootnode.service
sudo systemctl stop synergy-bootnode.service
sudo systemctl restart synergy-bootnode.service

# Start/Stop/Restart seed
sudo systemctl start synergy-seed.service
sudo systemctl stop synergy-seed.service
sudo systemctl restart synergy-seed.service

# View live logs
sudo journalctl -u synergy-bootnode.service -f
sudo journalctl -u synergy-seed.service -f
```

#### macOS:
```bash
# Start/Stop/Restart bootnode
launchctl start com.synergy.bootnode
launchctl stop com.synergy.bootnode
launchctl unload ~/Library/LaunchAgents/com.synergy.bootnode.plist
launchctl load ~/Library/LaunchAgents/com.synergy.bootnode.plist

# Start/Stop/Restart seed
launchctl start com.synergy.seed
launchctl stop com.synergy.seed
launchctl unload ~/Library/LaunchAgents/com.synergy.seed.plist
launchctl load ~/Library/LaunchAgents/com.synergy.seed.plist

# View live logs
tail -f ~/bootnode2/data/logs/launchd.out
tail -f ~/seed2/data/logs/launchd.out
```

### Using nodectl Scripts

All machines support the `nodectl.sh` script for local management:

```bash
# Bootnode control
cd ~/bootnodeX
./nodectl.sh start
./nodectl.sh stop
./nodectl.sh restart
./nodectl.sh status
./nodectl.sh logs --follow
./nodectl.sh info

# Seed service control
cd ~/seedX
./nodectl.sh start
./nodectl.sh stop
./nodectl.sh status
./nodectl.sh logs --follow
./nodectl.sh info
```

---

## Troubleshooting

### Bootnode Troubleshooting

#### Problem: Bootnode Won't Start

**Step 1: Check binary compatibility**
```bash
# Verify binary exists and is executable
ls -la ~/bootnodeX/bin/
file ~/bootnodeX/bin/synergy-testbeta-*

# Expected output:
# Linux: ELF 64-bit LSB executable, x86-64
# macOS: Mach-O 64-bit executable, arm64
```

**Step 2: Clear macOS quarantine (macOS only)**
```bash
# If you see "Permission denied" or binary won't execute
xattr -dr com.apple.quarantine ~/bootnode2
chmod +x ~/bootnode2/bin/synergy-testbeta-*
```

**Step 3: Check port conflicts**
```bash
# Ubuntu - check if port 38638 is already in use
sudo netstat -tlnp | grep 38638
# OR
sudo ss -tlnp | grep 38638

# macOS - check if port 38638 is already in use
lsof -i :38638

# If a process is using the port, either:
# 1. Stop the existing process
sudo kill <PID>
# 2. Or check if bootnode is already running
./nodectl.sh status
```

**Step 4: Check configuration file**
```bash
# Validate node.toml syntax
cat ~/bootnodeX/config/node.toml

# Ensure required fields are present:
# - [p2p] listen_address and public_address
# - [network] bootnodes list
# - [node] bootstrap_only = true
```

**Step 5: Test manual start (bypass service manager)**
```bash
cd ~/bootnodeX
./nodectl.sh stop  # Stop any existing instance

# Try starting manually to see errors
./install_and_start.sh

# Or run the binary directly for verbose output
./bin/synergy-testbeta-<platform> start --config config/node.toml
```

**Step 6: Check logs for errors**
```bash
# Ubuntu - systemd logs
sudo journalctl -u synergy-bootnode.service -n 100 --no-pager

# Ubuntu - Application logs
tail -n 100 ~/bootnodeX/data/logs/node.out

# macOS - launchd logs
tail -n 100 ~/bootnodeX/data/logs/launchd.err
tail -n 100 ~/bootnodeX/data/logs/launchd.out

# Look for errors like:
# - "Address already in use" → Port conflict
# - "Failed to bind" → Network interface issue
# - "Invalid configuration" → node.toml syntax error
# - "Permission denied" → File permissions or quarantine
```

**Step 7: Check disk space**
```bash
# Ensure sufficient disk space for chain data
df -h ~/bootnodeX

# If disk is full, consider pruning old data
# (Bootnodes typically don't store much chain data)
```

---

#### Problem: Bootnode Starts But Crashes

**Step 1: Check for OOM (Out of Memory)**
```bash
# Ubuntu - Check kernel OOM killer logs
dmesg | grep -i "killed process"
journalctl -k | grep -i "oom"

# macOS - Check system logs
log show --predicate 'eventMessage contains "kill"' --last 1h
```

**Step 2: Monitor resource usage**
```bash
# Ubuntu - Monitor in real-time
top -p $(cat ~/bootnodeX/data/node.pid)

# macOS - Monitor in real-time
top -pid $(cat ~/bootnodeX/data/node.pid)
```

**Step 3: Check for panic/crash logs**
```bash
# Ubuntu - Check for Rust panics in logs
grep -i "panic" ~/bootnodeX/data/logs/node.out
grep -i "thread.*panicked" ~/bootnodeX/data/logs/node.out

# macOS
grep -i "panic" ~/bootnodeX/data/logs/launchd.out
```

**Step 4: Increase logging verbosity**
```bash
# Edit node.env to add debug logging
echo 'RUST_LOG=debug' >> ~/bootnodeX/node.env

# Restart the service
# Ubuntu
sudo systemctl restart synergy-bootnode.service

# macOS
launchctl stop com.synergy.bootnode
launchctl start com.synergy.bootnode

# Check detailed logs
# Ubuntu
sudo journalctl -u synergy-bootnode.service -f

# macOS
tail -f ~/bootnodeX/data/logs/launchd.out
```

---

#### Problem: Bootnode Running But Not Accepting Connections

**Step 1: Verify P2P port is listening**
```bash
# Ubuntu
sudo netstat -tlnp | grep 38638

# macOS
lsof -i :38638

# Expected: LISTEN state on 0.0.0.0:38638 or *:38638
```

**Step 2: Check firewall rules**
```bash
# Ubuntu - Check ufw status
sudo ufw status
sudo ufw status numbered | grep 38638

# If port is blocked, allow it:
sudo ufw allow 38638/tcp comment "Synergy P2P"

# macOS - Check firewall
sudo /usr/libexec/ApplicationFirewall/socketfilterfw --getglobalstate
sudo /usr/libexec/ApplicationFirewall/socketfilterfw --listapps | grep synergy

# If needed, allow incoming connections in System Preferences → Security → Firewall
```

**Step 3: Test local connectivity**
```bash
# Test connecting to yourself
telnet localhost 38638
# OR
nc -zv localhost 38638

# Should show: Connection succeeded
```

**Step 4: Test external connectivity**
```bash
# From a different machine, test connection
nc -zv <bootnode-ip> 38638

# If connection times out:
# 1. Check cloud provider security groups (AWS, GCP, etc.)
# 2. Check ISP firewall
# 3. Verify NAT/port forwarding if behind router
```

**Step 5: Check peer connections**
```bash
# Check if bootnode has any peers
curl -s http://localhost:48638/rpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"synergy_getPeers","params":[],"id":1}'

# Expected: Should return peer count (may be 0 if isolated)
```

---

### Seed Server Troubleshooting

#### Problem: Seed Service Won't Start

**Step 1: Verify Python installation**
```bash
# Check Python 3 is available
python3 --version
# Should be 3.8 or higher

which python3
# Should return: /usr/bin/python3 (Ubuntu) or /usr/bin/python3 (macOS)
```

**Step 2: Check Python syntax**
```bash
# Validate seed_service.py syntax
python3 -m py_compile ~/seedX/seed_service.py

# If errors appear, check the reported line numbers
```

**Step 3: Check configuration file**
```bash
# Validate seed-service.json syntax
python3 -c "import json; json.load(open('~/seedX/config/seed-service.json'))"

# Or use jq if available
jq . ~/seedX/config/seed-service.json

# Ensure required fields:
# - service_name, domain, listen_host, listen_port
# - bootnodes array with at least one entry
# - seed_services array with all three seeds
```

**Step 4: Check port conflicts**
```bash
# Ubuntu - check if port 18080 is already in use
sudo netstat -tlnp | grep 18080
sudo ss -tlnp | grep 18080

# macOS
lsof -i :18080

# If port is in use:
# 1. Stop existing seed service
./nodectl.sh stop

# 2. Or kill the process
sudo kill <PID>
```

**Step 5: Test manual start**
```bash
cd ~/seedX
./nodectl.sh stop  # Stop any existing instance

# Try starting manually
./install_and_start.sh

# Or run Python directly for verbose output
python3 seed_service.py
```

**Step 6: Check logs for errors**
```bash
# Ubuntu - systemd logs
sudo journalctl -u synergy-seed.service -n 100 --no-pager

# Ubuntu - Application logs
tail -n 100 ~/seedX/data/logs/seed.out

# macOS - launchd logs
tail -n 100 ~/seedX/data/logs/launchd.err
tail -n 100 ~/seedX/data/logs/launchd.out

# Common errors:
# - "Address already in use" → Port 18080 conflict
# - "JSON decode error" → Invalid seed-service.json
# - "Permission denied" → Can't write to data/ directory
```

---

#### Problem: Seed Service Starts But Returns Errors

**Step 1: Test HTTP endpoint**
```bash
# Test peer-list.json endpoint
curl -v http://localhost:18080/peer-list.json

# Expected: JSON response with bootnodes and peers
# If 500 error: Check logs for Python traceback

# Test /peers endpoint
curl -v http://localhost:18080/peers

# Expected: JSON with peer list and TTL info
```

**Step 2: Check data directory permissions**
```bash
# Ensure seed service can write to data/
ls -la ~/seedX/data/

# Should be writable by the user running the service
# If not, fix permissions:
chmod -R 755 ~/seedX/data
chown -R $USER:$USER ~/seedX/data
```

**Step 3: Verify peers.json file**
```bash
# Check if peers.json exists and is valid JSON
cat ~/seedX/data/peers.json

# If corrupted or invalid, reset it:
echo '[]' > ~/seedX/data/peers.json

# Restart the service
# Ubuntu
sudo systemctl restart synergy-seed.service

# macOS
launchctl stop com.synergy.seed
launchctl start com.synergy.seed
```

**Step 4: Check inter-seed synchronization**
```bash
# Test connectivity to sibling seeds
curl -s http://73.79.66.255:18080/peers | jq .
curl -s http://64.227.107.57:18080/peers | jq .

# If connections fail:
# 1. Check firewall rules (see bootnode firewall section)
# 2. Verify sibling seeds are running
# 3. Check seed-service.json has correct sibling IPs
```

**Step 5: Monitor peer registration**
```bash
# Watch peers.json for updates
watch -n 5 'cat ~/seedX/data/peers.json | jq .'

# Register a test peer (simulate node registration)
curl -X POST http://localhost:18080/peer \
  -H "Content-Type: application/json" \
  -d '{
    "public_host": "test.example.com",
    "p2p_port": 38638,
    "node_id": "test-node",
    "version": "test"
  }'

# Check if peer was added
curl http://localhost:18080/peers | jq '.peers'
```

---

#### Problem: Seed Service Not Syncing with Sibling Seeds

**Step 1: Check seed-service.json configuration**
```bash
# Verify seed_services array has all three seeds
cat ~/seedX/config/seed-service.json | jq '.seed_services'

# Should include seed1, seed2, and seed3 with correct IPs
```

**Step 2: Test HTTP connectivity to siblings**
```bash
# From each seed server, test the others
curl -s http://74.208.227.23:18080/peers | jq '.active_peer_count'
curl -s http://73.79.66.255:18080/peers | jq '.active_peer_count'
curl -s http://64.227.107.57:18080/peers | jq '.active_peer_count'

# All should return a number (may be 0 if no peers registered)
```

**Step 3: Check sync logs**
```bash
# Look for sync-related messages in logs
grep -i "sync" ~/seedX/data/logs/seed.out
grep -i "sibling" ~/seedX/data/logs/seed.out
grep -i "merge" ~/seedX/data/logs/seed.out

# Ubuntu - systemd logs
sudo journalctl -u synergy-seed.service | grep -i sync
```

**Step 4: Force manual sync**
```bash
# The seed service syncs every 3rd refresh cycle (~90 seconds)
# Wait 2-3 minutes and check if peers.json updates

watch -n 10 'cat ~/seedX/data/peers.json | jq ". | length"'
```

---

### Service Manager Troubleshooting

#### Ubuntu systemd: Service Won't Start

**Step 1: Check service status**
```bash
sudo systemctl status synergy-bootnode.service
sudo systemctl status synergy-seed.service

# Look for:
# - "Active: failed" → Check exit code
# - "Active: activating" → Service is starting
# - "Loaded: not-found" → Service file missing
```

**Step 2: View detailed error information**
```bash
# Get full status report
sudo systemctl status synergy-bootnode.service -l --no-pager

# Check for start failures
sudo journalctl -u synergy-bootnode.service -n 50 --no-pager

# Check service configuration
sudo systemctl cat synergy-bootnode.service
```

**Step 3: Fix common issues**

**Issue: WorkingDirectory doesn't exist**
```bash
# Verify directory exists
ls -la /root/bootnode1  # or /root/bootnode3

# If missing, re-copy the bundle
```

**Issue: ExecStart script not executable**
```bash
chmod +x /root/bootnode1/install_and_start.sh
chmod +x /root/seed1/seed_service.py
```

**Issue: Permission denied**
```bash
# Check service is running as correct user
sudo systemctl show synergy-bootnode.service | grep User

# Should be User=root for bootnode/seed services
```

**Step 4: Reload and restart**
```bash
# After fixing issues, reload systemd
sudo systemctl daemon-reload

# Reset any failed state
sudo systemctl reset-failed synergy-bootnode.service
sudo systemctl reset-failed synergy-seed.service

# Restart services
sudo systemctl start synergy-bootnode.service
sudo systemctl start synergy-seed.service

# Verify
sudo systemctl status synergy-bootnode.service
sudo systemctl status synergy-seed.service
```

---

#### macOS launchd: Service Won't Start

**Step 1: Check if agent is loaded**
```bash
launchctl list | grep synergy

# Expected output:
# -    com.synergy.bootnode
# -    com.synergy.seed
```

**Step 2: Validate plist syntax**
```bash
plutil -lint ~/Library/LaunchAgents/com.synergy.bootnode.plist
plutil -lint ~/Library/LaunchAgents/com.synergy.seed.plist

# Should return: <filename>: OK
```

**Step 3: Check plist content**
```bash
# View the plist
cat ~/Library/LaunchAgents/com.synergy.bootnode.plist

# Verify:
# - ProgramArguments points to correct path
# - WorkingDirectory exists
# - Paths use /Users/synergynode/ (not /root/)
```

**Step 4: Load and start manually**
```bash
# Unload if already loaded
launchctl unload ~/Library/LaunchAgents/com.synergy.bootnode.plist 2>/dev/null || true
launchctl unload ~/Library/LaunchAgents/com.synergy.seed.plist 2>/dev/null || true

# Load the agents
launchctl load ~/Library/LaunchAgents/com.synergy.bootnode.plist
launchctl load ~/Library/LaunchAgents/com.synergy.seed.plist

# Start the services
launchctl start com.synergy.bootnode
launchctl start com.synergy.seed

# Verify
launchctl list | grep synergy
```

**Step 5: Check macOS-specific issues**

**Issue: Quarantine attribute**
```bash
# Clear quarantine from all files
xattr -dr com.apple.quarantine ~/bootnode2
xattr -dr com.apple.quarantine ~/seed2
```

**Issue: Full Disk Access required**
```bash
# macOS may block access to certain directories
# Check System Preferences → Security & Privacy → Privacy → Full Disk Access
# Add Terminal or your SSH client if needed
```

**Issue: Path differences**
```bash
# macOS uses /Users/username/ not /root/
# Verify paths in plist match actual directory
ls -la /Users/synergynode/bootnode2
ls -la /Users/synergynode/seed2
```

---

### Network Connectivity Troubleshooting

#### Problem: Bootnodes Can't Connect to Each Other

**Step 1: Verify DNS resolution**
```bash
# Test DNS resolution from each machine
nslookup bootnode1.synergynode.xyz
nslookup bootnode2.synergynode.xyz
nslookup bootnode3.synergynode.xyz

# Or use dig
dig bootnode1.synergynode.xyz +short
dig bootnode2.synergynode.xyz +short
dig bootnode3.synergynode.xyz +short

# Should return correct IPs:
# bootnode1 → 74.208.227.23
# bootnode2 → 73.79.66.255
# bootnode3 → 64.227.107.57
```

**Step 2: Test bidirectional connectivity**
```bash
# From Machine 1 (74.208.227.23)
nc -zv 73.79.66.255 38638  # To Machine 2
nc -zv 64.227.107.57 38638  # To Machine 3

# From Machine 2 (73.79.66.255)
nc -zv 74.208.227.23 38638  # To Machine 1
nc -zv 64.227.107.57 38638  # To Machine 3

# From Machine 3 (64.227.107.57)
nc -zv 74.208.227.23 38638  # To Machine 1
nc -zv 73.79.66.255 38638  # To Machine 2

# All should show: Connection succeeded
```

**Step 3: Check bootnode configuration**
```bash
# Verify node.toml has correct bootnode list
cat ~/bootnodeX/config/node.toml | grep -A 5 "bootnodes"

# Should include the OTHER two bootnodes (not itself)
# Example for bootnode1:
# bootnodes = [
#   "snr://bootstrap@bootnode2.synergynode.xyz:38638",
#   "snr://bootstrap@bootnode3.synergynode.xyz:38638"
# ]
```

**Step 4: Check P2P discovery**
```bash
# Check if bootnode is discovering peers
grep -i "peer" ~/bootnodeX/data/logs/node.out | tail -20

# Look for:
# - "Connected to peer"
# - "Handshake successful"
# - "Discovery" messages
```

---

#### Problem: Seed Servers Not Syncing Peer Lists

**Step 1: Verify all seed services are running**
```bash
# Check all three seeds respond
curl -s http://74.208.227.23:18080/peers | jq '.service'
curl -s http://73.79.66.255:18080/peers | jq '.service'
curl -s http://64.227.107.57:18080/peers | jq '.service'

# Should return: "seed1", "seed2", "seed3"
```

**Step 2: Check seed-service.json on each machine**
```bash
# Each seed should have all three in seed_services array
cat ~/seed1/config/seed-service.json | jq '.seed_services[].name'
cat ~/seed2/config/seed-service.json | jq '.seed_services[].name'
cat ~/seed3/config/seed-service.json | jq '.seed_services[].name'

# All should return: "seed1", "seed2", "seed3"
```

**Step 3: Verify inter-seed HTTP connectivity**
```bash
# From each seed, test HTTP access to siblings
# Machine 1:
curl -s http://73.79.66.255:18080/peers | jq '.active_peer_count'
curl -s http://64.227.107.57:18080/peers | jq '.active_peer_count'

# Machine 2:
curl -s http://74.208.227.23:18080/peers | jq '.active_peer_count'
curl -s http://64.227.107.57:18080/peers | jq '.active_peer_count'

# Machine 3:
curl -s http://74.208.227.23:18080/peers | jq '.active_peer_count'
curl -s http://73.79.66.255:18080/peers | jq '.active_peer_count'
```

**Step 4: Check sync logs**
```bash
# Look for sync activity in logs
grep -i "sync" ~/seedX/data/logs/seed.out | tail -20
grep -i "merge" ~/seedX/data/logs/seed.out | tail -20

# Sync happens every ~90 seconds (every 3rd refresh cycle)
```

---

### Services Don't Auto-Restart

#### Ubuntu systemd:

**Step 1: Check service configuration**
```bash
# View full service file
sudo systemctl cat synergy-bootnode.service

# Verify Restart=always is present
```

**Step 2: Check for restart limits**
```bash
# View restart counter
sudo systemctl show synergy-bootnode.service | grep -i restart

# Reset if needed
sudo systemctl reset-failed synergy-bootnode.service
```

**Step 3: Test auto-restart**
```bash
# Kill the process to test restart
sudo kill $(cat /root/bootnodeX/data/node.pid)

# Wait 10 seconds
sleep 10

# Check if it restarted
sudo systemctl status synergy-bootnode.service
# Should show "active (running)" with new PID
```

**Step 4: Enable at boot**
```bash
# Verify service is enabled
sudo systemctl is-enabled synergy-bootnode.service
sudo systemctl is-enabled synergy-seed.service

# Should return: enabled

# If not enabled:
sudo systemctl enable synergy-bootnode.service
sudo systemctl enable synergy-seed.service
```

---

#### macOS launchd:

**Step 1: Check plist syntax**
```bash
plutil -lint ~/Library/LaunchAgents/com.synergy.bootnode.plist
plutil -lint ~/Library/LaunchAgents/com.synergy.seed.plist
```

**Step 2: Verify KeepAlive configuration**
```bash
# Check KeepAlive settings in plist
cat ~/Library/LaunchAgents/com.synergy.bootnode.plist | grep -A 5 KeepAlive

# Should show:
# <key>KeepAlive</key>
# <dict>
#     <key>SuccessfulExit</key>
#     <false/>
#     <key>Crashed</key>
#     <true/>
# </dict>
```

**Step 3: Test auto-restart**
```bash
# Kill the process
kill $(cat ~/bootnode2/data/seed.pid)

# Wait 10 seconds
sleep 10

# Check if it restarted
launchctl list | grep synergy
ps aux | grep seed_service.py
```

**Step 4: Ensure agent loads at login**
```bash
# For services to start on boot, they need to be in:
# ~/Library/LaunchAgents/ (per-user, loads at login)
# OR
# /Library/LaunchDaemons/ (system-wide, loads at boot)

# Current setup uses per-user agents
# To test, reboot the machine and check:
launchctl list | grep synergy
```

---

### Quick Diagnostic Commands

```bash
# ===== BOOTNODE QUICK CHECK =====
# Check if running
ps aux | grep synergy-testbeta | grep -v grep

# Check P2P port
netstat -tlnp 2>/dev/null | grep 38638 || lsof -i :38638

# Check logs (last 20 lines)
tail -20 ~/bootnodeX/data/logs/node.out

# Quick status
./nodectl.sh status


# ===== SEED SERVICE QUICK CHECK =====
# Check if running
ps aux | grep seed_service.py | grep -v grep

# Check HTTP port
netstat -tlnp 2>/dev/null | grep 18080 || lsof -i :18080

# Test endpoint
curl -s http://localhost:18080/peers | jq '.active_peer_count'

# Check logs (last 20 lines)
tail -20 ~/seedX/data/logs/seed.out

# Quick status
./nodectl.sh status


# ===== NETWORK CONNECTIVITY =====
# Test all bootnode P2P ports
for ip in 74.208.227.23 73.79.66.255 64.227.107.57; do
  echo -n "$ip:38638 -> "
  nc -zv $ip 38638 2>&1 | grep succeeded || echo "FAILED"
done

# Test all seed HTTP endpoints
for ip in 74.208.227.23 73.79.66.255 64.227.107.57; do
  echo -n "http://$ip:18080/peers -> "
  curl -s -o /dev/null -w "%{http_code}" http://$ip:18080/peers
  echo
done
```

---

## DNS Configuration

Ensure the following DNS records are configured (see `DNS_RECORDS.txt`):

### A Records:
```
bootnode1.synergynode.xyz -> 74.208.227.23
bootnode2.synergynode.xyz -> 73.79.66.255
bootnode3.synergynode.xyz -> 64.227.107.57
seed1.synergynode.xyz -> 74.208.227.23
seed2.synergynode.xyz -> 73.79.66.255
seed3.synergynode.xyz -> 64.227.107.57
```

### TXT Records (for bootnode discovery):
```
_dnsaddr.bootstrap.synergynode.xyz -> "dnsaddr=/dns/bootnode1.synergynode.xyz/tcp/38638"
_dnsaddr.bootstrap.synergynode.xyz -> "dnsaddr=/dns/bootnode2.synergynode.xyz/tcp/38638"
_dnsaddr.bootstrap.synergynode.xyz -> "dnsaddr=/dns/bootnode3.synergynode.xyz/tcp/38638"
```

---

## Quick Reference

| Service | Machine 1 (Ubuntu) | Machine 2 (macOS) | Machine 3 (Ubuntu) |
|---------|-------------------|-------------------|-------------------|
| **Bootnode** | `systemctl status synergy-bootnode` | `launchctl list \| grep bootnode` | `systemctl status synergy-bootnode` |
| **Seed** | `systemctl status synergy-seed` | `launchctl list \| grep seed` | `systemctl status synergy-seed` |
| **Bootnode Logs** | `journalctl -u synergy-bootnode -f` | `tail -f ~/bootnode2/data/logs/launchd.out` | `journalctl -u synergy-bootnode -f` |
| **Seed Logs** | `journalctl -u synergy-seed -f` | `tail -f ~/seed2/data/logs/launchd.out` | `journalctl -u synergy-seed -f` |
| **P2P Port** | 38638 | 38638 | 38638 |
| **Seed HTTP Port** | 18080 | 18080 | 18080 |

---

## Support

For issues or questions:
1. Check logs first (see above)
2. Verify network connectivity between nodes
3. Ensure DNS records are properly configured
4. Review `DNS_RECORDS.txt` for required DNS configuration
