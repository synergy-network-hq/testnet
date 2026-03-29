#!/usr/bin/env bash
set -euo pipefail

SEED_DIR="${SEED_DIR:-$HOME/seed2}"
BOOTNODE_DIR="${BOOTNODE_DIR:-$HOME/bootnode2}"
EXPECTED_SEED_SHA256="62f0510f59435f20216d572ada9b1db587c826ccca010e4eae28856a991d1aed"

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "Missing required file: $path" >&2
    exit 1
  fi
}

print_step() {
  printf '\n==> %s\n' "$1"
}

require_file "$SEED_DIR/seed_service.py"
require_file "$SEED_DIR/install_and_start.sh"
require_file "$SEED_DIR/nodectl.sh"
require_file "$BOOTNODE_DIR/install_and_start.sh"
require_file "$BOOTNODE_DIR/nodectl.sh"
require_file "$BOOTNODE_DIR/bin/synergy-testbeta-darwin-arm64"

print_step "Checking seed2 bundle"
actual_seed_sha="$(shasum -a 256 "$SEED_DIR/seed_service.py" | awk '{print $1}')"
echo "seed2 sha256: $actual_seed_sha"
if [[ "$actual_seed_sha" != "$EXPECTED_SEED_SHA256" ]]; then
  echo "seed2 bundle is stale. Expected: $EXPECTED_SEED_SHA256" >&2
  exit 1
fi
grep -n "do_DELETE\|/peers/clear\|POST /peers/clear" "$SEED_DIR/seed_service.py"

print_step "Restarting seed2"
chmod +x "$SEED_DIR/install_and_start.sh" "$SEED_DIR/nodectl.sh" "$SEED_DIR/seed_service.py"
xattr -dr com.apple.quarantine "$SEED_DIR" 2>/dev/null || true
"$SEED_DIR/nodectl.sh" stop || true
pkill -f 'seed_service.py' || true
python3 -m py_compile "$SEED_DIR/seed_service.py"
"$SEED_DIR/install_and_start.sh"
sleep 2
curl -i http://127.0.0.1:5621/healthz
curl -i -X DELETE http://127.0.0.1:5621/peers
"$SEED_DIR/nodectl.sh" status
"$SEED_DIR/nodectl.sh" logs

print_step "Checking bootnode2 bundle"
arch="$(uname -m)"
echo "machine architecture: $arch"
if [[ "$arch" != "arm64" ]]; then
  echo "bootnode2 currently requires an Apple Silicon Mac. Found: $arch" >&2
  exit 1
fi
grep -n "xattr -dr com.apple.quarantine" "$BOOTNODE_DIR/install_and_start.sh"

print_step "Restarting bootnode2"
chmod +x \
  "$BOOTNODE_DIR/install_and_start.sh" \
  "$BOOTNODE_DIR/nodectl.sh" \
  "$BOOTNODE_DIR/bin/synergy-testbeta-darwin-arm64"
xattr -dr com.apple.quarantine "$BOOTNODE_DIR" 2>/dev/null || true
codesign --force --sign - "$BOOTNODE_DIR/bin/synergy-testbeta-darwin-arm64"
"$BOOTNODE_DIR/nodectl.sh" stop || true
"$BOOTNODE_DIR/install_and_start.sh"
"$BOOTNODE_DIR/nodectl.sh" status
"$BOOTNODE_DIR/nodectl.sh" logs

print_step "Completed"
echo "seed2 and bootnode2 were restarted from:"
echo "  $SEED_DIR"
echo "  $BOOTNODE_DIR"
