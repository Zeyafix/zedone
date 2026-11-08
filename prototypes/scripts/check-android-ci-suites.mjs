import assert from "node:assert/strict";
import { runInNewContext } from "node:vm";

// Execute the real dispatcher with only its process boundary stubbed. Never
// launch Cargo, an emulator or a second check while validating suite routing.
const source = (await Bun.file(`${import.meta.dir}/check-android-ci.mjs`).text())
  .replace('import assert from "node:assert/strict";', "")
  .replaceAll("import.meta.dir", '"/fixture"');
function dispatch(args, failure = {}) {
  const calls = [];
  let exit;
  const stopped = {};
  try {
    runInNewContext(source, {
      assert, console: { log() {} },
      process: { execPath: "fixture-bun", env: {}, exit(code) { exit = code; throw stopped; } },
      Bun: {
        argv: ["fixture-bun", "dispatcher", ...args],
        spawnSync(command) {
          calls.push(Array.from(command).slice(1).map(arg => arg.replace("/fixture/", "")));
          return { exitCode: 0, ...failure };
        },
      },
    });
  } catch (error) {
    if (error !== stopped) throw error;
  }
  return { calls, exit };
}
const product = "../../scripts/check-product.mjs";
const full = [
  [product, "--android"],
  ["check-android-webview-target.mjs"],
  ["check-automerge.mjs", "--android"],
  ["check-android.mjs"],
  ["../../scripts/check-ui-android-ci.mjs"],
];
for (const [args, expected] of [
  [[], full],
  [["native"], [full[0], full[2]]],
  [["product-native"], [[product, "--android-native"]]],
  [["product-arm64"], [[product, "--android-arm64"]]],
  [["prototype-native"], [full[2]]],
  [["prototype-ui"], [full[1], full[3]]],
  [["shell"], [full[4]]],
]) {
  assert.deepEqual(dispatch(args), { calls: expected, exit: undefined });
}
for (const args of [["unknown"], ["product-native", "extra"], ["product-arm64", "extra"]]) {
  assert.throws(() => dispatch(args), /Unknown Android suite/);
}
assert.deepEqual(dispatch([], { exitCode: 23 }), { calls: [full[0]], exit: 23 });
assert.deepEqual(dispatch(["product-arm64"], { signalCode: "SIGTERM" }), {
  calls: [[product, "--android-arm64"]], exit: 1,
});
console.log("Android CI routing: full/default/native compatibility, both ABI slices, UI suites, invalid input, child failure and signal passed.");
