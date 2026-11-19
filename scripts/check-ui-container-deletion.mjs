import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

export async function checkContainerDeletion(browser, executable) {
  const prefix = resolve(tmpdir(), "zedone-container-deletion-");
  for (const touch of [false, true]) {
    const folder = await mkdtemp(prefix);
    const fixture = Bun.spawn([executable, "--serve-container-deletion", folder], { stdout: "pipe", stderr: "inherit" });
    let context;
    try {
      const reader = fixture.stdout.getReader();
      let address = "";
      while (!address.includes("\n")) {
        const { value, done } = await reader.read();
        assert.ok(!done, "container deletion fixture exited before readiness");
        address += new TextDecoder().decode(value);
      }
      reader.releaseLock();
      context = await browser.newContext({
        viewport: touch ? { width: 360, height: 760 } : { width: 1280, height: 900 },
        hasTouch: touch, isMobile: touch, colorScheme: touch ? "dark" : "light",
      });
      const page = await context.newPage(), errors = [];
      page.on("pageerror", error => errors.push(error.message));
      const press = locator => touch ? locator.tap() : locator.click();
      const dialog = page.getByRole("dialog");
      const remote = index => `${address.trim()}/#/spaces/${index.toString(16).padStart(32, "0")}`;
      const resolve = async (index, selector, choice, stillOnBoard) => {
        await page.goto(remote(index));
        const button = page.locator(selector);
        await press(button);
        await dialog.getByText("Сохранённый текст", { exact: true }).waitFor();
        const submit = dialog.locator('[type="submit"]');
        assert.equal(await submit.isDisabled(), true);
        await press(dialog.getByRole("button", { name: "Отмена", exact: true }));
        await dialog.waitFor({ state: "hidden" });
        await press(button);
        await press(dialog.getByRole("radio", { name: choice, exact: true }));
        await press(submit);
        await dialog.waitFor({ state: "hidden" });
        await button.waitFor({ state: "hidden" });
        if (stillOnBoard) {
          await page.reload();
          await page.getByRole("button", { name: "Правка с телефона", exact: true }).waitFor();
        }
      };
      await resolve(1, ".resolve-column-deletion", touch ? "Оставить в корзине с содержимым" : "Восстановить с содержимым", !touch);
      await resolve(2, ".resolve-space-deletion", touch ? "Восстановить с содержимым" : "Оставить в корзине с содержимым", touch);
      await resolve(3, ".resolve-space-purge", touch ? "Подтвердить окончательную очистку" : "Восстановить показанные объекты", !touch);
      assert.deepEqual(errors, []);
      console.log(`Container deletion and purge decisions passed on ${touch ? "touch" : "desktop"}.`);
    } finally {
      await context?.close();
      fixture.kill();
      await fixture.exited;
      assert.ok(folder.startsWith(prefix));
      await rm(folder, { recursive: true });
    }
  }
}
