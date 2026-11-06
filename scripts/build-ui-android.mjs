import assert from "node:assert/strict";
import { resolve } from "node:path";

const root = resolve(import.meta.dir, "../ui");
const versions = await Bun.file(resolve(root, "../scripts/environment-versions.json")).json();
assert.equal(Bun.version, versions.bun);
const target = Bun.argv[2] || "x86_64";
const release = Bun.argv[3] === "--release";
assert.ok(["x86_64", "aarch64"].includes(target) && (Bun.argv.length <= 3 || Bun.argv.length === 4 && release));
for (const name of ["JAVA_HOME", "ANDROID_HOME", "NDK_HOME"]) assert.ok(process.env[name], `${name} required`);
assert.ok((await Bun.file(resolve(process.env.NDK_HOME, "source.properties")).text()).includes(versions.android.ndk));
const env = { ...process.env, RUSTUP_TOOLCHAIN: versions.rust, CARGO_NET_OFFLINE: "true", CARGO_TARGET_DIR: resolve(root, "../target") };
const cli = Bun.spawnSync(["cargo", `+${versions.rust}`, "tauri", "--version"], { cwd: root, env });
assert.equal(cli.exitCode, 0);
assert.equal(cli.stdout.toString().trim(), `tauri-cli ${versions.android.tauri_cli}`);
const java = Bun.spawnSync([resolve(process.env.JAVA_HOME, "bin", process.platform === "win32" ? "java.exe" : "java"), "-version"]);
assert.equal(java.exitCode, 0);
assert.ok(java.stderr.toString().includes(`"${versions.android.jdk}"`));
function run(args) {
  const result = Bun.spawnSync(args, { cwd: root, env, stdin: "ignore", stdout: "inherit", stderr: "inherit" });
  assert.equal(result.exitCode, 0, args.join(" "));
}
run(["cargo", `+${versions.rust}`, "metadata", "--manifest-path", "src-tauri/Cargo.toml", "--locked", "--offline", "--no-deps", "--format-version", "1"]);
run(["cargo", `+${versions.rust}`, "tauri", "android", "build", ...(release ? [] : ["--debug"]), "--features", "android-shell", "--target", target, "--split-per-abi", "--apk", "--ci"]);
const arch = target === "aarch64" ? "arm64" : "x86_64";
const profile = release ? "release" : "debug";
const apk = resolve(root, `src-tauri/gen/android/app/build/outputs/apk/${arch}/${profile}/app-${arch}-${profile}${release ? "-unsigned" : ""}.apk`);
const aapt = resolve(process.env.ANDROID_HOME, `build-tools/37.0.0/aapt${process.platform === "win32" ? ".exe" : ""}`);
const manifest = Bun.spawnSync([aapt, "dump", "xmltree", apk, "AndroidManifest.xml"]);
assert.equal(manifest.exitCode, 0);
for (const value of ["app.zedone.local", "MainActivity", "android.intent.category.LAUNCHER", "android.intent.category.DEFAULT"]) {
  assert.ok(manifest.stdout.toString().includes(value), `APK manifest missing ${value}`);
}
console.log("APK manifest identity and launch categories verified.");
