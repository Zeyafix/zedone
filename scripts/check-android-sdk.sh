#!/usr/bin/env bash
set -euo pipefail

fail() { echo "$*" >&2; exit 1; }
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
versions="$root/scripts/environment-versions.json"
sdk=${ANDROID_HOME:?Set ANDROID_HOME to the prepared SDK.}
[[ ${ANDROID_SDK_ROOT:-$sdk} == "$sdk" ]] || fail 'Android SDK paths disagree.'
tools="$sdk/cmdline-tools/$(jq -r .android.command_line_tools "$versions")/bin"
echo "$(jq -r .android.linux_cli_sha256 "$versions")  $tools/android" | sha256sum --check
expected=$(jq -r .android.cli "$versions")
"$tools/android" --no-metrics --version || fail 'Unable to initialize Android CLI.'
actual=$("$tools/android" --no-metrics --version) || fail 'Unable to read Android CLI version.'
[[ $actual == "$expected" ]] || \
    fail "Unexpected Android CLI version: expected $(printf '%q' "$expected"), got $(printf '%q' "$actual") (executable: $tools/android)."
while IFS=@ read -r package revision; do
    properties="$sdk/$package/source.properties"
    [[ -f $properties ]] || fail "Missing SDK package: $package"
    actual=$(sed -n 's/^Pkg.Revision *= *//p' "$properties" | tr -d '\r')
    actual=${actual%.0}; actual=${actual%.0}
    revision=${revision%.0}; revision=${revision%.0}
    [[ $actual == "$revision" ]] || fail "Unexpected $package revision: $actual"
done < <(jq -r '.android.packages[]' "$versions")
