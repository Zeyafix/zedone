import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const versions = await Bun.file(resolve(root, "../scripts/environment-versions.json")).json();
assert.equal(Bun.version, versions.bun);
const flags = Bun.argv.slice(2);
assert.ok(flags.every(f => ["--android", "--measure"].includes(f)) && new Set(flags).size === flags.length);
const android = flags.includes("--android");
const measure = flags.includes("--measure");
const profile = measure ? "release" : "debug";
const build = measure ? ["build", "--release"] : ["build"];
const env = { ...process.env, CARGO_TARGET_DIR: resolve(root, "target") };
const suffix = process.platform === "win32" ? ".exe" : "";
const phases = ["prepare", "resume", "conflicts-prepare", "conflicts-resolve", "conflicts-resume",
  "containers-prepare", "containers-resolve", "containers-resume",
  "order-prepare", "order-resolve", "order-resume",
  "tags-prepare", "tags-resolve", "tags-resume",
  "identity-prepare", "identity-rename", "identity-resume",
  "names-prepare", "names-resolve", "names-resume",
  "bridge-prepare", "bridge-resolve", "bridge-resume",
  "deletion-prepare", "deletion-resolve", "deletion-resume",
  "trash-prepare", "trash-resolve", "trash-resume",
  "column-trash-prepare", "column-trash-resolve", "column-trash-resume",
  "space-trash-prepare", "space-trash-resolve", "space-trash-resume",
  "purge-prepare", "purge-resolve", "purge-resume",
  "history-prepare", "history-clear", "history-resume",
  "lifecycle-prepare", "lifecycle-restore", "lifecycle-resume",
  "lifecycle-compact-prepare", "lifecycle-compact-run", "lifecycle-compact-resume",
  "lifecycle-measure-smoke"];
const runPhases = measure ? ["lifecycle-measure-small", "lifecycle-measure-medium",
  "lifecycle-measure-large", "lifecycle-measure-history", "lifecycle-measure-long",
  "lifecycle-measure-tags", "lifecycle-measure-order"] : phases;
const measurements = [];

function captureMeasurement(output) {
  const row = JSON.parse(output.trim().split(/\r?\n/).findLast(line => line.startsWith("{")));
  assert.equal(row.build, "release");
  assert.ok(row.memory.peak_working_set_bytes > 0);
  measurements.push(row);
  console.log(`Measured ${row.profile}: ${(row.elapsed_ms / 1000).toFixed(2)}s, Rust peak ${(row.memory.peak_working_set_bytes / 1048576).toFixed(1)} MiB`);
}

function run(command, capture = false, extraEnv = {}) {
  console.log(`> ${command.join(" ")}`);
  const result = Bun.spawnSync(command, {
    cwd: root, env: { ...env, ...extraEnv }, stdin: "ignore",
    stdout: capture ? "pipe" : "inherit", stderr: "inherit",
  });
  assert.equal(result.exitCode, 0, `Command failed: ${command[0]} (${result.signalCode || ""})`);
  return result.stdout?.toString();
}
const cargo = args => ["cargo", `+${versions.rust}`, ...args, "--locked", "--offline", "-p", "zedone-prototype-core"];

