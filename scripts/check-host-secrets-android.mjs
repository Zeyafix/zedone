import assert from "node:assert/strict";
import { resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const serial = process.env.ANDROID_SERIAL;
assert.match(serial || "", /^emulator-\d+$/);
assert.ok(process.env.ANDROID_HOME);
const adb = resolve(process.env.ANDROID_HOME, `platform-tools/adb${process.platform === "win32" ? ".exe" : ""}`);
function device(...args) {
  const result = Bun.spawnSync([adb, "-s", serial, ...args], { stdin: "ignore", timeout: 90000 });
  assert.equal(result.exitCode, 0, `Native secrets check: ${args[0]} failed`);
  return result.stdout.toString().trim();
}
const app = "app.zedone.local.test";
assert.equal(device("shell", "getprop", "ro.kernel.qemu"), "1");
assert.equal(device("shell", "pm", "list", "packages", "--user", "0", app), "", "Preserve an existing test installation");
const apk = resolve(root, "ui/src-tauri/gen/android/app/build/outputs/apk/androidTest/x86_64/debug/app-x86_64-debug-androidTest.apk");
assert.ok(await Bun.file(apk).exists(), "Build UI Android instrumentation first");
try {
  device("install", apk);
  const output = device("shell", "am", "instrument", "-w", `${app}/app.zedone.local.SecretStoreInstrumentation`);
  assert.match(output, /^INSTRUMENTATION_RESULT: zedone_secrets=passed$/m, "Native secrets assertions or cleanup failed");
  assert.match(output, /^INSTRUMENTATION_RESULT: zedone_lifecycle=passed$/m, "Native plugins retained an old Activity/WebView");
  assert.match(output, /^INSTRUMENTATION_CODE: -1$/m, "Instrumentation did not complete");
  console.log("Android Keystore: persistence, authenticated encryption, non-exportable key, no overwrite, concurrency, corruption/key loss, atomic recovery, deletion and cleanup passed.");
  console.log("Native app/path/opener/scanner/secrets plugins use the recreated Activity/WebView; lifecycle regression passed.");
} finally {
  device("uninstall", app);
}
