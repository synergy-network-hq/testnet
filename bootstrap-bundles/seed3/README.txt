seed3 seed-service deployment bundle
====================================

Purpose
- Runs a lightweight HTTP publisher for bootstrap metadata.
- This is not a validator, relayer, or P2P node.

Endpoint
- Hostname: seed3.synergynode.xyz
- IP: 64.227.107.57
- HTTP Port: 18080

Published endpoints
- /healthz
- /peer-list.json
- /dns/bootstrap.txt

Start
- Linux/macOS: ./install_and_start.sh
- Windows: powershell -ExecutionPolicy Bypass -File .\install_and_start.ps1

DNS
- Publish A record seed3.synergynode.xyz -> 64.227.107.57
- Optional SRV record: _synergy-seed._tcp.synergynode.xyz -> seed3.synergynode.xyz:18080
