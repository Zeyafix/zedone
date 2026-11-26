#!/usr/bin/env bash
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
mkdir -p "$root/temp"
fixture=$(mktemp -d "$root/temp/android-shell-preparation.XXXXXXXX")
cleanup() {
    status=$?
    if (( status != 0 )); then cat "$fixture/"*.log 2>/dev/null || true; fi
    [[ $fixture == "$root/temp/android-shell-preparation."* ]] || exit 99
    rm -rf -- "$fixture"
    exit "$status"
}
trap cleanup EXIT
mkdir -p "$fixture/bin"
cat > "$fixture/stub" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
tool=${0##*/}
printf '%s' "$tool" >> "$CALLS"
printf ' <%s>' "$@" >> "$CALLS"
printf '\n' >> "$CALLS"
if [[ $tool == jq ]]; then printf '%s\n' "$TEST_RUST"; fi
if [[ $tool == cargo ]]; then test "$CARGO_TARGET_DIR" = "$TEST_ROOT/target"; fi
if [[ $tool == "${FAIL_TOOL:-}" ]]; then exit 23; fi
STUB
for tool in chmod bun jq cargo; do
    cp "$fixture/stub" "$fixture/bin/$tool"
    chmod +x "$fixture/bin/$tool"
done
export CALLS="$fixture/calls.log" TEST_ROOT="$root" TEST_RUST=fixture-rust
export PATH="$fixture/bin:$PATH"
helper="$root/scripts/prepare-android-shell-ci.sh"
for suite in shell pairing-agent pairing-tags pairing-fields pairing-fields-space pairing-fields-column pairing-history pairing-placements pairing-container-deletion pairing-positions-cross pairing-positions-same; do
    : > "$CALLS"
    CI=true bash "$helper" "$suite"
    printf '%s\n' 'chmod <+x> <ui/src-tauri/gen/android/gradlew>' 'bun <scripts/build-ui-android.mjs> <x86_64>' > "$fixture/expected.log"
    if [[ $suite == shell ]]; then
        printf '%s\n' 'jq <-r> <.rust> <scripts/environment-versions.json>' "cargo <+$TEST_RUST> <build> <--manifest-path> <ui/src-tauri/Cargo.toml> <--example> <shell-fixture> <--locked> <--offline>" >> "$fixture/expected.log"
    fi
    cmp "$CALLS" "$fixture/expected.log"
done
for args in '' 'unknown' 'pairing' 'shell-arm64' 'shell extra'; do
    : > "$CALLS"
    read -ra invalid <<< "$args"
    result=0
    CI=true bash "$helper" "${invalid[@]}" > "$fixture/rejection.log" 2>&1 || result=$?
    test "$result" -eq 1
    test ! -s "$CALLS"
done
result=0
CI=false bash "$helper" shell > "$fixture/rejection.log" 2>&1 || result=$?
test "$result" -eq 1
test ! -s "$CALLS"
for FAIL_TOOL in chmod bun cargo; do
    export FAIL_TOOL
    : > "$CALLS"
    result=0
    CI=true bash "$helper" shell > "$fixture/failure.log" 2>&1 || result=$?
    test "$result" -eq 23
    test "$(tail -n 1 "$CALLS" | cut -d ' ' -f 1)" = "$FAIL_TOOL"
done
echo 'Android shell preparation: exact suite builds, CI/argument rejection and child failures passed.'
