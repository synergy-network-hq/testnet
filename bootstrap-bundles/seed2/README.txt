seed2 seed-service deployment bundle
====================================

Purpose
- Runs a lightweight HTTP publisher for bootstrap metadata.
- This is not a validator, relayer, or P2P node.

Endpoint
- Hostname: seed2.synergynode.xyz
- IP: 73.79.66.255
- HTTP Port: 18080

Published endpoints
- /healthz
- /peer-list.json
- /dns/bootstrap.txt

Start
- Linux/macOS: ./install_and_start.sh
- Windows: powershell -ExecutionPolicy Bypass -File .\install_and_start.ps1

DNS
- Publish A record seed2.synergynode.xyz -> 73.79.66.255
- Optional SRV record: _synergy-seed._tcp.synergynode.xyz -> seed2.synergynode.xyz:18080
