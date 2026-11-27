import assert from "node:assert/strict";
import { mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { resolve, dirname } from "node:path";
import { tmpdir } from "node:os";

export async function checkBackupCatalog(browser, executable) {
  for (const touch of [false,true]) {
    const directory=await mkdtemp(resolve(tmpdir(),"zedone-catalog-ui-"));
    const child=Bun.spawn([executable,"--serve-backups",directory],{stdout:"pipe",stderr:"inherit"});
    let context;
    try {
      const reader=child.stdout.getReader();let url="";
      while(!url.includes("\n")){const next=await reader.read();assert.ok(!next.done);url+=new TextDecoder().decode(next.value);}
      reader.releaseLock();url=url.trim();
      const credential=JSON.parse(await readFile(resolve(directory,"local-session.json"),"utf8"));
      const request=async(operation,parameters)=>{
        const response=await fetch(`${url}/v1/commands/${operation}`,{method:"POST",headers:{Authorization:`Bearer ${credential.key}`,"Content-Type":"application/json"},body:JSON.stringify({id:Array.from(crypto.getRandomValues(new Uint8Array(16))),operation,parameters})});
        const value=await response.json();assert.equal(value.status,"ok",JSON.stringify(value));return value.result;
      };
      const secret=resolve(directory,"catalog.key");await writeFile(secret,credential.key);
      const cli=resolve(dirname(executable),"..",process.platform==="win32"?"zedone.exe":"zedone");
      const grant=(mode,folder)=>{
        const r=Bun.spawnSync([cli,"--select-backup-folder","--database",resolve(directory,"zedone.sqlite"),"--key-file",secret,"--mode",mode,"--folder",folder],{stdout:"pipe",stderr:"pipe"});
        assert.equal(r.exitCode,0,r.stderr.toString());const value=JSON.parse(r.stdout.toString());assert.equal(value.status,"ok");return value.result;
      };
      const root=resolve(directory,"zedone.sqlite.backups"),target=resolve(directory,"catalog-target");
      const source=grant("read",root),destination=grant("manage",target),back=grant("manage",root);
      context=await browser.newContext({viewport:touch?{width:360,height:760}:{width:1280,height:900},isMobile:touch,hasTouch:touch,acceptDownloads:true});
      const page=await context.newPage(),errors=[];page.on("pageerror",e=>errors.push(e.message));
      let cancelled=false;
      let pickerProbe=false;
      await page.route("**/v1/commands/space.list",async route=>{
        if(!pickerProbe) {await route.continue();return;}
        const response=await route.fetch();
        await new Promise(resolve=>setTimeout(resolve,300));
        pickerProbe=false;await route.fulfill({response});
      });
      await page.route("**/host/backup-folder",async route=>{
        const selection=route.request().postDataJSON();
        pickerProbe=true;
        const probe=page.waitForRequest("**/v1/commands/space.list");
        await page.evaluate(()=>document.dispatchEvent(new Event("visibilitychange")));
        await probe;
        await route.fulfill({json:{status:"ok",replayed:false,result:cancelled?{status:"cancelled"}:{status:"selected",folder:selection.writable?selection.default?back:destination:source}}});
      });
      await page.goto(`${url}/#/settings`);
      const panel=page.getByRole("region",{name:"Резервные копии",exact:true});
      const button=name=>panel.getByRole("button",{name,exact:true});
      const before=await request("backups.location.inspect",{});
      await button("Открыть папку для просмотра").click();await panel.getByRole("heading",{name:"Ежедневные копии",exact:true}).waitFor();
      assert.deepEqual(await request("backups.location.inspect",{}),before);
      const download=page.waitForEvent("download");await button("Сохранить копию в файл").click();assert.equal((await readFile(await(await download).path())).subarray(0,8).toString(),"ZEDSPACE");
      cancelled=true;await button("Сменить папку копий").click();await panel.getByText("Выбор папки отменён.",{exact:true}).waitFor();cancelled=false;
      await panel.locator(".catalog-location p").filter({hasText:before.folder}).first().waitFor();
      await panel.getByRole("heading",{name:"Ежедневные копии",exact:true}).waitFor();
      await button("Сменить папку копий").click();await panel.getByRole("checkbox",{name:"Перенести существующие копии в эту папку",exact:true}).check();await button("Отмена смены папки").click();assert.deepEqual(await request("backups.location.inspect",{}),before);
      await button("Сменить папку копий").click();await panel.getByRole("checkbox",{name:"Перенести существующие копии в эту папку",exact:true}).check();await button("Подтвердить смену папки").click();await panel.getByText("Папка копий изменена.",{exact:false}).waitFor();
      assert.equal((await readdir(root)).length,0);assert.equal((await readdir(target)).length,1);
      assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true,"Folder path must fit the viewport");
      await panel.getByLabel("Разрешённая папка для просмотра",{exact:true}).selectOption(destination.id);await button("Подготовить восстановление").waitFor();
      await button("Подготовить восстановление").click();
      const recovery=page.getByRole("region",{name:"Восстановление пространства",exact:true});await recovery.getByRole("checkbox").waitFor();await recovery.getByRole("button",{name:"Убрать снимок",exact:true}).click();
      await page.goto(`${url}/#/settings`);await button("Вернуть папку по умолчанию").click();await panel.getByRole("checkbox",{name:"Перенести существующие копии в эту папку",exact:true}).check();await button("Подтвердить смену папки").click();await panel.getByText("Папка копий изменена.",{exact:false}).waitFor();
      assert.equal((await request("backups.location.inspect",{})).is_default,true);
      await panel.getByLabel("Разрешённая папка для просмотра",{exact:true}).selectOption(back.id);await button("Удалить эту сохранённую копию").waitFor();
      await button("Удалить эту сохранённую копию").click();await button("Отмена удаления копии").click();assert.equal((await readdir(root)).length,1);
      await button("Удалить эту сохранённую копию").click();await panel.getByRole("checkbox",{name:"Подтверждаю удаление выбранной копии",exact:true}).check();await button("Подтвердить удаление сохранённой копии").click();await panel.getByText("Исправные копии в этой папке не найдены.",{exact:true}).waitFor();
      assert.equal((await readdir(root)).length,0);
      await button("Убрать разрешение этой папки").click();await page.waitForFunction(()=>document.querySelector("#backup-folder")?.value==="");
      assert.deepEqual(errors,[]);
      console.log(`Backup catalog ${touch?"touch":"desktop"}: browse without relocation, archive, chooser/cancel with access recheck, move/return, explicit restore preparation/cancel, separate delete/cancel and permission removal passed.`);
    } finally {
      await context?.close();child.kill();await child.exited;
      assert.ok(directory.startsWith(resolve(tmpdir(),"zedone-catalog-ui-")));await rm(directory,{recursive:true});
    }
  }
}
