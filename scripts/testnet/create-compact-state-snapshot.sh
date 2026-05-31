#!/usr/bin/env bash
set -euo pipefail

node="${SYNERGY_NODE:-unknown-node}"
row="${SYNERGY_SPREADSHEET_ROW:-unknown-row}"
workspace="${SYNERGY_WORKSPACE:-$HOME/.synergy/testnet/nodes/validator-workspace}"
out_root="${SYNERGY_SNAPSHOT_ROOT:-$HOME/synergy-testnet-snapshots}"
mkdir -p "$out_root"

data_dir="$workspace/data"
test -d "$data_dir"

height="$(
  python3 - "$data_dir/canonical_locks.json" "$data_dir/canonical_locks.jsonl" <<'PY'
import json
import sys
from pathlib import Path
compact = Path(sys.argv[1])
journal = Path(sys.argv[2])
data = json.loads(compact.read_text()) if compact.is_file() else {}
if journal.is_file():
    for line in journal.read_text().splitlines():
        if not line.strip():
            continue
        lock = json.loads(line)
        key = str(lock["height"])
        if key in data and data[key]["block_hash"] != lock["block_hash"]:
            raise SystemExit(f"conflicting canonical lock journal entry at height {key}")
        data[key] = lock
print(max(int(key) for key in data.keys()))
PY
)"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
snapshot="$out_root/majority-${node// /_}-h${height}-compact-${timestamp}.tar"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/data"

for file in chain.json dag_state.json validator_registry.json token_state.json; do
  test -f "$data_dir/$file"
  cp -p "$data_dir/$file" "$tmp/data/$file"
done
python3 - "$data_dir/canonical_locks.json" "$data_dir/canonical_locks.jsonl" "$tmp/data/canonical_locks.json" <<'PY'
import json
import sys
from pathlib import Path
compact = Path(sys.argv[1])
journal = Path(sys.argv[2])
target = Path(sys.argv[3])
data = json.loads(compact.read_text()) if compact.is_file() else {}
if journal.is_file():
    for line in journal.read_text().splitlines():
        if not line.strip():
            continue
        lock = json.loads(line)
        key = str(lock["height"])
        if key in data and data[key]["block_hash"] != lock["block_hash"]:
            raise SystemExit(f"conflicting canonical lock journal entry at height {key}")
        data[key] = lock
target.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
PY
: > "$tmp/data/canonical_locks.jsonl"
test -f "$data_dir/committed_qcs.jsonl"
tail -200 "$data_dir/committed_qcs.jsonl" > "$tmp/data/committed_qcs.jsonl"

tar -C "$tmp" -cf "$snapshot" data
sha="$(sha256sum "$snapshot" | awk '{print $1}')"
size="$(wc -c < "$snapshot")"
echo "spreadsheet_row_used=true row=$row node=$node snapshot=$snapshot height=$height sha256=$sha size_bytes=$size"
