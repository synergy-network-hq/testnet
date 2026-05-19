# Synergy Archive Validator macOS Gatekeeper Notes

The normal macOS distribution artifact is `SynergyArchiveValidator.pkg` inside `synergy-archive-validator-testnet-v2-macos-universal.zip`.

The package must be signed with the Synergy Developer ID Installer certificate, all executables inside it must be signed with the Synergy Developer ID Application certificate using hardened runtime, and the package must be notarized and stapled before release.

Operators should not need to run `xattr -dr com.apple.quarantine`, disable Gatekeeper, or approve unsigned binaries manually. If Gatekeeper blocks the package, treat that as a release failure and rebuild through the signed/notarized CI path.
