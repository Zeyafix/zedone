import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

export async function checkFieldConflicts(browser, executable) {
  for (const touch of [false, true]) {
    for (const cardFirst of [true, false]) {
      const directory = await mkdtemp(resolve(tmpdir(), "zedone-field-conflicts-"));
      const fixture = Bun.spawn([executable, "--serve-field-conflicts", directory], { stdout: "pipe", stderr: "inherit" });
      let context;
      try {
        const reader = fixture.stdout.getReader();
        let address = "";
        while (!address.includes("\n")) {
          const { value, done } = await reader.read();
          assert.ok(!done, "combined conflict fixture exited before readiness");
          address += new TextDecoder().decode(value);
        }
        reader.releaseLock();
        const session = JSON.parse(await readFile(resolve(directory, "local-session.json"), "utf8"));
        context = await browser.newContext({ viewport: touch ? { width: 360, height: 760 } : { width: 1280, height: 900 }, colorScheme: touch ? "dark" : "light", hasTouch: touch, isMobile: touch });
        const page = await context.newPage(), errors = [];
        page.on("pageerror", error => errors.push(error.message));
        const press = locator => touch ? locator.tap() : locator.click();
        const button = (scope, name) => scope.getByRole("button", { name, exact: true });
        const space = "000000000000000000000000000000c9", column = "000000000000000000000000000000ca", card = "000000000000000000000000000000cb";
        const command = (operation, parameters) => page.evaluate(async ({ operation, parameters, key }) => (await fetch(`/v1/commands/${operation}`, {
          method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${key}` },
          body: JSON.stringify({ id: Array.from(crypto.getRandomValues(new Uint8Array(16))), operation, parameters }),
        })).json(), { operation, parameters, key: session.key });
        const read = async kind => {
          const response = await command(`${kind}.read`, { space, ...(kind === "space" ? {} : { [kind]: kind === "card" ? card : column }) });
          assert.equal(response.status, "ok"); return response.result;
        };
        await page.goto(address.trim());
        const spaceRow = page.locator(`[data-space="${space}"]`), dialog = page.getByRole("dialog");
        await spaceRow.waitFor();
        assert.equal(await spaceRow.count(), 1);
        await press(spaceRow.getByRole("link"));
        const container = page.locator(`section[data-column="${column}"]`), row = page.locator(`[data-card="${card}"]`);
        await row.waitFor();
        assert.equal(await row.count(), 1);
        assert.equal(await button(page, "Создать колонку").isDisabled(), false);
        assert.equal(await button(container, "Создать карточку").isDisabled(), false);
        const before = await read("card");
        await press(row.locator(".card-title"));
        assert.equal(await button(dialog, "Редактировать").count(), 0);
        assert.equal(await button(dialog, "Отметить выполненной").count(), 0);
        await press(button(dialog, "Разрешить конфликт текста"));
        await press(button(dialog, "Отмена"));
        assert.deepEqual(await read("card"), before);
        await press(dialog.locator("[data-workspace-dialog-close]"));
        const resolveCard = async () => {
          await press(row.locator(".card-title"));
          await press(button(dialog, "Разрешить конфликт текста"));
          await dialog.getByRole("textbox", { name: "Итоговый заголовок", exact: true }).fill("Итоговая карточка");
          await dialog.getByRole("textbox", { name: "Итоговое описание", exact: true }).fill("Сохранённое совместное решение 😀");
          await press(button(dialog, "Сохранить решение"));
          await button(dialog, "Редактировать").waitFor();
          await press(button(dialog, "Отметить выполненной"));
          await press(button(dialog, "Снять отметку выполнения"));
          await button(dialog, "Отметить выполненной").waitFor();
          await press(dialog.locator("[data-workspace-dialog-close]"));
        };
        const resolveNames = async () => {
          await press(button(container, "Разрешить название"));
          await press(button(dialog, "Колонка с телефона"));
          await press(button(dialog, "Разрешить название колонки"));
          await dialog.waitFor({ state: "hidden" });
          await container.getByRole("heading", { name: "Колонка с телефона", exact: true }).waitFor();
          await press(page.locator(".resolve-space-name"));
          await dialog.getByRole("textbox", { name: "Название", exact: true }).fill("Итоговое пространство");
          await press(button(dialog, "Разрешить название пространства"));
          await dialog.waitFor({ state: "hidden" });
          await page.getByRole("heading", { name: "Итоговое пространство", exact: true }).waitFor();
        };
        if (cardFirst) {
          await resolveCard();
          assert.equal((await read("space")).names.length, 2);
          assert.equal((await read("column")).names.length, 2);
          await resolveNames();
        } else {
          await resolveNames();
          assert.deepEqual(await read("card"), before);
          await press(row.locator(".card-title"));
          assert.equal(await button(dialog, "Редактировать").count(), 0);
          assert.equal(await button(dialog, "Отметить выполненной").count(), 0);
          await press(dialog.locator("[data-workspace-dialog-close]"));
          await resolveCard();
        }
        const after = await read("card");
        assert.deepEqual(after.titles, ["Итоговая карточка"]);
        assert.deepEqual(after.descriptions, ["Сохранённое совместное решение 😀"]);
        for (const key of ["card", "space", "columns", "positions", "placements", "completed", "tags", "deletion_groups"]) assert.deepEqual(after[key], before[key]);
        const history = await command("card.history", { space, card, after: null, expected: null, limit: 100 });
        assert.equal(history.status, "ok"); assert.equal(history.result.versions.length, 4);
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
        await page.reload();
        await page.getByRole("heading", { name: "Итоговое пространство", exact: true }).waitFor();
        assert.equal(await row.count(), 1);
        await press(page.getByRole("link", { name: "Все пространства", exact: true }));
        await page.getByRole("link", { name: "Итоговое пространство", exact: true }).waitFor();
        assert.equal(await spaceRow.count(), 1);
        assert.equal(await page.getByRole("list", { name: "Конфликты названий", exact: true }).count(), 0);
        assert.deepEqual(errors, []);
        console.log(`Combined field conflicts ${touch ? "touch dark" : "desktop light"} ${cardFirst ? "card first" : "parents first"}: independent blocking/cancel/resolve/history/ID/reload passed.`);
      } finally {
        await context?.close(); fixture.kill(); await fixture.exited;
        assert.ok(directory.startsWith(resolve(tmpdir(), "zedone-field-conflicts-")));
        await rm(directory, { recursive: true });
      }
    }
  }
}
