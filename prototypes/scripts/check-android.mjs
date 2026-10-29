import assert from "node:assert/strict";
import { resolve } from "node:path";
import { chromium } from "playwright";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { checkStorage, checkStorageUI } from "./storage-scenarios.mjs";
import { checkMarkdown } from "./markdown-scenarios.mjs";
import { checkBoard, checkBoardUI, checkBoardRejection, attemptMove, reconciled } from "./board-scenarios.mjs";

const root = resolve(import.meta.dir, "..");
const versions = await Bun.file(resolve(root, "../scripts/environment-versions.json")).json();
assert.equal(Bun.version, versions.bun);
const serial = process.env.ANDROID_SERIAL;
assert.match(serial || "", /^emulator-\d+$/, "Set ANDROID_SERIAL to the test emulator; physical devices are not modified");
assert.ok(process.env.ANDROID_HOME, "ANDROID_HOME is required");
const adb = resolve(process.env.ANDROID_HOME, `platform-tools/adb${process.platform === "win32" ? ".exe" : ""}`);
const app = "app.zedone.prototype";
const apk = resolve(root, "src-tauri/gen/android/app/build/outputs/apk/x86_64/debug/app-x86_64-debug.apk");
const directory = await mkdtemp(resolve(tmpdir(), "zedone-android-storage-"));
let browser, port, server, page;
let lock;
const errors = [];

