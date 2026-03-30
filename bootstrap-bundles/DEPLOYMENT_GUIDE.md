# Synergy Testnet-Beta Bootstrap Deployment Guide

## Launch Baseline

- Chain ID: 338639
- Network ID: synergy-testnet-beta
- Token symbol: SNRG
- Genesis validators: 4
- Bootnodes: 3
- Seed services: 3

## Assigned Bootstrap Hosts

| Role | Hostname | IP | Port |
| --- | --- | --- | --- |
| bootnode1 | bootnode1.synergynode.xyz | 74.208.227.23 | 5620/tcp |
| bootnode2 | bootnode2.synergynode.xyz | 73.79.66.255 | 5620/tcp |
| bootnode3 | bootnode3.synergynode.xyz | 64.227.107.57 | 5620/tcp |
| seed1 | seed1.synergynode.xyz | 74.208.227.23 | 5621/tcp |
| seed2 | seed2.synergynode.xyz | 73.79.66.255 | 5621/tcp |
| seed3 | seed3.synergynode.xyz | 64.227.107.57 | 5621/tcp |

## Port Freeze

| Purpose | Value |
| --- | --- |
| Bootnode listener | 5620/tcp |
| Seed-service listener | 5621/tcp |
| Sequential node listener base | 5622 + node assignment |
| Slotted node RPC base | 5640 + node assignment |
| Slotted node WS base | 5660 + node assignment |
| Slotted node discovery base | 5680 + node assignment |
| Slotted node metrics base | 6030 + port_slot |

## Bootnode Deployment

1. Download the assigned bootnode bundle from the Genesis Dashboard.
2. Transfer the bundle to the target host.
3. Extract the archive on the target host.
4. Open inbound TCP 5620 on the host firewall.
5. Confirm the A record for the assigned hostname points to the target IP.
6. Start the bundle with `./install_and_start.sh` on Linux or macOS, or `install_and_start.ps1` on Windows.
7. Confirm the process is running with `./nodectl.sh status` or `nodectl.ps1 status`.

## Seed-Service Deployment

1. Download the assigned seed bundle from the Genesis Dashboard.
2. Transfer the bundle to the target host.
3. Extract the archive on the target host.
4. Open inbound TCP 5621 on the host firewall.
5. Confirm the A record for the assigned hostname points to the target IP.
6. Start the service with `./install_and_start.sh` on Linux or macOS, or `install_and_start.ps1` on Windows.
7. Confirm the process is running with `./nodectl.sh status` or `nodectl.ps1 status`.

## Verification

Run these checks after the assigned bundle is started.

```bash
# Bootnode reachability
nc -zv bootnode1.synergynode.xyz 5620
nc -zv bootnode2.synergynode.xyz 5620
nc -zv bootnode3.synergynode.xyz 5620

# Seed-service health
curl -s http://seed1.synergynode.xyz:5621/healthz
curl -s http://seed2.synergynode.xyz:5621/healthz
curl -s http://seed3.synergynode.xyz:5621/healthz

# Seed-service discovery payload
curl -s http://seed1.synergynode.xyz:5621/peer-list.json
curl -s http://seed2.synergynode.xyz:5621/peer-list.json
curl -s http://seed3.synergynode.xyz:5621/peer-list.json
```

## DNS

Use the exact records in `DNS_RECORDS.txt`.
