bootnode2 bootstrap-only deployment bundle
======================================

Purpose
- Runs a Synergy Testnet Beta node in bootstrap-only mode.
- Discovery only: no validator self-registration, no consensus engine, no public RPC services.

Endpoint
- Hostname: bootnode2.synergyvps.xyz
- IP: 73.79.66.255
- P2P Port: 5620

Start
- Linux/macOS: ./install_and_start.sh
- Windows: powershell -ExecutionPolicy Bypass -File .\install_and_start.ps1

Control
- Linux/macOS: ./nodectl.sh status | logs --follow | stop
- Windows: powershell -ExecutionPolicy Bypass -File .\nodectl.ps1 status

Notes
- Open TCP 5620 on the target host firewall.
- Publish A record bootnode2.synergyvps.xyz -> 73.79.66.255
- Publish _dnsaddr.bootstrap TXT records from the root DNS_RECORDS.txt file in /Users/devpup/Desktop/Testnet-Beta/synergy-testnet-beta/bootstrap-bundles
