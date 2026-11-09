import assert from "node:assert/strict";
import { mkdtemp, readFile, rename, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { chromium } from "playwright";
import { checkWorkspace } from "./check-ui-workspace.mjs";
import { checkCards } from "./check-ui-cards.mjs";
import { checkEditor } from "./check-ui-editor.mjs";
import { checkCardMarkdown } from "./check-ui-markdown.mjs";
import { checkNativeControls } from "./check-ui-native-controls.mjs";

const root = resolve(import.meta.dir, "..");
const directory = await mkdtemp(resolve(tmpdir(), "zedone-ui-"));
const suffix = process.platform === "win32" ? ".exe" : "";
const target = resolve(process.env.CARGO_TARGET_DIR || resolve(root, "target"));
const fixture = Bun.spawn([resolve(target, `debug/examples/shell-fixture${suffix}`), "--serve", directory], {
  stdout: "pipe", stderr: "inherit", env: { ...process.env, ZEDONE_CLOCK_DIAGNOSTICS: "1" },
});
let browser;
let diagnosticPage;
let diagnosticReads;
try {
  const reader = fixture.stdout.getReader();
  let address = "";
  while (!address.includes("\n")) {
    const { value, done } = await reader.read();
    assert.ok(!done, "fixture exited before readiness");
    address += new TextDecoder().decode(value);
  }
  reader.releaseLock();
  const url = address.trim();
  assert.match(url, /^http:\/\/127\.0\.0\.1:\d+$/);
  const sessionPath = resolve(directory, "local-session.json");
  const originalSession = await readFile(sessionPath);
  const paths = process.env.UI_BROWSER_PATHS ? JSON.parse(process.env.UI_BROWSER_PATHS) : [undefined];
  for (const executablePath of paths) {
    browser = await chromium.launch({ executablePath, headless: true });
    console.log(`UI browser: ${browser.version()}`);
    await checkNativeControls(browser);
    const context = await browser.newContext({ viewport: { width: 1280, height: 900 }, colorScheme: "light" });
    const page = await context.newPage();
    diagnosticPage = page;
    const reads = [];
    diagnosticReads = reads;
    const errors = [];
    const external = [];
    page.on("pageerror", error => { errors.push(error.message); console.error("UI page error:", error.message); });
    page.on("response", async response => {
      const path = new URL(response.url()).pathname;
      if (!path.startsWith("/v1/commands/")) return;
      const body = await response.json().catch(() => undefined);
      if (/\/(space\.(list|read)|column\.list|card\.list)$/.test(path)) {
        reads.push({ path, http: response.status(), status: body?.status, error: body?.error?.code,
          count: body?.result?.spaces?.length ?? body?.result?.columns?.length ?? body?.result?.cards?.length,
          state: body?.result?.state });
        if (reads.length > 30) reads.shift();
      }
      if (body?.status === "error") console.error("UI API error:", path, body.error?.code);
    });
    page.on("requestfailed", request => console.error("UI request failed:", new URL(request.url()).pathname, request.failure()?.errorText));
    page.on("request", request => { if (!request.url().startsWith(url)) external.push(request.url()); });
    await page.goto(url);
    await page.getByRole("heading", { name: "Доступ подтверждён", exact: true }).waitFor();
    if (process.env.UI_SCREENSHOT_DIRECTORY) await page.screenshot({ path: resolve(process.env.UI_SCREENSHOT_DIRECTORY, "shell-desktop.png"), fullPage: true });
    assert.equal(await page.evaluate(() => localStorage.length), 0, "no cached notes, rights, keys or lists");
    await checkWorkspace(page, url, JSON.parse(originalSession.toString()));
    await checkCards(page, url, JSON.parse(originalSession.toString()));
    await checkEditor(page, url, JSON.parse(originalSession.toString()));
    // This dedicated context observes the deliberate external image/link requests.
    assert.deepEqual(external, []);
    const markdownContext = await browser.newContext({ viewport: { width: 360, height: 760 }, colorScheme: "dark" });
    try {
      const markdownPage = await markdownContext.newPage();
      markdownPage.on("pageerror", error => errors.push(error.message));
      await markdownPage.goto(url);
      await checkCardMarkdown(markdownPage);
      assert.equal(await markdownPage.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
    } finally { await markdownContext.close(); }
    await page.getByRole("link", { name: "Настройки", exact: true }).click();
    await page.getByRole("heading", { level: 1, name: "Настройки" }).waitFor();
    await page.getByRole("radio", { name: "Тёмная", exact: true }).check();
    assert.equal(await page.locator("html").getAttribute("data-theme"), "dark");
    await page.reload();
    assert.equal(await page.locator("html").getAttribute("data-theme"), "dark");
    await page.getByRole("radio", { name: "Светлая", exact: true }).check();
    assert.equal(await page.locator("html").getAttribute("data-theme"), "light");
    await page.getByRole("radio", { name: "Как на устройстве", exact: true }).check();
    await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" });
    await page.waitForFunction(() => document.documentElement.dataset.theme === "dark");
    assert.equal(await page.evaluate(() => localStorage.length), 0, "system choice removes preference");
    await page.getByRole("link", { name: "Пространства", exact: true }).click();
    await page.goBack();
    await page.getByRole("heading", { name: "Настройки", level: 1 }).waitFor();
    await page.keyboard.press("Tab");
    assert.equal(await page.evaluate(() => document.activeElement?.getAttribute("type")), "radio");

    await page.setViewportSize({ width: 360, height: 760 });
    await page.getByRole("button", { name: "Меню", exact: true }).click();
    const dialog = page.getByRole("dialog");
    await dialog.waitFor();
    await page.keyboard.press("Escape");
    assert.equal(await page.getByRole("button", { name: "Меню", exact: true }).evaluate(el => el === document.activeElement), true);
    await page.getByRole("button", { name: "Меню", exact: true }).click();
    await dialog.getByRole("link", { name: "Пространства", exact: true }).click();
    await page.getByRole("heading", { name: "Пространства", level: 1 }).waitFor();
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
    assert.equal(await page.getByRole("heading", { name: "Пространства", level: 1 }).evaluate(el => el === document.activeElement), true);
    if (process.env.UI_SCREENSHOT_DIRECTORY) await page.screenshot({ path: resolve(process.env.UI_SCREENSHOT_DIRECTORY, "shell-mobile.png"), fullPage: true });
    await page.evaluate(() => { document.documentElement.style.fontSize = "200%"; });
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
    await page.evaluate(() => { document.documentElement.style.fontSize = ""; });

    await rename(sessionPath, `${sessionPath}.saved`);
    await page.getByRole("button", { name: "Проверить снова", exact: true }).click();
    await page.getByRole("heading", { name: "Доступ не настроен", exact: true }).waitFor();
    await rename(`${sessionPath}.saved`, sessionPath);
    const invalid = JSON.parse(originalSession.toString());
    invalid.key = "0".repeat(64);
    await writeFile(sessionPath, JSON.stringify(invalid));
    await page.getByRole("button", { name: "Проверить снова", exact: true }).click();
    await page.getByRole("heading", { name: "Нет доступа", exact: true }).waitFor();
    await writeFile(sessionPath, "{");
    await page.getByRole("button", { name: "Проверить снова", exact: true }).click();
    await page.getByRole("heading", { name: "Не удалось связаться с ядром", exact: true }).waitFor();
    await writeFile(sessionPath, originalSession);
    await page.getByRole("button", { name: "Проверить снова", exact: true }).click();
    await page.getByRole("heading", { name: "Доступ подтверждён", exact: true }).waitFor();

    // Malformed envelopes cannot mark the core ready. This is a delivery fault,
    // unlike the successful/denied cases above which use the real Rust Service.
    await page.route("**/v1/commands/space.list", route => route.fulfill({ json: { status: "ok", result: {} } }));
    await page.getByRole("button", { name: "Проверить снова", exact: true }).click();
    await page.getByRole("heading", { name: "Не удалось связаться с ядром", exact: true }).waitFor();
    await page.unroute("**/v1/commands/space.list");
    assert.deepEqual(errors, []);
    assert.deepEqual(external, []);
    await context.close();

    const blocked = await browser.newContext();
    await blocked.addInitScript(() => {
      Object.defineProperty(window, "localStorage", { get() { throw new DOMException("blocked", "SecurityError"); } });
    });
    const blockedPage = await blocked.newPage();
    await blockedPage.goto(`${url}/#/settings`);
    await blockedPage.getByRole("radio", { name: "Тёмная", exact: true }).check();
    await blockedPage.getByRole("status").filter({ hasText: "Тема применена" }).waitFor();
    assert.equal(await blockedPage.locator("html").getAttribute("data-theme"), "dark");
    await blocked.close();
    await browser.close();
    browser = undefined;
    console.log("UI lifecycle, navigation/focus, themes/restart/system/reset, storage failure, genuine HTTP allow/deny/unconfigured, delivery fault passed.");
  }
} catch (error) {
  // This driver owns a synthetic installation. Record rendered state and API
  // error codes, never credentials, headers or persisted intention bodies.
  if (diagnosticPage && !diagnosticPage.isClosed()) {
    const state = await diagnosticPage.evaluate(() => ({
      hash: location.hash,
      headings: [...document.querySelectorAll("h1,h2,h3")].map(el => el.textContent),
      alerts: [...document.querySelectorAll('[role="alert"]')].map(el => el.textContent),
      statuses: [...document.querySelectorAll('[role="status"]')].map(el => el.textContent),
      regions: [...document.querySelectorAll('[role="region"]')].map(el => el.getAttribute("aria-label")),
      busy: document.querySelector(".spaces-view")?.getAttribute("aria-busy"),
    })).catch(cause => ({ unavailable: cause.message }));
    console.error("UI failure state:", JSON.stringify({ ...state, reads: diagnosticReads }));
  }
  throw error;
} finally {
  await browser?.close();
  fixture.kill();
  await fixture.exited;
  assert.ok(directory.startsWith(resolve(tmpdir(), "zedone-ui-")));
  await rm(directory, { recursive: true });
}
