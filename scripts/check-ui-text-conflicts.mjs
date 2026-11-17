import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

export async function checkTextConflicts(browser, executable) {
  for (const touch of [false, true]) {
    for (const stale of [false, true]) {
      const directory = await mkdtemp(resolve(tmpdir(), "zedone-text-conflicts-"));
      const fixture = Bun.spawn([executable, "--serve-text-conflicts", directory], { stdout: "pipe", stderr: "inherit" });
      let context;
      try {
        const reader = fixture.stdout.getReader();
        let address = "";
        while (!address.includes("\n")) {
          const { value, done } = await reader.read();
          assert.ok(!done, "text conflict fixture exited before readiness");
          address += new TextDecoder().decode(value);
        }
        reader.releaseLock();
        const session = JSON.parse(await readFile(resolve(directory, "local-session.json"), "utf8"));
        context = await browser.newContext({ viewport: touch ? { width: 360, height: 760 } : { width: 1280, height: 900 }, hasTouch: touch, isMobile: touch });
        const page = await context.newPage(), errors = [];
        page.on("pageerror", error => errors.push(error.message));
        const press = locator => touch ? locator.tap() : locator.click();
        const button = (scope, name) => scope.getByRole("button", { name, exact: true });
        const space = "000000000000000000000000000000c9", card = "000000000000000000000000000000cb";
        const command = (operation, parameters) => page.evaluate(async ({ operation, parameters, key }) => (await fetch(`/v1/commands/${operation}`, {
          method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${key}` },
          body: JSON.stringify({ id: Array.from(crypto.getRandomValues(new Uint8Array(16))), operation, parameters }),
        })).json(), { operation, parameters, key: session.key });
        await page.goto(address.trim());
        await press(page.locator(`[data-space="${space}"]`).getByRole("link"));
        const row = page.locator(`[data-card="${card}"]`), dialog = page.getByRole("dialog");
        await row.waitFor();
        assert.equal(await row.count(), 1);
        assert.equal(await row.locator("xpath=ancestor::section[@data-column]").count(), 1);
        await press(row.locator(".card-title"));
        await dialog.getByText("Заголовок с телефона", { exact: true }).waitFor();
        assert.equal(await button(dialog, "Редактировать").count(), 0);
        assert.equal(await button(dialog, "Отметить выполненной").count(), 0);
        const before = (await command("card.read", { space, card })).result;
        await press(button(dialog, "Разрешить конфликт текста"));
        const form = dialog.getByRole("form", { name: "Разрешение конфликта текста" });
        await form.waitFor();
        assert.equal(await form.getByRole("textbox", { name: "Итоговый заголовок", exact: true }).evaluate(element => element === document.activeElement), true);
        assert.equal(await button(form, "Сохранить решение").isDisabled(), true);
        await press(button(form, "Отмена"));
        assert.equal(await button(dialog, "Разрешить конфликт текста").evaluate(element => element === document.activeElement), true);
        assert.deepEqual((await command("card.read", { space, card })).result, before);
        await press(button(dialog, "Разрешить конфликт текста"));
        const title = form.getByRole("textbox", { name: "Итоговый заголовок", exact: true });
        const description = form.getByRole("textbox", { name: "Итоговое описание", exact: true });
        await press(form.locator(".text-variants li").filter({ hasText: "Заголовок с телефона" }).getByRole("button"));
        assert.equal(await title.inputValue(), "Заголовок с телефона");
        await title.press("Enter");
        assert.deepEqual((await command("card.read", { space, card })).result, before, "Enter must not implicitly discard descriptions");
        await press(form.locator(".text-variants li").filter({ hasText: "Описание с компьютера" }).getByRole("button"));
        assert.match(await description.inputValue(), /<script>сырой текст<\/script>/);
        await title.fill(" ");
        assert.equal(await button(form, "Сохранить решение").isDisabled(), true);
        await title.fill("Общий заголовок 😀");
        await description.fill("Объединённое описание\n<script>без выполнения</script>");
        if (stale) {
          assert.equal((await command("card.resolve_text", { space, card, title: "Другое решение", description: "Другой текст", expected: before.version })).status, "ok");
        }
        await press(button(form, "Сохранить решение"));
        if (stale) {
          await dialog.getByRole("alert").filter({ hasText: "Данные изменились" }).waitFor();
          assert.equal(await title.inputValue(), "Общий заголовок 😀");
          assert.match(await description.inputValue(), /Объединённое описание/);
          await press(button(dialog, "Убрать исходное действие"));
          await press(button(dialog, "Подтвердить удаление записи"));
          await press(button(dialog, "Сверить с сохранённым"));
          await press(button(dialog, "Использовать показанную карточку"));
          await dialog.getByRole("heading", { name: "Другое решение", exact: true }).waitFor();
          assert.equal(await form.count(), 0);
        } else {
          await form.waitFor({ state: "hidden" });
          await button(dialog, "Редактировать").waitFor();
          const after = (await command("card.read", { space, card })).result;
          assert.deepEqual(after.titles, ["Общий заголовок 😀"]);
          assert.deepEqual(after.descriptions, ["Объединённое описание\n<script>без выполнения</script>"]);
          for (const key of ["card", "space", "columns", "positions", "completed", "tags", "deletion_groups"]) assert.deepEqual(after[key], before[key]);
          const history = (await command("card.history", { space, card, after: null, expected: null, limit: 100 })).result;
          assert.equal(history.versions.length, 4);
          assert.equal(history.versions.at(-1).title, after.titles[0]);
          assert.equal(history.versions.at(-1).description, after.descriptions[0]);
          await press(button(dialog, "Отметить выполненной"));
          await button(dialog, "Снять отметку выполнения").waitFor();
        }
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
        await press(dialog.locator("[data-workspace-dialog-close]"));
        await page.reload();
        await page.getByRole("heading", { name: "Общее имя", exact: true }).waitFor();
        assert.equal(await page.locator(`[data-card="${card}"]`).count(), 1);
        assert.equal((await command("card.read", { space, card })).result.state, "active");
        assert.deepEqual(errors, []);
        console.log(`Text conflicts ${touch ? "touch" : "desktop"} ${stale ? "stale" : "merge"}: variants/cancel/validation/history/blocking/reload passed.`);
      } finally {
        await context?.close(); fixture.kill(); await fixture.exited;
        assert.ok(directory.startsWith(resolve(tmpdir(), "zedone-text-conflicts-")));
        await rm(directory, { recursive: true });
      }
    }
  }
}
