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
- /peers
- /peers/register
- /peers/clear (admin)

Start
- Linux/macOS: ./install_and_start.sh
- Windows: powershell -ExecutionPolicy Bypass -File .\install_and_start.ps1

Clear registered peers
- Local only without a token: curl -X DELETE http://127.0.0.1:18080/peers
- Remote with token: curl -X DELETE -H "X-Seed-Admin-Token: <token>" http://seed3.synergynode.xyz:18080/peers

DNS
- Publish A record seed3.synergynode.xyz -> 64.227.107.57
- Optional SRV record: _synergy-seed._tcp.synergynode.xyz -> seed3.synergynode.xyz:18080
