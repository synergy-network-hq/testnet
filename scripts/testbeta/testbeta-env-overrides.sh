#!/usr/bin/env bash

testbeta_env_dir() {
  if [[ -n "${TESTBETA_ENV_DIR_RESOLVED:-}" && -d "${TESTBETA_ENV_DIR_RESOLVED:-}" ]]; then
    printf '%s\n' "$TESTBETA_ENV_DIR_RESOLVED"
    return 0
  fi

  local candidate
  for candidate in \
    "${SYNERGY_TESTBETA_ENV_DIR:-}" \
    "${TESTBETA_ENV_DIR_DEFAULT:-}" \
    "$HOME/Downloads/synergy-env-files"
  do
    if [[ -n "${candidate:-}" && -d "$candidate" ]]; then
      TESTBETA_ENV_DIR_RESOLVED="$candidate"
      printf '%s\n' "$candidate"
      return 0
    fi
  done

  return 1
}

testbeta_setup_package_dir() {
  if [[ -n "${TESTBETA_SETUP_PACKAGE_DIR_RESOLVED:-}" && -d "${TESTBETA_SETUP_PACKAGE_DIR_RESOLVED:-}" ]]; then
    printf '%s\n' "$TESTBETA_SETUP_PACKAGE_DIR_RESOLVED"
    return 0
  fi

  local candidate
  for candidate in \
    "${SYNERGY_TESTBETA_SETUP_PACKAGE_DIR:-}" \
    "$HOME/Desktop/setup-packages" \
    "$HOME/Desktop/deliverables/launch-assets/packages" \
    "$HOME/Desktop/deliverables/launch-assets/deliverables" \
    "$HOME/Desktop/Testnet-Beta/genesis-nodes" \
    "$HOME/Desktop/Testnet-Beta/synergy-address-engine/genesis-app/tmp/ceremony/launch-assets/packages"
  do
    if [[ -n "${candidate:-}" && -d "$candidate" ]]; then
      TESTBETA_SETUP_PACKAGE_DIR_RESOLVED="$candidate"
      printf '%s\n' "$candidate"
      return 0
    fi
  done

  return 1
}

testbeta_find_setup_package_file() {
  local filename="$1"
  local setup_dir
  setup_dir="$(testbeta_setup_package_dir)" || return 1

  local candidate
  candidate="$(find "$setup_dir" -type f -name "$filename" 2>/dev/null | sort | head -n 1)"
  [[ -n "$candidate" ]] || return 1
  printf '%s\n' "$candidate"
}

testbeta_env_value() {
  local file="$1"
  local key="$2"
  [[ -f "$file" ]] || return 1

  awk -v key="$key" '
    /^[[:space:]]*#/ || /^[[:space:]]*$/ { next }
    {
      line = $0
      sub(/\r$/, "", line)
      eq = index(line, "=")
      if (eq == 0) {
        next
      }
      name = substr(line, 1, eq - 1)
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", name)
      if (name != key) {
        next
      }
      value = substr(line, eq + 1)
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
      print value
      exit
    }
  ' "$file"
}

testbeta_env_has_key() {
  local file="$1"
  local key="$2"
  [[ -f "$file" ]] || return 1

  awk -v key="$key" '
    /^[[:space:]]*#/ || /^[[:space:]]*$/ { next }
    {
      line = $0
      sub(/\r$/, "", line)
      eq = index(line, "=")
      if (eq == 0) {
        next
      }
      name = substr(line, 1, eq - 1)
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", name)
      if (name == key) {
        found = 1
        exit
      }
    }
    END {
      exit(found ? 0 : 1)
    }
  ' "$file"
}

testbeta_first_nonempty() {
  local value
  for value in "$@"; do
    if [[ -n "${value:-}" ]]; then
      printf '%s\n' "$value"
      return 0
    fi
  done
  return 1
}

testbeta_env_file_for_validator_address() {
  local validator_address="$1"
  local env_dir
  env_dir="$(testbeta_env_dir)" || return 1
  local file
  for file in "$env_dir"/genesisval*.env; do
    [[ -f "$file" ]] || continue
    if [[ "$(testbeta_env_value "$file" "NODE_WALLET" || true)" == "$validator_address" ]]; then
      printf '%s\n' "$file"
      return 0
    fi
  done
  return 1
}

