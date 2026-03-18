seed1 seed-service deployment bundle
====================================

Purpose
- Runs a lightweight HTTP publisher for bootstrap metadata.
- This is not a validator, relayer, or P2P node.

Endpoint
- Hostname: seed1.synergynode.xyz
- IP: 74.208.227.23
- HTTP Port: 18080

Published endpoints
- /healthz
- /peer-list.json
- /dns/bootstrap.txt

Start
- Linux/macOS: ./install_and_start.sh
- Windows: powershell -ExecutionPolicy Bypass -File .\install_and_start.ps1

DNS
- Publish A record seed1.synergynode.xyz -> 74.208.227.23
- Optional SRV record: _synergy-seed._tcp.synergynode.xyz -> seed1.synergynode.xyz:18080
