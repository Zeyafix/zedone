import assert from "node:assert/strict";
import { mkdtemp, rm, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

export async function checkSpaceTransfer(browser, executable) {
  for (const touch of [false, true]) {
    const prefix = resolve(tmpdir(), "zedone-transfer-");
    const directory = await mkdtemp(prefix);
    const fixture = Bun.spawn([executable, "--serve", directory], { stdout: "pipe", stderr: "inherit" });
    let context;
    try {
      const reader = fixture.stdout.getReader();
      let address = "";
      while (!address.includes("\n")) {
        const { value, done } = await reader.read();
        assert.ok(!done);
        address += new TextDecoder().decode(value);
      }
      reader.releaseLock();
      const url = address.trim();
      context = await browser.newContext({ viewport: touch ? { width: 360, height: 760 } : { width: 1280, height: 900 }, isMobile: touch, hasTouch: touch, acceptDownloads: true });
      const page = await context.newPage(), errors = [];
      page.on("pageerror", cause => errors.push(cause.message));
      const credential = JSON.parse(await readFile(resolve(directory, "local-session.json"), "utf8"));
      async function command(operation, parameters) {
        const response = await context.request.post(`${url}/v1/commands/${operation}`, { headers: { Authorization: `Bearer ${credential.key}` }, data: { id: Array.from(crypto.getRandomValues(new Uint8Array(16))), operation, parameters } });
        const body = await response.json(); assert.equal(body.status, "ok", `${operation}: ${body.error?.code}`); return body.result;
      }
      const id = () => crypto.randomUUID().replaceAll("-", "");
      const space = id(), column = id(), card = id(), tag = id(), deleted = id();
      await command("space.create", { space, name: "Перенос 😀" });
      await command("column.create", { space, column, name: "Мысли" });
      await command("card.create", { space, column, card, title: "История", description: "Первая версия" });
      const current = () => command("card.read", { space, card });
      await command("card.edit", { space, card, title: "Архив", description: "Текст\n<b>буквальный</b>", expected: (await current()).version });
      await command("tag.create", { space, tag, name: "Сохранить" });
      await command("card.set_tag", { space, card, tag, present: true, expected: (await current()).version, expected_tag: (await command("tag.read", { space, tag })).version });
      await command("card.create", { space, column, card: deleted, title: "Корзина", description: "Удалённая карточка" });
      await command("card.trash", { space, card: deleted, expected: (await command("card.read", { space, card: deleted })).version });
      await page.goto(`${url}/#/spaces/${space}`);
      const panel = page.getByRole("region", { name: "Перенос пространства" });
      await panel.locator("summary").click();
      const button = name => panel.getByRole("button", { name, exact: true });
      let dropExport = true;
      await page.route("**/v1/commands/space.export", async route => {
        const response = await route.fetch();
        if (dropExport) await route.abort(); else await route.fulfill({ response });
      });
      await button("Подготовить экспорт").click();
      await panel.getByRole("alert").waitFor();
      await page.reload();
      dropExport = false;
      await page.getByRole("button", { name: "Проверить результат", exact: true }).click();
      await button("Сохранить архив в файл").waitFor();
      await page.evaluate(() => document.dispatchEvent(new Event("visibilitychange")));
      await page.waitForFunction(() => document.querySelector(".connection-panel-ready"));
      await button("Сохранить архив в файл").waitFor();
      const download = page.waitForEvent("download");
      await button("Сохранить архив в файл").click();
      const file = await download;
      const bytes = await readFile(await file.path());
      assert.equal(bytes.subarray(0, 8).toString(), "ZEDSPACE");
      await command("tag.delete", { space, tag, expected: (await command("tag.read", { space, tag })).version });
      assert.deepEqual((await current()).tags, []);
      const original = await command("space.read", { space });
      async function choose(buffer = bytes) {
        const chooser = page.waitForEvent("filechooser");
        await button("Выбрать файл для импорта").click();
        await (await chooser).setFiles({ name: "space.zedspace", mimeType: "application/octet-stream", buffer });
      }
      const chooser = page.waitForEvent("filechooser");
      await button("Выбрать файл для импорта").click();
      await (await chooser).setFiles([]);
      assert.equal((await command("space.list", { include_trashed: true })).spaces.length, 1);
      await choose();
      await button("Убрать выбранный файл").click();
      assert.equal((await command("space.list", { include_trashed: true })).spaces.length, 1);
      await choose(Buffer.alloc(1024 * 1024 + 1));
      await panel.getByRole("alert").waitFor();
      assert.equal(await button("Импортировать выбранный файл").count(), 0);
      await choose(Buffer.from("corrupt"));
      await button("Импортировать выбранный файл").click();
      await page.getByRole("heading", { name: "Незавершённые действия" }).waitFor();
      assert.equal((await command("space.list", { include_trashed: true })).spaces.length, 1);
      await page.getByRole("button", { name: "Убрать из списка", exact: true }).click();
      await page.getByRole("dialog").getByRole("button", { name: "Убрать из списка", exact: true }).click();
      await page.getByRole("dialog").waitFor({ state: "hidden" });
      let drop = true;
      await page.route("**/v1/commands/space.import", async route => {
        const response = await route.fetch();
        if (drop) await route.abort(); else await route.fulfill({ response });
      });
      await choose();
      await button("Импортировать выбранный файл").click();
      await panel.getByRole("alert").waitFor();
      assert.equal((await command("space.list", { include_trashed: true })).spaces.length, 2);
      await page.reload();
      drop = false;
      await page.getByRole("button", { name: "Проверить результат", exact: true }).click();
      await page.getByRole("heading", { name: "Незавершённые действия" }).waitFor({ state: "hidden" });
      assert.equal((await command("space.list", { include_trashed: true })).spaces.length, 2);
      await panel.locator("summary").click();
      await choose();
      await button("Импортировать выбранный файл").click();
      await panel.getByText(/Независимое пространство импортировано/).waitFor();
      const copies = (await command("space.list", { include_trashed: true })).spaces.filter(item => item.space !== space);
      assert.equal(copies.length, 2);
      for (const copy of copies) {
        const columns = (await command("column.list", { space: copy.space, include_trashed: true })).columns;
        assert.equal(columns.length, 1); assert.notEqual(columns[0].column, column);
        const cards = (await command("card.list", { space: copy.space, include_trashed: true })).cards;
        const restored = cards.find(item => item.titles.includes("Архив"));
        assert.equal(cards.length, 2); assert.notEqual(restored.card, card);
        assert.equal(cards.find(item => item.titles.includes("Корзина")).state, "trashed");
        const history = await command("card.history", { space: copy.space, card: restored.card, after: null, expected: null, limit: 100 });
        assert.deepEqual(history.versions.map(item => item.title), ["История", "Архив"]);
        assert.equal(restored.tags.length, 1); assert.notEqual(restored.tags[0], tag);
      }
      const copy = await command("space.read", { space: copies[0].space });
      await command("space.rename", { space: copy.space, name: "Независимая правка", expected: copy.version });
      const renamed = await command("space.read", { space: copy.space });
      await command("space.trash", { space: copy.space, expected: renamed.version });
      await command("space.restore", { space: copy.space, expected: (await command("space.read", { space: copy.space })).version });
      assert.deepEqual(await command("space.read", { space }), original);
      assert.deepEqual((await command("space.read", { space: copies[1].space })).names, ["Перенос 😀"]);
      assert.deepEqual(errors, []);
      console.log(`File transfer, invalid/oversize/cancel, lost-reply reload/replay, two independent imports, history/tags/trash/reverse passed (${touch ? "touch" : "desktop"}).`);
    } finally {
      await context?.close(); fixture.kill(); await fixture.exited;
      assert.ok(resolve(directory).startsWith(prefix));
      await rm(directory, { recursive: true });
    }
  }
}

if (import.meta.main) {
  const { chromium } = await import("playwright");
  for (const executablePath of process.env.UI_BROWSER_PATHS ? JSON.parse(process.env.UI_BROWSER_PATHS) : [undefined]) {
    const browser = await chromium.launch({ executablePath, headless: true });
    try { await checkSpaceTransfer(browser, resolve(`target/debug/examples/shell-fixture${process.platform === "win32" ? ".exe" : ""}`)); }
    finally { await browser.close(); }
  }
}