testbeta_validator_env_value() {
  local validator_address="$1"
  local key="$2"
  local fallback="${3:-}"
  local file value
  file="$(testbeta_env_file_for_validator_address "$validator_address" || true)"
  if [[ -n "$file" ]]; then
    value="$(testbeta_env_value "$file" "$key" || true)"
    if [[ -n "$value" ]]; then
      printf '%s\n' "$value"
      return 0
    fi
  fi
  printf '%s\n' "$fallback"
}

testbeta_env_file_for_bootnode_name() {
  local name="$1"
  local env_dir
  env_dir="$(testbeta_env_dir)" || return 1
  case "$name" in
    node-0a|Node-0A|bootnode1) printf '%s\n' "$env_dir/genesisboot1.env" ;;
    node-0b|Node-0B|bootnode2) printf '%s\n' "$env_dir/genesisboot2.env" ;;
    node-0c|Node-0C|bootnode3) printf '%s\n' "$env_dir/genesisboot3.env" ;;
    bootnode1) printf '%s\n' "$env_dir/genesisboot1.env" ;;
    bootnode2) printf '%s\n' "$env_dir/genesisboot2.env" ;;
    bootnode3) printf '%s\n' "$env_dir/genesisboot3.env" ;;
    *) return 1 ;;
  esac
}

testbeta_legacy_key_dir_for_inventory_node() {
  local normalized
  normalized="$(printf '%s' "${1:-}" | tr '[:upper:]' '[:lower:]')"

  case "$normalized" in
    genval-01|genesisval1|node-01) printf '%s\n' "node-01" ;;
    genval-02|genesisval2|node-02) printf '%s\n' "node-02" ;;
    genval-03|genesisval3|node-04) printf '%s\n' "node-04" ;;
    genval-04|genesisval4|node-06) printf '%s\n' "node-06" ;;
    genval-05|genesisval5|node-16) printf '%s\n' "node-16" ;;
    node-rpc|genesisrpc|node-13) printf '%s\n' "node-13" ;;
    node-exp|genesisindexer|node-14) printf '%s\n' "node-14" ;;
    *) return 1 ;;
  esac
}

testbeta_setup_package_file_for_inventory_node() {
  local normalized_slot normalized_role normalized_type
  normalized_slot="$(printf '%s' "${1:-}" | tr '[:upper:]' '[:lower:]')"
  normalized_type="$(printf '%s' "${2:-}" | tr '[:upper:]' '[:lower:]')"
  normalized_role="$(printf '%s' "${3:-}" | tr '[:upper:]' '[:lower:]' | tr '-' '_')"

  case "$normalized_slot" in
    genval-01|genesisval1|node-01) testbeta_find_setup_package_file "validator-1-setup-package.json" && return 0 ;;
    genval-02|genesisval2|node-02) testbeta_find_setup_package_file "validator-2-setup-package.json" && return 0 ;;
    genval-03|genesisval3|node-04) testbeta_find_setup_package_file "validator-3-setup-package.json" && return 0 ;;
    genval-04|genesisval4|node-06) testbeta_find_setup_package_file "validator-4-setup-package.json" && return 0 ;;
    genval-05|genesisval5|node-16) testbeta_find_setup_package_file "validator-5-setup-package.json" && return 0 ;;
    node-rpc|genesisrpc|node-13) testbeta_find_setup_package_file "rpc-gateway-setup-package.json" && return 0 ;;
    node-exp|genesisindexer|node-14) testbeta_find_setup_package_file "indexer-explorer-setup-package.json" && return 0 ;;
  esac

  case "$normalized_type:$normalized_role" in
    validator:validator) return 1 ;;
    rpc-gateway:rpc_gateway|rpc-gateway:rpc-gateway) testbeta_find_setup_package_file "rpc-gateway-setup-package.json" && return 0 ;;
    indexer:indexer|indexer:indexer_explorer) testbeta_find_setup_package_file "indexer-explorer-setup-package.json" && return 0 ;;
  esac

  return 1
}

