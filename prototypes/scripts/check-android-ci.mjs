import assert from "node:assert/strict";

// Each CI suite owns an emulator through check-android-environment.sh. The
// default retains the complete sequential command for standalone use.
const checks = [
  ["native", ["../../scripts/check-product.mjs", "--android"]],
  ["prototype-ui", ["check-android-webview-target.mjs"]],
  ["native", ["check-automerge.mjs", "--android"]],
  ["prototype-ui", ["check-android.mjs"]],
  ["shell", ["../../scripts/check-ui-android-ci.mjs"]],
];
const suite = Bun.argv[2];
assert.ok(Bun.argv.length <= 3 && (suite === undefined || checks.some(([name]) => name === suite)), "Unknown Android suite");
for (const [name, args] of checks) {
  if (suite !== undefined && name !== suite) continue;
  console.log(`Android suite ${name}: ${args.join(" ")}`);
  const result = Bun.spawnSync([process.execPath, `${import.meta.dir}/${args[0]}`, ...args.slice(1)], {
    env: process.env, stdin: "ignore", stdout: "inherit", stderr: "inherit",
  });
  if (result.exitCode || result.signalCode) process.exit(result.exitCode || 1);
}
