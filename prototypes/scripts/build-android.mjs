import assert from "node:assert/strict";
import { resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const versions = await Bun.file(resolve(root, "../scripts/environment-versions.json")).json();
assert.equal(Bun.version, versions.bun, "Use the pinned Bun version");
const target = Bun.argv[2] || "x86_64";
assert.ok(Bun.argv.length <= 3 && ["x86_64", "aarch64"].includes(target), "Expected x86_64 or aarch64");
for (const name of ["JAVA_HOME", "ANDROID_HOME", "NDK_HOME"]) assert.ok(process.env[name], `${name} is required`);
const java = Bun.spawnSync([resolve(process.env.JAVA_HOME, "bin", process.platform === "win32" ? "java.exe" : "java"), "-version"]);
assert.equal(java.exitCode, 0, "JDK is required");
assert.ok(java.stderr.toString().includes(`"${versions.android.jdk}"`), "Use the pinned JDK");
const ndk = await Bun.file(resolve(process.env.NDK_HOME, "source.properties")).text();
assert.ok(ndk.includes(versions.android.ndk), "Use the pinned NDK");
for (const item of versions.android.packages.filter(item => /^(platforms|build-tools)\//.test(item))) {
  const [name, revision] = item.split("@");
  const properties = await Bun.file(resolve(process.env.ANDROID_HOME, name, "source.properties")).text();
  const actual = properties.match(/^Pkg.Revision\s*=\s*(.+)$/m)?.[1].trim();
  const normalize = value => value?.split(".").map(Number).concat([0, 0]).slice(0, 3).join(".");
  assert.equal(normalize(actual), normalize(revision), `SDK package ${name} must match the pinned revision`);
}
const env = {
  ...process.env,
  RUSTUP_TOOLCHAIN: versions.rust,
  CARGO_TARGET_DIR: resolve(root, "target"),
  CARGO_NET_OFFLINE: "true",
};
const cli = Bun.spawnSync(["cargo", `+${versions.rust}`, "tauri", "--version"], { cwd: root, env });
assert.equal(cli.exitCode, 0, "Tauri CLI is required");
assert.equal(cli.stdout.toString().trim(), `tauri-cli ${versions.android.tauri_cli}`);
const metadata = Bun.spawnSync(["cargo", `+${versions.rust}`, "metadata", "--locked", "--offline", "--format-version", "1"], { cwd: root, env });
assert.equal(metadata.exitCode, 0, metadata.stderr.toString());
// CLI's Gradle subprocesses inherit the pinned toolchain and workspace output.
const result = Bun.spawnSync([
  "cargo", `+${versions.rust}`, "tauri", "android", "build", "--debug", "--target", target, "--split-per-abi", "--apk", "--ci",
], { cwd: root, env, stdin: "ignore", stdout: "inherit", stderr: "inherit" });
if (result.exitCode || result.signalCode) process.exit(result.exitCode || 1);
// Compile the real Activity recreation regression against this ABI's built JNI.
const arch = target === "aarch64" ? "Arm64" : "X86_64";
const gradle = resolve(root, "src-tauri/gen/android", process.platform === "win32" ? "gradlew.bat" : "gradlew");
const tests = Bun.spawnSync([gradle, `:app:assemble${arch}DebugAndroidTest`, "-x", `:app:rustBuild${arch}Debug`], {
  cwd: resolve(root, "src-tauri/gen/android"), env, stdin: "ignore", stdout: "inherit", stderr: "inherit",
});
process.exit(tests.exitCode || (tests.signalCode ? 1 : 0));
