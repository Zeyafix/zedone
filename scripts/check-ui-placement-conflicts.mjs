import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

export async function checkPlacementConflicts(browser, executable) {
  for (const touch of [false, true]) for (const same of [false, true]) {
    const directory = await mkdtemp(resolve(tmpdir(), "zedone-placements-"));
    const fixture = Bun.spawn([executable, same ? "--serve-same-column-conflicts" : "--serve-placement-conflicts", directory], { stdout: "pipe", stderr: "inherit" });
    let context;
    try {
      const reader = fixture.stdout.getReader(); let address = "";
      while (!address.includes("\n")) {const { value, done } = await reader.read(); assert.ok(!done); address += new TextDecoder().decode(value);}
      reader.releaseLock();
      const session = JSON.parse(await readFile(resolve(directory, "local-session.json"), "utf8"));
      context = await browser.newContext({ viewport:touch ? {width:360,height:760} : {width:1280,height:900}, hasTouch:touch, isMobile:touch, colorScheme:touch ? "dark" : "light" });
      const page = await context.newPage(), errors = []; page.on("pageerror", error => errors.push(error.message));
      const press = locator => touch ? locator.tap() : locator.click();
      const space="000000000000000000000000000000c9", card="000000000000000000000000000000cb";
      const command = (operation, parameters) => page.evaluate(async ({operation,parameters,key}) => (await fetch(`/v1/commands/${operation}`, {
        method:"POST", headers:{"Content-Type":"application/json",Authorization:`Bearer ${key}`},body:JSON.stringify({id:Array.from(crypto.getRandomValues(new Uint8Array(16))),operation,parameters}),
      })).json(), {operation,parameters,key:session.key});
      const read = async () => (await command("card.read",{space,card})).result;
      const history = async () => (await command("card.history",{space,card,after:null,limit:100,expected:null})).result.versions;
      await page.goto(address.trim());
      await press(page.locator(`[data-space="${space}"]`).getByRole("link"));
      const row=page.locator(`[data-card="${card}"]`), dialog=page.getByRole("dialog");
      await row.waitFor(); assert.equal(await row.count(),1);
      assert.equal(await row.locator("xpath=ancestor::section[@aria-label='Конфликты']").count(),1);
      assert.equal(await row.locator("xpath=ancestor::section[@data-column]").count(),0);
      const before=await read(), versions=await history();
      await press(row.locator(".card-title"));
      assert.equal(await dialog.getByRole("button",{name:"Редактировать",exact:true}).count(),0);
      await press(dialog.locator("[data-workspace-dialog-close]"));
      await press(row.locator(".resolve-card-placement"));
      await dialog.getByRole("heading",{name:"Разрешить место карточки",exact:true}).waitFor();
      assert.equal(await dialog.locator('input[name="placement"]').count(),2);
      assert.equal(await dialog.getByRole("button",{name:"Сохранить решение",exact:true}).isDisabled(),true);
      await dialog.getByText("Первая колонка — перед: Сосед А",{exact:true}).waitFor();
      await dialog.getByText(same ? "Первая колонка — перед: Сосед Б" : "Вторая колонка — в конец",{exact:true}).waitFor();
      await press(dialog.getByRole("button",{name:"Отмена",exact:true}));
      assert.deepEqual(await read(),before);
      await press(row.locator(".resolve-card-placement"));
      await press(dialog.locator('input[name="placement"]').first());
      if (same) {
        const sibling="000000000000000000000000000000d2";
        const current=(await command("card.read",{space,card:sibling})).result;
        assert.equal((await command("card.set_completed",{space,card:sibling,completed:true,expected:current.version})).status,"ok");
      } else {
        const other=(await command("card.inspect_placements",{space,card})).result;
        assert.equal((await command("card.resolve_placement",{space,card,expected:other.card.version,placement:other.choices[1].placement})).status,"ok");
        await press(dialog.getByRole("button",{name:"Сохранить решение",exact:true}));
        await dialog.getByRole("alert").waitFor();
        assert.equal((await read()).placements.length,1);
        await press(dialog.locator("[data-workspace-dialog-close]"));
        await page.reload(); await row.waitFor();
        assert.equal(await row.count(),1); assert.equal(await row.locator("xpath=ancestor::section[@data-column]").count(),1);
        assert.deepEqual(await history(),versions); assert.deepEqual(errors,[]);
        console.log(`Card placement ${touch ? "touch" : "desktop"} cross-column: variants/cancel/stale/one object passed.`);
        continue;
      }
      await press(dialog.getByRole("button",{name:"Сохранить решение",exact:true}));
      await dialog.waitFor({state:"hidden"}); await row.locator(".move-card").waitFor();
      assert.equal(await row.count(),1); assert.equal(await row.locator("xpath=ancestor::section[@data-column]").count(),1);
      const chosen=await read(); assert.equal(chosen.state,"active"); assert.equal(chosen.placements.length,1);
      for (const key of ["titles","descriptions","tags","completed"]) assert.deepEqual(chosen[key],before[key]);
      assert.deepEqual(await history(),versions);
      await press(row.locator(".move-card"));
      await dialog.locator('select[name="column"]').selectOption("000000000000000000000000000000cd");
      await press(dialog.getByRole("button",{name:"Переместить",exact:true}));
      await dialog.waitFor({state:"hidden"}); await page.reload(); await row.waitFor();
      assert.deepEqual((await read()).columns,["000000000000000000000000000000cd"]);
      assert.equal(await row.count(),1); assert.deepEqual(await history(),versions); assert.deepEqual(errors,[]);
      console.log(`Card placement ${touch ? "touch" : "desktop"} same-column: choose/content/reverse/reload passed.`);
    } finally {
      await context?.close(); fixture.kill(); await fixture.exited;
      assert.ok(directory.startsWith(resolve(tmpdir(),"zedone-placements-"))); await rm(directory,{recursive:true,force:true});
    }
  }
}
