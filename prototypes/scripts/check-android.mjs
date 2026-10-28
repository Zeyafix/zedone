import assert from "node:assert/strict";
import { resolve } from "node:path";
import { chromium } from "playwright";

const root = resolve(import.meta.dir, "..");
const versions = await Bun.file(resolve(root, "../scripts/environment-versions.json")).json();
assert.equal(Bun.version, versions.bun);
const serial = process.env.ANDROID_SERIAL;
assert.match(serial || "", /^emulator-\d+$/, "Set ANDROID_SERIAL to the test emulator; physical devices are not modified");
assert.ok(process.env.ANDROID_HOME, "ANDROID_HOME is required");
const adb = resolve(process.env.ANDROID_HOME, `platform-tools/adb${process.platform === "win32" ? ".exe" : ""}`);
const app = "app.zedone.prototype";
const apk = resolve(root, "src-tauri/gen/android/app/build/outputs/apk/x86_64/debug/app-x86_64-debug.apk");
let browser, port, server, page;
const errors = [];

function device(...args) {
  const result = Bun.spawnSync([adb, "-s", serial, ...args], { timeout: 15000 });
  assert.equal(result.exitCode, 0, result.stderr.toString());
  return result.stdout.toString().trim();
}

async function until(fn, description) {
  const end = Date.now() + 20000;
  while (Date.now() < end) {
    const result = await fn();
    if (result) return result;
    await new Promise(resolve => setTimeout(resolve, 200));
  }
  throw new Error(`Timed out: ${description}`);
}

async function connect() {
  const pid = device("shell", "pidof", app);
  port = device("forward", "tcp:0", `localabstract:webview_devtools_remote_${pid}`);
  await until(async () => {
    try { return (await fetch(`http://127.0.0.1:${port}/json/version`, { signal: AbortSignal.timeout(1000) })).ok; }
    catch { return false; }
  }, "WebView debugging socket");
  // WebView does not implement desktop browser context/download management.
  browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, { timeout: 10000, noDefaults: true });
  const page = browser.contexts()[0].pages()[0];
  assert.ok(page, "APK must have a WebView");
  page.setDefaultTimeout(10000);
  page.on("pageerror", error => errors.push(error.message));
  await page.waitForFunction(() => document.querySelector("h1"));
  return page;
}

function launch() {
  device("shell", "am", "start", "-W", "-n", `${app}/.MainActivity`);
}

async function disconnect() {
  await browser?.close();
  browser = undefined;
  if (port) device("forward", "--remove", `tcp:${port}`);
  port = undefined;
}

