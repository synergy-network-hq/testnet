# Synergy Archive Validator macOS Gatekeeper Notes

The normal macOS distribution artifact is `SynergyArchiveValidator.pkg` inside `synergy-archive-validator-testnet-v2-macos-universal.zip`.

The package must be signed with the Synergy Developer ID Installer certificate, all executables inside it must be signed with the Synergy Developer ID Application certificate using hardened runtime, and the package must be notarized and stapled before release.

Operators should not need to disable Gatekeeper or approve unsigned binaries manually. Do not run `spctl --master-disable`.

## Temporary Testnet Archive Testing Workaround

For local operator-controlled extracted-zip deployment, use the dedicated source package after verifying the artifact checksum.

1. Verify checksum first:

   ```bash
   shasum -a 256 synergy-archive-validator-testnet-v2-macos-extracted.zip
   ```

2. Remove quarantine from the zip before extraction if needed:

   ```bash
   xattr -d com.apple.quarantine synergy-archive-validator-testnet-v2-macos-extracted.zip 2>/dev/null || true
   ```

3. Extract:

   ```bash
   unzip synergy-archive-validator-testnet-v2-macos-extracted.zip
   ```

4. Remove quarantine recursively from the extracted package:

   ```bash
   xattr -dr com.apple.quarantine ./archive-validator
   ```

5. Run setup:

   ```bash
   cd archive-validator
   sudo ./macos/setup-extracted-zip.sh \
     --archive-binary /trusted/path/synergy-archive \
     --node-binary /trusted/path/synergy-node \
     --genesis-file /trusted/path/genesis.testnet.json \
     --expected-genesis-hash <hash> \
     --wireguard-config /secure/path/archive-validator.conf
   ```

WireGuard config and private keys must be supplied outside the zip. This local source path is not the public release installer. If a signed/notarized `.pkg` is available and Gatekeeper blocks it, treat that as a release failure and rebuild through the signed/notarized CI path.
