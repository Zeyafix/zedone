// CI owns its emulator lifecycle. Local UI verification must use T3 Device;
// this driver deliberately refuses to run outside CI.
import assert from "node:assert/strict";
import { resolve } from "node:path";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { chromium } from "playwright";
import { checkShell, checkWorkspaceTouch, nativeControls } from "./check-ui-android-scenarios.mjs";
import { checkCardMarkdown } from "./check-ui-markdown.mjs";

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
  let pid;
  for (let attempt = 0; attempt < 60; attempt++) {
    pid = device("shell", "pidof", app);
    if (/^\d+$/.test(pid)) {
      const sockets = device("shell", "cat", "/proc/net/unix");
      if (sockets.includes(`webview_devtools_remote_${pid}`)) break;
    }
    await new Promise(resolve => setTimeout(resolve, 250));
  }
  assert.match(pid || "", /^\d+$/);
  port = device("forward", "tcp:0", `localabstract:webview_devtools_remote_${pid}`);
  assert.match(port, /^\d+$/);
  // WebView has no desktop browser context/download management. Preserve its
  // settings instead of sending Playwright's default browser overrides.
  browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, { noDefaults: true });
  const page = browser.contexts()[0]?.pages()[0];
  assert.ok(page, "ZeDone WebView must be present");
  return page;
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
try {
  device("install", "-r", resolve(root, "ui/src-tauri/gen/android/app/build/outputs/apk/x86_64/debug/app-x86_64-debug.apk"));
  let page = await openWebView();
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  await checkShell(page, () => device("shell", "input", "keyevent", "4"));
  assert.deepEqual(errors, [], "Shell must not raise page errors");
  console.log("Android shell: unconfigured session IPC, menu/Back, theme light/dark/reset/reload and layout passed.");
  await disconnect();
  device("shell", "am", "force-stop", app);
  const fixture = Bun.spawnSync([resolve(root, "target/debug/examples/shell-fixture"), "--prepare", fixtureDirectory], { stdout: "pipe", stderr: "pipe" });
  assert.equal(fixture.exitCode, 0, "Synthetic fixture preparation must succeed");
  // This driver runs only on CI's newly owned AVD. Never run on local data.
  device("shell", "run-as", app, "rm", "-rf", "product");
  device("push", `${fixtureDirectory}/.`, remote);
  device("shell", "run-as", app, "cp", "-r", remote, "product");
  page = await openWebView();
  page.on("pageerror", error => errors.push(error.message));
  await checkWorkspaceTouch(page, () => device("shell", "input", "keyevent", "4"), dismissKeyboard);
  const { tap, fill, visible } = await nativeControls(page);
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
} finally {
  await disconnect();
  device("shell", "am", "force-stop", app);
  device("shell", "run-as", app, "rm", "-rf", "product");
  device("shell", "rm", "-rf", remote);
  assert.ok(fixtureDirectory.startsWith(resolve(tmpdir(), "zedone-shell-android-")));
  await rm(fixtureDirectory, { recursive: true });
}
