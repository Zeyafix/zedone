import assert from "node:assert/strict";
import { relative, resolve } from "node:path";
import { runInNewContext } from "node:vm";
import { withoutComments } from "../../scripts/source-comments.mjs";

const root = resolve(import.meta.dir, "..");
const versions = await Bun.file(resolve(root, "../scripts/environment-versions.json")).json();
const openapi = Buffer.from(await Bun.file(resolve(root, "openapi.json")).arrayBuffer());
const types = await Bun.file(resolve(root, "client/api.d.ts")).text();
const source = (await Bun.file(resolve(import.meta.dir, "check.mjs")).text())
  .replace('import { resolve } from "node:path";', "")
  .replace('import { withoutComments } from "../../scripts/source-comments.mjs";', "")
  .replaceAll("import.meta.dir", JSON.stringify(import.meta.dir))
  .replace('await import("openapi-typescript")', "await importTypes()");
const execute = new (Object.getPrototypeOf(async function () {}).constructor)("Bun", "process", "resolve", "importTypes", "withoutComments", "console", source);
async function dispatch(args, failure = {}) {
  const calls = [], writes = [];
  let exit;
  const stopped = {};
  try {
    await execute({
      version: versions.bun, argv: ["fixture-bun", "driver", ...args], file: Bun.file,
      async write(path) { writes.push(path); },
      spawnSync(command, options) {
        calls.push(Array.from(command));
        assert.equal(options.cwd, root);
        assert.equal(options.env.CARGO_TARGET_DIR, resolve(root, "target"));
        const stdout = command.includes("generate-openapi") ? openapi
          : command[0] === "node" && command[1] === "--version" ? Buffer.from(`v${versions.node}\n`) : Buffer.alloc(0);
        return { exitCode: 0, stdout, ...failure };
      },
    }, {
      execPath: "fixture-bun", env: { CARGO_TARGET_DIR: "/foreign-output" },
      exit(code) { exit = code; throw stopped; },
    }, resolve, async () => ({ default() {}, astToString: () => types }), withoutComments, { log() {}, error() {} });
  } catch (error) { if (error !== stopped) throw error; }
  return { calls, writes, exit };
}
const full = await dispatch([]);
const native = await dispatch(["--host-native"]);
const ui = await dispatch(["--host-ui"]);
for (const result of [full, native, ui]) {
  assert.equal(result.exit, undefined);
  assert.deepEqual(result.writes, []);
}
const commands = calls => calls.map(command => JSON.stringify(command)).sort();
assert.deepEqual(commands([...native.calls, ...ui.calls]), commands(full.calls), "Slices must run every full check exactly once");
assert.ok(native.calls.some(command => command.includes("test")));
assert.ok(native.calls.some(command => command.includes("scripts/check-automerge.mjs")));
assert.ok(native.calls.some(command => command.includes("generate-openapi")));
assert.ok(!native.calls.some(command => command.includes("--release") || command[0] === "node"));
assert.ok(ui.calls.some(command => command.includes("--release")));
assert.ok(ui.calls.some(command => command.includes("scripts/check-http.mjs")));
assert.ok(!ui.calls.some(command => command.includes("test") || command.includes("generate-openapi")));
for (const args of [[], ["--host-native"], ["--host-ui"]]) {
  const failed = await dispatch(args, { exitCode: 23 });
  assert.equal(failed.exit, 23);
  assert.equal(failed.calls.length, 1);
  const interrupted = await dispatch(args, { signalCode: "SIGTERM" });
  assert.equal(interrupted.exit, 1);
  assert.equal(interrupted.calls.length, 1);
}
for (const args of [["unknown"], ["--host-ui", "extra"], ["--host-native", "extra"]]) {
  const invalid = await dispatch(args);
  assert.equal(invalid.exit, 1);
  assert.deepEqual(invalid.calls, []);
}
for (const [mode, count] of [["--write-openapi", 1], ["--check-openapi", 0], ["--write-types", 2], ["--check-types", 0]]) {
  const result = await dispatch([mode]);
  assert.equal(result.exit, undefined);
  assert.equal(result.calls.length, mode.endsWith("types") ? 2 : 1);
  assert.ok(result.calls[0].includes("generate-openapi"));
  assert.equal(result.writes.length, count);
}
const productRoot = resolve(root, "..");
const product = await Bun.file(resolve(productRoot, "scripts/check-product.mjs")).text();
const pairing = product.slice(product.indexOf("async function checkWithClients("), product.indexOf("\nconst metadata"));
const makePair = new Function("Bun", "process", "resolve", "root", "env", "assert", "console", `return (${pairing});`);
const deferred = () => {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
};
for (const first of ["client", "native"]) {
  for (const fail of [false, true]) {
    const client = deferred(), native = deferred();
    const nativeError = new Error("native control failure");
    let started = false, settled = false;
    const paired = makePair({ spawn(command) {
      assert.deepEqual(command, ["fixture-bun", "test", resolve(productRoot, "tests/commands.test.ts")]);
      return { exited: client.promise };
    } }, { execPath: "fixture-bun" }, resolve, productRoot, {}, assert, { log() {} });
    const result = paired(() => { started = true; return native.promise; }, "paired control");
    const outcome = result.then(() => { settled = true; }, error => { settled = true; return error; });
    await Promise.resolve();
    assert.equal(started, true, "native starts without waiting for delivery tests");
    const finishClient = () => client.resolve(fail ? 23 : 0);
    const finishNative = () => fail ? native.reject(nativeError) : native.resolve();
    if (first === "client") finishClient(); else finishNative();
    for (let turn = 0; turn < 5; turn++) await Promise.resolve();
    assert.equal(settled, false, "a finished or failed side must still await its peer");
    if (first === "client") finishNative(); else finishClient();
    const error = await outcome;
    if (fail) {
      assert.ok(error instanceof AggregateError);
      assert.equal(error.message, "paired control");
      assert.equal(error.errors.length, 2);
      assert.ok(error.errors.includes(nativeError));
      assert.match(error.errors[0].message, /Client tests failed/);
    } else assert.equal(error, undefined);
  }
}
const interrupted = makePair({ spawn() { return { exited: Promise.resolve(0), signalCode: "SIGTERM" }; } },
  { execPath: "fixture-bun" }, resolve, productRoot, {}, assert, { log() {} });
