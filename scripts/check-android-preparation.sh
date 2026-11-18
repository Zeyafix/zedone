#!/usr/bin/env bash
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
command -v setsid >/dev/null
mkdir -p "$root/temp"
fixture=$(mktemp -d "$root/temp/android-preparation.XXXXXXXX")
controller=''
cleanup() {
    status=$?
    if [[ -n $controller ]]; then kill -TERM -- "-$controller" 2>/dev/null || true; wait "$controller" 2>/dev/null || true; fi
    if (( status != 0 )); then cat "$fixture/"*.log 2>/dev/null || true; fi
    for pid_file in "$fixture/"*/preparation.pid "$fixture/"*/emulator.pid; do
        [[ -f $pid_file ]] || continue
        kill -KILL -- "-$(cat "$pid_file")" 2>/dev/null || true
    done
    [[ $fixture == "$root/temp/android-preparation."* ]] || exit 99
    rm -rf -- "$fixture"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
mkdir -p "$fixture/scripts" "$fixture/bin"
cp "$root/scripts/check-android-environment.sh" "$fixture/scripts/"
cp "$root/scripts/environment-versions.json" "$fixture/scripts/"
printf '#!/usr/bin/env bash\nexit 0\n' > "$fixture/scripts/check-android-sdk.sh"
cat > "$fixture/stub" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
case ${0##*/} in
    uname) if [[ $1 == -s ]]; then echo Linux; else echo x86_64; fi ;;
    javac) echo "javac $TEST_JDK" ;;
    bun) if [[ ${1:-} == --version ]]; then echo "$TEST_BUN"; fi ;;
    git|java|rustc|rustup|android|clang|ss) exit 0 ;;
    avdmanager) cat >/dev/null ;;
    emulator)
        [[ ${1:-} != -version && ${1:-} != -accel-check ]] || exit 0
        echo $$ > "$CASE/emulator.pid"
        if [[ $MODE == boot-failure ]]; then
            while [[ ! -e $CASE/preparation.started ]]; do sleep 0.02; done
            exit 17
        fi
        while :; do sleep 60; done ;;
    adb)
        [[ ${1:-} != version ]] || exit 0
        shift 2
        case "$*" in
            'shell getprop sys.boot_completed')
                [[ $MODE != boot-failure && -e $CASE/emulator.pid && -e $CASE/preparation.started ]] || exit 1
                if [[ $MODE == build-first && ! -e $CASE/preparation.done ]]; then exit 1; fi
                echo 1 ;;
            'shell getprop ro.build.version.release') echo 13 ;;
            'shell getprop ro.build.version.sdk') echo 33 ;;
            'shell getprop ro.product.cpu.abi') echo x86_64 ;;
            'emu avd name') basename "$(dirname "$ANDROID_AVD_HOME")" ;;
            get-state) echo device ;;
            shell\ /data/local/tmp/android-environment.*)
                touch "$CASE/boot.proved"
                echo ZEDONE_ANDROID_TOOLCHAIN_OK ;;
        esac ;;
    *) exit 98 ;;
esac
STUB
for tool in uname javac bun git java rustc rustup android clang ss avdmanager emulator adb; do
    cp "$fixture/stub" "$fixture/bin/$tool"
    chmod +x "$fixture/bin/$tool"
done
cat > "$fixture/prepare" <<'PREPARE'
#!/usr/bin/env bash
set -euo pipefail
echo $$ > "$CASE/preparation.pid"
touch "$CASE/preparation.started"
if [[ $MODE == failure ]]; then exit 23; fi
if [[ $MODE == signal || $MODE == boot-failure ]]; then
    sleep 120 &
    echo $! > "$CASE/descendant.pid"
    wait
fi
if [[ $MODE == boot-first ]]; then
    while [[ ! -e $CASE/boot.proved ]]; do sleep 0.02; done
fi
touch "$CASE/preparation.done"
PREPARE
cat > "$fixture/runtime" <<'RUNTIME'
#!/usr/bin/env bash
set -euo pipefail
test -e "$CASE/boot.proved"
test -e "$CASE/preparation.done"
touch "$CASE/runtime.called"
if [[ $MODE == runtime-failure ]]; then exit 31; fi
RUNTIME
export TEST_JDK TEST_BUN
TEST_JDK=$(jq -r .android.jdk "$root/scripts/environment-versions.json")
TEST_BUN=$(jq -r .bun "$root/scripts/environment-versions.json")
ndk=$(jq -r .android.ndk "$root/scripts/environment-versions.json")
sdk="$fixture/sdk"
mkdir -p "$sdk/ndk/$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin"
cp "$fixture/bin/clang" "$sdk/ndk/$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/clang"
export CI=true ANDROID_HOME="$sdk" ANDROID_SDK_ROOT="$sdk"
export PATH="$fixture/bin:$PATH"
running() { local state; state=$(ps -o stat= -p "$1" 2>/dev/null || true); [[ -n $state && $state != Z* ]]; }
for MODE in build-first boot-first failure boot-failure signal runtime-failure; do
    export MODE CASE="$fixture/$MODE"
    mkdir -p "$CASE"
    setsid bash "$fixture/scripts/check-android-environment.sh" --prepare bash "$fixture/prepare" -- bash "$fixture/runtime" > "$fixture/$MODE.log" 2>&1 &
    controller=$!
    if [[ $MODE == signal ]]; then
        deadline=$((SECONDS + 10))
        until [[ -e $CASE/emulator.pid && -e $CASE/descendant.pid ]]; do
            (( SECONDS < deadline )) || exit 96
            sleep 0.02
        done
        kill -TERM "$controller"
    fi
    result=0
    wait "$controller" || result=$?
    controller=''
    case $MODE in
        build-first|boot-first) test "$result" -eq 0; test -e "$CASE/runtime.called" ;;
        failure) test "$result" -eq 23; test ! -e "$CASE/runtime.called" ;;
        boot-failure) test "$result" -eq 1; test ! -e "$CASE/runtime.called" ;;
        signal) test "$result" -eq 143; test ! -e "$CASE/runtime.called" ;;
        runtime-failure) test "$result" -eq 31; test -e "$CASE/runtime.called" ;;
    esac
    for pid_file in "$CASE/"*.pid; do
        [[ -f $pid_file ]] || continue
        if running "$(cat "$pid_file")"; then exit 95; fi
    done
    rm -f -- "$CASE/"*.pid
    test -z "$(find "$fixture/temp" -mindepth 1 -maxdepth 1 -name 'android-environment.*' -print)"
done
for args in '--prepare' '--prepare bash --' '--prepare -- bash'; do
    result=0
    read -ra invalid <<< "$args"
    bash "$fixture/scripts/check-android-environment.sh" "${invalid[@]}" > "$fixture/invalid.log" 2>&1 || result=$?
    test "$result" -eq 1
done
echo 'Android preparation: both completion orders, build/boot/runtime failures, signal 143, descendant cleanup and argument rejection passed.'
