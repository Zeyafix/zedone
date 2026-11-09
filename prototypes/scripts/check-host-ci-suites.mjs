import assert from "node:assert/strict";
import { resolve } from "node:path";
import { runInNewContext } from "node:vm";

// Exercise the real driver, replacing only process/import/write boundaries.
// Never launch Cargo, a browser, another guard or an artifact rewrite here.
const root = resolve(import.meta.dir, "..");
const versions = await Bun.file(resolve(root, "../scripts/environment-versions.json")).json();
const openapi = Buffer.from(await Bun.file(resolve(root, "openapi.json")).arrayBuffer());
const types = await Bun.file(resolve(root, "client/api.d.ts")).text();
const source = (await Bun.file(resolve(import.meta.dir, "check.mjs")).text())
  .replace('import { resolve } from "node:path";', "")
  .replaceAll("import.meta.dir", JSON.stringify(import.meta.dir))
  .replace('await import("openapi-typescript")', "await importTypes()");
const execute = new (Object.getPrototypeOf(async function () {}).constructor)("Bun", "process", "resolve", "importTypes", "console", source);
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
    }, resolve, async () => ({ default() {}, astToString: () => types }), { log() {}, error() {} });
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
  assert.equal(result.calls.length, 1);
  assert.ok(result.calls[0].includes("generate-openapi"));
  assert.equal(result.writes.length, count);
}
// The product driver shares this pairing boundary between host and Android.
// A rejected side must never let the driver abandon the other running side.
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
    // Drain settlements without time-based sleeps or a detached rejection.
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

// The workflow must actually select every slice and reject incomplete matrices.
const workflow = Bun.YAML.parse(await Bun.file(resolve(productRoot, ".github/workflows/prototype-foundation.yml")).text());
const suites = ["product", "prototype-native", "prototype-ui", "shell"];
const expectedSteps = [
  ["matrix.suite == 'product'", "bun scripts/check-product.mjs", "."],
  ["matrix.suite == 'prototype-native'", "bun run check --host-native", "prototypes"],
  ["matrix.suite == 'prototype-ui'", "bun run check --host-ui", "prototypes"],
  ["matrix.suite == 'shell'", "bun run ui:check", "."],
];
for (const [os, runner, budget] of [["linux", "ubuntu-24.04", 10], ["windows", "windows-2025", 20]]) {
  const job = workflow.jobs[`${os}-checks`];
  assert.deepEqual(job.strategy.matrix.suite, suites);
  assert.equal(job.strategy["fail-fast"], false);
  assert.equal(job["runs-on"], runner);
  assert.equal(job["timeout-minutes"], budget);
  assert.equal(job.defaults.run.shell, "pwsh");
  assert.equal(job.steps.find(step => step.uses === "./.github/actions/prepare-host-check").with.suite, "${{ matrix.suite }}");
  assert.deepEqual(job.steps.filter(step => step.run).map(step => [step.if, step.run, step["working-directory"] || "."]), expectedSteps);
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
const restores = preparation.runs.steps.filter(step => step.uses?.startsWith("actions/cache/restore@"));
assert.equal(restores.length, 2);
assert.equal(restores[0].with["lookup-only"], true);
assert.equal(restores[0].with["restore-keys"], "zedone-rust-v1-prototype-host-${{ inputs.suite }}-${{ steps.versions.outputs.cache-platform }}-x64-");
assert.equal(restores[1].with["restore-keys"], "zedone-rust-v1-prototype-host-${{ steps.versions.outputs.cache-platform }}-x64-");
assert.equal(restores[1].with.path, "${{ steps.versions.outputs.legacy-cache-paths }}");
for (const os of ["Windows", "Linux", "macOS"]) {
  for (const suite of ["all", ...suites]) {
    for (const matchedKey of ["", "existing-suite-key"]) {
      const context = { runner: { os }, inputs: { suite }, matchedKey };
      const fallback = restores[1].if.replace("steps.suite-cache.outputs.cache-matched-key", "matchedKey");
      assert.equal(runInNewContext(fallback, context), os !== "macOS" && suite !== "all" && matchedKey === "");
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
