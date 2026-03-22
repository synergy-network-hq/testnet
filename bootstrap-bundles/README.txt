Bootstrap bundles
=================

Contents
- bootnode1, bootnode2, bootnode3: bootstrap-only Synergy node bundles
- seed1, seed2, seed3: lightweight peer-list publisher services
- DNS_RECORDS.txt: DNS records to create in Cloudflare or another DNS provider

How to rebuild
1. Populate /Users/devpup/Desktop/Testnet-Beta/synergy-testnet-beta/binaries with synergy-testbeta binaries for darwin-arm64, linux-amd64, and windows-amd64.
2. Run: ./scripts/testbeta/build-bootstrap-bundles.sh

Output directory
- /Users/devpup/Desktop/Testnet-Beta/synergy-testnet-beta/bootstrap-bundles
