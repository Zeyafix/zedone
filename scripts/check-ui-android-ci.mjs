// CI owns its emulator lifecycle. Local UI verification must use T3 Device;
// this driver deliberately refuses to run outside CI.
import assert from "node:assert/strict";
import { resolve } from "node:path";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { chromium } from "playwright";
import { checkShell, checkWorkspaceTouch, nativeControls } from "./check-ui-android-scenarios.mjs";
import { checkCardMarkdown } from "./check-ui-markdown.mjs";
import { checkBoardMoves } from "./check-ui-moves.mjs";
import { checkTags } from "./check-ui-tags.mjs";
import { checkSearch } from "./check-ui-search.mjs";
import { checkTrash } from "./check-ui-trash.mjs";
import { checkHistory } from "./check-ui-history.mjs";
import { shellDiagnosticState } from "./check-ui-diagnostics.mjs";
import { waitForWebViewTarget } from "../prototypes/scripts/android-webview-target.mjs";

assert.equal(process.env.CI, "true", "Local checks must use Device/agent-device");
const root = resolve(import.meta.dir, "..");
const serial = process.env.ANDROID_SERIAL;
assert.match(serial || "", /^emulator-\d+$/);
assert.ok(process.env.ANDROID_HOME);
const adb = resolve(process.env.ANDROID_HOME, "platform-tools/adb");
const app = "app.zedone.local";
function device(...args) {
  const result = Bun.spawnSync([adb, "-s", serial, ...args], { timeout: 90000 });
  assert.equal(result.exitCode, 0, `${args[0]} failed: ${result.stderr.toString()}`);
  return result.stdout.toString().trim();
}
let browser, port;
const fixtureDirectory = await mkdtemp(resolve(tmpdir(), "zedone-shell-android-"));
const remote = "/data/local/tmp/zedone-shell-ui-fixture";
async function openWebView() {
  device("shell", "am", "start", "-W", "-a", "android.intent.action.MAIN", "-c", "android.intent.category.DEFAULT", "-c", "android.intent.category.LAUNCHER", "-n", `${app}/.MainActivity`);
  // Socket, discoverable target and Playwright page are separate milestones.
  // Share the previous 60 x 250 ms readiness budget across all three.
  const deadline = Date.now() + 15000;
  async function until(read, description) {
    while (Date.now() < deadline) {
      const value = await read();
      if (value) return value;
      await new Promise(resolve => setTimeout(resolve, Math.min(250, Math.max(0, deadline - Date.now()))));
    }
    throw new Error(`Timed out: ${description}`);
  }
  const pid = await until(() => {
    const pid = device("shell", "pidof", app);
    return /^\d+$/.test(pid) && device("shell", "cat", "/proc/net/unix").includes(`webview_devtools_remote_${pid}`) && pid;
  }, "ZeDone WebView debugging socket");
  port = device("forward", "tcp:0", `localabstract:webview_devtools_remote_${pid}`);
  assert.match(port, /^\d+$/);
  const endpoint = `http://127.0.0.1:${port}`;
  try {
    await waitForWebViewTarget(endpoint, until);
    // Preserve WebView settings; desktop context/download overrides do not apply.
    browser = await chromium.connectOverCDP(endpoint, { timeout: Math.max(1, deadline - Date.now()), noDefaults: true });
    return await until(() => browser.contexts().flatMap(context => context.pages()).find(page => !page.isClosed()), "ZeDone WebView page");
  } catch (error) {
    for (const [label, collect] of [
      ["WebView targets", async () => (await fetch(`${endpoint}/json/list`, { signal: AbortSignal.timeout(1000) })).text()],
      ["Playwright pages", () => JSON.stringify(browser?.contexts().map(context => context.pages().map(page => ({ url: page.url(), closed: page.isClosed() }))))],
      ["Android lifecycle log", () => device("logcat", "-d", "-t", "80", "-v", "brief", "chromium:W", "AndroidRuntime:E", "ActivityManager:I", "*:S")],
    ]) {
      try { console.error(`${label}: ${String(await collect()).slice(0, 8000)}`); }
      catch { console.error(`${label}: unavailable`); }
    }
    throw error;
  }
}
async function disconnect() {
  await browser?.close(); browser = undefined;
  if (port) { device("forward", "--remove", `tcp:${port}`); port = undefined; }
}
async function dismissKeyboard() {
  const shown = () => {
    const state = device("shell", "dumpsys", "input_method");
    const input = /\bmInputShown=(true|false)\b/.exec(state);
    const view = /\bmIsInputViewShown=(true|false)\b/.exec(state);
    assert.ok(input && view, "Android 13 must report native IME and input-view state");
    // An active IME may have no input view (e.g. a hardware/test keyboard).
    // Conversely, the service may retain its view after its window is hidden.
    return input[1] === "true" && view[1] === "true";
  };
  if (!shown()) return;
  device("shell", "input", "keyevent", "4");
  const deadline = Date.now() + 5000;
  while (shown()) {
    assert.ok(Date.now() < deadline, "Native Back must hide the IME");
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  console.log("Android native Back hid the IME; checking form/board Back ordering next.");
}
let page, stage = "install";
const errors = [];
try {
  device("install", "-r", resolve(root, "ui/src-tauri/gen/android/app/build/outputs/apk/x86_64/debug/app-x86_64-debug.apk"));
  stage = "unconfigured-shell";
  page = await openWebView();
  page.on("pageerror", error => errors.push(error.message));
  await checkShell(page, () => device("shell", "input", "keyevent", "4"));
  assert.deepEqual(errors, [], "Shell must not raise page errors");
  console.log("Android shell: unconfigured session IPC, menu/Back, theme light/dark/reset/reload and layout passed.");
  await disconnect();
  stage = "prepare-fixture";
  device("shell", "am", "force-stop", app);
  const fixture = Bun.spawnSync([resolve(root, "target/debug/examples/shell-fixture"), "--prepare", fixtureDirectory], { stdout: "pipe", stderr: "pipe" });
  assert.equal(fixture.exitCode, 0, "Synthetic fixture preparation must succeed");
  // This driver runs only on CI's newly owned AVD. Never run on local data.
  device("shell", "run-as", app, "rm", "-rf", "product");
  device("push", `${fixtureDirectory}/.`, remote);
  device("shell", "run-as", app, "cp", "-r", remote, "product");
  page = await openWebView();
  page.on("pageerror", error => errors.push(error.message));
  stage = "workspace";
  await checkWorkspaceTouch(page, () => device("shell", "input", "keyevent", "4"), dismissKeyboard);
  const { tap, fill, visible } = await nativeControls(page);
  stage = "moves";
  await checkBoardMoves(page, { tap, back: () => device("shell", "input", "keyevent", "4") });
  stage = "tags";
  await checkTags(page, { tap, fill, back: () => device("shell", "input", "keyevent", "4"), dismissKeyboard });
  stage = "search";
  await checkSearch(page, { tap, fill, back: () => device("shell", "input", "keyevent", "4"), dismissKeyboard });
  stage = "trash";
  await checkTrash(page, { tap, back: () => device("shell", "input", "keyevent", "4") });
  await checkHistory(page, { tap, fill, back: () => device("shell", "input", "keyevent", "4"), dismissKeyboard });
  stage = "markdown";
  await checkCardMarkdown(page, { tap, fill, visible, dismissKeyboard, external: async (url, click) => {
    for (const forbidden of ["file:///secret", "mailto:test@example.invalid", "javascript:alert(1)"]) {
      await assert.rejects(page.evaluate(url => window.__TAURI_INTERNALS__.invoke("plugin:opener|open_url", { url }), forbidden));
    }
    await page.evaluate(() => {
      const original = window.ipc;
      window.ipc = { postMessage(message) {
        if (JSON.parse(message).cmd === "plugin:opener|open_url") { window.ipc = original; throw new Error("Injected opener failure"); }
        original.postMessage(message);
      } };
    });
    await click(); await page.waitForFunction(() => document.querySelector(".markdown-link-error"));
    const component = device("shell", "cmd", "package", "resolve-activity", "--brief", "-a", "android.intent.action.VIEW", "-d", url).split("\n").at(-1).trim();
    assert.ok(component.includes("/") && !component.startsWith(app));
    await click();
    const deadline = Date.now() + 10000;
    for (;;) {
      const activity = device("shell", "dumpsys", "activity", "activities");
      if (activity.includes(url) && activity.split("\n").some(line => /topResumedActivity=|ResumedActivity:/.test(line) && line.includes(component))) break;
      assert.ok(Date.now() < deadline, "External browser must receive exact URL");
      await new Promise(resolve => setTimeout(resolve, 100));
    }
    device("shell", "am", "start", "-W", "-a", "android.intent.action.MAIN", "-c", "android.intent.category.LAUNCHER", "-n", `${app}/.MainActivity`);
    await page.waitForFunction(() => document.querySelector(".markdown-content") && !document.querySelector(".markdown-link-error"));
  } });
  assert.deepEqual(errors, [], "Domain UI must not raise page errors");
} catch (error) {
  // Capture before disconnect/cleanup destroys the native failure state.
  console.error("Android shell failure state:", JSON.stringify({ stage, ...await shellDiagnosticState(page), pageErrors: errors.slice(-5) }));
  try { console.error("Android shell error log:", device("logcat", "-d", "-t", "80", "-v", "brief", "chromium:E", "AndroidRuntime:E", "*:S").slice(0, 8000)); }
  catch { console.error("Android shell error log: unavailable"); }
  throw error;
} finally {
  await disconnect();
  device("shell", "am", "force-stop", app);
  device("shell", "run-as", app, "rm", "-rf", "product");
  device("shell", "rm", "-rf", remote);
  assert.ok(fixtureDirectory.startsWith(resolve(tmpdir(), "zedone-shell-android-")));
  await rm(fixtureDirectory, { recursive: true });
}
