#!/usr/bin/env bash
set -euo pipefail
./scripts/verify-aegis-pqvm.sh
install -d -m 0750 /var/lib/synergy/archive-validator/keys
printf '%s\n' 'aegis-pqvm:ARCHIVE_PEER' > /var/lib/synergy/archive-validator/keys/aegis-archive-peer-key.ref
printf '%s\n' 'aegis-pqvm:ARCHIVE_SNAPSHOT_SIGNER' > /var/lib/synergy/archive-validator/keys/aegis-snapshot-signing-key.ref
echo "Aegis archive key references initialized; raw private keys are not stored in this package."
