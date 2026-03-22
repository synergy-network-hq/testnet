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

### Bootnode Won't Start

1. **Check binary compatibility**:
   ```bash
   # Verify binary exists and is executable
   ls -la ~/bootnodeX/bin/
   file ~/bootnodeX/bin/synergy-testbeta-*
   ```

2. **Check port conflicts**:
   ```bash
   # Ubuntu
   sudo netstat -tlnp | grep 38638
   
   # macOS
   lsof -i :38638
   ```

3. **Check logs**:
   ```bash
   # Ubuntu
   sudo journalctl -u synergy-bootnode.service -n 100
   
   # macOS
   tail -n 100 ~/bootnodeX/data/logs/launchd.err
   ```

### Seed Service Won't Start

1. **Verify Python**:
   ```bash
   python3 --version
   which python3
   ```

2. **Check syntax**:
   ```bash
   python3 -m py_compile ~/seedX/seed_service.py
   ```

3. **Check port conflicts**:
   ```bash
   # Ubuntu
   sudo netstat -tlnp | grep 18080
   
   # macOS
   lsof -i :18080
   ```

### Services Don't Auto-Restart

#### Ubuntu:
```bash
# Check systemd service configuration
sudo systemctl cat synergy-bootnode.service

# Check for restart limits
sudo systemctl show synergy-bootnode.service | grep -i restart

# Reset failed state
sudo systemctl reset-failed synergy-bootnode.service
```

#### macOS:
```bash
# Check launchd plist syntax
plutil -lint ~/Library/LaunchAgents/com.synergy.bootnode.plist

# Reload the agent
launchctl unload ~/Library/LaunchAgents/com.synergy.bootnode.plist
launchctl load ~/Library/LaunchAgents/com.synergy.bootnode.plist
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
