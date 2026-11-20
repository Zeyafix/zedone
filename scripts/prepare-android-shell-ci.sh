#!/usr/bin/env bash
set -euo pipefail

[[ ${CI:-} == true ]] || { echo 'This preparation is CI-only.' >&2; exit 1; }
(( $# == 1 )) || { echo 'Expected one Android shell suite.' >&2; exit 1; }
case $1 in
    shell|pairing-agent|pairing-tags|pairing-fields|pairing-placements|pairing-positions-cross|pairing-positions-same) ;;
    *) echo 'Unknown Android shell suite.' >&2; exit 1 ;;
esac
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
chmod +x ui/src-tauri/gen/android/gradlew
bun scripts/build-ui-android.mjs x86_64
if [[ $1 == shell ]]; then
    rust=$(jq -r .rust scripts/environment-versions.json)
    CARGO_TARGET_DIR="$PWD/target" cargo "+$rust" build --manifest-path ui/src-tauri/Cargo.toml --example shell-fixture --locked --offline
fi
