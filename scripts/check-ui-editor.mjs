import assert from "node:assert/strict";

/** Real Service/storage/history. Faults replace delivery or persistence only. */
export async function checkEditor(page, url, credential) {
  const random = () => crypto.randomUUID().replaceAll("-", "");
  const space = random(), column = random(), card = random();
  const command = (operation, parameters) => page.evaluate(async ({ operation, parameters, key }) => {
    return (await fetch(`/v1/commands/${operation}`, {
      method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${key}`, "X-Zedone-Test-Agent": "1" },
      body: JSON.stringify({ id: Array.from(crypto.getRandomValues(new Uint8Array(16))), operation, parameters }), signal: AbortSignal.timeout(10000),
    })).json();
  }, { operation, parameters, key: credential.key });
  async function ok(operation, parameters) {
    const response = await command(operation, parameters);
    assert.equal(response.status, "ok", `${operation}: ${response.error?.code}`); return response.result;
  }
  const current = () => ok("card.read", { space, card });
  const history = () => ok("card.history", { space, card, after: null, expected: null, limit: 100 });
  const dialog = page.getByRole("dialog");
  const button = name => dialog.getByRole("button", { name, exact: true });
  const title = () => dialog.getByRole("textbox", { name: "Заголовок", exact: true });
  const description = () => dialog.getByRole("textbox", { name: "Описание", exact: true });
  const saved = () => dialog.getByRole("status").filter({ hasText: /^Сохранено$/ }).waitFor();
  const drafts = () => page.getByRole("region", { name: "Черновики карточек", exact: true });
  const localDrafts = () => page.evaluate(() => Object.keys(localStorage).filter(key => key.startsWith("zedone.draft.")));
  const intentions = () => page.evaluate(() => Object.fromEntries(Object.entries(localStorage).filter(([key]) => key.startsWith("zedone.pending."))));
  async function open() { await page.locator(".card-title").click(); await dialog.waitFor(); }
  async function edit() { await open(); await button("Редактировать").click(); await title().waitFor(); await saved(); }
  async function fill(nextTitle, nextDescription) { await title().fill(nextTitle); await description().fill(nextDescription); }
  async function close() { await dialog.getByRole("button", { name: "Закрыть действие", exact: true }).click(); await dialog.waitFor({ state: "hidden" }); }
  async function forgetOriginal() {
    await button("Убрать исходное действие").click(); await button("Подтвердить удаление записи").click();
    await button("Проверить исходное действие").waitFor({ state: "hidden" });
  }
  async function discard() {
    await button("Удалить черновик").click(); await button("Подтвердить удаление черновика").click();
    await dialog.waitFor({ state: "hidden" });
  }
  await ok("space.create", { space, name: "Редактор" });
  await ok("column.create", { space, column, name: "Мысли" });
  await ok("card.create", { space, column, card, title: "Начало", description: "Исходный текст" });
  await page.goto(`${url}/#/spaces/${space}`);
  await edit();
  console.log("Editor: autosave, paired history and completion");
  const base = await current();
  await fill("Первая правка", "Пара полей\n<b>Текст</b>"); await saved();
  let state = await current(); assert.equal(state.titles[0], "Первая правка"); assert.equal(state.descriptions[0], "Пара полей\n<b>Текст</b>");
  await fill("Вторая правка", "Непрерывная серия"); await saved();
  assert.equal((await history()).versions.length, 1, "no per-autosave history versions");
  if (process.env.UI_SCREENSHOT_DIRECTORY) await page.screenshot({ path: `${process.env.UI_SCREENSHOT_DIRECTORY}/editor-desktop.png` });
  await page.setViewportSize({ width: 360, height: 760 });
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForFunction(() => document.documentElement.dataset.theme === "dark");
  if (process.env.UI_SCREENSHOT_DIRECTORY) await page.screenshot({ path: `${process.env.UI_SCREENSHOT_DIRECTORY}/editor-mobile-dark.png` });
  await page.emulateMedia({ colorScheme: "light" }); await page.setViewportSize({ width: 1280, height: 900 });
  await close();
  let versions = (await history()).versions;
  assert.deepEqual(versions.map(item => [item.title, item.description]), [["Начало", "Исходный текст"], ["Вторая правка", "Непрерывная серия"]]);
  assert.deepEqual(await localDrafts(), []);
  await open(); await button("Отметить выполненной").click(); await button("Снять отметку выполнения").waitFor();
  state = await current(); assert.deepEqual(state.completed, [true]);
  assert.deepEqual(state.positions, base.positions); assert.deepEqual(state.tags, base.tags); assert.equal((await history()).versions.length, 2);
  await button("Снять отметку выполнения").click(); await button("Отметить выполненной").waitFor();
  assert.deepEqual((await current()).completed, [false]); await close();

  console.log("Editor: new input during an admitted autosave");
  await edit();
  let release, started;
  const held = new Promise(resolve => { release = resolve; }), admitted = new Promise(resolve => { started = resolve; });
  let first = true;
  const submitted = [];
  await page.route("**/v1/commands/card.autosave", async route => {
    submitted.push(route.request().postData());
    const response = await route.fetch(); assert.equal((await response.json()).status, "ok");
    if (first) { first = false; started(); await held; }
    await route.fulfill({ response });
  });
  await fill("Отправленная пара", "Первый текст"); await admitted;
  await fill("Более новый ввод", "Не подменять ответом");
  release(); await saved();
  assert.equal(await title().inputValue(), "Более новый ввод");
  assert.equal((await current()).descriptions[0], "Не подменять ответом");
  assert.ok(submitted.length >= 2); assert.notEqual(JSON.parse(submitted[0]).id.join(), JSON.parse(submitted.at(-1)).id.join());
  await page.unrouteAll({ behavior: "wait" }); await close();

  console.log("Editor: IME composition stays local until compositionend");
  await edit(); let deliveries = 0;
  await page.route("**/v1/commands/card.autosave", route => { deliveries++; return route.continue(); });
  await title().dispatchEvent("compositionstart"); await title().fill("Составляемый заголовок");
  await page.waitForTimeout(1100); assert.equal(deliveries, 0);
  await title().dispatchEvent("compositionend"); await saved(); assert.equal(deliveries, 1);
  await page.unrouteAll({ behavior: "wait" }); await close();

  console.log("Editor: empty title, guarded navigation and durable input");
  await edit(); const beforeEmpty = await current(), beforeHistory = (await history()).versions.length;
  await fill("\u0085 ", "Несохранённое описание");
  await dialog.getByRole("status").filter({ hasText: "Заголовок обязателен" }).waitFor();
  await page.waitForTimeout(650);
  assert.deepEqual((await current()).version, beforeEmpty.version); assert.equal((await history()).versions.length, beforeHistory);
  await page.goto(`${url}/#/settings`);
  await button("Остаться в редакторе").waitFor(); assert.equal(new URL(page.url()).hash, `#/spaces/${space}`);
  assert.equal(await button("Остаться в редакторе").evaluate(el => el === document.activeElement), true, "leave warning receives focus");
  await button("Остаться в редакторе").click(); assert.equal(await description().inputValue(), "Несохранённое описание");
  await dialog.getByRole("button", { name: "Закрыть действие", exact: true }).click();
  await button("Закрыть, сохранив ввод").waitFor();
  await button("Закрыть, сохранив ввод").click(); await dialog.waitFor({ state: "hidden" });
  assert.equal(new URL(page.url()).hash, `#/spaces/${space}`, "staying cancels the earlier navigation request");
  await page.goto(`${url}/#/spaces/${space}`);
  await drafts().getByRole("button", { name: "Открыть черновик", exact: true }).click(); await title().waitFor();
  assert.equal(await description().inputValue(), "Несохранённое описание");
  await fill("Исправленный заголовок", "Несохранённое описание"); await saved();
  await page.goto(`${url}/#/settings`); await dialog.waitFor({ state: "hidden" });
  await page.getByRole("heading", { level: 1, name: "Настройки", exact: true }).waitFor();
  assert.equal((await localDrafts()).length, 0, "confirmed navigation finishes the editor and removes its draft");
  await page.goto(`${url}/#/spaces/${space}`);

  console.log("Editor: lost reply/restart keeps the exact intention and newer draft");
  await edit(); const lostBodies = [];
  await page.route("**/v1/commands/card.autosave", async route => {
    lostBodies.push(route.request().postData()); const response = await route.fetch();
    const actual = await response.json(); assert.equal(actual.status, "ok", actual.error?.code); await route.abort();
  });
  await fill("Принято без ответа", "Первая пара");
  await dialog.getByRole("alert").filter({ hasText: "Результат неизвестен" }).waitFor();
  await fill("Новее исходной команды", "Ввод после потери ответа");
  assert.equal(lostBodies.length, 3); assert.equal(new Set(lostBodies).size, 1);
  const original = await intentions(); assert.equal(Object.keys(original).length, 1);
  const draftKeys = await localDrafts(); assert.equal(draftKeys.length, 1);
  assert.ok(!JSON.stringify(await page.evaluate(() => Object.fromEntries(Object.entries(localStorage)))).includes(credential.key));
  page.once("dialog", dialog => dialog.accept()); await page.reload();
  await drafts().getByRole("button", { name: "Открыть черновик", exact: true }).click(); await title().waitFor();
  assert.equal(await title().inputValue(), "Новее исходной команды"); assert.deepEqual(await intentions(), original);
  assert.equal(lostBodies.length, 3, "no automatic retry on reload");
  await page.unrouteAll({ behavior: "wait" }); await button("Проверить исходное действие").click(); await saved();
  assert.equal((await current()).titles[0], "Новее исходной команды"); assert.deepEqual(await intentions(), {}); await close();

  console.log("Editor: late agent edit survives replay; rebase needs explicit consent");
  await edit();
  await page.route("**/v1/commands/card.autosave", async route => { const response = await route.fetch(); assert.equal((await response.json()).status, "ok"); await route.abort(); });
  await fill("Старая принятая пара", "Старый текст"); await dialog.getByRole("alert").filter({ hasText: "Результат неизвестен" }).waitFor();
  state = await current();
  await ok("card.edit", { space, card, expected: state.version, title: "Правка агента", description: "Поздние данные" });
  await fill("Мой следующий текст", "Требует явного решения");
  await page.unrouteAll({ behavior: "wait" }); await button("Проверить исходное действие").click();
  await dialog.getByRole("region", { name: "Сохранённая карточка" }).getByText("Правка агента", { exact: true }).waitFor();
  assert.equal(await title().inputValue(), "Мой следующий текст"); assert.equal((await current()).titles[0], "Правка агента");
  await button("Сохранить мой текст вместо показанного").click(); await saved();
  assert.equal((await current()).titles[0], "Мой следующий текст"); await close();

  console.log("Editor: stale save preserves external completion, input and original version");
  await edit(); state = await current(); const beforeStaleHistory = (await history()).versions.length;
  await ok("card.set_completed", { space, card, expected: state.version, completed: true });
  await fill("Не применять автоматически", "Оставить мой ввод");
  await dialog.getByRole("alert").filter({ hasText: "Данные изменились" }).waitFor();
  assert.deepEqual((await current()).completed, [true]); assert.equal((await history()).versions.length, beforeStaleHistory);
  const staleIntention = await intentions(); await button("Сверить с сохранённым").click();
  assert.deepEqual(await intentions(), staleIntention); assert.equal(await description().inputValue(), "Оставить мой ввод");
  await forgetOriginal(); await button("Сохранить мой текст вместо показанного").click(); await saved();
  assert.deepEqual((await current()).completed, [true]); await close();

  console.log("Editor: denied delivery retains both fields and original identity");
  await edit();
  await page.route("**/v1/commands/card.autosave", route => route.continue({ headers: { ...route.request().headers(), authorization: `Bearer ${"0".repeat(64)}` } }));
  await fill("Проверка доступа", "Не терять при отказе"); await dialog.getByRole("alert").filter({ hasText: "Нет доступа к этому действию" }).waitFor();
  assert.equal(await description().inputValue(), "Не терять при отказе");
  await page.unrouteAll({ behavior: "wait" }); await button("Проверить исходное действие").click(); await saved(); await close();

  console.log("Editor: draft persistence failure prevents any mutation");
  await open(); let mutations = 0;
  await page.route(/\/v1\/commands\/card\.(begin_editing|autosave)/, route => { mutations++; return route.continue(); });
  await page.evaluate(() => {
    window.__saveEditorItem = Storage.prototype.setItem;
    Storage.prototype.setItem = function(key, value) { if (key.startsWith("zedone.draft.")) throw new DOMException("quota", "QuotaExceededError"); return window.__saveEditorItem.call(this, key, value); };
  });
  await button("Редактировать").click(); await title().waitFor(); await fill("Локальное хранилище", "Остаться в памяти");
  await dialog.getByRole("alert").filter({ hasText: "Ввод не сохранён на устройстве" }).waitFor(); assert.equal(mutations, 0);
  await page.evaluate(() => { Storage.prototype.setItem = window.__saveEditorItem; delete window.__saveEditorItem; });
  await button("Повторить запись черновика").click(); await saved();
  assert.equal((await current()).titles[0], "Локальное хранилище"); await page.unrouteAll({ behavior: "wait" }); await close();

  console.log("Editor: unavailable credential acquisition is bounded before delivery");
  await edit(); let releaseCredential; const credentialHeld = new Promise(resolve => { releaseCredential = resolve; });
  let credentialMutations = 0;
  await page.route("**/host/session", async route => {
    const response = await route.fetch(); await credentialHeld;
    try { await route.fulfill({ response }); } catch { /* The deadline aborts this deliberately held request. */ }
  });
  await page.route("**/v1/commands/card.autosave", route => { credentialMutations++; return route.continue(); });
  await fill("Ожидание доступа", "Не отправлять до проверки доступа");
  await dialog.getByRole("alert").filter({ hasText: "Новое действие не отправлено" }).waitFor({ timeout: 15_000 });
  assert.equal(credentialMutations, 0); assert.deepEqual(await intentions(), {});
  releaseCredential(); await page.unrouteAll({ behavior: "wait" });
  assert.equal(await description().inputValue(), "Не отправлять до проверки доступа");
  await button("Сверить с сохранённым").click(); await button("Сохранить мой текст вместо показанного").click(); await saved(); await close();

  console.log("Editor: stale and replayed completion never overwrite newer state");
  await open(); state = await current(); const completedBefore = state.completed[0];
  await ok("card.edit", { space, card, expected: state.version, title: state.titles[0], description: "Новые пункты перед выполнением" });
  await button(completedBefore ? "Снять отметку выполнения" : "Отметить выполненной").click();
  await dialog.getByRole("alert").filter({ hasText: "Данные изменились" }).waitFor();
  assert.deepEqual((await current()).completed, [completedBefore]);
  await button("Сверить с сохранённым").click(); await forgetOriginal();
  await button("Использовать показанную карточку").click();
  await button(completedBefore ? "Снять отметку выполнения" : "Отметить выполненной").click();
  await button(completedBefore ? "Отметить выполненной" : "Снять отметку выполнения").waitFor();
  const completionBodies = [];
  await page.route("**/v1/commands/card.set_completed", async route => {
    if (route.request().headers()["x-zedone-test-agent"] === "1") return route.continue();
    completionBodies.push(route.request().postData()); const response = await route.fetch();
    assert.equal((await response.json()).status, "ok"); await route.abort();
  });
  await button(completedBefore ? "Отметить выполненной" : "Снять отметку выполнения").click();
  await dialog.getByRole("alert").filter({ hasText: "Результат неизвестен" }).waitFor();
  assert.equal(completionBodies.length, 3); assert.equal(new Set(completionBodies).size, 1);
  state = await current();
  await ok("card.set_completed", { space, card, expected: state.version, completed: !state.completed[0] });
  const newerCompleted = !state.completed[0];
  await page.unrouteAll({ behavior: "wait" }); await button("Проверить исходное действие").click();
  await dialog.getByRole("region", { name: "Сохранённая карточка" }).waitFor();
  assert.deepEqual((await current()).completed, [newerCompleted]); await close();

  console.log("Editor: continuous typing flushes before typing ends");
  await edit(); let savesDuringTyping = 0;
  await page.route("**/v1/commands/card.autosave", route => { savesDuringTyping++; return route.continue(); });
  await title().fill("Непрерывный ввод ");
  await title().pressSequentially("abcdefghijklmnopqrstuvwxyz0123456789", { delay: 80 });
  assert.ok(savesDuringTyping > 0, "max wait bounds autosave during uninterrupted input");
  await saved(); assert.equal((await current()).titles[0], await title().inputValue());
  await page.unrouteAll({ behavior: "wait" }); await close();

  console.log("Editor: visibility boundary flushes and begins a different session");
  await edit(); const sessions = [];
  await page.route("**/v1/commands/card.autosave", route => { sessions.push(JSON.parse(route.request().postData()).parameters.session); return route.continue(); });
  const finished = page.waitForResponse(response => response.url().endsWith("/card.finish_editing"));
  await fill("До скрытия", "Ввод перед сменой состояния");
  await page.evaluate(() => {
    Object.defineProperty(document, "visibilityState", { configurable: true, value: "hidden" });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  await finished; await saved(); assert.equal((await current()).titles[0], "До скрытия");
  const resumed = page.waitForResponse(response => response.url().endsWith("/card.begin_editing"));
  await page.evaluate(() => {
    Object.defineProperty(document, "visibilityState", { configurable: true, value: "visible" });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  await resumed; await saved();
  await fill("После возвращения", "Новая сессия"); await saved();
  assert.ok(sessions.length >= 2); assert.notEqual(sessions[0], sessions.at(-1));
  await page.evaluate(() => { delete document.visibilityState; });
  await page.unrouteAll({ behavior: "wait" }); await close();

  console.log("Editor: a second tab cannot overwrite the same local draft");
  await edit(); const other = await page.context().newPage();
  try {
    await other.goto(`${url}/#/spaces/${space}`);
    await other.getByRole("region", { name: "Черновики карточек" }).getByRole("button", { name: "Открыть черновик", exact: true }).click();
    await other.getByRole("dialog").getByRole("alert").filter({ hasText: "открыт в другой вкладке" }).waitFor();
    assert.equal(await other.getByRole("dialog").getByRole("textbox").count(), 0);
    await other.getByRole("dialog").getByRole("button", { name: "Закрыть действие", exact: true }).click();
  } finally { await other.close(); }
  await fill("После второй вкладки", "Ввод владельца"); await saved(); await close();

  console.log("Editor: an idle 30-second boundary materializes one paired version");
  await edit(); const count = (await history()).versions.length;
  await fill("Перед паузой", "Парная версия"); await saved();
  await page.waitForTimeout(31_000);
  versions = (await history()).versions; assert.equal(versions.length, count + 1); assert.equal(versions.at(-1).description, "Парная версия");
  await fill("После паузы", "Следующая серия"); await saved(); await close();
  versions = (await history()).versions; assert.equal(versions.length, count + 2);
  assert.deepEqual(await localDrafts(), []); assert.deepEqual(await intentions(), {});
  assert.equal((await current()).titles[0], "После паузы");
  await page.goto(`${url}/#/spaces`);
  console.log("Editor: full text/completion lifecycle, history/session boundaries, IME, draft recovery/navigation, concurrency, replay, denial and persistence passed.");
}
