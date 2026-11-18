import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

export async function checkPositionConflicts(browser, executable) {
  for (const touch of [false, true]) for (const same of [false, true]) for (const columnFirst of [false, true]) {
    const directory = await mkdtemp(resolve(tmpdir(), "zedone-position-conflicts-"));
    const fixture = Bun.spawn([executable, same ? "--serve-same-position-conflicts" : "--serve-position-conflicts", directory], { stdout: "pipe", stderr: "inherit" });
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
      const id = n => n.toString(16).padStart(32, "0");
      const space = id(201), column = id(202), card = id(207), child = id(203), other = id(205);
      const command = async (operation, parameters) => {
        const response = await page.evaluate(async ({ operation, parameters, key }) => (await fetch(`/v1/commands/${operation}`, {
          method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${key}` },
          body: JSON.stringify({ id: Array.from(crypto.getRandomValues(new Uint8Array(16))), operation, parameters }),
        })).json(), { operation, parameters, key: session.key });
        assert.equal(response.status, "ok", operation);
        return response.result;
      };
      const read = () => command("card.read", { space, card });
      const parent = () => command("column.read", { space, column });
      const history = async () => (await command("card.history", { space, card, after: null, limit: 100, expected: null })).versions;
      const row = page.locator(`[data-card="${card}"]`), panel = page.locator(`section[data-column="${column}"]`);
      const childRow = page.locator(`[data-card="${child}"]`), dialog = page.getByRole("dialog");
      const button = (root, name) => root.getByRole("button", { name, exact: true });
      const inConflicts = locator => locator.locator("xpath=ancestor::section[@aria-label='Конфликты']").count();
      const blocked = async () => {
        await press(row.locator(".card-title"));
        assert.equal(await button(dialog, "Редактировать").count(), 0);
        assert.equal(await button(dialog, "Отметить выполненной").count(), 0);
        await press(dialog.locator("[data-workspace-dialog-close]"));
      };
      const choose = async kind => {
        const locator = kind === "column" ? panel.locator(".resolve-column-position") : row.locator(".resolve-card-placement");
        const before = kind === "column" ? await parent() : await read();
        await press(locator);
        await dialog.getByRole("heading", { name: kind === "column" ? "Разрешить позицию колонки" : "Разрешить место карточки", exact: true }).waitFor();
        await dialog.getByRole("radio").nth(1).waitFor();
        assert.equal(await dialog.getByRole("radio").count(), 2);
        assert.equal(await dialog.locator("input:checked").count(), 0);
        assert.equal(await button(dialog, "Сохранить решение").isDisabled(), true);
        await press(button(dialog, "Отмена"));
        assert.deepEqual(kind === "column" ? await parent() : await read(), before);
        await page.waitForFunction(({ element, kind }) => kind === "column"
          ? element === document.activeElement
          : element.contains(document.activeElement) && document.activeElement.matches(".card-title, .resolve-card-placement"),
          { element: await (kind === "column" ? locator : row).elementHandle(), kind });
        await press(locator);
        await dialog.getByRole("radio").nth(1).waitFor();
        const current = await command(kind === "column" ? "column.inspect_positions" : "card.inspect_placements", kind === "column" ? { space, column } : { space, card });
        const index = kind === "column" ? 0 : current.choices.findIndex(choice => choice.placement.column === column);
        assert.ok(index >= 0);
        const selected = current.choices[index];
        await press(dialog.getByRole("radio").nth(index));
        await press(button(dialog, "Сохранить решение"));
        await dialog.waitFor({ state: "hidden" });
        await (kind === "column" ? panel.locator(".move-column:not(:disabled)") : row.locator(".move-card:not(:disabled)")).waitFor();
        if (kind === "column") assert.deepEqual((await parent()).positions, [selected.position]);
        else assert.deepEqual((await read()).placements, [selected.placement]);
      };
      await page.goto(`${address.trim()}/#/spaces/${space}`); await row.waitFor(); await panel.waitFor();
      assert.equal(await row.count(), 1); assert.equal(await panel.count(), 1);
      assert.equal(await inConflicts(row), 1); assert.equal(await inConflicts(panel), 1);
      assert.equal(await row.locator("xpath=ancestor::section[@data-column]").count(), 0);
      assert.equal(await panel.locator(".move-column").isDisabled(), true);
      assert.equal(await childRow.locator(".move-card").isDisabled(), false);
      const original = await read(), versions = await history();
      await blocked();
      await press(childRow.locator(".card-title"));
      await press(button(dialog, "Отметить выполненной"));
      await button(dialog, "Снять отметку выполнения").waitFor();
      await press(dialog.locator("[data-workspace-dialog-close]"));
      const first = columnFirst ? "column" : "card", second = columnFirst ? "card" : "column";
      await choose(first);
      if (columnFirst) {
        assert.equal(await inConflicts(panel), 0); assert.equal(await inConflicts(row), 1);
        assert.equal((await read()).placements.length, 2); await blocked();
      } else {
        assert.equal(await inConflicts(panel), 1);
        assert.equal(await row.locator(`xpath=ancestor::section[@data-column='${column}']`).count(), 1);
        assert.equal((await parent()).positions.length, 2);
        await press(row.locator(".card-title"));
        await button(dialog, "Редактировать").waitFor();
        await press(button(dialog, "Отметить выполненной"));
        await button(dialog, "Снять отметку выполнения").waitFor();
        await press(button(dialog, "Снять отметку выполнения"));
        await button(dialog, "Отметить выполненной").waitFor();
        await press(dialog.locator("[data-workspace-dialog-close]"));
      }
      await choose(second);
      assert.equal(await inConflicts(panel), 0); assert.equal(await inConflicts(row), 0);
      assert.equal(await row.count(), 1); assert.equal(await panel.count(), 1);
      for (const key of ["card", "titles", "descriptions", "tags", "completed", "deletion_groups"]) assert.deepEqual((await read())[key], original[key]);
      assert.deepEqual(await history(), versions);
      await press(row.locator(".move-card"));
      await dialog.locator('select[name="column"]').selectOption(other);
      await press(button(dialog, "Переместить")); await dialog.waitFor({ state: "hidden" });
      await press(panel.locator(".move-column"));
      await dialog.locator('select[name="before"]').selectOption(other);
      await press(button(dialog, "Переместить")); await dialog.waitFor({ state: "hidden" });
      await page.reload(); await row.waitFor(); await panel.waitFor();
      assert.equal(await row.count(), 1); assert.equal(await panel.count(), 1);
      assert.deepEqual((await read()).columns, [other]);
      assert.deepEqual(await history(), versions);
      assert.deepEqual((await command("card.read", { space, card: child })).completed, [true]);
      assert.deepEqual(errors, []);
      console.log(`Combined positions ${touch ? "touch" : "desktop"}/${same ? "same" : "cross"}-column/${first} first: groups, blocks, child actions, cancel, choices, reverse and reload passed.`);
    } finally {
      await context?.close(); fixture.kill(); await fixture.exited;
      assert.ok(directory.startsWith(resolve(tmpdir(), "zedone-position-conflicts-")));
      await rm(directory, { recursive: true, force: true });
    }
  }
}
