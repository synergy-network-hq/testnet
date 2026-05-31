# macOS Install

Expected operator flow:

```bash
unzip synergy-archive-validator-testnet-v2-macos-universal.zip
sudo installer -pkg SynergyArchiveValidator.pkg -target /
sudo /usr/local/synergy/bin/synergy-archive status
```

Install locations:

- Binary: `/usr/local/synergy/bin/synergy-archive`
- Config: `/Library/Application Support/Synergy/archive-validator/config`
- Data: `/Library/Application Support/Synergy/archive-validator`
- Logs: `/Library/Logs/Synergy/archive-validator`
- LaunchDaemons: `/Library/LaunchDaemons/io.synergynetwork.archive-*.plist`

The installer fails closed if `aegis-pqvm` is unavailable or if the post-install health check fails. Uninstall without deleting data:

```bash
sudo /usr/local/synergy/share/archive-validator/uninstall-macos.sh
```

Data deletion requires the explicit `--purge-data` flag.

## Temporary Testnet Archive Testing Workaround

For local operator-controlled deployment from extracted source, build the dedicated macOS extracted zip. Do not disable Gatekeeper globally and do not run `spctl --master-disable`.

```bash
./package-archive-validator.sh --macos-extracted
shasum -a 256 synergy-archive-validator-testnet-v2-macos-extracted.zip
unzip synergy-archive-validator-testnet-v2-macos-extracted.zip
xattr -dr com.apple.quarantine ./archive-validator
cd archive-validator
sudo ./macos/setup-extracted-zip.sh \
  --archive-binary /trusted/path/synergy-archive \
  --node-binary /trusted/path/synergy-node \
  --genesis-file /trusted/path/genesis.testnet.json \
  --expected-genesis-hash <hash> \
  --wireguard-config /secure/path/archive-validator.conf
```

The WireGuard input is operator supplied and copied locally with mode `0600`. The public zip contains only `config/wireguard/archive-validator.conf.template`, which has placeholders and no credentials.

After the archive state is verified on the majority branch, authorize initial snapshot creation through supported tooling:

```bash
sudo /usr/local/synergy/share/archive-validator/create-initial-snapshot.sh \
  --source-node-majority-branch-proven
```

Launchd persists WireGuard, the archive service, the snapshot API, and the scheduled snapshot worker. The scheduled worker invokes `synergy-node create-snapshot-if-due`, which uses the same supported signed snapshot path and keeps exactly two snapshots.

The extracted-zip path is local operator packaging only. The release-grade macOS path remains a signed, notarized, and stapled installer package.
