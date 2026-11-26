import assert from "node:assert/strict";
import { mkdtemp,readFile,rename,rm,access } from "node:fs/promises";
import { resolve,basename } from "node:path";
import { tmpdir } from "node:os";
export async function checkDailyBackups(browser,executable) {
  for (const touch of [false,true]) {
    const directory=await mkdtemp(resolve(tmpdir(),"zedone-daily-ui-"));
    const fixture=Bun.spawn([executable,"--serve-backups",directory],{stdout:"pipe",stderr:"inherit"});
    let context;
    try {
      const reader=fixture.stdout.getReader();let url="";
      while (!url.includes("\n")) {const {value,done}=await reader.read();assert.ok(!done);url+=new TextDecoder().decode(value);}
      reader.releaseLock();url=url.trim();
      const credential=JSON.parse(await readFile(resolve(directory,"local-session.json"),"utf8"));
      const space="1".padStart(32,"0"),card="3".padStart(32,"0");
      const command=async(operation,parameters)=> {
        const response=await fetch(`${url}/v1/commands/${operation}`,{method:"POST",headers:{Authorization:`Bearer ${credential.key}`,"Content-Type":"application/json"},body:JSON.stringify({id:Array.from(crypto.getRandomValues(new Uint8Array(16))),operation,parameters})});
        const value=await response.json();assert.equal(value.status,"ok",JSON.stringify(value));return value.result;
      };
      async function until(read,ready,label) {const deadline=Date.now()+15000;let value;do {value=await read();if(ready(value))return value;await Bun.sleep(50);}while(Date.now()<deadline);throw new Error(`${label}: ${JSON.stringify(value)}`);}
      let state=await until(()=>command("space.backups.inspect",{space}),v=>v.copies.length===1,"automatic startup copy");
      await until(async()=>{try {await access(resolve(directory,"zedone.sqlite.backups",state.copies[0].id,"commit.json"));return true;}catch{return false;}},v=>v,"committed marker");
      context=await browser.newContext({viewport:touch?{width:360,height:760}:{width:1280,height:900},isMobile:touch,hasTouch:touch,acceptDownloads:true});
      const page=await context.newPage(),errors=[];page.on("pageerror",e=>errors.push(e.message));
      await page.goto(`${url}/#/spaces/${space}`);
      const panel=page.getByRole("region",{name:"Ежедневные копии",exact:true});
      const button=name=>panel.getByRole("button",{name,exact:true});
      const open=async()=>{if(!await panel.locator("details").getAttribute("open").then(v=>v!==null))await panel.locator("summary").click();};
      await open();await button("Выбрать для восстановления").waitFor();
      const before=await command("card.read",{space,card});
      await command("card.edit",{space,card,title:"Поздняя правка",description:"После снимка",expected:before.version});
      await button("Обновить состояние копий").click();await panel.getByText("За сегодняшний день копия уже сохранена.",{exact:false}).waitFor();
      assert.equal((await command("space.backups.inspect",{space})).copies.length,1);
      const download=page.waitForEvent("download");await button("Сохранить копию в файл").click();
      assert.equal((await readFile(await (await download).path())).subarray(0,8).toString(),"ZEDSPACE");
      await button("Выбрать для восстановления").click();
      const recovery=page.getByRole("region",{name:"Восстановление пространства",exact:true});
      await recovery.getByRole("checkbox").waitFor();
      assert.deepEqual((await command("card.read",{space,card})).titles,["Поздняя правка"]);
      await recovery.getByRole("button",{name:"Убрать снимок",exact:true}).click();
      await button("Выбрать для восстановления").click();await recovery.getByRole("checkbox").check();
      await recovery.getByRole("button",{name:"Подтвердить восстановление пространства",exact:true}).click();
      await recovery.getByText("Пространство восстановлено.",{exact:false}).waitFor();
      assert.deepEqual((await command("card.read",{space,card})).titles,["Сохранённый текст"]);
      await button("Удалить эту копию").click();await button("Отмена удаления").click();assert.equal((await command("space.backups.inspect",{space})).copies.length,1);
      let drop=true;await page.route("**/v1/commands/space.backups.delete",async route=>{const response=await route.fetch();if(drop)await route.abort();else await route.fulfill({response});});
      await button("Удалить эту копию").click();await button("Подтвердить удаление копии").click();await panel.getByRole("alert").waitFor();
      await page.reload();drop=false;
      await page.getByRole("button",{name:"Проверить результат",exact:true}).click();
      await page.getByRole("heading",{name:"Незавершённые действия",exact:true}).waitFor({state:"hidden"});
      await open();await panel.getByText("Сохранённых ежедневных копий в этой установке пока нет.",{exact:true}).waitFor();
      assert.equal((await command("space.backups.inspect",{space})).copies.length,0);
      const root=resolve(directory,"zedone.sqlite.backups"),away=resolve(directory,"unavailable-backups");
      await rename(root,away);
      const second="5".padStart(32,"0");await command("space.create",{space:second,name:"После ошибки папки"});
      const failed=await until(()=>command("space.backups.inspect",{space:second}),v=>v.failed_sequence>0,"unavailable folder pause");
      assert.ok(failed.retry_after_ms>0);await rename(away,root);
      await page.goto(`${url}/#/spaces/${second}`);await open();await button("Повторить сейчас").click();
      await button("Выбрать для восстановления").waitFor();
      assert.equal((await command("space.backups.inspect",{space:second})).copies.length,1);
      assert.deepEqual(errors,[]);
      await page.screenshot({path:resolve("docs/reports/p07-04-complete",`${basename(directory)}-${touch?"touch":"desktop"}.png`),fullPage:true});
      console.log(`Daily backups ${touch?"touch":"desktop"}: startup/limit, download, restore choice/cancel/confirmation, delete cancel, lost reply/reload/replay, folder error and explicit retry passed.`);
    } finally {await context?.close();fixture.kill();await fixture.exited;assert.ok(directory.startsWith(resolve(tmpdir(),"zedone-daily-ui-")));await rm(directory,{recursive:true});}
  }
}
