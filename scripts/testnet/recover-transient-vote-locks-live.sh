#!/usr/bin/env bash
set -euo pipefail

node="${SYNERGY_NODE:-unknown-node}"
row="${SYNERGY_SPREADSHEET_ROW:-unknown-row}"
canonical_height="${SYNERGY_CANONICAL_HEIGHT:?set SYNERGY_CANONICAL_HEIGHT to the finalized canonical height}"
start_after="${SYNERGY_START_AFTER:-true}"
expected_runtime_sha="${SYNERGY_RUNTIME_SHA:-}"

workspace="${SYNERGY_WORKSPACE:-}"
if [[ -z "$workspace" ]]; then
  for candidate in \
    "$HOME/.synergy/testnet/nodes/validator-workspace" \
    "$HOME/.synergy/testnet/nodes/relayer-workspace" \
    "/opt/synergy/testnet/relayer" \
    "/opt/synergy/Node-RPC" \
    "/opt/synergy/Node-EXP"; do
    if [[ -d "$candidate" ]]; then
      workspace="$candidate"
      break
    fi
  done
fi

if [[ -z "$workspace" || ! -d "$workspace" ]]; then
  echo "unable to resolve workspace for $node" >&2
  exit 2
fi

data_dir="$workspace/data"
locks_file="$data_dir/consensus_vote_locks.json"
proposals_dir="$data_dir/consensus_proposals"
binary="$workspace/bin/synergy-testnet-linux-amd64"

ts="$(date -u +%Y%m%dT%H%M%SZ)"
backup_root="$HOME/synergy-testnet-state-backups"
backup="$backup_root/${ts}-${node// /_}-transient-vote-lock-recovery"
mkdir -p "$backup/data" "$backup/transient" "$backup/logs"

rpc_call() {
  local method="$1"
  local params="${2:-[]}"
  curl -sS --max-time 8 \
    -H "content-type: application/json" \
    --data "{\"jsonrpc\":\"2.0\",\"method\":\"${method}\",\"params\":${params},\"id\":1}" \
    "http://127.0.0.1:${SYNERGY_QRPC_PORT:-5640}" || true
}

rpc_call synergy_getLatestBlock > "$backup/latest_block_before.json"
rpc_call synergy_getCanonicalLock > "$backup/canonical_lock_before.json"
rpc_call synergy_getCommittedQC > "$backup/committed_qc_before.json"

for file in canonical_locks.json canonical_locks.jsonl committed_qcs.jsonl consensus_vote_locks.json; do
  [[ -f "$data_dir/$file" ]] && cp -p "$data_dir/$file" "$backup/data/$file"
done
if [[ -d "$proposals_dir" ]]; then
  tar -C "$data_dir" -cf "$backup/transient/consensus_proposals.before.tar" consensus_proposals
fi
find "$data_dir/logs" -maxdepth 1 -type f 2>/dev/null | while read -r log_file; do
  tail -500 "$log_file" > "$backup/logs/$(basename "$log_file").tail" 2>/dev/null || true
done

if [[ -f "$binary" && -n "$expected_runtime_sha" ]]; then
  actual_runtime_sha="$(sha256sum "$binary" | awk '{print $1}')"
  if [[ "$actual_runtime_sha" != "$expected_runtime_sha" ]]; then
    echo "runtime checksum mismatch before recovery: $actual_runtime_sha" >&2
    exit 3
  fi
fi

if [[ -x "$workspace/nodectl.sh" ]]; then
  (cd "$workspace" && ./nodectl.sh stop) || true
fi
for pid in $(pgrep -f "synergy-testnet-linux-amd64 start --config" || true); do
  proc_cwd="$(readlink "/proc/$pid/cwd" 2>/dev/null || true)"
  proc_exe="$(readlink "/proc/$pid/exe" 2>/dev/null || true)"
  if [[ "$proc_cwd" == "$workspace" || "$proc_exe" == "$workspace"/bin/* ]]; then
    kill "$pid" 2>/dev/null || true
  fi
done
sleep 2
for pid in $(pgrep -f "synergy-testnet-linux-amd64 start --config" || true); do
  proc_cwd="$(readlink "/proc/$pid/cwd" 2>/dev/null || true)"
  proc_exe="$(readlink "/proc/$pid/exe" 2>/dev/null || true)"
  if [[ "$proc_cwd" == "$workspace" || "$proc_exe" == "$workspace"/bin/* ]]; then
    kill -9 "$pid" 2>/dev/null || true
  fi
done

python3 - "$locks_file" "$canonical_height" "$backup/transient/removed_vote_locks_above_${canonical_height}.json" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
canonical_height = int(sys.argv[2])
removed_path = Path(sys.argv[3])
try:
    locks = json.loads(path.read_text()) if path.exists() else {}
except Exception:
    locks = {}
kept = {}
removed = {}
if isinstance(locks, dict):
    for key, value in locks.items():
        if not isinstance(value, dict):
            removed[key] = value
            continue
        try:
            block_index = int(
                value.get("block_index")
                or value.get("height")
                or value.get("block_height")
                or 0
            )
        except Exception:
            block_index = 0
        if block_index > canonical_height:
            removed[key] = value
        else:
            kept[key] = value
path.write_text(json.dumps(kept, indent=2, sort_keys=True) + "\n")
removed_path.write_text(json.dumps(removed, indent=2, sort_keys=True) + "\n")
print(f"kept={len(kept)} removed={len(removed)}")
PY

if [[ -d "$proposals_dir" ]]; then
  mkdir -p "$backup/transient/removed_consensus_proposals"
  find "$proposals_dir" -maxdepth 1 -type f -name '*.json' -print0 | while IFS= read -r -d '' proposal; do
    proposal_height="$(python3 - "$proposal" <<'PY'
import json
import sys
from pathlib import Path
try:
    data = json.loads(Path(sys.argv[1]).read_text())
    print(data.get("block_index") or data.get("height") or data.get("block_height") or "")
except Exception:
    print("")
PY
)"
    if [[ "$proposal_height" =~ ^[0-9]+$ && "$proposal_height" -gt "$canonical_height" ]]; then
      mv "$proposal" "$backup/transient/removed_consensus_proposals/"
    fi
  done
fi

if [[ "$start_after" == "true" ]]; then
  if [[ -x "$workspace/nodectl.sh" ]]; then
    (cd "$workspace" && ./nodectl.sh start)
  else
    mkdir -p "$data_dir/logs"
    (cd "$workspace" && nohup ./bin/synergy-testnet-linux-amd64 start --config config/node.toml >> data/logs/node.out 2>&1 &)
  fi
fi

sleep 3
post_sha=""
if [[ -f "$binary" ]]; then
  post_sha="$(sha256sum "$binary" | awk '{print $1}')"
fi
rpc_call synergy_getLatestBlock > "$backup/latest_block_after.json"
rpc_call synergy_getCanonicalLock > "$backup/canonical_lock_after.json"

echo "spreadsheet_row_used=true row=$row node=$node workspace=$workspace backup=$backup canonical_height=$canonical_height runtime_sha=$post_sha start_after=$start_after"
