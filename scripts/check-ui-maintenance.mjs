import assert from "node:assert/strict";
import { mkdtemp, readFile, writeFile, readdir, rm, mkdir, stat } from "node:fs/promises";
import { resolve, dirname, basename } from "node:path";
import { tmpdir } from "node:os";
import { createHash } from "node:crypto";
import { chromium } from "playwright";

const root = resolve(import.meta.dir, "..");
const reports = resolve(root, "docs/reports/ui-maintenance");
await mkdir(reports, { recursive: true });
const binary = resolve(process.env.CARGO_TARGET_DIR || resolve(root, "target"), `debug/examples/maintenance-fixture${process.platform === "win32" ? ".exe" : ""}`);
let sequence = 30;
async function command(page, key, parameters) {
  return page.evaluate(async ({ key, parameters, id }) => fetch("/v1/commands/maintenance.execute", { method: "POST", headers: { "content-type": "application/json", authorization: `Bearer ${key}` }, body: JSON.stringify({ id: Array(16).fill(id), operation: "maintenance.execute", parameters }) }).then(r => r.json()), { key, parameters, id: sequence++ });
}
async function confirm(page, name) {
  const dialog = page.getByRole("dialog");
  await dialog.getByRole("checkbox", { name: "Подтверждаю это действие", exact: true }).check();
  await dialog.getByRole("button", { name, exact: true }).click();
  await dialog.waitFor({ state: "hidden" });
}
async function waitText(page, text) { await page.getByText(text, { exact: true }).first().waitFor(); }
async function digestTree(path) {
  const output = [];
  for (const name of (await readdir(path)).sort()) {
    const child = resolve(path, name);
    if ((await stat(child)).isDirectory()) output.push([name, await digestTree(child)]);
    else output.push([name, createHash("sha256").update(await readFile(child)).digest("hex")]);
  }
  return output;
}
async function fullBackups(path) { return (await digestTree(path)).filter(([name]) => name.startsWith("zedone-migration-candidate-")); }
const browserPaths = process.env.UI_BROWSER_PATHS ? JSON.parse(process.env.UI_BROWSER_PATHS) : [undefined];
for (const executablePath of browserPaths) for (const touch of [false, true]) for (const unsupported of [false, true]) {
  const directory = await mkdtemp(resolve(tmpdir(), "zedone-maintenance-ui-"));
  const fixture = Bun.spawn([binary, unsupported ? "--serve-unsupported" : "--serve", directory], { stdout: "pipe", stderr: "inherit" });
  let browser;
  try {
    const reader = fixture.stdout.getReader(); let address = "";
    while (!address.includes("\n")) { const result = await reader.read(); assert.ok(!result.done, "maintenance fixture exited before readiness"); address += new TextDecoder().decode(result.value); }
    reader.releaseLock(); address = address.trim(); assert.match(address, /^http:\/\/127\.0\.0\.1:\d+$/);
    const old = JSON.parse(await readFile(resolve(directory, "local-session.json"), "utf8"));
    const original = await readFile(resolve(directory, "zedone.sqlite"));
    const backups = await fullBackups(resolve(directory, "zedone.sqlite.backups"));
    browser = await chromium.launch({ headless: true, executablePath });
    const page = await browser.newPage({ viewport: touch ? { width: 412, height: 860 } : { width: 1280, height: 900 }, hasTouch: touch, isMobile: touch });
    await page.goto(address);
    const panel = page.locator(".maintenance-panel");
    await panel.getByRole("heading", { name: "Служебный режим", exact: true }).waitFor();
    if (unsupported) {
      const durable = async () => Promise.all(["", "-wal", "-journal"].map(suffix => readFile(resolve(directory, `zedone.sqlite${suffix}`)).catch(error => { assert.equal(error.code, "ENOENT"); return null; })));
      const before = await durable();
      const session = await page.evaluate(() => fetch("/host/maintenance-session").then(r => r.json()));
      const inspect = () => command(page, session.key, { action: "inspect" });
      const guidance = panel.getByRole("status").filter({ hasText: "Установите подходящую новую версию ZeDone" });
      await guidance.waitFor();
      assert.equal((await inspect()).result.reason, "UnsupportedSchema");
      assert.equal(await page.getByRole("button", { name: "Создать пространство", exact: true }).count(), 0);
      const denied = await page.evaluate(async key => fetch("/v1/commands/space.list", { method: "POST", headers: { "content-type": "application/json", authorization: `Bearer ${key}` }, body: JSON.stringify({ id: Array(16).fill(22), operation: "space.list", parameters: { include_trashed: false } }) }).then(r => r.json()), old.key);
      assert.equal(denied.error.code, "unavailable");
      await panel.getByRole("button", { name: "Повторить запуск", exact: true }).click();
      await panel.getByRole("alert").waitFor();
      await page.reload();
      await guidance.waitFor();
      assert.equal((await inspect()).result.reason, "UnsupportedSchema");
      assert.deepEqual(await durable(), before);
      assert.deepEqual(await fullBackups(resolve(directory, "zedone.sqlite.backups")), backups);
      await page.screenshot({ path: resolve(reports, `unsupported-${touch ? "touch" : "desktop"}.png`), fullPage: true });
      await writeFile(resolve(directory, "zedone.sqlite"), await readFile(resolve(directory, "supported.sqlite")));
      assert.equal((await inspect()).result.normal_ready, false);
      await panel.getByRole("button", { name: "Повторить исходную команду", exact: true }).click();
      await page.getByText("База до сбоя", { exact: true }).first().waitFor();
      await page.getByText("База до сбоя", { exact: true }).first().click();
      await page.getByText("Вернуть документы", { exact: true }).first().waitFor();
      assert.equal((await inspect()).result.normal_ready, true);
      assert.deepEqual(await fullBackups(resolve(directory, "zedone.sqlite.backups")), backups);
      console.log(`Unsupported schema ${touch ? "touch" : "desktop"}: guidance/API/closed access/unchanged files/failed retry/reload/explicit return passed.`);
      continue;
    }
    await panel.getByRole("radio").first().check();
    await panel.getByRole("button", { name: "Проверить копию", exact: true }).click();
    await waitText(page, "Копия проверена и подготовлена");
    assert.deepEqual(await readFile(resolve(directory, "zedone.sqlite")), original);
    await panel.getByRole("button", { name: "Восстановить базу", exact: true }).click();
    assert.equal(await page.getByRole("dialog").getByRole("button", { name: "Восстановить всю базу", exact: true }).isDisabled(), true);
    await page.getByRole("dialog").getByRole("button", { name: "Отмена", exact: true }).click();
    await panel.getByRole("button", { name: "Отменить подготовку", exact: true }).click();
    await panel.getByRole("button", { name: "Проверить копию", exact: true }).waitFor();
    await panel.getByRole("button", { name: "Удалить сохранённое состояние 1", exact: true }).click();
    await confirm(page, "Подтвердить действие");
    await panel.getByRole("button", { name: "Проверить копию", exact: true }).click();
    await waitText(page, "Копия проверена и подготовлена");
    let lost = false; const applyBodies = [];
    await page.route("**/v1/commands/maintenance.execute", async route => {
      const body = route.request().postData();
      if (JSON.parse(body).parameters.action === "apply") {
        applyBodies.push(body);
        if (!lost) { lost = true; await route.fetch(); await route.abort("failed"); return; }
      }
      await route.continue();
    });
    await panel.getByRole("button", { name: "Восстановить базу", exact: true }).click();
    await confirm(page, "Восстановить всю базу");
    await panel.getByRole("button", { name: "Вернуть проблемное состояние", exact: true }).waitFor();
    assert.ok(applyBodies.length >= 2); assert.ok(applyBodies.every(body => body === applyBodies[0]));
    await page.unroute("**/v1/commands/maintenance.execute");
    assert.deepEqual(await fullBackups(resolve(directory, "zedone.sqlite.backups")), backups);
    await panel.getByRole("button", { name: "Вернуть проблемное состояние", exact: true }).click();
    await confirm(page, "Подтвердить действие");
    assert.deepEqual(await readFile(resolve(directory, "zedone.sqlite")), original);
    await panel.getByRole("button", { name: "Проверить копию", exact: true }).click();
    await waitText(page, "Копия проверена и подготовлена");
    await panel.getByRole("button", { name: "Восстановить базу", exact: true }).click();
    await confirm(page, "Восстановить всю базу");
    await panel.getByRole("button", { name: "Открыть пространства", exact: true }).click();
    await page.getByText("База до сбоя", { exact: true }).first().waitFor();
    await page.getByText("База до сбоя", { exact: true }).first().click();
    await page.getByText("Вернуть документы", { exact: true }).first().waitFor();
    const denied = await page.evaluate(async key => fetch("/v1/commands/space.list", { method: "POST", headers: { "content-type": "application/json", authorization: `Bearer ${key}` }, body: JSON.stringify({ id: Array(16).fill(9), operation: "space.list", parameters: { include_trashed: false } }) }).then(r => r.json()), old.key);
    assert.equal(denied.error.code, "access_denied");
    if (touch) {
      await page.getByRole("button", { name: "Меню", exact: true }).click();
      await page.getByRole("dialog").getByRole("link", { name: "Настройки", exact: true }).click();
    } else await page.getByRole("link", { name: "Настройки", exact: true }).first().click();
    await panel.locator("summary").click();
    await panel.getByRole("button", { name: "Подготовить ключ доступа", exact: true }).click();
    await panel.getByRole("textbox", { name: "Название доступа", exact: true }).fill("Проверка агента");
    const key = await panel.getByRole("textbox", { name: "Ключ для агента", exact: true }).inputValue();
    assert.equal(await panel.getByRole("checkbox", { name: "Разрешить восстановление всей базы и возврат проблемного состояния", exact: true }).isChecked(), false);
    await panel.getByRole("checkbox", { name: "Ключ сохранён, передать этот служебный доступ", exact: true }).check();
    await panel.getByRole("button", { name: "Выдать служебный доступ", exact: true }).click();
    await panel.getByRole("button", { name: "Отозвать Проверка агента", exact: true }).waitFor();
    const read = await command(page, key, { action: "inspect" }); assert.equal(read.result.writable, false); assert.equal(read.result.owner, false);
    await panel.getByRole("button", { name: "Отозвать Проверка агента", exact: true }).click(); await confirm(page, "Подтвердить действие");
    assert.equal((await command(page, key, { action: "inspect" })).error.code, "access_denied");
    await panel.getByRole("button", { name: "Перейти в служебный режим", exact: true }).click(); await confirm(page, "Подтвердить действие");
    await panel.getByRole("heading", { name: "Служебный режим", exact: true }).waitFor();
    await page.screenshot({ path: resolve(reports, `${touch ? "touch" : "desktop"}.png`), fullPage: true });
    await panel.getByRole("button", { name: "Открыть пространства", exact: true }).click();
    await panel.getByRole("heading", { name: "Аварийное восстановление", exact: true }).waitFor();
    assert.deepEqual(await fullBackups(resolve(directory, "zedone.sqlite.backups")), backups);
    console.log(`Maintenance ${touch ? "touch" : "desktop"}: cancel/delete/restore/lost reply/rollback/retry/access/revoke/manual mode passed.`);
  } finally {
    await browser?.close(); fixture.kill(); await fixture.exited;
    assert.equal(dirname(directory), resolve(tmpdir())); assert.ok(basename(directory).startsWith("zedone-maintenance-ui-"));
    await rm(directory, { recursive: true, force: true });
  }
}