await assert.rejects(interrupted(async () => {}, "signal control"), error =>
  error instanceof AggregateError && /Client tests interrupted/.test(error.errors[0].message));

const contractStart = product.indexOf("async function checkContracts() {");
const contractEnd = product.indexOf('\nconst generated = [', contractStart);
const unsafeStart = product.indexOf("async function checkUnsafe(");
const unsafeEnd = product.indexOf("\nif (!mode ||", unsafeStart);
assert.ok(unsafeStart > 0 && unsafeEnd > unsafeStart && contractStart > unsafeEnd && contractEnd > contractStart);
const productDriver = product.slice(0, unsafeStart)
  + "async function checkUnsafe(targets = []) { probe(targets); }\n"
  + product.slice(unsafeEnd, contractStart)
  + "async function checkContracts() { return contract(); }\n"
  + product.slice(contractEnd);
const executeProduct = new (Object.getPrototypeOf(async function () {}).constructor)(
  "Bun", "process", "assert", "resolve", "relative", "probe", "contract", "checkChannelPair", "randomUUID", "console", productDriver
    .replace(/^import .*;\r?\n/gm, "")
    .replaceAll("import.meta.dir", JSON.stringify(resolve(root, "../scripts"))));
async function productDispatch(mode, { failure, mismatch = false, serial = "emulator-5554" } = {}) {
  const calls = [], probes = [], paired = [];
  let clients = 0, contracts = 0, error;
  const generated = Buffer.from("product contract fixture\n");
  try {
    await executeProduct({
      ...Bun, argv: ["fixture-bun", "product", ...(mode ? [mode] : [])],
      file(path) {
        return path.endsWith("source.properties") ? { text: async () => versions.android.ndk } : Bun.file(path);
      },
      spawn() { clients++; return { exited: Promise.resolve(0) }; },
      spawnSync(command) {
        calls.push(command);
        let stdout = Buffer.alloc(0);
        if (command.includes("metadata")) stdout = Buffer.from(JSON.stringify({
          workspace_members: ["core"], packages: [{ id: "core", name: "zedone-core", manifest_path: resolve(productRoot, "crates/core/Cargo.toml"), dependencies: [] }],
        }));
        else if (command.includes("--message-format=json")) stdout = Buffer.from(JSON.stringify({ reason: "compiler-artifact", profile: { test: true }, target: { name: "zedone_adapters" }, executable: "/fixture/native-tests" }) + "\n");
        else if (command.includes("getprop")) stdout = Buffer.from(({ "ro.kernel.qemu": "1", "ro.build.version.sdk": "33", "ro.product.cpu.abi": "x86_64" })[command.at(-1)]);
        else if (command.includes("shell") && command.at(-1).endsWith("/export-contract")) stdout = mismatch ? Buffer.from("wrong contract") : generated;
        else if (command.includes("shell") && command.at(-1).includes("/test-")) stdout = Buffer.from("test result: ok. 1 passed");
        return { exitCode: 0, stdout, ...(command.includes(mode === "--host-tests" ? "clippy" : "build") ? failure : {}) };
      },
    }, {
      execPath: "fixture-bun", platform: process.platform,
      env: { ANDROID_SERIAL: serial, ANDROID_HOME: "/fixture/sdk", NDK_HOME: "/fixture/ndk" },
    }, assert, resolve, relative,
    targets => probes.push(targets), () => { contracts++; return generated; },
    async options => paired.push(options.test || "channel::tests::cross_platform_probe"),
    () => "00000000-0000-4000-8000-000000000001", { log() {}, error() {} });
  } catch (caught) { error = caught; }
  return { calls, probes, paired, clients, contracts, error };
}
for (const mode of [undefined, "--host-tests", "--host-build", "--check-types", "--android", "--android-native", "--android-runtime", "--android-x86_64", "--android-arm64"]) {
  const result = await productDispatch(mode, { serial: ["--android-x86_64", "--android-arm64"].includes(mode) ? null : "emulator-5554" });
  assert.equal(result.error, undefined, mode);
  assert.equal(result.clients, [undefined, "--host-tests"].includes(mode) ? 1 : 0, "Android slices must not rerun host client tests");
  assert.equal(result.contracts, ["--host-build", "--android-x86_64", "--android-arm64"].includes(mode) ? 0 : 1, "Runtime slice owns standalone Android contract checks");
  if (mode?.startsWith("--android")) {
    const targets = ["--android-native", "--android-runtime", "--android-x86_64"].includes(mode) ? ["x86_64-linux-android"] : mode === "--android-arm64" ? ["aarch64-linux-android"] : ["x86_64-linux-android", "aarch64-linux-android"];
    assert.deepEqual(result.probes, mode === "--android-runtime" ? [] : targets.map(target => ["--target", target]));
    assert.deepEqual(result.calls.filter(command => command.includes("build")).map(command => command[command.indexOf("--target") + 1]), targets);
    if (["--android-x86_64", "--android-arm64"].includes(mode)) {
      assert.ok(!result.calls.some(command => command.includes("shell") || command.includes("push") || command.includes("test")));
      assert.deepEqual(result.paired, []);
    } else {
      assert.ok(result.calls.some(command => command.includes("shell") && command.at(-1).includes("/test-")), "Native Android tests must execute");
      assert.deepEqual(result.paired, ["channel::tests::cross_platform_probe", "pairing::tests::cross_platform_probe"]);
      assert.ok(result.calls.some(command => command.includes("shell") && command.at(-1).endsWith("/export-contract")));
      assert.ok((await productDispatch(mode, { mismatch: true })).error, "A different device contract must fail");
      assert.ok((await productDispatch(mode, { serial: "physical-device" })).error);
    }
  }
}
for (const mode of [undefined, "--host-tests", "--host-build", "--android", "--android-native", "--android-runtime", "--android-x86_64", "--android-arm64"]) {
  for (const failure of [{ exitCode: 23 }, { exitCode: 0, signalCode: "SIGTERM" }]) {
    const failed = await productDispatch(mode, { failure });
    assert.ok(failed.error, "Product build failure/signal must fail its slice");
  }
}

