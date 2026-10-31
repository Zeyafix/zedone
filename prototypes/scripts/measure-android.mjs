import assert from "node:assert/strict";
import { resolve } from "node:path";
import { mkdir } from "node:fs/promises";
import { createHash } from "node:crypto";
import { chromium } from "playwright";
import { fixtureInfo, measurementSources, measureInteractions, workingScreen, summarize, uiMeasurementEvidence } from "./measure-scenarios.mjs";

const root = resolve(import.meta.dir, "..");
const pixels = Bun.argv[2] === "--pixels";
const release = Bun.argv[3] === "--release";
const physical = Bun.argv[4] === "--physical";
assert.ok(Bun.argv.length === 2 || (pixels && (Bun.argv.length === 3 || (Bun.argv.length === 4 && release) || (Bun.argv.length === 5 && release && physical))), "Expected no arguments, --pixels, --pixels --release, or --pixels --release --physical");
const versions = await Bun.file(resolve(root, "../scripts/environment-versions.json")).json();
assert.equal(Bun.version, versions.bun);
const serial = process.env.ANDROID_SERIAL;
assert.ok(serial && (physical ? !serial.startsWith("emulator-") : /^emulator-\d+$/.test(serial)), "Explicit test emulator required; a selected physical device requires --pixels --release --physical");
assert.ok(process.env.ANDROID_HOME, "ANDROID_HOME is required");
const adb = resolve(process.env.ANDROID_HOME, `platform-tools/adb${process.platform === "win32" ? ".exe" : ""}`);
const app = "app.zedone.prototype";
const build = release ? "release" : "debug";
const abi = physical ? "arm64" : "x86_64";
const apk = resolve(root, `src-tauri/gen/android/app/build/outputs/apk/${abi}/${build}/app-${abi}-${build}.apk`);
const testApk = resolve(root, `src-tauri/gen/android/app/build/outputs/apk/androidTest/${abi}/release/app-${abi}-release-androidTest.apk`);
const reportDirectory = resolve(root, "../docs/reports/p01-11");
await mkdir(reportDirectory, { recursive: true });
const reportPath = resolve(reportDirectory, `android${physical ? "-physical" : ""}${release ? "-release" : ""}${pixels ? "-pixels" : ""}-${Date.now()}.json`);
const report = { status: "running", revision: Bun.spawnSync(["git", "rev-parse", "HEAD"], { cwd: root }).stdout.toString().trim(),
  ui_measurement_evidence: uiMeasurementEvidence,
  apk_sha256: createHash("sha256").update(Buffer.from(await Bun.file(apk).arrayBuffer())).digest("hex"),
  sources: await measurementSources(root),
  environment: { build: release ? "release APK with local debug-key signing; instrumentation driver, no CDP in measurement; not a distribution build" : "existing debug APK; diagnostic only, not release or Galaxy A32 acceptance", device_kind: physical ? "physical" : "emulator", command_driver: release ? "test-APK WebView.evaluateJavascript" : "CDP", serial, bun: Bun.version },
  startup: { stopped_processes: [], running_processes: [] } };
