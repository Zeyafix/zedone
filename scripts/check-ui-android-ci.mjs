// CI owns its emulator lifecycle. Local UI verification must use T3 Device;
// this driver deliberately refuses to run outside CI.
import assert from "node:assert/strict";
import { resolve } from "node:path";
import { chromium } from "playwright";

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
try {
  device("install", "-r", resolve(root, "ui/src-tauri/gen/android/app/build/outputs/apk/x86_64/debug/app-x86_64-debug.apk"));
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
  browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  const page = browser.contexts()[0]?.pages()[0];
  assert.ok(page, "ZeDone WebView must be present");
  await page.getByRole("heading", { name: "Доступ не настроен", exact: true }).waitFor();
  await page.getByRole("button", { name: "Меню", exact: true }).click();
  await page.getByRole("dialog").waitFor();
  device("shell", "input", "keyevent", "4");
  await page.getByRole("dialog").waitFor({ state: "hidden" });
  await page.getByRole("button", { name: "Меню", exact: true }).click();
  await page.getByRole("dialog").getByRole("link", { name: "Настройки", exact: true }).click();
  await page.getByRole("radio", { name: "Тёмная", exact: true }).check();
  assert.equal(await page.locator("html").getAttribute("data-theme"), "dark");
  await page.reload();
  await page.getByRole("radio", { name: "Тёмная", exact: true }).waitFor();
  assert.equal(await page.locator("html").getAttribute("data-theme"), "dark");
  await page.getByRole("radio", { name: "Светлая", exact: true }).check();
  assert.equal(await page.locator("html").getAttribute("data-theme"), "light");
  await page.getByRole("radio", { name: "Как на устройстве", exact: true }).check();
  assert.equal(await page.evaluate(() => localStorage.length), 0);
  device("shell", "input", "keyevent", "4");
  await page.getByRole("heading", { level: 1, name: "Пространства" }).waitFor();
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  console.log("Android shell: local session IPC, menu/Back, theme light/dark/reset/reload and layout passed. Provisioned domain IPC remains a separate acceptance cell.");
} finally {
  await browser?.close();
  if (port) device("forward", "--remove", `tcp:${port}`);
  device("shell", "am", "force-stop", app);
}
