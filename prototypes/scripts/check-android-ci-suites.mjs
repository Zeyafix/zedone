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
// Validate the real workflow's build/runtime boundary independently of the
// dispatcher. The arm64 shell job builds an APK; it never owns an emulator.
const workflow = Bun.YAML.parse(await Bun.file(`${import.meta.dir}/../../.github/workflows/android-environment.yml`).text());
const jobs = workflow.jobs["android-checks"];
const suites = ["product-native", "product-arm64", "prototype-native", "prototype-ui", "shell", "shell-arm64"];
assert.deepEqual(jobs.strategy.matrix.suite, suites);
assert.equal(jobs.strategy["fail-fast"], false);
assert.equal(jobs["timeout-minutes"], 10);
const enabled = (step, suite) => runInNewContext(step.if || "true", { matrix: { suite } });
for (const [target, owner] of [["x86_64", "shell"], ["aarch64", "shell-arm64"]]) {
  const builds = jobs.steps.filter(step => step.run?.includes(`bun scripts/build-ui-android.mjs ${target}`));
  assert.equal(builds.length, 1, `${target} must be built exactly once`);
  assert.deepEqual(suites.filter(suite => enabled(builds[0], suite)), [owner]);
}
const fixture = jobs.steps.find(step => step.run?.includes("--example shell-fixture"));
assert.deepEqual(suites.filter(suite => enabled(fixture, suite)), ["shell"]);
const runtime = jobs.steps.find(step => step.name === "Prepare emulator runtime and KVM");
assert.deepEqual(suites.filter(suite => enabled(runtime, suite)), suites,
  "Every compiler-failure preflight needs emulator libraries and KVM access");
const hostPreparation = Bun.YAML.parse(await Bun.file(`${import.meta.dir}/../../.github/actions/prepare-host-check/action.yml`).text());
const mirrors = hostPreparation.runs.steps.find(step => step.name === "Use Ubuntu HTTPS mirrors for browser dependencies");
assert.ok(runtime.run.startsWith(mirrors.run), "Android must use the same signed Ubuntu mirror setup as Linux host checks");
assert.ok(runtime.run.includes("sudo apt-get update\nsudo apt-get install --no-install-recommends -y libpulse0\n"),
  "Mirror selection cannot replace installing emulator dependencies");
const environment = jobs.steps.find(step => step.name === "Check Android environment");
assert.deepEqual(suites.filter(suite => enabled(environment, suite)), suites.filter(suite => suite !== "shell-arm64"));
const dependencies = jobs.steps.find(step => step.name === "Prepare locked product JavaScript dependencies");
assert.ok(enabled(dependencies, "shell-arm64"));
assert.ok(dependencies.run.includes("'${{ matrix.suite }}' == shell-arm64"));
assert.ok(dependencies.run.includes("--manifest-path ui/src-tauri/Cargo.toml"));
const negative = jobs.steps.find(step => step.name === "Check rejection and cleanup on compiler failure");
assert.ok(suites.every(suite => enabled(negative, suite)));
assert.equal(workflow.jobs.android.needs, "android-checks");
assert.equal(workflow.jobs.android.if, "always()");
assert.equal(workflow.jobs.android.steps[0].run, 'test "$RESULT" = success');
assert.throws(() => dispatch(["shell-arm64"]), /Unknown Android suite/, "Build-only job cannot enter the emulator dispatcher");
assert.deepEqual(dispatch([], { exitCode: 23 }), { calls: [full[0]], exit: 23 });
assert.deepEqual(dispatch(["product-arm64"], { signalCode: "SIGTERM" }), {
  calls: [[product, "--android-arm64"]], exit: 1,
});
console.log("Android CI routing: full/default/native compatibility, exact ABI job ownership, all runtime/negative checks, strict aggregate, invalid input, child failure and signal passed.");
