import assert from "node:assert/strict";

/** Public Service commands on HTTP/IPC, shared desktop/touch/native workflow. */
export async function checkHistory(page, { tap, fill, back: platformBack = () => page.keyboard.press("Escape"), dismissKeyboard = async () => {} } = {}) {
  const click = tap || (selector => page.locator(selector).click());
  const input = fill || ((selector, value) => page.locator(selector).fill(value));
  const back = async () => { await dismissKeyboard(); await platformBack(); };
  const api = (operation, parameters) => page.evaluate(async ({ operation, parameters }) => {
    const request = JSON.stringify({ id: Array.from(crypto.getRandomValues(new Uint8Array(16))), operation, parameters });
    const native = window.__TAURI_INTERNALS__;
    const session = native ? await native.invoke("shell_session") : await (await fetch("/host/session")).json();
    return native ? JSON.parse(await native.invoke("zedone_command", { invocation: { operation, request }, credential: session.credential.key }))
      : await (await fetch(`/v1/commands/${operation}`, { method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${session.credential.key}` }, body: request })).json();
  }, { operation, parameters });
  async function command(operation, parameters) {
    const response = await api(operation, parameters);
    assert.equal(response.status, "ok", `${operation}: ${response.error?.code}`); return response.result;
  }
  const random = () => crypto.randomUUID().replaceAll("-", "");
  const space = random(), column = random(), card = random(), tag = random();
  const current = () => command("card.read", { space, card });
  const history = () => command("card.history", { space, card, after: null, expected: null, limit: 100 });
  const edit = async (title, description) => command("card.edit", { space, card, title, description, expected: (await current()).version });
  const closed = () => page.waitForFunction(() => !document.querySelector('[role="dialog"]'));
  const loaded = () => page.waitForFunction(() => !!document.querySelector(".history-list") && !document.querySelector('.card-history [role="status"]') && !document.querySelector('.card-editor > [role="status"]')?.textContent.includes("Сохраняем"));
  const idle = () => page.waitForFunction(() => !document.querySelector('.card-editor > [role="status"]')?.textContent.includes("Сохраняем"));
  const open = async () => { await click(`.card-note[data-card="${card}"] .card-title`); await click("[data-history-open]"); await loaded(); };
  const choose = async id => {
    await click(`[data-text-version="${id}"] button`);
    await page.waitForFunction(() => document.activeElement === document.querySelector(".history-preview h4"));
  };
  async function restore(id) {
    await choose(id); await click("[data-history-restore]"); await idle(); await loaded();
    await page.waitForFunction(() => !document.querySelector("[data-history-restore]"));
  }
  async function abandon() {
    await click("[data-editor-forget]"); await click("[data-editor-forget-confirm]");
    await page.waitForFunction(() => !document.querySelector("[data-editor-retry]"));
    await click("[data-history-refresh]"); await loaded();
  }
  await command("space.create", { space, name: "История" });
  await command("column.create", { space, column, name: "Мысли" });
  await command("card.create", { space, column, card, title: "Первая версия", description: "Первый текст\n<b>Буквальный HTML</b>\n![Без загрузки](https://example.invalid/image.png)" });
  await edit("Вторая версия", "Второй текст"); await edit("Третья версия", "Третий текст 😀");
  await command("tag.create", { space, tag, name: "Сохранить" });
  await command("card.set_tag", { space, card, tag, present: true, expected: (await current()).version, expected_tag: (await command("tag.read", { space, tag })).version });
  await command("card.set_completed", { space, card, completed: true, expected: (await current()).version });
  const before = await current(), original = (await history()).versions;
  const pairs = original.map(v => [v.title, v.description]);
  assert.equal(original.length, 3);
  await page.evaluate(space => { location.hash = `#/spaces/${space}`; }, space);
  await open();
  assert.equal(await page.locator("[data-text-version]").count(), 3);
  await choose(original[0].id);
  assert.equal(await page.locator("[data-history-title]").textContent(), pairs[0][0]);
  assert.equal(await page.locator("[data-history-description]").textContent(), pairs[0][1]);
  assert.equal(await page.locator(".history-preview img, .history-preview b, .history-preview a").count(), 0);
  // Choosing and returning from a preview changes no data/history.
  await click("[data-history-close]");
  assert.equal(await page.evaluate(() => document.activeElement === document.querySelector("[data-history-open]")), true);
  assert.deepEqual(await current(), before); assert.deepEqual((await history()).versions, original);
  await click("[data-history-open]"); await loaded();
  await restore(original[0].id);
  let versions = (await history()).versions;
  assert.deepEqual(versions.map(v => [v.title, v.description]), [...pairs, pairs[0]]);
  await restore(original[2].id);
  versions = (await history()).versions;
  assert.deepEqual(versions.map(v => [v.title, v.description]), [...pairs, pairs[0], pairs[2]]);
  // An explicit restoration of an identical pair is still one new visible version.
  await restore(original[2].id); assert.equal((await history()).versions.length, 6);
  const after = await current();
  for (const key of ["tags", "completed", "columns", "positions", "placements", "state"]) assert.deepEqual(after[key], before[key], key);
  if (process.env.UI_SCREENSHOT_DIRECTORY) {
    await choose(original[0].id);
    await page.screenshot({ path: `${process.env.UI_SCREENSHOT_DIRECTORY}/history-${tap ? "touch" : "desktop"}.png`, fullPage: true });
  }
  await back(); await closed(); await open();
  assert.equal(await page.locator("[data-text-version]").count(), 6, "history survives editor remount");
  await click("[data-history-close]"); await click("[data-editor-edit]");
  await input('input[name="card-title"]', "Непрерывный ввод");
  await input('textarea[name="card-description"]', "Последняя сохранённая пара");
  await dismissKeyboard();
  // Bounded, read-only evidence for native taps during editor blur.
  // No input values, text, credentials or stored intentions are recorded.
  await page.evaluate(() => {
    const entries = [];
    const record = event => {
      const target = event.target instanceof Element ? event.target : undefined;
      const button = document.querySelector("[data-history-open]");
      const box = button?.getBoundingClientRect();
      entries.push({ type: event.type, tag: target?.tagName, name: target?.getAttribute("name"),
        historyButton: !!target?.closest("[data-history-open]"), closeButton: !!target?.closest("[data-workspace-dialog-close]"),
        inDialog: !!target?.closest('[role="dialog"]'), dialog: !!document.querySelector('[role="dialog"]'),
        x: event.clientX, y: event.clientY, button: box && { x: box.x, y: box.y, width: box.width, height: box.height } });
      if (entries.length > 20) entries.shift();
    };
    const types = ["pointerdown", "pointerup", "click", "focusin", "focusout"];
    for (const type of types) document.addEventListener(type, record, true);
    window.__zedoneHistoryTransition = { entries, dispose: () => { for (const type of types) document.removeEventListener(type, record, true); } };
  });
  try { await click("[data-history-open]"); await loaded(); }
  catch (cause) {
    const evidence = await page.evaluate(() => ({ events: window.__zedoneHistoryTransition?.entries,
      dialog: !!document.querySelector('[role="dialog"]'), history: !!document.querySelector(".card-history"),
      editor: !!document.querySelector('textarea[name="card-description"]') })).catch(() => ({ unavailable: true }));
    console.error("History editor transition:", JSON.stringify(evidence));
    throw cause;
  } finally {
    await page.evaluate(() => { window.__zedoneHistoryTransition?.dispose(); delete window.__zedoneHistoryTransition; }).catch(() => {});
  }
  versions = (await history()).versions;
  assert.equal(versions.length, 7, "history entry finishes one pending autosave series");
  assert.deepEqual([versions.at(-1).title, versions.at(-1).description], ["Непрерывный ввод", "Последняя сохранённая пара"]);
  assert.equal(await page.evaluate(() => Object.keys(localStorage).filter(key => key.startsWith("zedone.draft.")).length), 0);

  // Strict stale is the real Service rejecting a now-stale UI intention.
  await choose(original[0].id);
  await edit("Поздняя правка", "Сохранить поздний текст");
  const late = await current(), lateHistory = (await history()).versions;
  await click("[data-history-restore]");
  await page.waitForFunction(() => document.querySelector('.card-editor > [role="alert"]')?.textContent.includes("Данные изменились"));
  assert.deepEqual(await current(), late); assert.deepEqual((await history()).versions, lateHistory);
  assert.equal(await page.evaluate(() => document.querySelector("[data-history-restore]").disabled), true);
  await abandon(); await restore(original[0].id);
  assert.equal((await current()).titles[0], original[0].title);
  await back(); await closed();

  // Pagination is read from one history generation, never silently appended across a change.
  for (let i = 0; i < 22; i++) await edit(`Продолжение ${i}`, `Текст ${i}`);
  await open(); assert.equal(await page.locator("[data-text-version]").count(), 25);
  await click("[data-history-more]"); await loaded();
  let all = (await history()).versions;
  assert.equal(await page.locator("[data-text-version]").count(), all.length);
  assert.equal(new Set(await page.evaluate(() => Array.from(document.querySelectorAll("[data-text-version]"), el => el.dataset.textVersion))).size, all.length);
  await click("[data-history-refresh]"); await loaded();
  await edit("После первой страницы", "Новое поколение");
  await click("[data-history-more]");
  await page.waitForFunction(() => document.querySelector('.card-history [role="alert"]')?.textContent.includes("Данные изменились"));
  assert.equal(await page.locator("[data-text-version]").count(), 25);
  await click("[data-history-refresh]"); await loaded(); await click("[data-history-more]"); await loaded();
  all = (await history()).versions; assert.equal(await page.locator("[data-text-version]").count(), all.length);
  await restore(all.at(-1).id);
  assert.equal((await current()).titles[0], "После первой страницы");
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);

  if (!await page.evaluate(() => !!window.__TAURI_INTERNALS__)) {
    await choose(original[0].id);
    // No persistence means no delivery, including a history restoration.
    let sent = 0;
    await page.route("**/v1/commands/card.restore_text", route => { sent++; return route.continue(); });
    await page.evaluate(() => {
      window.historySavedSetItem = Storage.prototype.setItem;
      Storage.prototype.setItem = function(key, value) { if (key.includes("pending")) throw new DOMException("blocked", "QuotaExceededError"); window.historySavedSetItem.call(this, key, value); };
    });
    const preserved = await current(), preservedHistory = (await history()).versions;
    await click("[data-history-restore]");
    await page.waitForFunction(() => document.querySelector('.card-editor > [role="alert"]')?.textContent.includes("Запрос не отправлен"));
    assert.equal(sent, 0); assert.deepEqual(await current(), preserved);
    await page.evaluate(() => { Storage.prototype.setItem = window.historySavedSetItem; delete window.historySavedSetItem; });
    await page.unroute("**/v1/commands/card.restore_text"); await click("[data-history-refresh]"); await loaded(); await choose(original[0].id);
    // Explicit access denial is not automatically retried, and retains the pair.
    sent = 0;
    await page.route("**/v1/commands/card.restore_text", route => { sent++; return route.fulfill({ json: { status: "error", error: { code: "access_denied" } } }); });
    await click("[data-history-restore]");
    await page.waitForFunction(() => document.querySelector('.card-editor > [role="alert"]')?.textContent.includes("Нет доступа"));
    assert.equal(sent, 1); assert.deepEqual(await current(), preserved); assert.deepEqual((await history()).versions, preservedHistory);
    await page.unroute("**/v1/commands/card.restore_text"); await abandon();
    // Lost responses: exact persisted bytes, three attempts, restart and replay.
    const bodies = [];
    await page.route("**/v1/commands/card.restore_text", async route => {
      bodies.push(route.request().postData()); assert.equal((await (await route.fetch()).json()).status, "ok"); await route.abort();
    });
    await choose(original[0].id); await click("[data-history-restore]");
    await page.waitForFunction(() => document.querySelector('.card-editor > [role="alert"]')?.textContent.includes("Результат неизвестен"));
    assert.equal(bodies.length, 3); assert.equal(new Set(bodies).size, 1);
    const pending = await page.evaluate(() => Object.fromEntries(Object.entries(localStorage).filter(([key]) => key.includes("pending"))));
    await edit("После восстановления", "Не отменять повтором");
    const latest = await current(), latestHistory = (await history()).versions;
    await page.reload(); await page.locator(".pending-list li button:first-child").waitFor();
    assert.deepEqual(await page.evaluate(() => Object.fromEntries(Object.entries(localStorage).filter(([key]) => key.includes("pending")))), pending);
    await page.unroute("**/v1/commands/card.restore_text");
    let replay;
    await page.route("**/v1/commands/card.restore_text", route => { replay = route.request().postData(); return route.continue(); });
    await click(".pending-list li button:first-child");
    await page.waitForFunction(() => !document.querySelector(".pending-list li"));
    assert.equal(replay, bodies[0]); assert.deepEqual(await current(), latest); assert.deepEqual((await history()).versions, latestHistory);
    await page.unroute("**/v1/commands/card.restore_text"); await open(); await click("[data-history-more]"); await loaded();
    assert.equal(await page.locator("[data-text-version]").count(), latestHistory.length);
    // Failed reads show an error, not an empty history or a false preview.
    await page.route("**/v1/commands/card.history", route => route.fulfill({ json: { status: "error", error: { code: "unavailable" } } }));
    await click("[data-history-refresh]");
    await page.waitForFunction(() => !!document.querySelector('.card-history [role="alert"]'));
    assert.equal(await page.locator(".card-history .empty-view").count(), 0);
    await page.unroute("**/v1/commands/card.history"); await click("[data-history-refresh]"); await loaded();
  }
  await back(); await closed();
  // Empty history is legal. New changes continue the core's causal sequence.
  const preview = await command("history.inspect", { space, card });
  await command("history.clear", { space, card, expected: preview.version });
  const emptyState = await current(); await open();
  assert.equal(await page.locator("[data-text-version]").count(), 0);
  assert.ok((await page.locator(".card-history .empty-view").textContent()).includes("Сохранённых версий пока нет"));
  assert.deepEqual(await current(), emptyState);
  await back(); await closed(); await edit("После пустой истории", "Новая пара"); await open();
  assert.equal(await page.locator("[data-text-version]").count(), 1);
  assert.ok((await history()).versions[0].sequence > 3);
  await back(); await closed();
  console.log("History: paired preview/restore/reverse/same text, autosave boundary, stale, pagination, empty history, preservation and host failure/replay checks passed.");
}
