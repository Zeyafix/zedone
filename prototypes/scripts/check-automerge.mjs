import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const versions = await Bun.file(resolve(root, "../scripts/environment-versions.json")).json();
assert.equal(Bun.version, versions.bun);
assert.ok(Bun.argv.length <= 3 && (!Bun.argv[2] || Bun.argv[2] === "--android"));
const android = Bun.argv[2] === "--android";
const env = { ...process.env, CARGO_TARGET_DIR: resolve(root, "target") };
const suffix = process.platform === "win32" ? ".exe" : "";
const phases = ["prepare", "resume", "conflicts-prepare", "conflicts-resolve", "conflicts-resume",
  "containers-prepare", "containers-resolve", "containers-resume",
  "order-prepare", "order-resolve", "order-resume",
  "tags-prepare", "tags-resolve", "tags-resume",
  "identity-prepare", "identity-rename", "identity-resume",
  "names-prepare", "names-resolve", "names-resume"];

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
  run(cargo(["build", "--bin", "check-automerge"]));
  const directory = await mkdtemp(resolve(tmpdir(), "zedone-automerge-"));
  try {
    for (const phase of phases) {
      run([resolve(root, `target/debug/check-automerge${suffix}`), directory, phase]);
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
    run(cargo(["build", "--bin", "check-automerge", "--target", target]), false, targetEnv(target));
  }
  const target = "x86_64-linux-android";
  const artifacts = run(cargo(["test", "--lib", "--no-run", "--message-format=json", "--target", target]), true, targetEnv(target));
  const testBinary = artifacts.split("\n").filter(Boolean).map(line => JSON.parse(line))
    .find(item => item.reason === "compiler-artifact" && item.profile.test && item.executable)?.executable;
  assert.ok(testBinary, "Expected a Rust test executable");
  const remote = `/data/local/tmp/zedone-automerge-${randomUUID()}`;
  device("shell", "mkdir", remote);
  try {
    device("push", resolve(root, `target/${target}/debug/check-automerge`), `${remote}/probe`);
    device("push", testBinary, `${remote}/tests`);
    device("shell", "chmod", "700", `${remote}/probe`, `${remote}/tests`);
    // Distinct native processes, no shared in-memory document between phases.
    for (const phase of phases) device("shell", `${remote}/probe`, remote, phase);
    device("shell", "env", `TMPDIR=${remote}`, `${remote}/tests`, "automerge_probe::", "--test-threads=1");
  } finally {
    assert.match(remote, /^\/data\/local\/tmp\/zedone-automerge-[0-9a-f-]+$/);
    device("shell", "rm", "-r", remote);
  }
}
