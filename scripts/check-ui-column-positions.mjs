import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

export async function checkColumnPositions(browser, executable) {
  for (const touch of [false, true]) {
    const directory = await mkdtemp(resolve(tmpdir(), "zedone-column-positions-"));
    const fixture = Bun.spawn([executable, "--serve-column-position-conflicts", directory], { stdout: "pipe", stderr: "inherit" });
    let context;
    try {
      const reader = fixture.stdout.getReader(); let address = "";
      while (!address.includes("\n")) { const { value, done } = await reader.read(); assert.ok(!done); address += new TextDecoder().decode(value); }
      reader.releaseLock();
      const session = JSON.parse(await readFile(resolve(directory, "local-session.json"), "utf8"));
      context = await browser.newContext({ viewport: touch ? { width: 360, height: 760 } : { width: 1280, height: 900 }, hasTouch: touch, isMobile: touch, colorScheme: touch ? "dark" : "light" });
      const page = await context.newPage(), errors = [];
      page.on("pageerror", error => errors.push(error.message));
      const press = locator => touch ? locator.tap() : locator.click();
      const space = "000000000000000000000000000000c9", column = "000000000000000000000000000000ca", card = "000000000000000000000000000000cb";
      const a = "000000000000000000000000000000cd", b = "000000000000000000000000000000ce";
      const command = (operation, parameters) => page.evaluate(async ({ operation, parameters, key }) => (await fetch(`/v1/commands/${operation}`, {
        method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${key}` },
        body: JSON.stringify({ id: Array.from(crypto.getRandomValues(new Uint8Array(16))), operation, parameters }),
      })).json(), { operation, parameters, key: session.key });
      const read = async () => (await command("column.read", { space, column })).result;
      const child = async () => (await command("card.read", { space, card })).result;
      const history = async () => (await command("card.history", { space, card, after: null, limit: 100, expected: null })).result.versions;
      const button = (root, name) => root.getByRole("button", { name, exact: true });
      await page.goto(`${address.trim()}/#/spaces/${space}`);
      const panel = page.locator(`section[data-column="${column}"]`), row = page.locator(`[data-card="${card}"]`), dialog = page.getByRole("dialog");
      await panel.waitFor();
      assert.equal(await panel.count(), 1);
      assert.equal(await panel.locator("xpath=ancestor::section[@aria-label='Конфликты']").count(), 1);
      assert.equal(await panel.locator("xpath=ancestor::*[@aria-label='Колонки']").count(), 0);
      assert.equal(await panel.locator(".move-column").isDisabled(), true);
      assert.equal(await panel.locator(".rename-column").isDisabled(), true);
      assert.equal(await panel.locator(".create-card").isDisabled(), false);
      assert.equal(await row.locator(".move-card").isDisabled(), false);
      await press(row.locator(".card-title"));
      await press(button(dialog, "Отметить выполненной"));
      await button(dialog, "Снять отметку выполнения").waitFor();
      await press(dialog.locator("[data-workspace-dialog-close]"));
      await press(button(row, "В корзину"));
      await press(button(dialog, "В корзину"));
      await dialog.waitFor({ state: "hidden" });
      await press(button(page, "Корзина карточек"));
      await button(row, "Восстановить").waitFor();
      await press(button(row, "Восстановить"));
      await press(dialog.getByRole("radio", { name: "Спорная колонка", exact: true }));
      await press(button(dialog, "Восстановить"));
      await dialog.waitFor({ state: "hidden" });
      await press(button(page, "Активные карточки"));
      await row.locator(".card-title").waitFor();
      const before = await read(), content = await child(), versions = await history();
      assert.deepEqual(content.completed, [true]);
      await press(panel.locator(".resolve-column-position"));
      await dialog.getByRole("heading", { name: "Разрешить позицию колонки", exact: true }).waitFor();
      await dialog.getByText("Перед: Сосед А", { exact: true }).waitFor();
      await dialog.getByText("В конец", { exact: true }).waitFor();
      assert.equal(await dialog.getByRole("radio").count(), 2);
      assert.equal(await dialog.locator("input:checked").count(), 0);
      assert.equal(await button(dialog, "Сохранить решение").isDisabled(), true);
      await press(button(dialog, "Отмена"));
      assert.deepEqual(await read(), before);
      assert.equal(await panel.locator(".resolve-column-position").evaluate(element => element === document.activeElement), true);
      await press(panel.locator(".resolve-column-position"));
      await press(dialog.getByRole("radio").first());
      if (touch) {
        const current = (await command("column.read", { space, column: a })).result;
        assert.equal((await command("column.move", { space, column: a, before: null, expected: current.version })).status, "ok");
      } else {
        const current = await child();
        assert.equal((await command("card.set_completed", { space, card, completed: false, expected: current.version })).status, "ok");
        await press(button(dialog, "Сохранить решение"));
        await dialog.getByRole("alert").waitFor();
        assert.equal((await read()).positions.length, 2);
        assert.equal(await dialog.locator("input:checked").count(), 1);
        await press(dialog.locator("[data-workspace-dialog-close]"));
        await press(button(page, "Убрать из списка"));
        await press(button(dialog, "Убрать из списка"));
        await dialog.waitFor({ state: "hidden" });
        await press(button(page, "Обновить список"));
        await press(panel.locator(".resolve-column-position"));
        await press(dialog.getByRole("radio").first());
      }
      const selected = (await command("column.inspect_positions", { space, column })).result.choices[0].position;
      await press(button(dialog, "Сохранить решение"));
      await dialog.waitFor({ state: "hidden" });
      await panel.locator(".move-column:not(:disabled)").waitFor();
      assert.equal(await panel.count(), 1);
      assert.equal(await panel.locator("xpath=ancestor::*[@aria-label='Колонки']").count(), 1);
      assert.deepEqual((await read()).positions, [selected]);
      assert.deepEqual(await history(), versions);
      for (const key of ["card", "titles", "descriptions", "tags"]) assert.deepEqual((await child())[key], content[key]);
      await press(panel.locator(".move-column"));
      await dialog.locator('select[name="before"]').selectOption(touch ? a : b);
      await press(button(dialog, "Переместить"));
      await dialog.waitFor({ state: "hidden" });
      await page.reload(); await panel.waitFor();
      assert.equal(await panel.count(), 1);
      assert.equal(await row.count(), 1);
      assert.equal((await read()).positions.length, 1);
      assert.notDeepEqual((await read()).positions, [selected]);
      assert.deepEqual(await history(), versions);
      assert.deepEqual(errors, []);
      console.log(`Column positions ${touch ? "touch" : "desktop"}: group/content/cancel/selection/stale or moved neighbor/reverse/reload passed.`);
    } finally {
      await context?.close(); fixture.kill(); await fixture.exited;
      assert.ok(directory.startsWith(resolve(tmpdir(), "zedone-column-positions-")));
      await rm(directory, { recursive: true, force: true });
    }
  }
}
