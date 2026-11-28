import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { tmpdir } from "node:os";

export async function checkReinstallation(browser, executable) {
  for (const touch of [false, true]) {
    const directory = await mkdtemp(resolve(tmpdir(), "zedone-reinstallation-ui-"));
    const children = [];
    let context;
    try {
      async function host(name, backups = false) {
        const path = resolve(directory, name);
        const child = Bun.spawn([executable, backups ? "--serve-backups" : "--serve", path], { stdout: "pipe", stderr: "inherit" });
        children.push(child);
        const reader = child.stdout.getReader(); let address = "";
        while (!address.includes("\n")) { const next = await reader.read(); assert.ok(!next.done); address += new TextDecoder().decode(next.value); }
        reader.releaseLock();
        const url = address.trim(), credential = JSON.parse(await readFile(resolve(path, "local-session.json"), "utf8"));
        const call = async (operation, parameters) => {
          const reply = await fetch(`${url}/v1/commands/${operation}`, { method: "POST", headers: { Authorization: `Bearer ${credential.key}`, "Content-Type": "application/json" }, body: JSON.stringify({ id: Array.from(crypto.getRandomValues(new Uint8Array(16))), operation, parameters }) });
          const value = await reply.json(); assert.equal(value.status, "ok", JSON.stringify(value)); return value.result;
        };
        return { url, call, path, credential };
      }
      const source = await host("source", true), target = await host("target");
      const id = () => crypto.randomUUID().replaceAll("-", "");
      const space = id(), other = id(), race = id(), column = id(), card = id();
      await source.call("space.create", { space, name: "После переустановки 😀" });
      await source.call("column.create", { space, column, name: "Мысли" });
      await source.call("card.create", { space, column, card, title: "Сохранённая карточка", description: "История" });
      const archive = Buffer.from((await source.call("space.export", { space })).archive, "hex");
      await source.call("space.create", { space: race, name: "Снимок гонки" });
      const raceArchive = Buffer.from((await source.call("space.export", { space: race })).archive, "hex");
      await target.call("space.create", { space: other, name: "Осталось на месте" });
      context = await browser.newContext({ viewport: touch ? { width: 360, height: 760 } : { width: 1280, height: 900 }, isMobile: touch, hasTouch: touch });
      const page = await context.newPage(), errors = [];
      page.on("pageerror", e => errors.push(e.message));
      await page.goto(`${target.url}/#/spaces`);
      const panel = page.getByRole("region", { name: "Восстановление после переустановки", exact: true });
      const button = name => panel.getByRole("button", { name, exact: true });
      const open = async () => { if (await panel.locator("details").getAttribute("open") === null) await panel.locator("summary").click(); };
      await open();
      async function choose(buffer = archive) {
        const chooser = page.waitForEvent("filechooser"); await button("Выбрать копию после переустановки").click();
        await (await chooser).setFiles(buffer ? { name: "saved.zedspace", mimeType: "application/octet-stream", buffer } : []);
      }
      await choose(null);
      await choose(Buffer.alloc(1024 * 1024 + 1)); await panel.getByRole("alert").waitFor();
      await choose(Buffer.from("bad")); await panel.getByRole("alert").waitFor();
      await choose(); await panel.getByRole("checkbox").waitFor();
      assert.equal(await button("Восстановить отсутствующее пространство").isEnabled(), false);
      await button("Отменить подготовку восстановления").click();
      assert.equal((await target.call("space.list", { include_trashed: true })).spaces.length, 1);
      await choose(raceArchive); await panel.getByRole("checkbox").waitFor();
      await target.call("space.create", { space: race, name: "Появилось позже" });
      await panel.getByRole("checkbox").check(); await button("Восстановить отсутствующее пространство").click();
      await panel.getByRole("alert").waitFor();
      assert.deepEqual((await target.call("space.read", { space: race })).names, ["Появилось позже"]);
      const forget = page.getByRole("button", { name: "Убрать из списка", exact: true });
      await forget.click();
      await page.getByRole("dialog").getByRole("button", { name: "Убрать из списка", exact: true }).click();
      await choose(); await panel.getByRole("checkbox").check();
      let drop = true;
      await page.route("**/v1/commands/space.recovery.restore_missing", async route => { const response = await route.fetch(); if (drop) await route.abort(); else await route.fulfill({ response }); });
      await button("Восстановить отсутствующее пространство").click(); await panel.getByRole("alert").waitFor();
      await page.reload(); drop = false;
      await page.getByRole("button", { name: "Проверить результат", exact: true }).click();
      await page.getByRole("heading", { name: "Незавершённые действия", exact: true }).waitFor({ state: "hidden" });
      assert.deepEqual((await target.call("card.read", { space, card })).titles, ["Сохранённая карточка"]);
      const state = await target.call("space.recovery.inspect", { space });
      assert.equal(state.paused, true); assert.equal(state.backup, null);
      assert.deepEqual((await target.call("space.read", { space: other })).names, ["Осталось на месте"]);
      await page.goto(`${target.url}/#/spaces/${space}`);
      const recovery = page.getByRole("region", { name: "Восстановление пространства", exact: true });
      await recovery.locator("summary").click();
      await recovery.getByRole("button", { name: "Проверить состояние восстановления", exact: true }).click();
      await recovery.getByRole("button", { name: "Возобновить обмен пространства", exact: true }).click();
      await recovery.getByRole("button", { name: "Приостановить обмен пространства", exact: true }).waitFor();
      await open(); await choose(); await panel.getByRole("alert").waitFor();
      const keyFile = resolve(target.path, "catalog.key"); await writeFile(keyFile, target.credential.key);
      const cli = resolve(dirname(executable), "..", process.platform === "win32" ? "zedone.exe" : "zedone");
      const granted = Bun.spawnSync([cli, "--select-backup-folder", "--database", resolve(target.path, "zedone.sqlite"), "--key-file", keyFile, "--mode", "read", "--folder", resolve(source.path, "zedone.sqlite.backups")], { stdout: "pipe", stderr: "pipe" });
      assert.equal(granted.exitCode, 0, granted.stderr.toString());
      const folder = JSON.parse(granted.stdout.toString()).result;
      await page.route("**/host/backup-folder", route => route.fulfill({ json: { status: "ok", replayed: false, result: { status: "selected", folder } } }));
      await page.goto(`${target.url}/#/settings`);
      const catalog = page.getByRole("region", { name: "Резервные копии", exact: true });
      await catalog.getByRole("button", { name: "Открыть папку для просмотра", exact: true }).click();
      await catalog.getByRole("region", { name: "Ежедневные копии", exact: true }).getByRole("button", { name: "Подготовить восстановление", exact: true }).click();
      await panel.getByRole("checkbox").waitFor();
      await button("Отменить подготовку восстановления").click();
      assert.equal((await target.call("space.list", { include_trashed: true })).spaces.length, 3);
      await page.goto(`${target.url}/#/settings`);
      await catalog.getByRole("button", { name: "Открыть папку для просмотра", exact: true }).click();
      await catalog.getByRole("region", { name: "Ежедневные копии", exact: true }).getByRole("button", { name: "Подготовить восстановление", exact: true }).click();
      await panel.getByRole("checkbox").check();
      await button("Восстановить отсутствующее пространство").click();
      await panel.getByText("Пространство восстановлено с прежним ID.", { exact: false }).waitFor();
      assert.equal((await target.call("space.recovery.inspect", { space: "00000000000000000000000000000001" })).paused, true);
      assert.deepEqual((await target.call("card.read", { space: "00000000000000000000000000000001", card: "00000000000000000000000000000003" })).titles, ["Сохранённый текст"]);
      assert.deepEqual(errors, []);
      console.log(`Reinstallation ${touch ? "touch" : "desktop"}: file cancel/budget/corruption, confirmation/cancel, appearing state, lost reply/restart/replay, same IDs/isolation/pause and explicit resume passed.`);
    } finally {
      await context?.close();
      for (const child of children) { child.kill(); await child.exited; }
      assert.ok(directory.startsWith(resolve(tmpdir(), "zedone-reinstallation-ui-")));
      await rm(directory, { recursive: true });
    }
  }
}

if (import.meta.main) {
  const { chromium } = await import("playwright");
  const browser = await chromium.launch({ headless: true });
  try { await checkReinstallation(browser, resolve(`target/debug/examples/shell-fixture${process.platform === "win32" ? ".exe" : ""}`)); }
  finally { await browser.close(); }
}