const hostFull = await productDispatch();
const hostTests = await productDispatch('--host-tests');
const hostBuild = await productDispatch('--host-build');
assert.deepEqual(commands([...hostTests.calls, ...hostBuild.calls].filter(call => !call.includes('metadata'))),
  commands(hostFull.calls.filter(call => !call.includes('metadata'))), 'Host slices retain every command exactly once except per-job metadata');
assert.deepEqual([...hostTests.probes, ...hostBuild.probes], hostFull.probes);
const nativeRuntime = await productDispatch('--android-runtime');
assert.ok(nativeRuntime.calls.some(call => call.includes('--bins') && call.includes('--release')), 'Runtime must execute release CLI/exporter');
const nativeBuild = await productDispatch('--android-x86_64', { serial: null });
assert.ok(nativeBuild.calls.some(call => call.includes('--all-targets') && call.includes('--release')));
assert.deepEqual(nativeBuild.probes, [['--target', 'x86_64-linux-android']]);

const uiSource = (await Bun.file(resolve(productRoot, 'scripts/check-ui.mjs')).text())
  .replace(/^import .*;\r?\n/gm, '').replaceAll('import.meta.dir', JSON.stringify(resolve(productRoot, 'scripts')));
const executeUI = new (Object.getPrototypeOf(async function () {}).constructor)('Bun', 'process', 'assert', 'resolve', 'console', uiSource);
async function shellDispatch(mode, failure = {}, failureAt = -1) {
  const calls = [];
  let error;
  try {
    await executeUI({ ...Bun, argv: ['fixture-bun', 'ui', ...(mode ? [mode] : [])],
      spawnSync(command) {
        calls.push(command);
        return { exitCode: 0, stdout: Buffer.from(`v${versions.node}\n`), ...(calls.length - 1 === failureAt ? failure : {}) };
      },
    }, { execPath: 'fixture-bun', env: {} }, assert, resolve, { log() {} });
  } catch (cause) { error = cause; }
  return { calls, error };
}
const shellFull = await shellDispatch();
const shellMain = await shellDispatch('--main');
const shellConflicts = await shellDispatch('--conflicts');
for (const result of [shellFull, shellMain, shellConflicts]) assert.equal(result.error, undefined);
const sharedShell = command => command.includes('--version') || command.includes('ui:build') || command.includes('build') || command.includes('scripts/check-ui-browser.mjs');
assert.deepEqual(commands([...shellMain.calls, ...shellConflicts.calls].filter(call => !sharedShell(call))),
  commands(shellFull.calls.filter(call => !sharedShell(call))), 'Shell slices retain every nonshared check exactly once');
