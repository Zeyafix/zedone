import assert from "node:assert/strict";

/** One Service contract on desktop/touch and native IPC; no direct data writes. */
export async function checkHistoryCleanup(page, { tap, back = () => page.keyboard.press("Escape") } = {}) {
  const settled = () => page.waitForFunction(() => document.querySelector(".spaces-view")?.getAttribute("aria-busy") === "false");
  const click = async selector => { if (tap) await tap(selector); else await page.locator(selector).click(); await settled(); };
  const command = (operation, parameters, id) => page.evaluate(async ({ operation, parameters, id }) => {
    const native = window.__TAURI_INTERNALS__;
    const session = native ? await native.invoke("shell_session") : await (await fetch("/host/session")).json();
    const requestId = id ? Array.from(id.matchAll(/../g), m => parseInt(m[0], 16)) : Array.from(crypto.getRandomValues(new Uint8Array(16)));
    const request = JSON.stringify({ id: requestId, operation, parameters });
    const response = native ? JSON.parse(await native.invoke("zedone_command", { invocation: { operation, request }, credential: session.credential.key }))
      : await (await fetch(`/v1/commands/${operation}`, { method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${session.credential.key}` }, body: request })).json();
    if (response.status !== "ok") throw new Error(`${operation}: ${response.error?.code}`);
    return response.result;
  }, { operation, parameters, id });
  const random = () => crypto.randomUUID().replaceAll("-", "");
  const space = random(), column = random(), card = random(), trash = random(), tag = random();
  const snapshot = id => command("card.read", { space, card: id });
  const history = id => command("history.inspect", { space, card: id });
  const edit = async (title, description) => command("card.edit", { space, card, title, description, expected: (await snapshot(card)).version });
  const all = () => command("card.list", { space, include_trashed: true });
  const submit = '[role="dialog"] button[type="submit"]';
  const closed = () => page.waitForFunction(() => !document.querySelector('[role="dialog"]'));
  const text = selector => page.evaluate(s => document.querySelector(s)?.textContent ?? "", selector);
  const disabled = selector => page.evaluate(s => document.querySelector(s)?.disabled, selector);
  async function openCard() {
    await click(`.card-note[data-card="${card}"] .card-title`); await click("[data-history-open]");
    await page.waitForFunction(() => !!document.querySelector(".history-list") && !document.querySelector('.card-history [role="status"]'));
    await click("[data-history-cleanup]");
  }
  const openSpace = () => click(".space-history-cleanup");
  const confirm = async () => { await click(".history-confirmation input"); await click(submit); await closed(); await settled(); };
  async function abandon() {
    await back(); await closed();
    await click(".pending-list li button:last-child"); await click(submit); await closed();
    await click(".view-toolbar .view-actions button:nth-child(2)");
  }
  async function verifyPreview(preview) {
    const shown = await page.evaluate(() => [...document.querySelectorAll("[data-history-entry]")].map(e => [e.dataset.historyCard, e.dataset.historyEntry, e.dataset.historyRetained]));
    assert.deepEqual(shown, preview.entries.map(e => [e.card, e.version.id, String(e.retained)]));
    assert.equal((await text("[data-history-bytes]")).replace(/\D/g, ""), String(preview.text_bytes));
    assert.equal((await text("[data-history-removable]")).replace(/\D/g, ""), String(preview.removable_bytes));
    assert.equal(await disabled(submit), true, "destructive action needs a separate explicit confirmation");
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  }
  await command("space.create", { space, name: "Очистка истории" });
  await command("column.create", { space, column, name: "Мысли" });
  await command("card.create", { space, column, card, title: "Мысль 😀", description: "<b>Буквально</b>\n![Без загрузки](https://example.invalid/image.png)" });
  await edit("Новая мысль", "Текущий текст");
  await command("card.create", { space, column, card: trash, title: "Прежняя корзина", description: "Сохранить текст" });
  await command("card.trash", { space, card: trash, expected: (await snapshot(trash)).version });
  await command("tag.create", { space, tag, name: "Важное" });
  await command("card.set_tag", { space, card, tag, present: true, expected: (await snapshot(card)).version, expected_tag: (await command("tag.read", { space, tag })).version });
  await command("card.set_completed", { space, card, completed: true, expected: (await snapshot(card)).version });
  await page.evaluate(space => { location.hash = `#/spaces/${space}`; }, space);
  await page.waitForFunction(() => document.querySelector(".board-heading h2")?.textContent === "Очистка истории"); await settled();
  const original = await all(), prior = await history(card), trashPrior = await history(trash);
  assert.equal(prior.text_bytes, prior.entries.reduce((n, e) => n + Buffer.byteLength(e.version.title + e.version.description), 0));
  await openCard(); await verifyPreview(prior);
  await click(".history-cleanup-list li:first-child summary");
  assert.ok((await text(".history-pair")).includes(prior.entries[0].version.description));
  assert.equal(await page.locator(".history-scope b, .history-scope img, .history-scope a").count(), 0);
  await back(); await closed();
  assert.deepEqual(await all(), original); assert.deepEqual(await history(card), prior);

  // The real core rejects a stale scope without clearing any version.
  await openCard(); await edit("После просмотра", "Не очищать по прежнему составу");
  const late = await history(card), lateCards = await all();
  await click(".history-confirmation input"); await click(submit);
  await page.waitForFunction(() => document.querySelector('[role="dialog"] [role="alert"]')?.textContent.includes("Данные изменились"));
  assert.equal(await disabled(submit), true);
  assert.equal(await page.locator("[data-history-entry]").count(), prior.entries.length, "the original scope stays visible");
  assert.deepEqual(await history(card), late); assert.deepEqual(await all(), lateCards);
  await abandon(); await openCard(); await verifyPreview(late); await confirm();
  assert.equal((await history(card)).entries.length, 0);
  assert.deepEqual(await all(), lateCards); assert.deepEqual(await history(trash), trashPrior);
  await openCard(); await verifyPreview(await history(card));
  assert.ok((await text(".history-scope .empty-view")).includes("Сохранённых версий"));
  assert.equal(await disabled(".history-confirmation input"), true);
  await back(); await closed();
  await edit("После очистки", "Новая версия");
  assert.ok((await history(card)).entries[0].version.sequence > late.entries.at(-1).version.sequence);
  await openSpace(); const spaceBefore = await history(null);
  await edit("После просмотра пространства", "Новая область");
  const spaceLate = await history(null);
  await click(".history-confirmation input"); await click(submit);
  await page.waitForFunction(() => document.querySelector('[role="dialog"] [role="alert"]')?.textContent.includes("Данные изменились"));
  assert.equal(await page.locator("[data-history-entry]").count(), spaceBefore.entries.length);
  assert.deepEqual(await history(null), spaceLate); await abandon();

  // Cleanup does not finish another client's in-progress editing series.
  const session = random();
  // begin_editing's request id is its session id.
  await command("card.begin_editing", { space, card, expected: (await snapshot(card)).version }, session);
  await command("card.autosave", { space, card, session, expected: (await snapshot(card)).version, title: "Продолжающийся ввод", description: "Будущая версия" });
  const pendingFields = await all();
  await openSpace(); const scope = await history(null); await verifyPreview(scope);
  assert.ok(scope.entries.some(e => e.card === trash), "space scope includes old trash");
  if (process.env.UI_SCREENSHOT_DIRECTORY) await page.screenshot({ path: `${process.env.UI_SCREENSHOT_DIRECTORY}/history-cleanup-${tap ? "touch" : "desktop"}.png`, fullPage: true });
  await confirm();
  assert.deepEqual(await all(), pendingFields); assert.equal((await history(null)).entries.length, 0);
  await command("card.finish_editing", { space, card, session, expected: (await snapshot(card)).version });
  const finished = await history(card);
  assert.equal(finished.entries.length, 1); assert.equal(finished.entries[0].version.description, "Будущая версия");

  if (!await page.evaluate(() => !!window.__TAURI_INTERNALS__)) {
    const pattern = "**/v1/commands/history.inspect";
    await edit("Ещё одна версия", "Сохраняется рядом с конфликтом");
    const preview = await history(null);
    // A contract-valid retained projection tests presentation, not CRDT sync.
    const retained = structuredClone(preview); retained.entries[0].retained = true;
    retained.removable_bytes -= Buffer.byteLength(retained.entries[0].version.title + retained.entries[0].version.description);
    await page.route(pattern, route => route.fulfill({ json: { status: "ok", result: retained, replayed: false } }));
    await openSpace(); await verifyPreview(retained);
    assert.equal(await disabled(".history-confirmation input"), false);
    await back(); await closed();
    retained.entries.forEach(e => { e.retained = true; }); retained.removable_bytes = 0;
    await openSpace(); await verifyPreview(retained);
    assert.equal(await disabled(".history-confirmation input"), true);
    assert.ok((await text(".history-cleanup-list")).includes("Сохраняется"));
    await back(); await closed(); await page.unroute(pattern);
    await click(`.card-note[data-card="${card}"] .card-title`); await click("[data-history-open]");
    await page.waitForFunction(() => !!document.querySelector(".history-list") && !document.querySelector('.card-history [role="status"]'));
    await page.route(pattern, route => route.fulfill({ json: { malformed: true } }));
    await click("[data-history-cleanup]");
    assert.ok((await text('[role="dialog"] [role="alert"]')).includes("Действие не отправлено"));
    assert.equal(await page.locator(".history-list").count(), 1, "failed cleanup inspection keeps the card history accessible");
    await page.unroute(pattern); await back(); await closed();
    await click(".view-toolbar .view-actions button:nth-child(2)");
    // Closing the existing card while inspect is in flight must not reopen it.
    await click(`.card-note[data-card="${card}"] .card-title`); await click("[data-history-open]");
    await page.waitForFunction(() => !!document.querySelector(".history-list") && !document.querySelector('.card-history [role="status"]'));
    let release, admit, finish;
    let started = false;
    const bounded = async (promise, label) => {
      let timer;
      try { return await Promise.race([promise, new Promise((_, reject) => { timer = setTimeout(() => reject(new Error(`cleanup inspection ${label} deadline`)), 10000); })]); }
      finally { clearTimeout(timer); }
    };
    const gate = new Promise(resolve => { release = resolve; });
    const admitted = new Promise(resolve => { admit = resolve; });
    const finishedRead = new Promise(resolve => { finish = resolve; });
    await page.route(pattern, async route => { started = true; try { const response = await route.fetch({ timeout: 10000 }); admit(); await gate; await route.fulfill({ response }); } finally { finish(); } });
    try {
      if (tap) await tap("[data-history-cleanup]"); else await page.locator("[data-history-cleanup]").click();
      await bounded(admitted, "admission");
      await back(); await closed(); release(); await bounded(finishedRead, "delivery"); await settled();
      assert.equal(await page.locator('[role="dialog"]').count(), 0);
      assert.deepEqual(await history(null), preview);
    } finally { release(); if (started) await bounded(finishedRead, "cleanup"); await page.unroute(pattern); }
    for (const response of [{ status: "error", error: { code: "access_denied" } }, { malformed: true }]) {
      await page.route(pattern, route => route.fulfill({ json: response }));
      await openSpace();
      assert.equal(await page.locator('[role="dialog"]').count(), 0);
      assert.equal(await page.locator(".pending-list li").count(), 0);
      assert.ok(await text('.spaces-view > [role="alert"]'));
      await page.unroute(pattern); await click(".view-toolbar .view-actions button:nth-child(2)");
    }
    let sent = 0;
    await page.route("**/v1/commands/history.clear", route => { sent++; return route.continue(); });
    await openSpace(); await click(".history-confirmation input");
    await page.evaluate(() => { window.cleanupSetItem = Storage.prototype.setItem; Storage.prototype.setItem = () => { throw new DOMException("quota", "QuotaExceededError"); }; });
    await click(submit);
    assert.ok((await text('[role="dialog"] [role="alert"]')).includes("Запрос не отправлен")); assert.equal(sent, 0);
    await page.evaluate(() => { Storage.prototype.setItem = window.cleanupSetItem; delete window.cleanupSetItem; });
    await back(); await closed(); await page.unroute("**/v1/commands/history.clear");
    const preserved = await history(null), fields = await all();
    const denied = [];
    await page.route("**/v1/commands/history.clear", route => { denied.push(route.request().postData()); return route.continue({ headers: { ...route.request().headers(), authorization: `Bearer ${"0".repeat(64)}` } }); });
    await openSpace(); await click(".history-confirmation input"); await click(submit);
    assert.ok((await text('[role="dialog"] [role="alert"]')).includes("Нет доступа")); assert.equal(denied.length, 1);
    assert.deepEqual(await history(null), preserved); assert.deepEqual(await all(), fields);
    await page.unroute("**/v1/commands/history.clear"); await abandon();
    const bodies = [];
    await page.route("**/v1/commands/history.clear", async route => { bodies.push(route.request().postData()); assert.equal((await (await route.fetch()).json()).status, "ok"); await route.abort(); });
    await openSpace(); await click(".history-confirmation input"); await click(submit);
    assert.ok((await text('[role="dialog"] [role="alert"]')).includes("Результат неизвестен"));
    assert.equal(bodies.length, 3); assert.equal(new Set(bodies).size, 1);
    await edit("После потерянного ответа", "Новая история должна сохраниться");
    const newer = await history(null), newerFields = await all();
    const pending = await page.evaluate(() => Object.fromEntries(Object.entries(localStorage).filter(([key]) => key.includes("pending"))));
    await page.reload(); await page.waitForFunction(() => !!document.querySelector(".pending-list li"));
    assert.deepEqual(await page.evaluate(() => Object.fromEntries(Object.entries(localStorage).filter(([key]) => key.includes("pending")))), pending);
    await page.unroute("**/v1/commands/history.clear");
    let replay;
    await page.route("**/v1/commands/history.clear", route => { replay = route.request().postData(); return route.continue(); });
    await click(".pending-list li button:first-child"); await page.waitForFunction(() => !document.querySelector(".pending-list li"));
    assert.equal(replay, bodies[0]); assert.deepEqual(await history(null), newer); assert.deepEqual(await all(), newerFields);
    await page.unroute("**/v1/commands/history.clear");
  }
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  console.log("History cleanup: exact scope/UTF-8 size/cancel, card/space/trash, stale, preservation, pending series, retained/empty, host late inspect/persistence/access/read failures and original replay passed.");
}
