import assert from "node:assert/strict";
import { resolve } from "node:path";

assert.equal(process.env.CI, "true", "Local checks must use Device/agent-device");
const serial = process.env.ANDROID_SERIAL;
assert.match(serial || "", /^emulator-\d+$/);
assert.ok(process.env.ANDROID_HOME);
const adb = resolve(process.env.ANDROID_HOME, "platform-tools/adb");
const apk = resolve(import.meta.dir, "../ui/src-tauri/gen/android/app/build/outputs/apk/x86_64/debug/app-x86_64-debug.apk");
const result = Bun.spawnSync([adb, "-s", serial, "install", "-r", apk], {
  env: process.env, stdin: "ignore", stdout: "inherit", stderr: "inherit", timeout: 90000,
});
assert.ok(!result.signalCode, "Production APK installation was interrupted");
assert.equal(result.exitCode, 0, "Production APK installation failed");
