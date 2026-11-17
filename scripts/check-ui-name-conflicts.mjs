import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

export async function checkNameConflicts(browser, executable) {
  for (const touch of [false, true]) {
    const directory = await mkdtemp(resolve(tmpdir(), "zedone-name-conflicts-"));
    const fixture = Bun.spawn([executable, "--serve-conflicts", directory], { stdout: "pipe", stderr: "inherit" });
    let context;
    try {
      const reader = fixture.stdout.getReader();
      let address = "";
      while (!address.includes("\n")) {
        const { value, done } = await reader.read();
        assert.ok(!done, "name conflict fixture exited before readiness");
        address += new TextDecoder().decode(value);
      }
      reader.releaseLock();
      const session = JSON.parse(await readFile(resolve(directory, "local-session.json"), "utf8"));
      context = await browser.newContext({ viewport: touch ? { width: 360, height: 760 } : { width: 1280, height: 900 }, hasTouch: touch, isMobile: touch });
      const page = await context.newPage(), errors = [];
      page.on("pageerror", error => errors.push(error.message));
      const press = locator => touch ? locator.tap() : locator.click();
      const button = (scope, name) => scope.getByRole("button", { name, exact: true });
      const space = "000000000000000000000000000000c9";
      const command = (operation, parameters) => page.evaluate(async ({ operation, parameters, key }) => (await fetch(`/v1/commands/${operation}`, {
        method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${key}` },
        body: JSON.stringify({ id: Array.from(crypto.getRandomValues(new Uint8Array(16))), operation, parameters }),
      })).json(), { operation, parameters, key: session.key });
      await page.goto(address.trim());
      const row = page.locator(`[data-space="${space}"]`), dialog = page.getByRole("dialog");
      await row.waitFor();
      assert.equal(await button(row,"Переименовать").isDisabled(), true);
      await press(button(row,"Разрешить название"));
      await dialog.getByText("Название с телефона", { exact: true }).waitFor();
      assert.equal(await dialog.getByRole("textbox",{name:"Название",exact:true}).evaluate(element => element === document.activeElement),true);
      await press(button(dialog,"Отмена"));
      assert.equal(await button(row,"Разрешить название").evaluate(element => element === document.activeElement),true);
      assert.deepEqual((await command("space.read",{space})).result.names,["Название с компьютера","Название с телефона"]);
      await press(row.getByRole("link"));
      const column = page.locator('section[data-column="000000000000000000000000000000ca"]');
      await column.waitFor();
      assert.equal(await button(page,"Создать колонку").isDisabled(),false);
      assert.equal(await button(column,"Создать карточку").isDisabled(),false);
      assert.equal(await button(column,"В корзину").last().isDisabled(),true);
      await press(column.locator('.card-title'));
      await press(button(dialog,"Отметить выполненной"));
      await button(dialog,"Снять отметку выполнения").waitFor();
      await press(dialog.locator('[data-workspace-dialog-close]'));
      await press(button(column,"Разрешить название"));
      assert.equal(await dialog.locator(".name-variants").evaluate(element => getComputedStyle(element).display),"grid");
      await press(button(dialog,"Колонка с телефона"));
      await press(button(dialog,"Разрешить название колонки"));
      await dialog.waitFor({state:"hidden"});
      await column.getByRole("heading",{name:"Колонка с телефона",exact:true}).waitFor();
      await press(page.locator('.resolve-space-name'));
      await dialog.getByRole("textbox",{name:"Название",exact:true}).fill("Мой выбор");
      const current = await command("space.read",{space});
      assert.equal((await command("space.resolve_name",{space,name:"Другой выбор",expected:current.result.version})).status,"ok");
      await press(button(dialog,"Разрешить название пространства"));
      await dialog.getByRole("alert").filter({hasText:"Данные изменились"}).waitFor();
      assert.equal(await dialog.getByRole("textbox",{name:"Название",exact:true}).inputValue(),"Мой выбор");
      await press(button(dialog,"К списку действий"));
      await press(button(page,"Убрать из списка"));
      await press(button(dialog,"Убрать из списка"));
      await dialog.waitFor({state:"hidden"});
      await press(button(page,"Обновить список"));
      await page.getByRole("heading",{name:"Другой выбор",exact:true}).waitFor();
      await press(page.getByRole("link",{name:"Все пространства",exact:true}));
      await page.getByRole("link",{name:"Другой выбор",exact:true}).waitFor();
      assert.equal(await page.getByRole("list",{name:"Конфликты названий",exact:true}).count(),0);
      assert.equal(await page.locator(`[data-space="${space}"]`).count(),1);
      await press(page.getByRole("link",{name:"Другой выбор",exact:true}));
      await page.reload();
      await page.getByRole("heading",{name:"Другой выбор",exact:true}).waitFor();
      assert.deepEqual((await command("card.read",{space,card:"000000000000000000000000000000cb"})).result.completed,[true]);
      assert.deepEqual(errors,[]);
      console.log(`Name conflicts ${touch ? "touch" : "desktop"}: variants/cancel/content/selection/stale draft/reload passed.`);
    } finally {
      await context?.close(); fixture.kill(); await fixture.exited;
      assert.ok(directory.startsWith(resolve(tmpdir(), "zedone-name-conflicts-")));
      await rm(directory,{recursive:true});
    }
  }
}
