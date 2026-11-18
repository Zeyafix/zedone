import assert from "node:assert/strict";
import { resolve } from "node:path";
import { runInNewContext } from "node:vm";
import { positionConflictCases } from "../../scripts/check-position-conflicts-exchange.mjs";

const source = (await Bun.file(`${import.meta.dir}/check-android-ci.mjs`).text())
  .replace('import assert from "node:assert/strict";', "")
  .replaceAll("import.meta.dir", '"/fixture"');
function dispatch(args, failure = {}, failureAt = 0) {
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
          return { exitCode: 0, ...(calls.length - 1 === failureAt ? failure : {}) };
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
  ["../../scripts/prepare-pairing-app-ci.mjs"],
  ["../../scripts/build-ui-android.mjs", "x86_64", "--pairing-acceptance"],
  ["../../scripts/check-pairing-app.mjs"],
];
for (const [args, expected] of [
  [[], full],
  [["native"], [full[0], full[2]]],
  [["product-native"], [[product, "--android-runtime"]]],
  [["product-x86_64"], [[product, "--android-x86_64"]]],
  [["product-arm64"], [[product, "--android-arm64"]]],
  [["prototype-native"], [full[2]]],
  [["prototype-ui"], [full[1], full[3]]],
  [["shell"], [full[4]]],
  [["pairing"], full.slice(5)],
  ...['tags', 'fields', 'placements', 'positions', 'positions-cross', 'positions-same'].map(part => [[`pairing-${part}`], [...full.slice(5, 7), [...full[7], `--${part}`]]]),
]) {
  assert.deepEqual(dispatch(args), { calls: expected, exit: undefined });
}
for (const args of [["unknown"], ["product-native", "extra"], ["product-arm64", "extra"], ["shell", "extra"], ["pairing", "extra"]]) {
  assert.throws(() => dispatch(args), /Unknown Android suite/);
}
const workflow = Bun.YAML.parse(await Bun.file(`${import.meta.dir}/../../.github/workflows/android-environment.yml`).text());
const jobs = workflow.jobs["android-checks"];
const suites = ["product-native", "product-x86_64", "product-arm64", "prototype-native", "prototype-ui", "prototype-arm64", "shell", "pairing-tags", "pairing-fields", "pairing-placements", "pairing-positions-cross", "pairing-positions-same", "shell-arm64"];
const pairedSuites = ["pairing-tags", "pairing-fields", "pairing-placements", "pairing-positions-cross", "pairing-positions-same"];
assert.deepEqual(jobs.strategy.matrix.suite, suites);
assert.equal(jobs.strategy["fail-fast"], false);
assert.equal(jobs["timeout-minutes"], 15);
const enabled = (step, suite) => runInNewContext(step.if || "true", { matrix: { suite }, startsWith: (value, prefix) => value.startsWith(prefix) });
const arm64ShellBuilds = jobs.steps.filter(step => step.run?.includes("bun scripts/build-ui-android.mjs aarch64"));
assert.equal(arm64ShellBuilds.length, 1, "aarch64 must be built exactly once");
assert.deepEqual(suites.filter(suite => enabled(arm64ShellBuilds[0], suite)), ["shell-arm64"]);
assert.ok(!jobs.steps.some(step => step.run?.includes("bun scripts/build-ui-android.mjs x86_64")), "x86_64 preparation must overlap emulator boot");
const shellPreparation = await Bun.file(`${import.meta.dir}/../../scripts/prepare-android-shell-ci.sh`).text();
assert.ok(shellPreparation.includes(["shell", ...pairedSuites].join("|")));
assert.equal(shellPreparation.split("bun scripts/build-ui-android.mjs x86_64").length, 2);
assert.ok(shellPreparation.includes('if [[ $1 == shell ]]; then'), "Paired API probes must not rebuild the unused browser shell fixture");
assert.ok(shellPreparation.includes('CARGO_TARGET_DIR="$PWD/target" cargo "+$rust" build --manifest-path ui/src-tauri/Cargo.toml --example shell-fixture --locked --offline'));
const prototypeArm64 = jobs.steps.find(step => step.name === 'Build Android arm64 APK and instrumentation');
assert.deepEqual(suites.filter(suite => enabled(prototypeArm64, suite)), ['prototype-arm64']);
assert.equal(prototypeArm64.run, 'bun run android:build aarch64');
const prototypeDependencies = jobs.steps.find(step => step.name === 'Prepare locked prototype dependencies and contracts');
assert.ok(enabled(prototypeDependencies, 'prototype-arm64'));
const viewerDependencies = jobs.steps.find(step => step.name === 'Prepare locked shared viewer dependencies');
assert.deepEqual(suites.filter(suite => enabled(viewerDependencies, suite)), ['prototype-ui', 'prototype-arm64']);
const runtime = jobs.steps.find(step => step.name === "Prepare emulator runtime and KVM");
const runtimeSuites = suites.filter(suite => !["product-x86_64", "product-arm64", "prototype-arm64", "shell-arm64"].includes(suite));
assert.deepEqual(suites.filter(suite => enabled(runtime, suite)), runtimeSuites);
const hostPreparation = Bun.YAML.parse(await Bun.file(`${import.meta.dir}/../../.github/actions/prepare-host-check/action.yml`).text());
const mirrors = hostPreparation.runs.steps.find(step => step.name === "Use Ubuntu HTTPS mirrors for browser dependencies");
assert.ok(runtime.run.startsWith(mirrors.run), "Android must use the same signed Ubuntu mirror setup as Linux host checks");
assert.ok(runtime.run.includes("sudo apt-get update\nsudo apt-get install --no-install-recommends -y libpulse0\n"),
  "Mirror selection cannot replace installing emulator dependencies");