for (const [mode, result] of [[undefined, shellFull], ['--main', shellMain], ['--conflicts', shellConflicts]]) {
  assert.ok(result.calls.some(call => call.includes('ui:build')));
  assert.ok(result.calls.some(call => call.includes('build') && call.includes('shell-fixture')));
  assert.deepEqual(result.calls.filter(call => call.includes('scripts/check-ui-browser.mjs')).map(call => call.at(-1)), mode ? [mode] : ['--main', '--conflicts']);
  for (let index = 1; index < result.calls.length; index++) for (const failure of [{ exitCode: 23 }, { signalCode: 'SIGTERM' }]) {
    const failed = await shellDispatch(mode, failure, index);
    assert.ok(failed.error);
    assert.equal(failed.calls.length, index + 1, 'Shell failures/signals stop the selected pipeline');
  }
}
assert.ok((await shellDispatch('invalid')).error);

const workflow = Bun.YAML.parse(await Bun.file(resolve(productRoot, ".github/workflows/prototype-foundation.yml")).text());
const suites = ["product", "product-build", "prototype-native", "prototype-ui", "shell", "shell-conflicts"];
const expectedSteps = [
  ["matrix.suite == 'product'", "bun scripts/check-product.mjs --host-tests", "."],
  ["matrix.suite == 'product-build'", "bun scripts/check-product.mjs --host-build", "."],
  ["matrix.suite == 'prototype-native'", "bun run check --host-native", "prototypes"],
  ["matrix.suite == 'prototype-ui'", "bun run check --host-ui", "prototypes"],
  ["matrix.suite == 'shell'", "bun run ui:check --main", "."],
  ["matrix.suite == 'shell-conflicts'", "bun run ui:check --conflicts", "."],
];
const linuxSteps = [
  ["matrix.suite == 'product'", "bun scripts/check-product.mjs", "."],
  ["matrix.suite == 'prototype-ui'", "bun run check", "prototypes"],
  ["matrix.suite == 'shell'", "bun run ui:check", "."],
];
for (const [os, runner, budget] of [["linux", "ubuntu-24.04", 10], ["windows", "windows-2025", 20]]) {
  const job = workflow.jobs[`${os}-checks`];
  assert.deepEqual(job.strategy.matrix.suite, os === 'windows'
    ? ['shell', 'shell-conflicts', 'product', 'product-build', 'prototype-ui', 'prototype-native'] : ['product', 'shell', 'prototype-ui']);
  assert.equal(job.strategy['max-parallel'], os === 'linux' ? 2 : undefined);
  assert.equal(job.strategy["fail-fast"], false);
  assert.equal(job["runs-on"], runner);
  assert.equal(job["timeout-minutes"], budget);
  assert.equal(job.defaults.run.shell, "pwsh");
  assert.equal(job.steps.find(step => step.uses === "./.github/actions/prepare-host-check").with.suite, "${{ matrix.suite }}");
  assert.deepEqual(job.steps.filter(step => step.run).map(step => [step.if, step.run, step["working-directory"] || "."]), os === 'linux' ? linuxSteps : expectedSteps);
  const gate = workflow.jobs[os];
  assert.equal(gate.name, `check (${runner})`);
  assert.equal(gate.needs, `${os}-checks`);
  assert.equal(gate.if, "always()");
  assert.equal(gate["timeout-minutes"], 1);
  assert.equal(gate.steps.length, 1);
  assert.equal(gate.steps[0].env.RESULT, "${{ needs." + os + "-checks.result }}");
  assert.equal(gate.steps[0].run, 'test "$RESULT" = success');
}
assert.ok(!JSON.stringify(workflow).includes("continue-on-error"));

