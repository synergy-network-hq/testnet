# Relayer Snapshot Retrieval

Relayers retrieve archive snapshots through the operator-configured WireGuard path. Do not publish archive WireGuard private configs or archive signing keys in release artifacts.

Use the archive validator's relayer-facing WireGuard hostname or private address supplied by the operator:

```bash
ARCHIVE_URL="http://<archive-wireguard-host>:48640"
curl --fail --show-error --location \
  "${ARCHIVE_URL}/catalog.json" \
  --output catalog.json
curl --fail --show-error --location \
  "${ARCHIVE_URL}/snapshots/<snapshot-name>.tar.zst" \
  --output snapshot.tar.zst
curl --fail --show-error --location \
  "${ARCHIVE_URL}/snapshots/<snapshot-name>-manifest.json" \
  --output snapshot-manifest.json
```

Verify the downloaded snapshot through supported tooling before retrieval is used for any recovery or validator sync workflow:

```bash
synergy-node verify-snapshot \
  --manifest ./snapshot-manifest.json \
  --snapshot-root ./snapshot-staging \
  --chain-id 1264 \
  --network-id synergy-testnet-v2
```

The snapshot API and catalog publication commands currently remain fail-closed until verified archive serving is fully wired. Treat retrieval as operator guidance for the relayer-facing path, not as proof that serving is live.
