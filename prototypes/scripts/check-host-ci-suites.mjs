import assert from "node:assert/strict";
import { resolve } from "node:path";

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
console.log("Host CI routing/pairing: exact full/slice coverage, debug/release ownership, legacy modes, failures/signals, overlap and draining both outcomes passed.");