const preparation = Bun.YAML.parse(await Bun.file(resolve(productRoot, ".github/actions/prepare-host-check/action.yml")).text());
const restores = preparation.runs.steps.filter(step => step.uses?.startsWith("actions/cache/restore@") && step.id !== 'browser-cache');
assert.equal(restores.length, 3);
assert.equal(restores[0].with["lookup-only"], true);
assert.equal(restores[0].with["restore-keys"], "zedone-rust-v1-prototype-host-${{ inputs.suite }}-${{ steps.versions.outputs.cache-platform }}-x64-");
assert.equal(restores[1].with["restore-keys"], "zedone-rust-v1-prototype-host-${{ steps.versions.outputs.cache-platform }}-x64-");
assert.equal(restores[1].with.path, "${{ steps.versions.outputs.legacy-cache-paths }}");
assert.equal(restores[2].with.path, "${{ steps.versions.outputs.cache-paths }}");
assert.equal(restores[2].with['restore-keys'], 'zedone-rust-v1-prototype-host-${{ steps.versions.outputs.cache-seed }}-${{ steps.versions.outputs.cache-platform }}-x64-');
for (const os of ["Windows", "Linux", "macOS"]) {
  for (const suite of ["all", ...suites]) {
    for (const matchedKey of ["", "existing-suite-key"]) {
      const context = { runner: { os }, inputs: { suite }, matchedKey };
      const fallback = restores[1].if.replace("steps.suite-cache.outputs.cache-matched-key", "matchedKey");
      assert.equal(runInNewContext(fallback, context), os !== "macOS" && !['all', 'product-build', 'shell-conflicts'].includes(suite) && matchedKey === "");
      const seed = restores[2].if.replace("steps.suite-cache.outputs.cache-matched-key", "matchedKey");
      assert.equal(runInNewContext(seed, context), os !== 'macOS' && ['product-build', 'shell-conflicts'].includes(suite) && matchedKey === '');
      assert.equal(runInNewContext(restores[0].if, context), os !== "macOS" && suite !== "all");
    }
  }
}
const cache = preparation.runs.steps.find(step => step.uses?.startsWith("Swatinem/rust-cache@"));
assert.equal(cache.with["save-if"], "${{ github.ref == 'refs/heads/main' }}");
const cacheExpression = cache.with["shared-key"].replace(/^\$\{\{\s*|\s*\}\}$/g, "");
const cacheNames = ["all", ...suites].map(suite => runInNewContext(cacheExpression, {
  inputs: { suite }, format: (pattern, value) => pattern.replace("{0}", value),
}));
assert.deepEqual(cacheNames, ["prototype-host", ...suites.map(suite => `prototype-host-${suite}`)]);
console.log("Host CI routing/pairing/workflow: exact full/slice coverage, both OS matrices/strict gates, cache owners/fallback, legacy modes and failure draining passed.");
await import('./check-ci-tool-caches.mjs');
await import('./check-ui-main-slices.mjs');
