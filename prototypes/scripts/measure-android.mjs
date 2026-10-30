import assert from "node:assert/strict";
import { resolve } from "node:path";
import { mkdir } from "node:fs/promises";
import { createHash } from "node:crypto";
import { chromium } from "playwright";
import { fixtureInfo, measurementSources, measureInteractions, workingScreen, summarize } from "./measure-scenarios.mjs";

const root = resolve(import.meta.dir, "..");
const versions = await Bun.file(resolve(root, "../scripts/environment-versions.json")).json();
assert.equal(Bun.version, versions.bun);
const serial = process.env.ANDROID_SERIAL;
assert.match(serial || "", /^emulator-\d+$/, "Only an explicitly selected test emulator is modified");
assert.ok(process.env.ANDROID_HOME, "ANDROID_HOME is required");
const adb = resolve(process.env.ANDROID_HOME, `platform-tools/adb${process.platform === "win32" ? ".exe" : ""}`);
const app = "app.zedone.prototype";
const apk = resolve(root, "src-tauri/gen/android/app/build/outputs/apk/x86_64/debug/app-x86_64-debug.apk");
const reportDirectory = resolve(root, "../docs/reports/p01-11");
await mkdir(reportDirectory, { recursive: true });
const reportPath = resolve(reportDirectory, `android-${Date.now()}.json`);
const report = { status: "running", revision: Bun.spawnSync(["git", "rev-parse", "HEAD"], { cwd: root }).stdout.toString().trim(),
  apk_sha256: createHash("sha256").update(Buffer.from(await Bun.file(apk).arrayBuffer())).digest("hex"),
  sources: await measurementSources(root),
  environment: { build: "existing debug APK; diagnostic only, not release or Galaxy A32 acceptance", serial, bun: Bun.version },
  startup: { stopped_processes: [], running_processes: [] } };
let browser, page, port, original, originalKnown = false;
function device(...args) {
  const result = Bun.spawnSync([adb, "-s", serial, ...args], { timeout: args[0] === "install" ? 120000 : 15000 });
  assert.equal(result.exitCode, 0, `${args[0]}: ${result.stderr}`);
  return result.stdout.toString().trim();
}
async function until(fn) {
  const end = performance.now() + 20000;
  while (performance.now() < end) {
    const value = await fn();
    if (value) return value;
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw new Error("WebView connection timed out");
}
async function disconnect() {
  await browser?.close(); browser = undefined; page = undefined;
  if (port) device("forward", "--remove", `tcp:${port}`);
  port = undefined;
}
async function connect() {
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
const call = request => page.evaluate(({ command, request }) => window.__TAURI_INTERNALS__.invoke(command, { request }), { command, request });
let watchdog;
let interrupt;
const deadline = new Promise((_, reject) => {
  watchdog = setTimeout(() => reject(new Error("Android measurement deadline exceeded")), 300000);
  interrupt = () => reject(new Error("Measurement interrupted"));
});
process.once("SIGINT", interrupt);
async function measure() {
  assert.equal(device("shell", "getprop", "sys.boot_completed"), "1");
  assert.equal(device("shell", "getprop", "ro.build.version.sdk"), "33");
  assert.equal(device("shell", "pm", "list", "packages", "com.google.android.gms"), "");
  device("install", "-r", apk);
  report.environment.webview = device("shell", "dumpsys", "webviewupdate");
  report.environment.abi = device("shell", "getprop", "ro.product.cpu.abi");
  device("shell", "am", "force-stop", app); launch(); await connect();
  original = (await call({ action: "snapshot", space: "A" })).snapshot;
  originalKnown = true;
  await call({ action: "delete", space: "A" });
  report.fixture = fixtureInfo((await call({ action: "seed", space: "A" })).snapshot);
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
try {
  await Promise.race([measure(), deadline]);
} catch (error) {
  report.status = "failed"; report.error = String(error.stack || error); process.exitCode = 1;
} finally {
  try {
    if (originalKnown) {
      if (!page || page.isClosed()) { await disconnect(); launch(); await connect(); }
      if (original) await call({ action: "restore", snapshot: original });
      else await call({ action: "delete", space: "A" });
      assert.deepEqual((await call({ action: "snapshot", space: "A" })).snapshot, original);
    }
    await disconnect(); device("shell", "am", "force-stop", app); report.cleanup = true;
  } catch (error) { report.cleanup = false; report.cleanup_error = String(error); process.exitCode = 1; }
  clearTimeout(watchdog);
  process.removeListener("SIGINT", interrupt);
  await Bun.write(reportPath, JSON.stringify(report, null, 2) + "\n");
  console.log(`Measurement ${report.status}; local report: ${reportPath}`);
}