const driverDeadlineMs = physical ? 600000 : 300000;
report.driver_deadline_ms = driverDeadlineMs;
let browser, page, port, original, originalKnown = false;
let cancellation, cleaningUp = false;
let nativeDriver;
let physicalBackup, originalPhysicalSnapshots;
function ensureActive() { if (cancellation && !cleaningUp) throw cancellation; }
function device(...args) {
  ensureActive();
  const result = Bun.spawnSync([adb, "-s", serial, ...args], { timeout: ["install", "pull"].includes(args[0]) ? 120000 : 15000 });
  assert.equal(result.exitCode, 0, `${args[0]}: ${result.stderr}`);
  return result.stdout.toString().trim();
}
async function until(fn) {
  const end = performance.now() + 20000;
  while (performance.now() < end) {
    ensureActive();
    const value = await fn();
    if (value) return value;
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw new Error("WebView connection timed out");
}
async function disconnect() {
  if (release) { await nativeDriver?.disconnect(); page = undefined; return; }
  await browser?.close(); browser = undefined; page = undefined;
  if (port) device("forward", "--remove", `tcp:${port}`);
  port = undefined;
}
async function connect() {
  if (release) { await nativeDriver.connect(); page = nativeDriver.page; return; }
  const pid = device("shell", "pidof", app);
  port = device("forward", "tcp:0", `localabstract:webview_devtools_remote_${pid}`);
  await until(async () => {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/json/list`, { signal: AbortSignal.timeout(1000) });
      return response.ok && (await response.json()).some(target => target.type === "page");
    }
    catch { return false; }
  });
  browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, { noDefaults: true, timeout: 10000 });
  page = await until(() => browser.contexts()[0]?.pages()[0]);
  page.setDefaultTimeout(10000);
  await page.waitForFunction(() => window.__TAURI_INTERNALS__ && document.querySelector("#board-read"));
}
const launch = () => device("shell", "am", "start", "-W", "-n", `${app}/.MainActivity`);
const schema = await Bun.file(resolve(root, "openapi.json")).json();
const command = schema.paths["/prototype/storage"].post.operationId;
const call = request => { ensureActive(); return page.evaluate(({ command, request }) => window.__TAURI_INTERNALS__.invoke(command, { request }), { command, request }); };
let watchdog;
let interrupt;
const deadline = new Promise((_, reject) => {
  watchdog = setTimeout(() => { cancellation = new Error("Android measurement deadline exceeded"); reject(cancellation); }, driverDeadlineMs);
  interrupt = () => { cancellation = new Error("Measurement interrupted"); reject(cancellation); };
});
process.once("SIGINT", interrupt);
async function measure() {
  assert.equal(device("shell", "getprop", "sys.boot_completed"), "1");
  assert.equal(device("shell", "getprop", "ro.build.version.sdk"), "33");
  if (!physical) assert.equal(device("shell", "pm", "list", "packages", "com.google.android.gms"), "");
  else {
    assert.notEqual(device("shell", "getprop", "ro.kernel.qemu"), "1", "Physical profile must not target an emulator");
    assert.equal(device("shell", "getprop", "ro.product.cpu.abi"), "arm64-v8a");
    assert.ok(await Bun.file(testApk).exists(), "Build the release instrumentation APK before touching the phone");
    const backupModule = await import("./android-physical-backup.mjs");
    originalPhysicalSnapshots = backupModule.originalPhysicalSnapshots;
    physicalBackup = await backupModule.backupPhysical({ root, apk, device, reportDirectory, report });
    // Keep only the test interval awake while USB-powered; restore the exact
    // original global setting in finally, including an originally absent key.
    device("shell", "svc", "power", "stayon", "usb");
  }
  device("install", "-r", apk);
  if (release) {
    const { releaseDriver } = await import("./android-release-driver.mjs");
    nativeDriver = releaseDriver({ adb, serial, device, ensureActive, report, reportDirectory });
    report.test_apk_sha256 = createHash("sha256").update(Buffer.from(await Bun.file(testApk).arrayBuffer())).digest("hex");
    device("install", "-r", testApk);
    report.environment.package = device("shell", "dumpsys", "package", app);
    assert.ok(!/pkgFlags=\[[^\]]*DEBUGGABLE/.test(report.environment.package), "Release APK must not be debuggable");
  }
  report.environment.webview = device("shell", "dumpsys", "webviewupdate");
  report.environment.abi = device("shell", "getprop", "ro.product.cpu.abi");
  device("shell", "am", "force-stop", app); launch(); await connect();
  if (release) {
    const pid = device("shell", "pidof", app);
    const socketPresent = device("shell", "cat", "/proc/net/unix").split("\n").some(line => line.trim().endsWith(`@webview_devtools_remote_${pid}`));
    const osBuild = device("shell", "getprop", "ro.build.type");
    // Chromium enables WebView debugging unconditionally on eng/userdebug OS
    // images, even in uninstrumented non-debuggable applications. Record this
    // environment limitation rather than modifying the OS or application.
    assert.ok(!socketPresent || ["eng", "userdebug"].includes(osBuild), "Unexpected WebView debugging socket on a user OS build");
    report.environment.os_build_type = osBuild;
    report.environment.webview_debug_socket_present = socketPresent;
    report.environment.cdp_used = false;
    report.environment.debugging_limitation = socketPresent ? "Debug Android OS forces WebView debugging; no CDP connection is made. This does not validate a retail user OS." : null;
    report.environment.application_debuggable = false;
  }
  original = (await call({ action: "snapshot", space: "A" })).snapshot;
  originalKnown = true;
  if (physical) {
    assert.deepEqual(original, physicalBackup.snapshots.A, "APK update must preserve A");
    assert.deepEqual((await call({ action: "snapshot", space: "B" })).snapshot, physicalBackup.snapshots.B, "APK update must preserve B");
  }
  await call({ action: "delete", space: "A" });
  report.fixture = fixtureInfo((await call({ action: "seed", space: "A" })).snapshot);
  if (pixels) {
    const { measureAndroidPixels } = await import("./measure-android-pixels.mjs");
    await measureAndroidPixels({ getPage: () => page, call, device, adb, serial, report, reportDirectory, disconnect, launch, connect, ensureActive });
    return;
  }
  for (let i = 0; i < 10; i++) {
    report.active_attempt = { series: "stopped_processes", repeat: i };
    await disconnect(); device("shell", "am", "force-stop", app);
    const start = performance.now();
    launch(); await connect(); await workingScreen(page);
    report.startup.stopped_processes.push({ ok: true, ms: performance.now() - start });
    console.log(`Stopped-process startup ${i + 1}/10 recorded.`);
  }
  for (let i = 0; i < 10; i++) {
    report.active_attempt = { series: "running_processes", repeat: i };
    device("shell", "input", "keyevent", "KEYCODE_HOME");
    const start = performance.now();
    launch(); await workingScreen(page);
    report.startup.running_processes.push({ ok: true, ms: performance.now() - start });
    console.log(`Running-process startup ${i + 1}/10 recorded.`);
  }
  report.startup.summaries = Object.fromEntries(Object.entries(report.startup).map(([name, values]) => [name, summarize(values, 3000)]));
  report.startup.method = "Driver monotonic clock: before adb am start -W through CDP and usable persisted board. Includes adb/CDP overhead; force-stop vs Home/resume. No OS cache reset.";
  await measureInteractions(page, call, report);
  report.status = "passed_available_slice_debug_emulator";
}
const measurement = measure();
try {
  await Promise.race([measurement, deadline]);
} catch (error) {
  cancellation ||= error;
  // Drain bounded operations before restoring A: no late command may race cleanup.
  await measurement.catch(() => {});
  report.status = "failed"; report.error = String(error.stack || error); process.exitCode = 1;
} finally {
  cleaningUp = true;
  try {
    if (originalKnown) {
      if (!page || page.isClosed()) { await disconnect(); launch(); await connect(); }
      if (original) await call({ action: "restore", snapshot: original });
      else await call({ action: "delete", space: "A" });
      assert.deepEqual((await call({ action: "snapshot", space: "A" })).snapshot, original);
      if (physical) assert.deepEqual((await call({ action: "snapshot", space: "B" })).snapshot, physicalBackup.snapshots.B, "B must remain unchanged");
    }
    await disconnect(); device("shell", "am", "force-stop", app); report.cleanup = true;
  } catch (error) { report.cleanup = false; report.cleanup_error = String(error); process.exitCode = 1; }
  if (physicalBackup) {
    try {
      device("install", "-r", physicalBackup.apk);
      if (physicalBackup.test_apk) device("install", "-r", physicalBackup.test_apk);
      else device("uninstall", `${app}.test`);
      assert.deepEqual(await originalPhysicalSnapshots({ root, device }), physicalBackup.snapshots);
      report.original_apk_and_snapshots_restored = true;
    } catch (error) { report.original_apk_and_snapshots_restored = false; report.restore_error = String(error.stack || error); process.exitCode = 1; }
    finally {
      try {
        if (physicalBackup.stay_on === "null") device("shell", "settings", "delete", "global", "stay_on_while_plugged_in");
        else device("shell", "settings", "put", "global", "stay_on_while_plugged_in", physicalBackup.stay_on);
        assert.equal(device("shell", "settings", "get", "global", "stay_on_while_plugged_in"), physicalBackup.stay_on);
        device("shell", "am", "force-stop", app);
        report.device_setting_restored = true;
      } catch (error) { report.device_setting_restored = false; report.setting_restore_error = String(error); process.exitCode = 1; }
    }
  }
  clearTimeout(watchdog);
  process.removeListener("SIGINT", interrupt);
  await Bun.write(reportPath, JSON.stringify(report, null, 2) + "\n");
  console.log(`Measurement ${report.status}; physical display not validated; local report: ${reportPath}`);
}
