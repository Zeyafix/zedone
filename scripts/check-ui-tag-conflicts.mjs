import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

export async function checkTagConflicts(browser, executable) {
  for (const touch of [false, true]) for (const namesFirst of [false, true]) {
    const directory = await mkdtemp(resolve(tmpdir(), "zedone-tag-conflicts-"));
    const fixture = Bun.spawn([executable, "--serve-tag-conflicts", directory], { stdout: "pipe", stderr: "inherit" });
    let context;
    try {
      const reader = fixture.stdout.getReader(); let address = "";
      while (!address.includes("\n")) {
        const { value, done } = await reader.read(); assert.ok(!done, "tag fixture exited before readiness");
        address += new TextDecoder().decode(value);
      }
      reader.releaseLock();
      const session = JSON.parse(await readFile(resolve(directory, "local-session.json"), "utf8"));
      context = await browser.newContext({ viewport: touch ? { width: 360, height: 760 } : { width: 1280, height: 900 }, hasTouch: touch, isMobile: touch, colorScheme: touch ? "dark" : "light" });
      const page = await context.newPage(), errors = [];
      page.on("pageerror", e => errors.push(e.message));
      const press = target => touch ? target.tap() : target.click();
      const button = (scope, name) => scope.getByRole("button", { name, exact: true });
      const id = n => n.toString(16).padStart(32,"0"), space = id(201), card = id(209);
      const command = (operation, parameters) => page.evaluate(async ({ operation, parameters, key }) => (await fetch(`/v1/commands/${operation}`, {
        method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${key}` },
        body: JSON.stringify({ id: Array.from(crypto.getRandomValues(new Uint8Array(16))), operation, parameters }),
      })).json(), { operation, parameters, key: session.key });
      const readCard = async value => { const r = await command("card.read", { space, card: value }); assert.equal(r.status,"ok"); return r.result; };
      await page.goto(address.trim());
      await press(page.locator(`[data-space="${space}"]`).getByRole("link"));
      const dialog = page.getByRole("dialog"), tags = page.locator(".space-tags");
      const before = await readCard(id(203));
      const resolveNames = async () => {
        if (await tags.getAttribute("open") === null) await press(tags.locator("summary"));
        await press(tags.locator(".resolve-tag-names").first());
        await dialog.getByRole("textbox").first().waitFor();
        assert.equal(await dialog.getByRole("textbox").count(),3);
        await page.evaluate(() => { location.hash = "#/spaces"; });
        await dialog.waitFor({ state: "hidden" });
        await press(page.locator(`[data-space="${space}"]`).getByRole("link"));
        await press(tags.locator("summary"));
        await press(tags.locator(".resolve-tag-names").first());
        await dialog.getByRole("textbox").first().waitFor();
        await press(button(dialog,"Отмена"));
        assert.deepEqual((await readCard(id(203))).tags,before.tags);
        await press(tags.locator(".resolve-tag-names").first());
        for (const [i,name] of ["Личное","Все","Всё"].entries()) await dialog.getByRole("textbox",{name:`Имя тега ${i+1}`,exact:true}).fill(name);
        await command("tag.create",{space,tag:id(230),name:"Позднее изменение"});
        await press(button(dialog,"Сохранить имена группы"));
        await button(dialog,"Обновить варианты имён").waitFor();
        assert.equal(await dialog.getByRole("textbox",{name:"Имя тега 2",exact:true}).inputValue(),"Все");
        assert.equal(await button(dialog,"Сохранить имена группы").isDisabled(),true);
        await press(button(dialog,"Обновить варианты имён"));
        await button(dialog,"Сохранить имена группы").waitFor({state:"visible"});
        await press(button(dialog,"Сохранить имена группы"));
        await dialog.waitFor({state:"hidden"});
        await tags.locator(".resolve-tag-names").first().waitFor({state:"hidden"});
        assert.equal(await tags.locator(".resolve-tag-names").count(),0);
        assert.deepEqual((await readCard(id(203))).tags,before.tags);
      };
      const resolveAssignment = async () => {
        await press(page.locator(`[data-card="${card}"] .card-title`));
        await dialog.locator(".resolve-card-tag-present").waitFor();
        assert.equal(await button(dialog,"Редактировать").count(),0);
        const snapshot = await readCard(card);
        await press(dialog.locator(namesFirst ? ".resolve-card-tag-present" : ".resolve-card-tag-absent"));
        await button(dialog,"Редактировать").waitFor();
        const resolved = await readCard(card);
        assert.equal(resolved.tags.includes(id(208)),namesFirst);
        for (const key of ["titles","descriptions","columns","positions","completed"]) assert.deepEqual(resolved[key],snapshot[key]);
        await press(dialog.locator(`[data-tag="${id(208)}"] .set-card-tag`));
        await press(dialog.locator(`[data-tag="${id(208)}"] .set-card-tag`));
        await press(dialog.locator("[data-workspace-dialog-close]"));
      };
      if (namesFirst) { await resolveNames(); await resolveAssignment(); }
      else { await resolveAssignment(); await resolveNames(); }
      await page.reload();
      await page.locator(`[data-card="${card}"]`).waitFor();
      assert.equal((await readCard(card)).state,"active");
      assert.deepEqual(errors,[]);
      console.log(`Tag conflicts UI passed: touch=${touch}, namesFirst=${namesFirst}, stale input/reverse/preservation.`);
    } finally {
      await context?.close(); fixture.kill(); await fixture.exited;
      await rm(directory, { recursive: true, force: true });
    }
  }
}