function device(...args) {
  const result = Bun.spawnSync([adb, "-s", serial, ...args], { timeout: args[0] === "install" ? 90000 : 15000 });
  assert.equal(result.exitCode, 0, `${args[0]}: ${result.signalCode || ""} ${result.stderr}`);
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
  const page = await until(() => browser.contexts()[0]?.pages()[0], "APK WebView page target");
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
  server = Bun.spawn([executable, "0", directory], { cwd: root, stdout: "pipe", stderr: "inherit" });
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
      const element = document.querySelector(selector);
      element.scrollIntoView({ block: "center" });
      const box = element.getBoundingClientRect();
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
  await page.context().setOffline(false);
  await checkMarkdown(page, { tap, external: async (url, click) => {
    for (const url of ["file:///secret", "mailto:test@example.invalid", "javascript:alert(1)"]) {
      await assert.rejects(page.evaluate(url => window.__TAURI_INTERNALS__.invoke("plugin:opener|open_url", { url }), url), "Opener capability must reject non-HTTP URLs");
    }
    await page.evaluate(() => {
      const original = window.ipc;
      window.ipc = { postMessage(message) {
        if (JSON.parse(message).cmd === "plugin:opener|open_url") {
          window.ipc = original;
          throw new Error("Injected opener bridge failure");
        }
        original.postMessage(message);
      } };
    });
    await click();
    await page.waitForFunction(() => document.querySelector("#markdown-link-error"));
    const component = device("shell", "cmd", "package", "resolve-activity", "--brief", "-a", "android.intent.action.VIEW", "-d", url).split("\n").at(-1).trim();
    assert.ok(component.includes("/") && !component.startsWith(app), "A separate browser must handle HTTP URLs");
    await click();
    await until(() => {
      const activity = device("shell", "dumpsys", "activity", "activities");
      return activity.includes(url) && activity.split("\n").some(line => /topResumedActivity=|ResumedActivity:/.test(line) && line.includes(component));
    }, "external browser receives the exact HTTP URL");
    launch();
    await page.waitForFunction(() => document.querySelector("#markdown-content"));
    assert.equal(await page.locator("#markdown-link-error").count(), 0);
  } });
  await page.context().setOffline(true);
  const storageCommand = api.paths["/prototype/storage"].post.operationId;
  const storage = request => page.evaluate(({ command, request }) => window.__TAURI_INTERNALS__.invoke(command, { request }), { command: storageCommand, request });
  const expected = await checkStorage(storage);
  const hostExpected = await checkStorage(async request => {
    const response = await fetch(`${address.trim()}/prototype/storage`, {
      method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(request),
    });
    if (!response.ok) throw new Error(`Storage HTTP ${response.status}`);
    return response.json();
  });
  assert.deepEqual(expected, hostExpected, "IPC and HTTP must return identical persisted state");
  await checkStorageUI(page, tap);
  await checkBoard(storage);
  const boardDriver = await checkBoardUI(page, storage, tap, cdp);
  await checkBoardRejection(page, storage, tap);
  await tap("#card-move-1");
  device("shell", "input", "keyevent", "KEYCODE_BACK");
  await page.waitForFunction(() => !document.querySelector('[role="dialog"]'));
  assert.equal(await page.evaluate(() => document.activeElement.id), "card-move-1");
  // Real IPC write first, then lose only its reply. A read must recover the committed order.
  assert.equal(await page.evaluate(() => {
    const original = window.ipc;
    window.ipc = { postMessage(message) {
      const data = JSON.parse(message);
      if (data.payload?.request?.action?.startsWith("move_")) {
        window.ipc = original;
        const internals = window.__TAURI_INTERNALS__;
        internals.callbacks.set(data.callback, () => {
          internals.unregisterCallback(data.callback);
          internals.runCallback(data.error, { message: "Injected lost IPC reply after commit" });
        });
      }
      original.postMessage(message);
    } };
    return window.ipc !== original;
  }), true);
  await attemptMove(page, tap);
  await reconciled(page);
  const moved = (await storage({ action: "snapshot", space: "A" })).snapshot;
  assert.equal(moved.cards.find(card => card.id === 1).column, 1);
  assert.equal(new Set(moved.cards.map(card => card.id)).size, 30);
  await storage({ action: "restore", snapshot: expected.a });
  await tap("#board-read");
  await page.waitForFunction(() => !document.querySelector("#board-busy"));
  const boardSql = sql => {
    const result = Bun.spawnSync([adb, "-s", serial, "shell", "run-as", app, "sqlite3", "p01-05-synthetic/synthetic.sqlite3"], {
      stdin: Buffer.from(sql), timeout: 5000,
    });
    assert.equal(result.exitCode, 0, result.stderr.toString());
  };
  boardSql("CREATE TRIGGER board_fail BEFORE INSERT ON prototype_cards WHEN NEW.id = 2 BEGIN SELECT RAISE(ABORT, 'board failure'); END;");
  try {
    await boardDriver.reorder();
    await reconciled(page);
    assert.deepEqual((await storage({ action: "snapshot", space: "A" })).snapshot, expected.a);
    assert.equal(await page.evaluate(() => document.querySelector('.card-list[data-column="0"] .board-card').dataset.card), "1");
  } finally { boardSql("DROP TRIGGER IF EXISTS board_fail;"); }
  console.log("Android board: system Back, real SQL rollback and lost committed IPC reply reconciled without duplicates.");
  // Test-only sqlite3 holds a real lock; all data changes still go through Rust.
  lock = Bun.spawn([adb, "-s", serial, "shell", "run-as", app, "sqlite3", "p01-05-synthetic/synthetic.sqlite3"], {
    stdin: "pipe", stdout: "pipe", stderr: "inherit",
  });
  lock.stdin.write("BEGIN EXCLUSIVE;\n.print LOCKED\n");
  await lock.stdin.flush();
  const lockReader = lock.stdout.getReader();
  let lockTimer;
  try {
    await Promise.race([
      (async () => {
        let output = "";
        while (!output.includes("LOCKED")) {
          const { value, done } = await lockReader.read();
          assert.ok(!done, `SQLite lock helper exited: ${output}`);
          output += new TextDecoder().decode(value);
        }
      })(),
      new Promise((_, reject) => { lockTimer = setTimeout(() => reject(new Error("SQLite lock helper timeout")), 5000); }),
    ]);
  } finally { clearTimeout(lockTimer); }
  lockReader.releaseLock();
  try {
    await tap("#storage-delete");
    await page.waitForFunction(() => document.querySelector("#storage-busy"));
    await fill("Android остаётся отзывчивым");
    await send();
    await wait("status");
    assert.equal(await result(), "Android остаётся отзывчивым");
    assert.equal(await page.locator("#storage-busy").count(), 1, "IPC echo completes while SQLite is locked");
    await page.waitForFunction(() => document.querySelector("#storage-error"));
  } finally {
    lock.stdin.end("ROLLBACK;\n.quit\n");
    await lock.exited;
    lock = undefined;
  }
  assert.deepEqual((await storage({ action: "snapshot", space: "A" })).snapshot, expected.a);
  await tap("#storage-read");
  await page.waitForFunction(() => !document.querySelector("#storage-busy"));
  assert.equal(await page.locator("#storage-error").count(), 0);
  console.log("Real Android SQLite lock: UI/IPC responsive, failed write unchanged, explicit read recovered.");
  const persisted = (await storage({ action: "move_card", expected: expected.a, card: 1, column: 2, position: 0 })).snapshot;
  await disconnect();
  device("shell", "am", "force-stop", app);
  launch();
  page = await connect();
  assert.equal(await input(), "", "Synthetic session is not persisted");
  assert.equal(await count("status"), 0);
  assert.deepEqual((await storage({ action: "snapshot", space: "A" })).snapshot, persisted);
  assert.deepEqual((await storage({ action: "snapshot", space: "B" })).snapshot, expected.b);
  await storage({ action: "delete", space: "A" });
  await storage({ action: "delete", space: "B" });
  console.log("SQLite data survives real Android force-stop/restart; synthetic sets removed through the API.");
  assert.deepEqual(errors, []);
  console.log("Android IPC/HTTP parity, offline UI, Unicode, rejection/retry, keyboard/Back and restart passed.");
} catch (error) {
  if (page && !page.isClosed()) console.error(await page.evaluate(() => document.body.innerText.slice(0, 2000)).catch(() => "WebView closed"));
  throw error;
} finally {
  if (lock) { lock.stdin.end("ROLLBACK;\n.quit\n"); await lock.exited; }
  await disconnect();
  server?.kill();
  if (server) await server.exited;
  await rm(directory, { recursive: true });
  device("shell", "am", "force-stop", app);
}
