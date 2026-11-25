import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { resolve } from "node:path";
import { tmpdir } from "node:os";
export async function checkRecovery(browser, executable) {
  for (const touch of [false, true]) {
    const directory = await mkdtemp(resolve(tmpdir(), "zedone-recovery-ui-"));
    const fixture = Bun.spawn([executable, "--serve", directory], { stdout: "pipe", stderr: "inherit" });
    let context;
    try {
      const reader = fixture.stdout.getReader(); let address = "";
      while (!address.includes("\n")) { const { value, done } = await reader.read(); assert.ok(!done); address += new TextDecoder().decode(value); }
      reader.releaseLock(); const url = address.trim();
      const credential = JSON.parse(await readFile(resolve(directory, "local-session.json"), "utf8"));
      context = await browser.newContext({ viewport: touch ? { width: 360, height: 760 } : { width: 1280, height: 900 }, isMobile: touch, hasTouch: touch, acceptDownloads: true });
      const page = await context.newPage(), errors = [];
      page.on("pageerror", cause => errors.push(cause.message));
      async function command(operation, parameters) {
        const result = await context.request.post(`${url}/v1/commands/${operation}`, { headers: { Authorization: `Bearer ${credential.key}` }, data: { id: Array.from(crypto.getRandomValues(new Uint8Array(16))), operation, parameters } });
        const body = await result.json(); assert.equal(body.status, "ok", `${operation}: ${body.error?.code}`); return body.result;
      }
      const id = () => crypto.randomUUID().replaceAll("-", "");
      const space = id(), column = id(), card = id(), tag = id(), deleted = id(), other = id();
      await command("space.create", { space, name: "Восстановление 😀" });
      await command("space.create", { space: other, name: "Другое пространство" });
      await command("column.create", { space, column, name: "Мысли" });
      await command("card.create", { space, column, card, title: "Снимок", description: "Исходный <b>текст</b>" });
      await command("card.create", { space, column, card: deleted, title: "В корзине", description: "Сохранить" });
      await command("card.trash", { space, card: deleted, expected: (await command("card.read", { space, card: deleted })).version });
      await command("tag.create", { space, tag, name: "Сохранить" });
      await command("card.set_tag", { space, card, tag, present: true, expected: (await command("card.read", { space, card })).version, expected_tag: (await command("tag.read", { space, tag })).version });
      const archive = Buffer.from((await command("space.export", { space })).archive, "hex");
      await command("card.edit", { space, card, title: "Поздняя правка", description: "Нужно вернуть", expected: (await command("card.read", { space, card })).version });
      await command("tag.delete", { space, tag, expected: (await command("tag.read", { space, tag })).version });
      await page.goto(`${url}/#/spaces/${space}`);
      const panel = page.getByRole("region", { name: "Восстановление пространства", exact: true });
      const button = name => panel.getByRole("button", { name, exact: true });
      const open = async () => { if (!await panel.locator("details").getAttribute("open").then(x => x !== null)) await panel.locator("summary").click(); };
      await open();
      async function choose(buffer = archive) {
        const chooser = page.waitForEvent("filechooser"); await button("Выбрать снимок для восстановления").click();
        await (await chooser).setFiles(buffer ? { name: "source.zedspace", mimeType: "application/octet-stream", buffer } : []);
      }
      await choose(null);
      assert.deepEqual((await command("card.read", { space, card })).titles, ["Поздняя правка"]);
      await choose(Buffer.alloc(1024 * 1024 + 1)); await panel.getByRole("alert").waitFor();
      await choose(); await button("Убрать снимок").click();
      await choose();
      const confirm = panel.getByRole("checkbox"); await confirm.waitFor();
      assert.equal(await button("Подтвердить восстановление пространства").isEnabled(), false);
      await confirm.check();
      let drop = true;
      await page.route("**/v1/commands/space.recovery.restore", async route => { const response = await route.fetch(); if (drop) await route.abort(); else await route.fulfill({ response }); });
      await button("Подтвердить восстановление пространства").click();
      await panel.getByRole("alert").waitFor();
      await page.reload(); drop = false;
      await page.getByRole("button", { name: "Проверить результат", exact: true }).click();
      await page.getByRole("heading", { name: "Незавершённые действия", exact: true }).waitFor({ state: "hidden" });
      await open(); await button("Проверить состояние восстановления").click();
      await button("Возобновить обмен пространства").waitFor();
      assert.deepEqual((await command("card.read", { space, card })).titles, ["Снимок"]);
      assert.deepEqual((await command("card.read", { space, card })).tags, [tag]);
      assert.equal((await command("card.read", { space, card: deleted })).state, "trashed");
      assert.deepEqual((await command("space.read", { space: other })).names, ["Другое пространство"]);
      const download = page.waitForEvent("download"); await button("Сохранить страховочный снимок в файл").click();
      const saved = await download, firstSafety = await readFile(await saved.path()); assert.equal(firstSafety.subarray(0, 8).toString(), "ZEDSPACE");
      await button("Выбрать состояние до восстановления").click();
      await confirm.check(); await button("Подтвердить восстановление пространства").click();
      await panel.getByText("Пространство восстановлено.", { exact: false }).waitFor();
      assert.deepEqual((await command("card.read", { space, card })).titles, ["Поздняя правка"]);
      assert.deepEqual((await command("card.read", { space, card })).tags, []);
      const nextDownload = page.waitForEvent("download"); await button("Сохранить страховочный снимок в файл").click();
      assert.notDeepEqual(await readFile(await (await nextDownload).path()), firstSafety);
      await button("Возобновить обмен пространства").click();
      await button("Приостановить обмен пространства").waitFor();
      await page.reload(); await open(); await button("Проверить состояние восстановления").click(); await button("Приостановить обмен пространства").waitFor();
      assert.deepEqual(errors, []);
      console.log(`Recovery ${touch ? "touch" : "desktop"}: confirmation, file cancel/budget, lost reply/reload/replay, same IDs/history/tags/trash/isolation, safety download, reverse and explicit resume passed.`);
    } finally { await context?.close(); fixture.kill(); await fixture.exited; assert.ok(directory.startsWith(resolve(tmpdir(), "zedone-recovery-ui-"))); await rm(directory, { recursive: true }); }
  }
}
if (import.meta.main) {
  const { chromium } = await import("playwright");
  for (const executablePath of process.env.UI_BROWSER_PATHS ? JSON.parse(process.env.UI_BROWSER_PATHS) : [undefined]) {
    const browser = await chromium.launch({ executablePath, headless: true, args: ["--disable-frame-rate-limit"] });
    try { await checkRecovery(browser, resolve(`target/debug/examples/shell-fixture${process.platform === "win32" ? ".exe" : ""}`)); } finally { await browser.close(); }
  }
}
