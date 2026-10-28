#!/usr/bin/env bash
set -euo pipefail

fail() { echo "$*" >&2; exit 1; }
[[ $(uname -s) == Linux && $(uname -m) == x86_64 ]] || fail 'This check requires Linux x64.'
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
versions="$root/scripts/environment-versions.json"
rust=$(jq -r .rust "$versions")
android_home=${ANDROID_HOME:?Set ANDROID_HOME to the prepared SDK.}
[[ ${ANDROID_SDK_ROOT:-$android_home} == "$android_home" ]] || fail 'Android SDK paths disagree.'
tools="$android_home/cmdline-tools/$(jq -r .android.command_line_tools "$versions")/bin"
ndk="$android_home/ndk/$(jq -r .android.ndk "$versions")"
export PATH="$android_home/platform-tools:$android_home/emulator:$tools:$PATH"
image=$(jq -r .android.image "$versions")
[[ $(bun --version) == "$(jq -r .bun "$versions")" ]] || fail 'Unexpected Bun version.'
[[ $(javac -version 2>&1) == "javac $(jq -r .android.jdk "$versions")" ]] || fail 'Unexpected JDK version.'
[[ $(android --no-metrics --version) == "$(jq -r .android.cli "$versions")" ]] || fail 'Unexpected Android CLI version.'
while IFS=@ read -r package revision; do
    properties="$android_home/$package/source.properties"
    [[ -f $properties ]] || fail "Missing SDK package: $package"
    actual=$(sed -n 's/^Pkg.Revision *= *//p' "$properties" | tr -d '\r')
    # SDK properties may spell revision 2.0.0 as 2.
    actual=${actual%.0}; actual=${actual%.0}
    revision=${revision%.0}; revision=${revision%.0}
    [[ $actual == "$revision" ]] || fail "Unexpected $package revision: $actual"
done < <(jq -r '.android.packages[]' "$versions")
git --version
java -version
android --no-metrics "--sdk=$android_home" sdk list
adb version
emulator -version
emulator -accel-check
[[ -r /dev/kvm && -w /dev/kvm ]] || fail 'KVM is not accessible; hardware acceleration is required.'
rustc "+$rust" -vV
rustup target list --toolchain "$rust" --installed
bun -e 'console.log("Bun runtime OK")'

# Own all generated data; never reuse or remove a developer's AVD.
mkdir -p "$root/temp"
probe=$(mktemp -d "$root/temp/android-environment.XXXXXXXX")
avd_name=$(basename -- "$probe")
emulator_pid=''
cleanup() {
    status=$?
    trap - EXIT
    if [[ -n $emulator_pid ]]; then
        if [[ $status != 0 ]]; then
            tail -n 80 "$probe/emulator.log" >&2 || true
        fi
        # Own a separate process group. QEMU can ignore TERM even after a successful boot.
        kill -TERM -- "-$emulator_pid" 2>/dev/null || true
        timeout 10 tail --pid="$emulator_pid" -f /dev/null >/dev/null 2>&1 || true
        kill -KILL -- "-$emulator_pid" 2>/dev/null || true
        wait "$emulator_pid" 2>/dev/null || true
    fi
    rm -rf -- "$probe"
    echo 'Android probe cleanup completed.'
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
export ANDROID_AVD_HOME="$probe/avd"
mkdir -p "$ANDROID_AVD_HOME"
printf 'fn main() { println!("ZEDONE_ANDROID_TOOLCHAIN_OK"); }\n' > "$probe/main.rs"
clang="$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/clang"
"$clang" --version
for target in aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android; do
    clang_target=$target
    [[ $target != armv7-linux-androideabi ]] || clang_target=armv7a-linux-androideabi
    rustc "+$rust" --edition 2024 --target "$target" -C "linker=$clang" \
        -C "link-arg=--target=${clang_target}33" "$probe/main.rs" -o "$probe/$target"
done
printf 'no\n' | avdmanager create avd --name "$avd_name" --package "${image//\//;}" --device pixel_5
# Require a free port instead of touching an existing emulator.
ports=$(ss -H -ltn)
echo "$ports" | awk '{print $4}' | grep -Eq ':(5554|5555)$' && fail 'Emulator ports 5554/5555 are occupied.'
setsid emulator -avd "$avd_name" -port 5554 -no-window -no-audio -no-boot-anim \
    -no-snapshot -gpu software -accel on -memory 2048 -cores 2 > "$probe/emulator.log" 2>&1 &
emulator_pid=$!
booted=false
deadline=$((SECONDS + 300))
while (( SECONDS < deadline )); do
    kill -0 "$emulator_pid" 2>/dev/null || fail 'Emulator exited before boot.'
    if [[ $(timeout 10 adb -s emulator-5554 shell getprop sys.boot_completed 2>/dev/null | tr -d '\r') == 1 ]]; then
        booted=true
        break
    fi
    sleep 3
done
[[ $booted == true ]] || fail 'Emulator did not boot within 300 seconds.'
device_adb() { timeout 30 adb -s emulator-5554 "$@"; }
running_avd=$(device_adb emu avd name | tr -d '\r')
[[ ${running_avd%%$'\n'*} == "$avd_name" ]] || fail 'adb connected to a different AVD.'
[[ $(device_adb get-state) == device ]] || fail 'Emulator is not online.'
for entry in ro.build.version.release:13 ro.build.version.sdk:33 ro.product.cpu.abi:x86_64; do
    [[ $(device_adb shell getprop "${entry%:*}" | tr -d '\r') == "${entry#*:}" ]] || fail "Unexpected $entry"
done
gms=$(device_adb shell pm list packages com.google.android.gms)
[[ -z $gms ]] || fail 'AOSP must not contain Google Play Services.'
remote_probe="/data/local/tmp/$(basename -- "$probe")"
device_adb push "$probe/x86_64-linux-android" "$remote_probe"
device_adb shell chmod 700 "$remote_probe"
result=$(device_adb shell "$remote_probe" | tr -d '\r')
[[ $result == ZEDONE_ANDROID_TOOLCHAIN_OK ]] || fail "Unexpected probe result: $result"
device_adb shell rm -f "$remote_probe"
echo 'Android 13 / API 33 / x86_64 / GMS absent; ZEDONE_ANDROID_TOOLCHAIN_OK'
echo 'Android environment check passed.'

# Optional application check uses this owned AVD before the existing cleanup trap.
# Arguments are an executable and its argv, never an evaluated shell string.
if (( $# )); then
    ANDROID_SERIAL=emulator-5554 "$@"
fi
