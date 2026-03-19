#!/usr/bin/env bash
set -euo pipefail

BASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SEED_SRC_DIR="$BASE_DIR/seed2"
BOOTNODE_SRC_DIR="$BASE_DIR/bootnode2"
HELPER_SCRIPT="$BASE_DIR/restart-machine2-bootstrap-macos.sh"
SEED_DST_DIR="${SEED_DIR:-$HOME/seed2}"
BOOTNODE_DST_DIR="${BOOTNODE_DIR:-$HOME/bootnode2}"

require_dir() {
  local path="$1"
  if [[ ! -d "$path" ]]; then
    echo "Missing required directory: $path" >&2
    exit 1
  fi
}

copy_tree() {
  local src="$1"
  local dst="$2"
  mkdir -p "$dst"
  if command -v ditto >/dev/null 2>&1; then
    ditto "$src" "$dst"
    return
  fi

  cp -R "$src"/. "$dst"/
}

require_dir "$SEED_SRC_DIR"
require_dir "$BOOTNODE_SRC_DIR"
if [[ ! -f "$HELPER_SCRIPT" ]]; then
  echo "Missing helper script: $HELPER_SCRIPT" >&2
  exit 1
fi

printf '\n==> Installing seed2 into %s\n' "$SEED_DST_DIR"
copy_tree "$SEED_SRC_DIR" "$SEED_DST_DIR"

printf '\n==> Installing bootnode2 into %s\n' "$BOOTNODE_DST_DIR"
copy_tree "$BOOTNODE_SRC_DIR" "$BOOTNODE_DST_DIR"

chmod +x "$HELPER_SCRIPT"
printf '\n==> Running seed2 + bootnode2 restart workflow\n'
SEED_DIR="$SEED_DST_DIR" BOOTNODE_DIR="$BOOTNODE_DST_DIR" "$HELPER_SCRIPT"