if (!android) {
  run(cargo([...build, "--bin", "check-automerge"]));
  const directory = await mkdtemp(resolve(tmpdir(), "zedone-automerge-"));
  try {
    for (const phase of runPhases) {
      // Each measured profile owns a clean database, not the previous workload.
      const targetDirectory = measure ? resolve(directory, phase) : directory;
      if (measure) await mkdir(targetDirectory);
      const output = run([resolve(root, `target/${profile}/check-automerge${suffix}`), targetDirectory, phase], measure);
      if (measure) captureMeasurement(output);
    }
  } finally {
    await rm(directory, { recursive: true });
  }
} else {
  const serial = env.ANDROID_SERIAL;
  assert.match(serial || "", /^emulator-\d+$/, "Explicit test emulator required; physical devices are not modified");
  assert.ok(env.ANDROID_HOME && env.NDK_HOME, "ANDROID_HOME and pinned NDK_HOME required");
  assert.ok((await Bun.file(resolve(env.NDK_HOME, "source.properties")).text()).includes(versions.android.ndk));
  const adb = resolve(env.ANDROID_HOME, `platform-tools/adb${suffix}`);
  const device = (...args) => run([adb, "-s", serial, ...args]);
  assert.equal(run([adb, "-s", serial, "shell", "getprop", "ro.kernel.qemu"], true).trim(), "1");
  assert.equal(run([adb, "-s", serial, "shell", "getprop", "ro.build.version.sdk"], true).trim(), "33");
  assert.equal(run([adb, "-s", serial, "shell", "getprop", "ro.product.cpu.abi"], true).trim(), "x86_64");
  const host = process.platform === "win32" ? "windows-x86_64" : process.platform === "darwin" ? "darwin-x86_64" : "linux-x86_64";
  const tools = resolve(env.NDK_HOME, `toolchains/llvm/prebuilt/${host}/bin`);
  function targetEnv(target) {
    const lower = target.replaceAll("-", "_");
    return {
      [`CC_${lower}`]: resolve(tools, `clang${suffix}`),
      [`AR_${lower}`]: resolve(tools, `llvm-ar${suffix}`),
      [`CFLAGS_${lower}`]: `--target=${target}33`,
      [`CARGO_TARGET_${lower.toUpperCase()}_LINKER`]: resolve(tools, `clang${suffix}`),
      [`CARGO_TARGET_${lower.toUpperCase()}_RUSTFLAGS`]: `-C link-arg=--target=${target}33`,
    };
  }
  for (const target of ["x86_64-linux-android", "aarch64-linux-android"]) {
    run(cargo([...build, "--bin", "check-automerge", "--target", target]), false, targetEnv(target));
  }
  const target = "x86_64-linux-android";
  let testBinary;
  if (!measure) {
    const artifacts = run(cargo(["test", "--lib", "--no-run", "--message-format=json", "--target", target]), true, targetEnv(target));
    testBinary = artifacts.split("\n").filter(Boolean).map(line => JSON.parse(line))
      .find(item => item.reason === "compiler-artifact" && item.profile.test && item.executable)?.executable;
    assert.ok(testBinary, "Expected a Rust test executable");
  }
  const remote = `/data/local/tmp/zedone-automerge-${randomUUID()}`;
  device("shell", "mkdir", remote);
  try {
    device("push", resolve(root, `target/${target}/${profile}/check-automerge`), `${remote}/probe`);
    if (testBinary) device("push", testBinary, `${remote}/tests`);
    device("shell", "chmod", "700", `${remote}/probe`, ...(testBinary ? [`${remote}/tests`] : []));
    // Distinct native processes, no shared in-memory document between phases.
    for (const phase of runPhases) {
      const targetDirectory = measure ? `${remote}/${phase}` : remote;
      if (measure) device("shell", "mkdir", targetDirectory);
      if (measure) captureMeasurement(run([adb, "-s", serial, "shell", `${remote}/probe`, targetDirectory, phase], true));
      else device("shell", `${remote}/probe`, remote, phase);
    }
    if (testBinary) device("shell", "env", `TMPDIR=${remote}`, `${remote}/tests`, "automerge_probe::", "--test-threads=1");
  } finally {
    assert.match(remote, /^\/data\/local\/tmp\/zedone-automerge-[0-9a-f-]+$/);
    device("shell", "rm", "-r", remote);
  }
}
if (measure) {
  const report = resolve(process.env.ZEDONE_AUTOMERGE_REPORT || resolve(root, `../docs/reports/p02-06/measurements-${android ? "android" : "host"}.json`));
  await mkdir(dirname(report), { recursive: true });
  await Bun.write(report, JSON.stringify({ checked: new Date().toISOString(), platform: android ? "Android x86_64 emulator/API33" : process.platform,
    scope: "one native Rust process per release workload; no UI/renderer or whole-app memory acceptance", measurements }, null, 2) + "\n");
  console.log(`Measurement report: ${report}`);
}