const environment = jobs.steps.find(step => step.name === "Check Android environment");
assert.deepEqual(suites.filter(suite => enabled(environment, suite)), runtimeSuites);
assert.equal(environment.env.SUITE, '${{ matrix.suite }}');
assert.ok(environment.run.includes('bash scripts/check-android-environment.sh bun prototypes/scripts/check-android-ci.mjs "$SUITE"'));
assert.ok(environment.run.includes('if [[ "$SUITE" == prototype-ui ]]; then'));
assert.ok(environment.run.includes("--prepare bash scripts/prepare-android-prototype-ci.sh -- bun prototypes/scripts/check-android-ci.mjs"));
assert.ok(environment.run.includes('elif [[ "$SUITE" == shell || "$SUITE" == pairing-* ]]; then'));
assert.ok(environment.run.includes('--prepare bash scripts/prepare-android-shell-ci.sh "$SUITE" -- bun prototypes/scripts/check-android-ci.mjs "$SUITE"'));
assert.ok(!jobs.steps.some(step => step.name === 'Build host comparison endpoint' || step.name === 'Build Android x86_64 APK and instrumentation'));
const preparation = await Bun.file(`${import.meta.dir}/../../scripts/prepare-android-prototype-ci.sh`).text();
assert.ok(preparation.includes('cargo "+$rust" build --release --locked --offline --bin serve-echo'));
assert.ok(preparation.includes('CARGO_TARGET_DIR="$PWD/target"'));
assert.ok(preparation.includes('bun run android:build'));
assert.ok(preparation.indexOf('serve-echo') < preparation.indexOf('bun run android:build'));
const arm64 = jobs.steps.find(step => step.name === "Check product ABI build without emulator");
assert.deepEqual(suites.filter(suite => enabled(arm64, suite)), ["product-x86_64", "product-arm64"]);
assert.equal(arm64.run, "bun prototypes/scripts/check-android-ci.mjs '${{ matrix.suite }}'");
const dependencies = jobs.steps.find(step => step.name === "Prepare locked product JavaScript dependencies");
assert.ok(enabled(dependencies, "shell-arm64"));
assert.ok(enabled(dependencies, "pairing-fields"));
assert.equal(dependencies.env.SUITE, '${{ matrix.suite }}');
assert.ok(dependencies.run.includes('"$SUITE" == shell-arm64'));
assert.ok(dependencies.run.includes('"$SUITE" == pairing-*'));
assert.ok(dependencies.run.includes("--manifest-path ui/src-tauri/Cargo.toml"));
const negative = jobs.steps.find(step => step.name === "Check rejection and cleanup on compiler failure");
const installer = jobs.steps.find(step => step.name === "Check Tauri CLI cache fallback and cleanup");
for (const control of [negative, installer]) {
  assert.deepEqual(suites.filter(suite => enabled(control, suite)), ["prototype-native"], "One mandatory owner must run shared infrastructure controls");
}
assert.ok(installer.run.includes("bash scripts/check-ci-tauri.sh"));
assert.ok(installer.run.includes("bun prototypes/scripts/check-android-ci-suites.mjs"), "Standalone Android workflow must check its own routing even without host workflow triggers");
assert.ok(installer.run.includes("bash scripts/check-android-preparation.sh"));
assert.ok(installer.run.includes("bash scripts/check-android-shell-preparation.sh"));
for (const required of ["Unexpected Bun version", 'test "$status" -eq 23', 'cmp "$test_dir/before" "$test_dir/after"', 'existing-avd/sentinel']) {
  assert.ok(negative.run.includes(required), "Negative controls must retain version rejection, exit code and cleanup evidence");
}
assert.ok(!JSON.stringify(workflow).includes("continue-on-error"));
assert.equal(workflow.jobs.android.needs, "android-checks");
assert.equal(workflow.jobs.android.if, "always()");
assert.equal(workflow.jobs.android.steps[0].run, 'test "$RESULT" = success');
assert.throws(() => dispatch(["shell-arm64"]), /Unknown Android suite/, "Build-only job cannot enter the emulator dispatcher");
assert.deepEqual(dispatch([], { exitCode: 23 }), { calls: [full[0]], exit: 23 });
assert.deepEqual(dispatch(["product-arm64"], { signalCode: "SIGTERM" }), {
  calls: [[product, "--android-arm64"]], exit: 1,
});
for (const [suite, commands] of [["shell", [full[4]]], ["pairing", full.slice(5)],
  ...['tags', 'fields', 'placements', 'positions', 'positions-cross', 'positions-same'].map(part => [`pairing-${part}`, [...full.slice(5, 7), [...full[7], `--${part}`]]])]) {
  for (let index = 0; index < commands.length; index++) {
    assert.deepEqual(dispatch([suite], { exitCode: 23 }, index), {
      calls: commands.slice(0, index + 1), exit: 23,
    }, "A failing UI, bootstrap, acceptance build or APK check must stop its suite");
    assert.deepEqual(dispatch([suite], { signalCode: "SIGTERM" }, index), {
      calls: commands.slice(0, index + 1), exit: 1,
    });
  }
}
const bootstrap = (await Bun.file(`${import.meta.dir}/../../scripts/prepare-pairing-app-ci.mjs`).text())
  .replace('import assert from "node:assert/strict";', "")
  .replace('import { resolve } from "node:path";', "")
  .replaceAll("import.meta.dir", '"/fixture/scripts"');
