# Synergy Archive Validator Node

This package installs a non-consensus Archive Validator Node for Synergy Testnet.

Protocol role: `ARCHIVE_OBSERVER`

The archive node verifies finalized chain data, stores full archival data, creates signed snapshots every 5,000 finalized blocks, retains exactly two snapshots, and serves verified snapshots to new validators and self-healing validators. It never votes, never proposes, never aggregates QCs, and never counts toward quorum.

Linux install:

```bash
unzip synergy-archive-validator-testnet-v2-linux-x64.zip
cd archive-validator
sudo ./setup-archive-validator.sh --chain-id 1264 --network-id synergy-testnet-v2 --genesis-file ./config/genesis.testnet.json.template --expected-genesis-hash <hash> --yes
```

macOS install:

```bash
unzip synergy-archive-validator-testnet-v2-macos-universal.zip
sudo installer -pkg SynergyArchiveValidator.pkg -target /
sudo /usr/local/synergy/bin/synergy-archive status
```

The macOS zip is valid only when it contains a signed, notarized, and stapled `SynergyArchiveValidator.pkg`. Do not distribute raw unsigned scripts as the macOS installer.

Local macOS extracted-zip package source:

```bash
./package-archive-validator.sh --macos-extracted
unzip synergy-archive-validator-testnet-v2-macos-extracted.zip
cd archive-validator
sudo ./macos/setup-extracted-zip.sh \
  --archive-binary /trusted/path/synergy-archive \
  --node-binary /trusted/path/synergy-node \
  --genesis-file /trusted/path/genesis.testnet.json \
  --expected-genesis-hash <hash> \
  --wireguard-config /secure/path/archive-validator.conf
```

The extracted-zip installer is for local operator-controlled deployment. WireGuard material must be supplied at install time and is never embedded in the package source zip.

Private keys are not included. Aegis PQC archive peer and snapshot signing identities must be generated or referenced through `aegis-pqvm`.
