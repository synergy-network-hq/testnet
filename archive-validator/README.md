# Synergy Archive Validator Node

This package installs a non-consensus Archive Validator Node for Synergy Testnet.

Protocol role: `ARCHIVE_OBSERVER`

The archive node verifies finalized chain data, stores full archival data, creates signed snapshots every 10,000 finalized blocks, and serves verified snapshots to new validators and self-healing validators. It never votes, never proposes, never aggregates QCs, and never counts toward quorum.

Install:

```bash
unzip synergy-archive-validator-testnet-v2.zip
cd archive-validator
sudo ./setup-archive-validator.sh --chain-id 1264 --network-id synergy-testnet-v2 --genesis-file ./config/genesis.testnet.json.template --expected-genesis-hash <hash> --yes
```

Private keys are not included. Aegis PQC archive peer and snapshot signing identities must be generated or referenced through `aegis-pqvm`.