const environmentFixture = { CI: "true", ANDROID_SERIAL: "emulator-5554", ANDROID_HOME: "/sdk" };
function prepare(overrides = {}, result = { exitCode: 0 }) {
  const calls = [];
  const env = { ...environmentFixture, ...overrides };
  let error;
  try {
    runInNewContext(bootstrap, {
      assert, resolve, process: { env },
      Bun: { spawnSync(command, options) { calls.push({ command: Array.from(command), options }); return result; } },
    });
  } catch (caught) { error = caught; }
  return { calls, error, env };
}
const prepared = prepare();
assert.equal(prepared.error, undefined);
assert.equal(prepared.calls.length, 1);
assert.deepEqual(prepared.calls[0].command, [
  resolve("/sdk", "platform-tools/adb"), "-s", "emulator-5554", "install", "-r",
  resolve("/fixture/scripts", "../ui/src-tauri/gen/android/app/build/outputs/apk/x86_64/debug/app-x86_64-debug.apk"),
]);
const options = prepared.calls[0].options;
assert.equal(options.env, prepared.env);
assert.equal(options.timeout, 90000);
for (const [key, value] of [["stdin", "ignore"], ["stdout", "inherit"], ["stderr", "inherit"]]) assert.equal(options[key], value);
for (const overrides of [{ CI: undefined }, { CI: "false" }, { ANDROID_SERIAL: "physical-device" }, { ANDROID_SERIAL: undefined }, { ANDROID_HOME: undefined }]) {
  const rejected = prepare(overrides);
  assert.ok(rejected.error, "Invalid CI/emulator inputs must be rejected before installing");
  assert.equal(rejected.calls.length, 0);
}
for (const result of [{ exitCode: 23 }, { exitCode: 0, signalCode: "SIGTERM" }, { exitCode: null, signalCode: "SIGKILL" }]) {
  const rejected = prepare({}, result);
  assert.ok(rejected.error, "Install failure or timeout must fail pairing preparation");
  assert.equal(rejected.calls.length, 1);
}
const pairingSource = await Bun.file(`${import.meta.dir}/../../scripts/check-pairing-app.mjs`).text();
const hashesSource = pairingSource.slice(pairingSource.indexOf("const hashes = () => {"), pairingSource.indexOf("const normalized ="));
assert.ok(hashesSource.startsWith("const hashes = () => {"));
function manifest(text) {
  return Array.from(runInNewContext(`${hashesSource}\nhashes()`, {
    assert, app: "app.zedone.local", device: () => Buffer.from(text),
  }), entry => Array.from(entry));
}
assert.deepEqual(manifest(""), [], "A fresh installed APK can have no application files");
assert.deepEqual(manifest(`${"a".repeat(64)}  ./files/data.sqlite\n${"b".repeat(64)}  ./files/state\n`), [
  ["./files/data.sqlite", "a".repeat(64)], ["./files/state", "b".repeat(64)],
]);
for (const text of ["not a checksum", `${"a".repeat(63)}  ./files/data.sqlite`, `${"a".repeat(64)}  /absolute/path`]) {
  assert.throws(() => manifest(text), /Invalid application file manifest/);
}
const pairedDispatch = pairingSource.slice(pairingSource.indexOf("const suite = Bun.argv[2];"), pairingSource.indexOf("const serial = process.env.ANDROID_SERIAL;"))
  .replaceAll("import.meta.path", '"/fixture/check-pairing-app.mjs"');