testbeta_env_file_for_inventory_node() {
  local node_slot_id="$1"
  local node_type="$2"
  local validator_address="$3"
  local host="${4:-}"
  local env_dir
  local normalized_slot
  env_dir="$(testbeta_env_dir)" || return 1
  normalized_slot="$(printf '%s' "$node_slot_id" | tr '[:upper:]' '[:lower:]')"

  if [[ -n "$validator_address" ]]; then
    testbeta_env_file_for_validator_address "$validator_address" && return 0
  fi

  case "$node_type" in
    bootnode)
      testbeta_env_file_for_bootnode_name "$normalized_slot" && return 0
      ;;
    rpc-gateway)
      printf '%s\n' "$env_dir/genesisrpc.env"
      return 0
      ;;
    indexer)
      printf '%s\n' "$env_dir/genesisindexer.env"
      return 0
      ;;
  esac

  case "$normalized_slot" in
    node-0a|bootnode1) printf '%s\n' "$env_dir/genesisboot1.env" ; return 0 ;;
    node-0b|bootnode2) printf '%s\n' "$env_dir/genesisboot2.env" ; return 0 ;;
    node-0c|bootnode3) printf '%s\n' "$env_dir/genesisboot3.env" ; return 0 ;;
    genval-01|genesisval1|node-01) printf '%s\n' "$env_dir/genesisval01.env" ; return 0 ;;
    genval-02|genesisval2|node-02) printf '%s\n' "$env_dir/genesisval02.env" ; return 0 ;;
    genval-03|genesisval3|node-04) printf '%s\n' "$env_dir/genesisval03.env" ; return 0 ;;
    genval-04|genesisval4|node-06) printf '%s\n' "$env_dir/genesisval04.env" ; return 0 ;;
    genval-05|genesisval5|node-16) printf '%s\n' "$env_dir/genesisval05.env" ; return 0 ;;
    node-rpc|genesisrpc|node-13) printf '%s\n' "$env_dir/genesisrpc.env" ; return 0 ;;
    node-exp|genesisindexer|node-14) printf '%s\n' "$env_dir/genesisindexer.env" ; return 0 ;;
  esac

  if [[ -n "$host" ]]; then
    local file
    for file in "$env_dir"/*.env; do
      [[ -f "$file" ]] || continue
      if [[ "$(testbeta_env_value "$file" "HOSTNAME" || true)" == "$host" ]]; then
        printf '%s\n' "$file"
        return 0
      fi
    done
  fi

  return 1
}

testbeta_inventory_env_value() {
  local node_slot_id="$1"
  local node_type="$2"
  local validator_address="$3"
  local host="$4"
  local key="$5"
  local fallback="${6:-}"
  local file value
  file="$(testbeta_env_file_for_inventory_node "$node_slot_id" "$node_type" "$validator_address" "$host" || true)"
  if [[ -n "$file" ]]; then
    value="$(testbeta_env_value "$file" "$key" || true)"
    if [[ -n "$value" ]]; then
      printf '%s\n' "$value"
      return 0
    fi
  fi
  printf '%s\n' "$fallback"
}

testbeta_inventory_env_value_allow_empty() {
  local node_slot_id="$1"
  local node_type="$2"
  local validator_address="$3"
  local host="$4"
  local key="$5"
  local fallback="${6:-}"
  local file
  file="$(testbeta_env_file_for_inventory_node "$node_slot_id" "$node_type" "$validator_address" "$host" || true)"
  if [[ -n "$file" ]] && testbeta_env_has_key "$file" "$key"; then
    testbeta_env_value "$file" "$key" || true
    return 0
  fi
  printf '%s\n' "$fallback"
}

testbeta_bootnode_env_value() {
  local name="$1"
  local key="$2"
  local fallback="${3:-}"
  local file value
  file="$(testbeta_env_file_for_bootnode_name "$name" || true)"
  if [[ -n "$file" ]]; then
    value="$(testbeta_env_value "$file" "$key" || true)"
    if [[ -n "$value" ]]; then
      printf '%s\n' "$value"
      return 0
    fi
  fi
  printf '%s\n' "$fallback"
}
