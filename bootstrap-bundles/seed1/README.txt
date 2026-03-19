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
- /peers
- /peers/register
- /peers/clear (admin)

Start
- Linux/macOS: ./install_and_start.sh
- Windows: powershell -ExecutionPolicy Bypass -File .\install_and_start.ps1

Clear registered peers
- Local only without a token: curl -X DELETE http://127.0.0.1:18080/peers
- Remote with token: curl -X DELETE -H "X-Seed-Admin-Token: <token>" http://seed1.synergynode.xyz:18080/peers

DNS
- Publish A record seed1.synergynode.xyz -> 74.208.227.23
- Optional SRV record: _synergy-seed._tcp.synergynode.xyz -> seed1.synergynode.xyz:18080
