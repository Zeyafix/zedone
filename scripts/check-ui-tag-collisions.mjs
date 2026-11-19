import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

export async function checkTagCollisions(browser, executable) {
  for (const touch of [false, true]) for (const keep of [false, true]) {
    const directory = await mkdtemp(resolve(tmpdir(), "zedone-tag-collisions-"));
    const fixture = Bun.spawn([executable, "--serve-tag-conflicts", directory], { stdout: "pipe", stderr: "inherit" });
    let context;
    try {
      const reader = fixture.stdout.getReader(); let address = "";
      while (!address.includes("\n")) { const { value, done } = await reader.read(); assert.ok(!done); address += new TextDecoder().decode(value); }
      reader.releaseLock();
      const session = JSON.parse(await readFile(resolve(directory, "local-session.json"), "utf8"));
      context = await browser.newContext({ viewport: touch ? { width: 360, height: 760 } : { width: 1280, height: 900 }, hasTouch: touch, isMobile: touch, colorScheme: touch ? "dark" : "light" });
      const page = await context.newPage(), errors = [];
      page.on("pageerror", e => errors.push(e.message));
      const id = n => n.toString(16).padStart(32, "0"), space = id(221);
      const press = locator => touch ? locator.tap() : locator.click();
      const button = (scope, name) => scope.getByRole("button", { name, exact: true });
      const command = (operation, parameters) => page.evaluate(async ({ operation, parameters, key }) => (await fetch(`/v1/commands/${operation}`, {
        method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${key}` },
        body: JSON.stringify({ id: Array.from(crypto.getRandomValues(new Uint8Array(16))), operation, parameters }),
      })).json(), { operation, parameters, key: session.key });
      const readCard = async n => { const r = await command("card.read", { space, card: id(n) }); assert.equal(r.status, "ok"); return r.result; };
      const readTag = async n => { const r = await command("tag.read", { space, tag: id(n) }); assert.equal(r.status, "ok"); return r.result; };
      await page.goto(address.trim()); await press(page.locator(`[data-space="${space}"]`).getByRole("link"));
      const tags = page.locator(".space-tags"), dialog = page.getByRole("dialog");
      await press(tags.locator("summary"));
      const before = await readCard(224);
      await press(tags.locator(".resolve-tag-deletion"));
      await button(dialog, "Подтвердить решение").waitFor();
      assert.equal(await button(dialog, "Подтвердить решение").isDisabled(), true);
      await press(button(dialog, "Отмена"));
      assert.equal((await readTag(229)).deletion_conflict, true);
      await press(tags.locator(".resolve-tag-deletion"));
      await press(dialog.getByRole("radio", { name: keep ? "Сохранить тег с назначениями" : "Подтвердить удаление тега", exact: true }));
      assert.equal((await command("tag.create", { space, tag: id(231), name: "Поздний тег" })).status, "ok");
      await press(button(dialog, "Подтвердить решение"));
      await button(dialog, "Обновить конфликт тега").waitFor();
      assert.equal(await button(dialog, "Подтвердить решение").isDisabled(), true);
      await press(button(dialog, "Обновить конфликт тега"));
      await page.waitForFunction(() => ![...document.querySelectorAll("button")].find(b => b.textContent === "Подтвердить решение")?.disabled);
      await press(button(dialog, "Подтвердить решение")); await dialog.waitFor({ state: "hidden" });
      assert.equal((await readTag(229)).state, keep ? "active" : "deleted");
      for (const card of [224, 225]) { const c = await readCard(card); assert.equal(c.state, "active"); assert.equal(c.tags.includes(id(229)), keep); }
      for (const first of [226, 228]) {
        await press(tags.locator(`[data-tag="${id(first)}"] .resolve-tag-names`));
        await dialog.getByRole("textbox").first().waitFor();
        assert.equal(await dialog.getByRole("textbox").count(), 2);
        const names = first === 226 ? ["Все", "Всё"] : ["Дом", "Работа"];
        for (const [i, name] of names.entries()) await dialog.getByRole("textbox").nth(i).fill(name);
        await press(button(dialog, "Сохранить имена группы")); await dialog.waitFor({ state: "hidden" });
        assert.equal((await readTag(first)).state, "active");
      }
      const after = await readCard(224);
      for (const field of ["titles", "descriptions", "positions", "columns", "completed"]) assert.deepEqual(after[field], before[field]);
      assert.ok(after.tags.includes(id(228)) && after.tags.includes(id(230)));
      await page.reload(); await page.locator(`[data-card="${id(223)}"]`).waitFor();
      assert.equal((await readCard(225)).state, "active"); assert.deepEqual(errors, []);
      console.log(`Tag collisions UI passed: touch=${touch}, keep=${keep}, A52/A53/A40/cancel/stale/explicit choice/preservation/reload.`);
    } finally { await context?.close(); fixture.kill(); await fixture.exited; await rm(directory, { recursive: true, force: true }); }
  }
}
