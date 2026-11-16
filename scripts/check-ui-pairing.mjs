import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { chromium } from "playwright";
import { newAccessCredential } from "../client/commands.ts";

const root=resolve(import.meta.dir,"..");
const binary=resolve(root,`target/debug/examples/shell-fixture${process.platform==="win32"?".exe":""}`);
const directories=[], processes=[];
let browser;
let diagnostic;
async function installation() {
  const directory=await mkdtemp(resolve(tmpdir(),"zedone-pairing-ui-"));directories.push(directory);
  const child=Bun.spawn([binary,"--serve",directory],{stdout:"pipe",stderr:"inherit"});processes.push(child);
  const reader=child.stdout.getReader();let text="";
  while(!text.includes("\n")){const {value,done}=await reader.read();assert.ok(!done);text+=new TextDecoder().decode(value);}
  reader.releaseLock();
  return {url:text.trim(),directory,credential:JSON.parse(await readFile(resolve(directory,"local-session.json"),"utf8"))};
}
async function api(installation,operation,parameters,credential=installation.credential) {
  const response=await fetch(`${installation.url}/v1/commands/${operation}`,{method:"POST",headers:{"content-type":"application/json",authorization:`Bearer ${credential.key}`},body:JSON.stringify({id:Array.from(crypto.getRandomValues(new Uint8Array(16))),operation,parameters})});
  return response.json();
}
const click=(page,name)=>page.getByRole("button",{name,exact:true}).click();
const done=page=>page.locator('[role="dialog"]').waitFor({state:"detached"});
async function confirm(page) {await page.locator("[data-confirm-access]").check();await page.locator("[data-access-submit]").click();await done(page);}
try {
  const a=await installation(),b=await installation();
  const space="a".repeat(32);
  const readOnlySpace="c".repeat(32);
  assert.equal((await api(a,"space.create",{space,name:"Личное"})).status,"ok");
  assert.equal((await api(a,"space.create",{space:readOnlySpace,name:"Только чтение"})).status,"ok");
  browser=await chromium.launch({headless:true,args:["--disable-frame-rate-limit"]});
  const context=await browser.newContext({viewport:{width:1280,height:900}});
  const first=await context.newPage(),second=await context.newPage();
  diagnostic=first;
  const errors=[];for(const page of [first,second])page.on("pageerror",error=>errors.push(error.message));
  first.on('response',async response=>{if(response.url().includes('/v1/commands/')){const body=await response.json().catch(()=>null);if(body?.status==='error')console.log('Expected/check API outcome:',new URL(response.url()).pathname,body.error.code);}});
  await Promise.all([first.goto(`${a.url}/#/settings`),second.goto(`${b.url}/#/settings`)]);
  await second.locator('#peer-address').fill('127.0.0.1:45670');
  await second.evaluate(()=>document.dispatchEvent(new Event('visibilitychange')));
  await second.locator('#peer-address').waitFor();
  assert.equal(await second.locator('#peer-address').inputValue(),'127.0.0.1:45670','Visibility access probe destroyed the open connection form');
  await first.locator("#listen-address").fill("127.0.0.1:0");await click(first,"Начать приём подключений");
  await first.locator("[data-listener-state]").filter({hasText:"Приём включён:"}).waitFor();
  const host=await first.locator("[data-listener-state]").innerText();const address=host.replace("Приём включён: ","");
  async function request() {
    const [response]=await Promise.all([first.waitForResponse(response=>response.url().endsWith('/host/devices/offer.create')),click(first,"Создать или показать приглашение")]);
    const created=await response.json();assert.equal(created.status,'ok');
    await first.waitForFunction(value=>document.querySelector('#outgoing-code')?.value===value,created.result.offer);
    await first.locator("#outgoing-code").waitFor();const offer=await first.locator("#outgoing-code").inputValue();
    await second.locator("#peer-address").fill(address);await second.locator("#peer-code").fill(offer);await second.locator("#peer-label").fill("Телефон");
    await click(second,"Запросить подключение");await second.locator("[data-remote-state]").filter({hasText:"Ожидаем подтверждения"}).waitFor();
    const rows=await api(a,"pairing.list",{after:null});const pending=rows.result.entries.find(entry=>entry.status==="pending");assert.ok(pending);
    assert.equal((await api(a,"access.inspect",{subject:pending.request})).error.code,"not_found");
    await click(first,"Обновить доступы");return {pending,offer};
  }
  const firstRequest=await request();console.log('Initial pending request accepted without a grant.');
  const row=first.locator(`[data-request="${firstRequest.pending.request}"]`);
  await row.getByRole("button",{name:"Подтвердить подключение",exact:true}).click();
  assert.equal(await first.locator('[data-access-write="grant"]').isChecked(),false);
  assert.equal(await first.locator('[data-access-admin="grant"]').isChecked(),false);
  assert.equal(await first.locator('[data-access-limited="grant"]').isChecked(),false);
  await first.locator(`input[data-space="${space}"]`).check();await confirm(first);
  await click(second,"Проверить состояние на другом устройстве");await second.locator("[data-remote-state]").filter({hasText:"Подключение подтверждено"}).waitFor();
  const grant=(await api(a,"access.inspect",{subject:firstRequest.pending.request})).result;
  assert.deepEqual(grant.scope,{kind:"spaces",spaces:{[space]:"read"}});assert.equal(grant.expires_at_unix_ms,null);
  await second.reload();await second.locator("[data-session-state]").filter({hasText:"Подтверждённое подключение сохранено"}).waitFor();
  await click(second,"Отвязаться от устройства");await second.locator('[role="dialog"]').waitFor();await second.locator('[role="dialog"]').getByRole("button",{name:"Подтвердить",exact:true}).click();await done(second);
  await second.locator("[data-session-state]").filter({hasText:"Нет сохранённого подключения"}).waitFor();
  await click(first,"Обновить подключения");await first.locator("#outgoing-code").waitFor({state:"detached"});
  console.log('Confirmation, saved state and own unlink passed.');
  const rejected=await request();
  await first.locator(`[data-request="${rejected.pending.request}"]`).getByRole("button",{name:"Отклонить запрос",exact:true}).click();await confirm(first);
  await click(second,"Проверить состояние на другом устройстве");await second.locator("[data-session-state]").filter({hasText:"Нет сохранённого подключения"}).waitFor();
  console.log('Administrative rejection passed.');
  const cancelled=await request();
  await click(second,"Отменить подключение");await second.locator('[role="dialog"]').getByRole("button",{name:"Подтвердить",exact:true}).click();await done(second);
  await second.locator("[data-session-state]").filter({hasText:"Нет сохранённого подключения"}).waitFor();
  assert.equal((await api(a,"pairing.inspect",{request:cancelled.pending.request})).result.status,"cancelled");
  const enrollment=await newAccessCredential();
  await click(first,"Обновить доступы");await click(first,"Выдать доступ");
  await first.locator("#access-subject").fill(enrollment.credential.subject);
  await first.locator("#access-verifier").fill(enrollment.verifier.map(b=>b.toString(16).padStart(2,"0")).join(""));
  await first.locator(`input[data-space="${space}"]`).check();await confirm(first);
  assert.equal((await api(a,"space.read",{space},enrollment.credential)).status,"ok");
  const snapshot=(await api(a,"space.read",{space})).result;
  assert.equal((await api(a,"space.rename",{space,name:"Запрещено",expected:snapshot.version},enrollment.credential)).error.code,"access_denied");
  const accessRow=first.locator(`[data-subject="${enrollment.credential.subject}"]`);
  const deliveries=[];
  await first.route('**/v1/commands/access.revoke',async route=>{
    deliveries.push(route.request().postData());
    const response=await route.fetch();
    if(deliveries.length===1)await route.abort();else await route.fulfill({response});
  });
  await accessRow.getByRole("button",{name:"Отозвать доступ",exact:true}).click();await confirm(first);
  await first.unroute('**/v1/commands/access.revoke');
  assert.ok(deliveries.length>=2);assert.ok(deliveries.every(body=>body===deliveries[0]),'Lost reply changed the original revocation');
  assert.equal((await api(a,"space.read",{space},enrollment.credential)).error.code,"access_denied");
  const replacement=await newAccessCredential();
  await accessRow.getByRole('button',{name:'Заменить доступ новым ключом',exact:true}).click();
  assert.equal(await first.locator('#access-subject').inputValue(),enrollment.credential.subject);
  await first.locator('#access-verifier').fill(replacement.verifier.map(b=>b.toString(16).padStart(2,'0')).join(''));
  await first.locator(`input[data-space="${space}"]`).check();
  await first.locator('[data-access-write="grant"]').check();
  await first.locator(`input[data-space="${readOnlySpace}"]`).check();
  await first.locator(`#grant-mode-${readOnlySpace}`).selectOption('read');
  await first.locator('[data-access-admin="grant"]').check();
  await first.locator('[data-access-limited="grant"]').check();
  const until=new Date(Date.now()+120000);until.setSeconds(0,0);
  const localDate=new Date(until.getTime()-until.getTimezoneOffset()*60000).toISOString().slice(0,16);
  await first.locator('#grant-until').fill(localDate);await confirm(first);
  const updated={subject:enrollment.credential.subject,key:replacement.credential.key};
  const saved=(await api(a,'access.inspect',{subject:updated.subject})).result;
  assert.equal(saved.expires_at_unix_ms,until.getTime());assert.equal(saved.scope.kind,'administration_and_spaces');
  assert.deepEqual(saved.scope.spaces,{[space]:'read_write',[readOnlySpace]:'read'});
  assert.equal((await api(a,'access.list',{after:null},updated)).status,'ok');
  assert.equal((await api(a,'space.read',{space},enrollment.credential)).error.code,'access_denied');
  const current=(await api(a,'space.read',{space},updated)).result;
  assert.equal((await api(a,'space.rename',{space,name:'Изменено явно',expected:current.version},updated)).status,'ok');
  const renamed=(await api(a,'space.read',{space},updated)).result;
  assert.equal((await api(a,'space.rename',{space,name:'Личное',expected:renamed.version},updated)).status,'ok');
  await accessRow.getByRole('button',{name:'Отозвать доступ',exact:true}).click();
  await first.locator('[role="dialog"]').waitFor();
  const newest=await newAccessCredential();
  assert.equal((await api(a,'access.replace',{subject:updated.subject,verifier:newest.verifier,expected:saved.version,scope:{kind:'spaces',spaces:{[space]:'read'}},expires_at_unix_ms:null})).status,'ok');
  await first.locator('[data-confirm-access]').check();await first.locator('[data-access-submit]').click();
  console.log('Stale form submitted; awaiting explicit refusal.');
  await first.locator('[role="dialog"] [role="alert"]').waitFor();
  assert.match(await first.locator('[role="dialog"] [role="alert"]').innerText(),/Состояние изменилось/);
  assert.equal(await first.locator('[data-access-submit]').isDisabled(),true);
  await first.locator('[role="dialog"]').getByRole('button',{name:'Закрыть',exact:true}).click();
  await first.locator('.pending-actions').getByRole('button',{name:'Убрать из списка',exact:true}).click();
  await confirm(first);
  await first.locator('.pending-actions').waitFor({state:'detached'});
  assert.equal((await api(a,'space.read',{space},{subject:updated.subject,key:newest.credential.key})).status,'ok');
  assert.deepEqual((await api(a,"space.read",{space})).result.names,["Личное"]);
  assert.deepEqual(errors,[]);
  assert.equal(await first.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
  const persisted=await second.evaluate(()=>Object.values(localStorage).join(""));
  assert.ok(!persisted.includes(firstRequest.offer));
  await click(first,"Остановить приём подключений");
  console.log("Manual pairing UI: two installations, default/explicit/administrative/finite rights, confirm/restart/unlink/reject/cancel, issue/revoke/replace, stale refusal, lost-reply immutable retry and preserved data passed.");
} catch(error) {
  if(diagnostic)console.error('Failure state:',{alerts:await diagnostic.locator('[role="alert"]').allTextContents(),dialogs:await diagnostic.locator('[role="dialog"]').count(),formErrors:await diagnostic.locator('.form-error').allTextContents(),pending:await diagnostic.locator('.pending-actions').allTextContents()});
  throw error;
} finally {
  await browser?.close();
  for(const child of processes){child.kill();await child.exited;}
  for(const directory of directories){
    if(process.platform==="win32")assert.equal(Bun.spawnSync([binary,"--cleanup-devices",directory]).exitCode,0,"OS fixture cleanup failed");
    await rm(directory,{recursive:true});
  }
}
