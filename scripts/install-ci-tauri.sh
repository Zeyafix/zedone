#!/usr/bin/env bash
# The CI cache contains only this tool's binary, never Cargo/rustup or signing state.
set -euo pipefail
[[ $# == 0 ]] || { echo 'Expected no arguments.' >&2; exit 1; }
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
versions="$root/scripts/environment-versions.json"
rust=$(jq -er .rust "$versions")
cli=$(jq -er .android.tauri_cli "$versions")
# Resolve the existing temporary parent before creating/removing anything below it.
runner_temp=$(cd -- "${RUNNER_TEMP:?RUNNER_TEMP is required}" && pwd -P)
[[ $runner_temp != / ]] || { echo 'RUNNER_TEMP must not be the filesystem root.' >&2; exit 1; }
install_root="$runner_temp/zedone-tauri-cli"
binary="$install_root/bin/cargo-tauri"
expected="tauri-cli $cli"

if [[ -x $binary ]] && [[ $("$binary" --version) == "$expected" ]]; then
    echo "Reusing $expected."
    exit 0
fi

build_dir=$(mktemp -d "$runner_temp/zedone-tauri-build.XXXXXXXX")
cleanup() {
    status=$?
    trap - EXIT
    du -sh -- "$build_dir" || true
    case "$build_dir" in
        "$runner_temp"/zedone-tauri-build.*) rm -rf -- "$build_dir" ;;
        *) echo 'Refusing to remove an unexpected build directory.' >&2; exit 1 ;;
    esac
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# Override both Cargo output locations for this invocation; always discard the
# large install build after success/failure instead of retaining it beside APKs.
CARGO_TARGET_DIR="$build_dir/target" CARGO_BUILD_BUILD_DIR="$build_dir/build" \
    cargo "+$rust" install tauri-cli --version "$cli" --locked --force --root "$install_root"
[[ $("$binary" --version) == "$expected" ]] || { echo 'Unexpected installed Tauri CLI version.' >&2; exit 1; }
echo "Installed $expected."
