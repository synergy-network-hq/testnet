#!/usr/bin/env bash
set -euo pipefail
height="${1:---latest-eligible}"
synergy-archive create-snapshot "${height}"
