#!/usr/bin/env bash
# Exercise the installer without downloading or compiling tools.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
mkdir -p "$root/temp"
probe=$(mktemp -d "$root/temp/ci-tauri-test.XXXXXXXX")
cleanup() {
    status=$?
    trap - EXIT
    case "$probe" in
        "$root"/temp/ci-tauri-test.*) rm -rf -- "$probe" ;;
        *) echo 'Refusing to remove an unexpected test directory.' >&2; exit 1 ;;
    esac
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
mkdir "$probe/bin"
export CI_TAURI_TEST_RUST CI_TAURI_TEST_CLI
CI_TAURI_TEST_RUST=$(jq -er .rust "$root/scripts/environment-versions.json")
CI_TAURI_TEST_CLI=$(jq -er .android.tauri_cli "$root/scripts/environment-versions.json")
cat > "$probe/bin/cargo" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
[[ $# == 9 && $1 == "+$CI_TAURI_TEST_RUST" && $2 == install && $3 == tauri-cli &&
   $4 == --version && $5 == "$CI_TAURI_TEST_CLI" && $6 == --locked &&
   $7 == --force && $8 == --root && $9 == "$RUNNER_TEMP/zedone-tauri-cli" ]]
[[ $CARGO_TARGET_DIR == "$RUNNER_TEMP"/zedone-tauri-build.*/target ]]
[[ $CARGO_BUILD_BUILD_DIR == "${CARGO_TARGET_DIR%/target}/build" ]]
echo called >> "$RUNNER_TEMP/cargo-calls"
mkdir -p "$CARGO_TARGET_DIR" "$CARGO_BUILD_BUILD_DIR"
touch "$CARGO_TARGET_DIR/partial" "$CARGO_BUILD_BUILD_DIR/partial"
[[ ${CI_TAURI_TEST_MODE:-ok} != fail ]] || exit 23
mkdir -p "$9/bin"
version=$CI_TAURI_TEST_CLI
[[ ${CI_TAURI_TEST_MODE:-ok} != wrong-version ]] || version=0.0.0
printf '#!/usr/bin/env bash\nprintf "tauri-cli %s\\n"\n' "$version" > "$9/bin/cargo-tauri"
chmod +x "$9/bin/cargo-tauri"
STUB
chmod +x "$probe/bin/cargo"
export PATH="$probe/bin:$PATH"

new_case() {
    export RUNNER_TEMP="$probe/$1"
    mkdir "$RUNNER_TEMP"
    echo preserve > "$RUNNER_TEMP/sentinel"
}
check_cleanup() {
    [[ -z $(find "$RUNNER_TEMP" -maxdepth 1 -name 'zedone-tauri-build.*' -print -quit) ]]
    [[ $(cat "$RUNNER_TEMP/sentinel") == preserve ]]
}
install() { bash "$root/scripts/install-ci-tauri.sh"; }

new_case 'cold cache'
install
[[ $(wc -l < "$RUNNER_TEMP/cargo-calls") == 1 ]]
check_cleanup
echo 'PASS: cache miss installs pinned CLI and cleans both build directories.'

export CI_TAURI_TEST_MODE=fail
install
[[ $(wc -l < "$RUNNER_TEMP/cargo-calls") == 1 ]]
check_cleanup
echo 'PASS: valid cache does not invoke Cargo.'

export CI_TAURI_TEST_MODE=ok
printf '#!/usr/bin/env bash\necho "tauri-cli 0.0.0"\n' > "$RUNNER_TEMP/zedone-tauri-cli/bin/cargo-tauri"
install
[[ $(wc -l < "$RUNNER_TEMP/cargo-calls") == 2 ]]
check_cleanup
echo 'PASS: stale cached version is replaced.'

printf '#!/usr/bin/env bash\nexit 42\n' > "$RUNNER_TEMP/zedone-tauri-cli/bin/cargo-tauri"
install
[[ $(wc -l < "$RUNNER_TEMP/cargo-calls") == 3 ]]
check_cleanup
echo 'PASS: broken cached executable is replaced.'

new_case failure
export CI_TAURI_TEST_MODE=fail
status=0
install || status=$?
[[ $status == 23 ]]
check_cleanup
echo 'PASS: compiler failure keeps exit 23 and cleans partial build output.'

new_case 'wrong installed version'
export CI_TAURI_TEST_MODE=wrong-version
status=0
install || status=$?
[[ $status == 1 ]]
check_cleanup
echo 'PASS: installation with an unexpected CLI version fails.'

status=0
RUNNER_TEMP=/ install || status=$?
[[ $status == 1 ]]
status=0
bash "$root/scripts/install-ci-tauri.sh" unexpected || status=$?
[[ $status == 1 ]]
echo 'PASS: filesystem root and unexpected arguments are rejected.'
