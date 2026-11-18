#!/usr/bin/env bash
set -euo pipefail

[[ ${CI:-} == true ]] || { echo 'This preparation is CI-only.' >&2; exit 1; }
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
rust=$(jq -r .rust "$root/scripts/environment-versions.json")
cd "$root/prototypes"
CARGO_TARGET_DIR="$PWD/target" cargo "+$rust" build --release --locked --offline --bin serve-echo
bun run android:build
