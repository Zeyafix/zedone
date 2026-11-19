import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

export async function checkCardDeletion(browser, executable) {
  for (const touch of [false, true]) {
    const prefix = resolve(tmpdir(), "zedone-card-deletion-");
    const directory = await mkdtemp(prefix);
    const fixture = Bun.spawn([executable, "--serve-card-deletion", directory], { stdout: "pipe", stderr: "inherit" });
    let context;
    try {
      const reader = fixture.stdout.getReader();
      let address = "";
      while (!address.includes("\n")) {
        const { value, done } = await reader.read();
        assert.ok(!done, "card deletion fixture exited before readiness");
        address += new TextDecoder().decode(value);
      }
      reader.releaseLock();
      context = await browser.newContext({
        viewport: touch ? { width: 360, height: 760 } : { width: 1280, height: 900 },
        hasTouch: touch, isMobile: touch, colorScheme: touch ? "dark" : "light",
      });
      const page = await context.newPage();
      const errors = [];
      page.on("pageerror", error => errors.push(error.message));
      const press = locator => touch ? locator.tap() : locator.click();
      const space = "000000000000000000000000000000c9";
      const card = "000000000000000000000000000000cb";
      const boardCard = page.locator(`[data-card="${card}"]`);
      const conflict = page.locator(`[data-deletion-card="${card}"]`);
      const resolveDeletion = conflict.getByRole("button", { name: "Разрешить удаление", exact: true });
      const dialog = page.getByRole("dialog");
      const save = dialog.getByRole("button", { name: "Разрешить удаление карточки", exact: true });
      const resolvePlacement = async scope => {
        await press(scope.getByRole("button", { name: "Разрешить место", exact: true }));
        await press(dialog.getByRole("radio", { name: /Колонка с телефона/ }));
        await press(dialog.getByRole("button", { name: "Сохранить решение", exact: true }));
        await dialog.waitFor({ state: "hidden" });
      };
      await page.goto(`${address.trim()}/#/spaces/${space}`);
      await conflict.waitFor();
      assert.equal(await boardCard.count(), 0);
      await press(resolveDeletion);
      await dialog.getByText("Заголовок с телефона", { exact: true }).waitFor();
      await dialog.getByText(/Описание с телефона/).waitFor();
      assert.equal(await save.isDisabled(), true);
      await press(dialog.getByRole("button", { name: "Отмена", exact: true }));
      await dialog.waitFor({ state: "hidden" });
      await conflict.waitFor();
      if (!touch) await resolvePlacement(conflict);
      await press(resolveDeletion);
      await press(dialog.getByRole("radio", { name: touch ? "Оставить в корзине с правками" : "Восстановить с правками" }));
      await press(save);
      await dialog.waitFor({ state: "hidden" });
      await conflict.waitFor({ state: "hidden" });
      if (touch) {
        await press(page.getByRole("button", { name: "Активные карточки", exact: true }));
        assert.equal(await boardCard.count(), 0);
        await press(page.getByRole("button", { name: "Корзина карточек", exact: true }));
        await resolvePlacement(boardCard);
      }
      await press(boardCard.getByRole("button", { name: "Заголовок с телефона", exact: true }));
      const resolveTag = dialog.getByRole("button", { name: "Оставить тег", exact: true });
      await resolveTag.waitFor();
      await press(resolveTag);
      await resolveTag.waitFor({ state: "hidden" });
      await press(dialog.locator("[data-workspace-dialog-close]"));
      await dialog.waitFor({ state: "hidden" });
      await page.reload();
      await page.getByRole("heading", { name: "Общее имя", exact: true }).waitFor();
      if (touch) {
        assert.equal(await boardCard.count(), 0);
        await press(page.getByRole("button", { name: "Корзина карточек", exact: true }));
      }
      await boardCard.getByText("Заголовок с телефона", { exact: true }).waitFor();
      assert.equal(await boardCard.count(), 1);
      assert.equal(await page.locator('[data-column="000000000000000000000000000000d0"]').locator(boardCard).count(), 1);
      await boardCard.getByText("Спорный тег", { exact: true }).waitFor();
      assert.equal(await conflict.count(), 0);
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
      assert.deepEqual(errors, []);
      console.log(`Card deletion ${touch ? "touch dark / keep in trash" : "desktop light / restore"} passed.`);
    } finally {
      await context?.close();
      fixture.kill();
      await fixture.exited;
      assert.ok(directory.startsWith(prefix));
      await rm(directory, { recursive: true });
    }
  }
}
