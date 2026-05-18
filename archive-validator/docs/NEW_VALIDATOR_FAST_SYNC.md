# New Validator Fast Sync

A new validator in `SYNCING` selects the highest verified archive snapshot at or below the latest finalized height, verifies it, speed-syncs remaining finalized blocks, then enters `SNAPSHOT_VERIFIED` and `REPLAYING` before `SHADOW`.
