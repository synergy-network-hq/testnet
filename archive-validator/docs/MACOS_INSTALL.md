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
