import assert from "node:assert/strict";

const checks = [
  ["product-native", ["../../scripts/check-product.mjs", "--android"]],
  ["prototype-ui", ["check-android-webview-target.mjs"]],
  ["prototype-native", ["check-automerge.mjs", "--android"]],
  ["prototype-ui", ["check-android.mjs"]],
  ["shell", ["../../scripts/check-ui-android-ci.mjs"]],
  ["pairing", ["../../scripts/prepare-pairing-app-ci.mjs"]],
  ["pairing", ["../../scripts/build-ui-android.mjs", "x86_64", "--pairing-acceptance"]],
  ["pairing", ["../../scripts/check-pairing-app.mjs"]],
];
const productSlices = new Map([
  ["product-native", ["../../scripts/check-product.mjs", "--android-runtime"]],
  ["product-x86_64", ["../../scripts/check-product.mjs", "--android-x86_64"]],
  ["product-arm64", ["../../scripts/check-product.mjs", "--android-arm64"]],
]);
const pairingSlices = new Map([['pairing-tags', '--tags'], ['pairing-fields', '--fields'], ['pairing-placements', '--placements'], ['pairing-positions', '--positions'], ['pairing-positions-cross', '--positions-cross'], ['pairing-positions-same', '--positions-same']]);
const suite = Bun.argv[2];
assert.ok(Bun.argv.length <= 3 && (suite === undefined || suite === "native" || productSlices.has(suite) || pairingSlices.has(suite) || checks.some(([name]) => name === suite)), "Unknown Android suite");
const selected = productSlices.has(suite) ? [[suite, productSlices.get(suite)]]
  : pairingSlices.has(suite) ? checks.filter(([name]) => name === 'pairing').map(([name, args]) => [name, args[0].endsWith('/check-pairing-app.mjs') ? [...args, pairingSlices.get(suite)] : args])
  : checks.filter(([name]) => suite === undefined || (suite === "native" ? name.endsWith("-native") : name === suite));
for (const [name, args] of selected) {
  console.log(`Android suite ${name}: ${args.join(" ")}`);
  const result = Bun.spawnSync([process.execPath, `${import.meta.dir}/${args[0]}`, ...args.slice(1)], {
    env: process.env, stdin: "ignore", stdout: "inherit", stderr: "inherit",
  });
  if (result.exitCode || result.signalCode) process.exit(result.exitCode || 1);
}
