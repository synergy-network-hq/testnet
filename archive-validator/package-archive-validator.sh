#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIST_DIR="${ROOT_DIR}/dist"
GENERIC_ARTIFACT="${ROOT_DIR}/synergy-archive-validator-testnet-v2.zip"
LINUX_ARTIFACT="${ROOT_DIR}/synergy-archive-validator-testnet-v2-linux-x64.zip"
MACOS_ARTIFACT="${ROOT_DIR}/synergy-archive-validator-testnet-v2-macos-universal.zip"
MACOS_EXTRACTED_ARTIFACT="${ROOT_DIR}/synergy-archive-validator-testnet-v2-macos-extracted.zip"
TARGET="linux"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --linux) TARGET="linux"; shift ;;
    --macos) TARGET="macos"; shift ;;
    --macos-extracted) TARGET="macos-extracted"; shift ;;
    --all) TARGET="all"; shift ;;
    *) echo "Unknown argument: $1" >&2; exit 1 ;;
  esac
done

forbidden_files="$(
  find "${ROOT_DIR}" \
    \( -name '*.key' -o -name '*.pem' -o -name '*.p12' -o -name '.env' -o -name 'id_*' \) \
    -type f ! -name '.env.example' -print
)"
forbidden_dirs="$(
  find "${ROOT_DIR}" \
    \( -path '*/data/*' -o -path '*/snapshots/*' -o -path '*/logs/*' -o -path '*/evidence/*' -o -path '*/chain-data/*' \) \
    -type f -print
)"
wireguard_configs="$(
  find "${ROOT_DIR}" \
    -path '*/config/wireguard/*.conf' \
    -type f -print
)"
if [[ -n "${forbidden_files}${forbidden_dirs}${wireguard_configs}" ]]; then
  echo "Refusing to package private keys, identity files, secrets, WireGuard configs, chain data, snapshots, logs, or evidence." >&2
  printf '%s\n%s\n%s\n' "${forbidden_files}" "${forbidden_dirs}" "${wireguard_configs}" >&2
  exit 1
fi

mkdir -p "${DIST_DIR}"

zip_source_tree() {
  local artifact="$1"
  rm -f "${artifact}"
  (
    cd "${ROOT_DIR}/.."
    COPYFILE_DISABLE=1 zip -qr "${artifact}" archive-validator \
      -x 'archive-validator/synergy-archive-validator-testnet-v2*.zip' \
      -x 'archive-validator/dist/*' \
      -x 'archive-validator/macos/dist/*' \
      -x 'archive-validator/macos/build/*' \
      -x 'archive-validator/**/.DS_Store' \
      -x 'archive-validator/**/*.key' \
      -x 'archive-validator/**/*.pem' \
      -x 'archive-validator/**/*.p12' \
      -x 'archive-validator/**/id_*' \
      -x 'archive-validator/.env' \
      -x 'archive-validator/**/config/wireguard/*.conf' \
      -x 'archive-validator/**/data/**' \
      -x 'archive-validator/**/snapshots/**' \
      -x 'archive-validator/**/logs/**' \
      -x 'archive-validator/**/evidence/**'
  )
}

package_linux() {
  rm -f "${LINUX_ARTIFACT}" "${GENERIC_ARTIFACT}"
  zip_source_tree "${LINUX_ARTIFACT}"
  cp "${LINUX_ARTIFACT}" "${GENERIC_ARTIFACT}"
  (cd "${ROOT_DIR}" && shasum -a 256 "$(basename "${LINUX_ARTIFACT}")" > "${DIST_DIR}/SHA256SUMS.linux")
  (cd "${ROOT_DIR}" && shasum -a 256 "$(basename "${GENERIC_ARTIFACT}")" > "${DIST_DIR}/SHA256SUMS")
  echo "Created ${LINUX_ARTIFACT}"
  echo "Created ${GENERIC_ARTIFACT}"
}

package_macos_extracted() {
  zip_source_tree "${MACOS_EXTRACTED_ARTIFACT}"
  (cd "${ROOT_DIR}" && shasum -a 256 "$(basename "${MACOS_EXTRACTED_ARTIFACT}")" > "${DIST_DIR}/SHA256SUMS.macos-extracted")
  echo "Created ${MACOS_EXTRACTED_ARTIFACT}"
}

package_macos() {
  "${ROOT_DIR}/macos/build-macos-pkg.sh"
  local pkg="${ROOT_DIR}/macos/dist/SynergyArchiveValidator.pkg"
  [[ -f "${pkg}" ]] || { echo "macOS package was not produced" >&2; exit 1; }
  rm -f "${MACOS_ARTIFACT}"
  rm -rf "${DIST_DIR}/macos-universal"
  mkdir -p "${DIST_DIR}/macos-universal"
  install -m 0644 "${pkg}" "${DIST_DIR}/macos-universal/SynergyArchiveValidator.pkg"
  install -m 0644 "${ROOT_DIR}/macos/README-GATEKEEPER.md" "${DIST_DIR}/macos-universal/README-GATEKEEPER.md"
  install -m 0644 "${ROOT_DIR}/docs/MACOS_INSTALL.md" "${DIST_DIR}/macos-universal/MACOS_INSTALL.md"
  (cd "${DIST_DIR}/macos-universal" && shasum -a 256 SynergyArchiveValidator.pkg > SHA256SUMS)
  (cd "${DIST_DIR}/macos-universal" && zip -r "${MACOS_ARTIFACT}" SynergyArchiveValidator.pkg SHA256SUMS README-GATEKEEPER.md MACOS_INSTALL.md)
  echo "Created ${MACOS_ARTIFACT}"
}

case "${TARGET}" in
  linux) package_linux ;;
  macos) package_macos ;;
  macos-extracted) package_macos_extracted ;;
  all) package_linux; package_macos_extracted; package_macos ;;
esac
