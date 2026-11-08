import assert from "node:assert/strict";

// Each CI suite owns an emulator through check-android-environment.sh. The
// default retains the complete sequential command for standalone use.
const checks = [
  ["product-native", ["../../scripts/check-product.mjs", "--android"]],
  ["prototype-ui", ["check-android-webview-target.mjs"]],
  ["prototype-native", ["check-automerge.mjs", "--android"]],
  ["prototype-ui", ["check-android.mjs"]],
  ["shell", ["../../scripts/check-ui-android-ci.mjs"]],
];
// Explicit CI slices have separate hosts: compilation must not compete with
// the emulator's timing guards. Standalone/default/native keep the full run.
const productSlices = new Map([
  ["product-native", ["../../scripts/check-product.mjs", "--android-native"]],
  ["product-arm64", ["../../scripts/check-product.mjs", "--android-arm64"]],
]);
const suite = Bun.argv[2];
assert.ok(Bun.argv.length <= 3 && (suite === undefined || suite === "native" || productSlices.has(suite) || checks.some(([name]) => name === suite)), "Unknown Android suite");
const selected = productSlices.has(suite) ? [[suite, productSlices.get(suite)]]
  : checks.filter(([name]) => suite === undefined || (suite === "native" ? name.endsWith("-native") : name === suite));
for (const [name, args] of selected) {
  console.log(`Android suite ${name}: ${args.join(" ")}`);
  const result = Bun.spawnSync([process.execPath, `${import.meta.dir}/${args[0]}`, ...args.slice(1)], {
    env: process.env, stdin: "ignore", stdout: "inherit", stderr: "inherit",
  });
  if (result.exitCode || result.signalCode) process.exit(result.exitCode || 1);
}