async function pairingSuites(env = {}, args = [], failure = {}, failureAt = 0) {
  const calls = [], stopped = {};
  let exit, error;
  try {
    await runInNewContext(`(async () => { ${pairedDispatch} })()`, {
      assert, root: "/fixture", console: { log() {} },
      process: { execPath: "fixture-bun", env, exit(code) { exit = code; throw stopped; } },
      Bun: {
        argv: ["fixture-bun", "pairing", ...args],
        spawn(command, options) {
          calls.push({ command: Array.from(command), env: { ...options.env } });
          const result = calls.length - 1 === failureAt ? failure : {};
          return { exited: Promise.resolve(Object.hasOwn(result, "exitCode") ? result.exitCode : 0), signalCode: result.signalCode };
        },
      },
    });
  } catch (cause) { if (cause !== stopped) error = cause; }
  return { calls, exit, error };
}
for (const env of [{ CI: "true" }, {}, { P06_EXCHANGE_UI: "0" }, { P06_EXCHANGE_UI: "1" }, { P06_EXCHANGE_UI: "1", P06_POSITION_CONFLICTS_UI: "1" }, { P06_EXCHANGE_UI: "1", P06_TAG_CONFLICTS_UI: "1" }]) {
  const result = await pairingSuites(env);
  assert.equal(result.error, undefined); assert.equal(result.exit, 0);
  assert.deepEqual(result.calls.map(call => call.command), ["--standard", "--positions", "--tags"].map(suite => ["fixture-bun", "/fixture/check-pairing-app.mjs", suite]));
  for (const call of result.calls) assert.equal(Object.hasOwn(call.env, "P06_EXCHANGE_UI"), Object.hasOwn(env, "P06_EXCHANGE_UI"), "Fixture timeout presence must not change");
  if (env.P06_EXCHANGE_UI === "1") {
    const ui = env.P06_TAG_CONFLICTS_UI === "1" ? 2 : env.P06_POSITION_CONFLICTS_UI === "1" ? 1 : 0;
    assert.equal(result.calls[ui].env.P06_EXCHANGE_UI, "1");
    for (let i = 0; i < 3; i++) if (i !== ui) assert.equal(result.calls[i].env.P06_EXCHANGE_UI, "0");
  }
}
for (const suite of ["--tags", "--standard", "--fields", "--placements", "--positions", "--positions-cross", "--positions-same"]) assert.equal((await pairingSuites({}, [suite])).calls.length, 0);
const exchangeStart = pairingSource.indexOf("    if (suite === '--standard' || suite === '--fields')");
assert.ok(exchangeStart > 0);
const splitExchanges = pairingSource.slice(exchangeStart, pairingSource.indexOf('    for (const [operation, parameters]', exchangeStart));
function exchanges(suite) {
  const calls = [];
  const names = ['checkTagConflictsExchange', 'checkTextExchange', 'checkFieldConflictsExchange', 'checkPlacementExchange', 'checkColumnPositionExchange', 'checkPositionConflictsExchange'];
  runInNewContext(splitExchanges, { suite, positions: suite.startsWith('--positions'), positionScope: suite === '--positions-cross' ? false : suite === '--positions-same' ? true : undefined, host: {}, android: {}, space: {}, second: {}, call: {}, command: {}, ok: {}, error: {},
    ...Object.fromEntries(names.map(name => [name, options => {
      calls.push(name);
      if (name === 'checkPositionConflictsExchange') assert.equal(options.sameColumn,
        suite === '--positions-cross' ? false : suite === '--positions-same' ? true : undefined);
    }])) });
  return calls;
}
assert.deepEqual([...exchanges('--fields'), ...exchanges('--placements')], exchanges('--standard'));
assert.equal(exchanges('--standard').length, 4);
assert.deepEqual(exchanges('--tags'), ['checkTagConflictsExchange']);
assert.deepEqual(exchanges('--positions'), ['checkPositionConflictsExchange']);
for (const scope of ['--positions-cross', '--positions-same']) assert.deepEqual(exchanges(scope), ['checkPositionConflictsExchange']);
assert.deepEqual(positionConflictCases(), [
  {same:false,columnFirst:false}, {same:false,columnFirst:true}, {same:true,columnFirst:false}, {same:true,columnFirst:true},
]);
assert.deepEqual([...positionConflictCases(false), ...positionConflictCases(true)], positionConflictCases(), 'Both mandatory slices retain all four scenarios and both resolution orders exactly once');
for (const invalid of ['same', null, 0, 1]) assert.throws(() => positionConflictCases(invalid));
const failureStart = pairingSource.indexOf('    const serverCode = await server.exited;');
const failureReport = pairingSource.slice(failureStart, pairingSource.indexOf('    if (cleanupError)', failureStart));
assert.ok(failureStart > 0 && failureReport.includes('assert.equal(serverCode, 0'));
for (const [code, stderr, reason] of [[0,'',''], [101,'private-key-secret fixture timeout','fixture_timeout'], [101,'private-key-secret other panic','unclassified']]) {
  const messages = [];
  let error, reads = 0;
  try {
    await runInNewContext('(async () => {' + failureReport + '})()', {
      server: {exited:Promise.resolve(code)}, serverStarted:100, serverDirectory:'/fixture', Date:{now:()=>200}, resolve,
      Bun:{file:()=>({text:async()=>{reads++;return stderr;}})}, assert, console:{error:value=>messages.push(value)},
    });
  } catch (cause) { error = cause; }
  assert.equal(Boolean(error),code!==0,'Fixture failure must still fail acceptance');
  assert.equal(reads,code===0?0:1);
  assert.deepEqual(messages,code===0?[]:[`Host fixture failure: exit=${code} reason=${reason} elapsed_ms=100`]);
  assert.ok(!messages.join('').includes('private-key-secret'));
}

assert.ok((await pairingSuites({}, ["invalid"])).error);
for (const index of [0, 1]) for (const failure of [{ exitCode: 23 }, { exitCode: null }, { exitCode: 0, signalCode: "SIGTERM" }]) {
  const result = await pairingSuites({}, [], failure, index);
  assert.ok(result.error); assert.equal(result.exit, undefined);
  assert.equal(result.calls.length, index + 1, "A failed suite must prevent a successful aggregate");
}
console.log("Android CI routing: full/default/native compatibility, independent UI/paired APK suites, exact ABI job ownership, CI-only production APK bootstrap, empty/valid/invalid file manifests, all runtime/negative checks, 15m budgets, strict aggregate, invalid input, child failures and signals passed.");
await import('./check-ci-tool-caches.mjs');