try {
  assert.equal(device("shell", "getprop", "sys.boot_completed"), "1");
  assert.equal(device("shell", "getprop", "ro.build.version.sdk"), "33");
  assert.equal(device("shell", "pm", "list", "packages", "com.google.android.gms"), "", "Use the AOSP image without GMS");
  device("install", "-r", apk);
  device("shell", "am", "force-stop", app);
  launch();
  page = await connect();
  console.log(await page.evaluate(() => navigator.userAgent));
  const api = await Bun.file(resolve(root, "openapi.json")).json();
  const command = api.paths["/prototype/echo"].post.operationId;
  const executable = resolve(root, `target/release/serve-echo${process.platform === "win32" ? ".exe" : ""}`);
  server = Bun.spawn([executable, "0"], { cwd: root, stdout: "pipe", stderr: "inherit" });
  const reader = server.stdout.getReader();
  let address = "";
  while (!address.includes("\n")) {
    const { value, done } = await reader.read();
    assert.ok(!done, "HTTP server exited");
    address += new TextDecoder().decode(value);
  }
  reader.releaseLock();
  assert.match(address.trim(), /^http:\/\/127\.0\.0\.1:\d+$/);
  const cdp = await page.context().newCDPSession(page);
  // Playwright 1.64 visibility checks require newer Chromium than AOSP WebView 101.
  // Use native CDP touch events and DOM assertions; do not patch the application.
  const input = () => page.evaluate(() => document.querySelector("#text").value);
  const result = () => page.evaluate(() => document.querySelector('[role="status"]').textContent);
  const count = role => page.evaluate(role => document.querySelectorAll(`[role="${role}"]`).length, role);
  const wait = role => page.waitForFunction(role => document.querySelector(`[role="${role}"]`), role);
  const fill = text => page.evaluate(value => {
    const element = document.querySelector("#text");
    element.value = value;
    element.dispatchEvent(new Event("input", { bubbles: true }));
  }, text);
  async function tap(selector) {
    const { x, y } = await page.evaluate(selector => {
      const box = document.querySelector(selector).getBoundingClientRect();
      return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
    }, selector);
    await cdp.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x, y }] });
    await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
  }
  const send = () => tap('button[type="submit"]');
  const clear = async () => {
    await tap('button[type="button"]');
    await until(async () => await input() === "" && await count("status") === 0 && await count("alert") === 0, "clear form");
  };
  const invoke = args => page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), { command, args });
  // Block WebView networking: successful form requests must still reach local Rust.
  await page.context().setOffline(true);
  for (const text of ["", "Ёж, ЕЖ, café, 😀\n\"строка\"\\\t <script>alert(1)</script>"]) {
    console.log(`Checking IPC/HTTP and form: ${JSON.stringify(text)}`);
    const response = await fetch(`${address.trim()}/prototype/echo`, {
      method: "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ text }), signal: AbortSignal.timeout(5000),
    });
    assert.equal(response.status, 200);
    assert.deepEqual(await invoke({ request: { text } }), await response.json());
    await fill(text);
    await send();
    await wait("status");
    assert.equal(await result(), text);
    await clear();
    assert.equal(await input(), "");
    assert.equal(await count("status"), 0);
  }
  for (const args of [{}, { request: {} }, { request: { text: null } }, { request: { text: 42 } }]) {
    await assert.rejects(invoke(args), "IPC must reject invalid arguments");
  }
  await assert.rejects(page.evaluate(() => window.__TAURI_INTERNALS__.invoke("unknown_command")));
  console.log("IPC parity, offline form, clear and argument rejection passed.");
  // One injected IPC failure must retain input, clear a previous success and allow retry.
  await fill("Повтор после отказа");
  assert.equal(await page.evaluate(() => {
    const original = window.ipc;
    window.ipc = { postMessage() {
      window.ipc = original;
      throw new Error("Injected Android bridge failure");
    } };
    return window.ipc !== original;
  }), true, "Failure injection must replace the Android bridge");
  await send();
  await wait("alert");
  assert.equal(await input(), "Повтор после отказа");
  assert.equal(await count("status"), 0);
  await send();
  await wait("status");
  assert.equal(await result(), "Повтор после отказа");
  assert.equal(await count("alert"), 0);
  await clear();
  console.log("IPC failure and explicit retry passed.");
  assert.equal(await page.evaluate(() => {
    const original = window.ipc;
    window.ipc = { postMessage() { window.ipc = original; } };
    return window.ipc !== original;
  }), true);
  await fill("Повтор после тайм-аута");
  await send();
  await wait("alert");
  assert.equal(await input(), "Повтор после тайм-аута");
  assert.equal(await count("status"), 0);
  await send();
  await wait("status");
  assert.equal(await result(), "Повтор после тайм-аута");
  await clear();
  console.log("Lost IPC reply times out and permits retry.");
  await tap("#text");
  await until(() => /mInputShown=true/.test(device("shell", "dumpsys", "input_method")), "software keyboard opens");
  await page.keyboard.insertText("Русский ввод: ёж");
  assert.equal(await input(), "Русский ввод: ёж");
  device("shell", "input", "keyevent", "KEYCODE_BACK");
  await until(() => /mInputShown=false/.test(device("shell", "dumpsys", "input_method")), "Back hides keyboard");
  assert.equal(await input(), "Русский ввод: ёж");
  await send();
  await wait("status");
  assert.equal(await result(), "Русский ввод: ёж");
  console.log("Russian text input and keyboard/Back passed.");
  device("shell", "input", "keyevent", "KEYCODE_BACK");
  launch();
  await page.waitForFunction(() => document.querySelector("h1"));
  assert.equal(await input(), "Русский ввод: ёж", "Returning to the same activity keeps its form");
  await disconnect();
  device("shell", "am", "force-stop", app);
  launch();
  page = await connect();
  assert.equal(await input(), "", "Synthetic session is not persisted");
  assert.equal(await count("status"), 0);
  assert.deepEqual(errors, []);
  console.log("Android IPC/HTTP parity, offline UI, Unicode, rejection/retry, keyboard/Back and restart passed.");
} catch (error) {
  if (page && !page.isClosed()) console.error(await page.evaluate(() => document.body.innerText).catch(() => "WebView closed"));
  throw error;
} finally {
  await disconnect();
  server?.kill();
  if (server) await server.exited;
  device("shell", "am", "force-stop", app);
}
