seed2 seed-service deployment bundle
====================================

Purpose
- Runs a lightweight HTTP publisher for bootstrap metadata.
- This is not a validator, relayer, or P2P node.

Endpoint
- Hostname: seed2.synergynode.xyz
- IP: 73.79.66.255
- HTTP Port: 5621

Published endpoints
- /healthz
- /peer-list.json
- /dns/bootstrap.txt
- /peers
- /peers/register
- /peers/clear (admin)

Start
- Linux/macOS: ./install_and_start.sh
- Windows: powershell -ExecutionPolicy Bypass -File .\install_and_start.ps1

Clear registered peers
- Local only without a token: curl -X DELETE http://127.0.0.1:5621/peers
- Remote with token: curl -X DELETE -H "X-Seed-Admin-Token: <token>" http://seed2.synergynode.xyz:5621/peers

DNS
- Publish A record seed2.synergynode.xyz -> 73.79.66.255
- Optional SRV record: _synergy-seed._tcp.synergynode.xyz -> seed2.synergynode.xyz:5621
