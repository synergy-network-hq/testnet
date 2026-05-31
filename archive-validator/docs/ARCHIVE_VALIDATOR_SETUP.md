# Archive Validator Setup

The archive validator must run as `ARCHIVE_OBSERVER` on chain `1264`, network `synergy-testnet-v2`.

It fails closed when `aegis-pqvm` is unavailable, when genesis hash validation fails, or when archive/snapshot signing identities cannot be verified.

Archive snapshots are scheduled every `5000` finalized blocks. Retention is exactly two snapshots. Initial creation and scheduled creation use the supported `synergy-node create-snapshot` diagnostics tooling and require explicit majority-branch proof.

Relayer retrieval guidance is in `RELAYER_SNAPSHOT_RETRIEVAL.md`.
