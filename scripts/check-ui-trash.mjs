import assert from "node:assert/strict";

/** Shared real Service/HTTP/IPC matrix; touch uses the native controls driver. */
export async function checkTrash(page, { tap, back = () => page.keyboard.press("Escape") } = {}) {
  const click = async selector => {
    if (tap) await tap(selector); else await page.locator(selector).click();
    await page.waitForFunction(() => document.querySelector('.spaces-view')?.getAttribute('aria-busy') === 'false');
  };
  const api = (operation, parameters) => page.evaluate(async ({ operation, parameters }) => {
    const native = window.__TAURI_INTERNALS__;
    const session = native ? await native.invoke("shell_session") : await (await fetch("/host/session")).json();
    const request = JSON.stringify({ id: Array.from(crypto.getRandomValues(new Uint8Array(16))), operation, parameters });
    return native ? JSON.parse(await native.invoke("zedone_command", { invocation: { operation, request }, credential: session.credential.key }))
      : await (await fetch(`/v1/commands/${operation}`, { method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${session.credential.key}` }, body: request })).json();
  }, { operation, parameters });
  const command = async (operation, parameters) => {
    const response = await api(operation, parameters);
    assert.equal(response.status, "ok", `${operation}: ${response.error?.code}`); return response.result;
  };
  const random = () => crypto.randomUUID().replaceAll("-", "");
  const space = random(), first = random(), second = random(), oldColumn = random();
  const one = random(), two = random(), early = random(), old = random(), tag = random();
  const name = `Корзина ${random()}`;
  const snapshot = (operation, id) => command(`${operation}.read`, { space, [operation]: id });
  const life = async (kind, id, action) => command(`${kind}.${action}`, { space, [kind]: id, expected: (await snapshot(kind, id)).version });
  const allCards = () => command("card.list", { space, include_trashed: true });
  const history = card => command("card.history", { space, card, after: null, limit: 100, expected: null });
  const count = selector => page.evaluate(s => document.querySelectorAll(s).length, selector);
  const text = selector => page.evaluate(s => document.querySelector(s)?.textContent ?? "", selector);
  const disabled = selector => page.evaluate(s => document.querySelector(s)?.disabled, selector);
  const stale = () => page.waitForFunction(() => document.querySelector('[role="dialog"] [role="alert"]')?.textContent.includes("Данные изменились"));
  const dialog = '[role="dialog"]';
  const submit = `${dialog} button[type="submit"]`;
  const rootRow = `.space-row[data-space="${space}"]`;
  const columnButton = id => `.column-panel[data-column="${id}"] .column-actions button:last-child`;
  const trashCard = id => `.trash-card[data-trash-card="${id}"]`;
  const closed = () => page.waitForFunction(() => !document.querySelector('[role="dialog"]'));
  const settled = () => page.waitForFunction(() => document.querySelector('.spaces-view')?.getAttribute('aria-busy') === 'false');
  const confirm = async selector => { await click(selector); await click(submit); await closed(); await settled(); };
  const go = async id => {
    await page.evaluate(id => { location.hash = id ? `/spaces/${id}` : "/spaces"; }, id);
    await page.waitForFunction(id => id ? document.querySelector('.board-heading h2')?.textContent === id : !document.querySelector('.board-heading'), id ? name : null);
    await settled();
  };
  const abandon = async () => {
    await click('[data-workspace-dialog-close]'); await closed();
    await click(".pending-list li button:last-child"); await click(submit); await closed();
    await click(".view-toolbar .view-actions button:nth-child(2)"); await settled();
  };
  await command("space.create", { space, name });
  for (const [column, title] of [[first, "Первая"], [second, "Вторая"], [oldColumn, "Прежняя корзина"]]) await command("column.create", { space, column, name: title });
  for (const [column, card, title] of [[first, one, "Мысль один"], [first, two, "Мысль два"], [first, early, "Удалена раньше"], [oldColumn, old, "Старая колонка"]]) {
    await command("card.create", { space, column, card, title, description: "<b>Буквально</b>\nВторая строка" });
  }
  await command("tag.create", { space, tag, name: "Сохранить" });
  await command("card.set_tag", { space, card: one, tag, present: true, expected: (await snapshot("card", one)).version, expected_tag: (await snapshot("tag", tag)).version });
  await command("card.set_completed", { space, card: one, expected: (await snapshot("card", one)).version, completed: true });
  await life("card", early, "trash"); await life("column", oldColumn, "trash");
  const original = await allCards(), priorHistory = await history(one);
  await go(space);

  // A complete scope is shown before the group action; cancel writes nothing.
  await click(columnButton(first));
  assert.equal(await count(".trash-scope [data-scope-card]"), 3);
  assert.ok((await text(`[data-scope-card="${early}"]`)).includes("Останется в прежней корзине"));
  assert.equal(await count(".trash-scope b, .trash-scope img"), 0);
  await back(); await closed();
  assert.equal(await page.evaluate(s => document.querySelector(s) === document.activeElement, columnButton(first)), true);
  assert.deepEqual(await allCards(), original);

  // A later edit changes the group token: no card or column is partly removed.
  await click(columnButton(first));
  await command("card.edit", { space, card: one, expected: (await snapshot("card", one)).version, title: "Мысль один", description: "Поздняя правка агента" });
  await click(submit); await stale();
  assert.equal((await snapshot("column", first)).state, "active");
  assert.deepEqual((await allCards()).cards.map(item => item.state), original.cards.map(item => item.state));
  const lateHistory = await history(one);
  await abandon(); await confirm(columnButton(first));
  assert.equal((await snapshot("card", one)).state, "trashed");
  const earlyGroup = (await snapshot("card", early)).deletion_groups;
  await click('.view-toolbar button[aria-pressed="false"]');

  // Restore only one card to another column while the original remains trashed.
  await click(`${trashCard(one)} .view-actions button:last-child`);
  assert.equal(await count(`${dialog} input[name="restore-column"]:checked`), 0);
  await click(`${dialog} input[name="restore-column"][value="${second}"]`);
  await command("column.rename", { space, column: second, expected: (await snapshot("column", second)).version, name: "Новое место" });
  await click(submit); await stale();
  assert.equal((await snapshot("card", one)).state, "trashed", "stale destination never restores a card");
  await abandon();
  await click(`${trashCard(one)} .view-actions button:last-child`);
  await click(`${dialog} input[name="restore-column"][value="${second}"]`);
  await click(submit); await closed(); await settled();
  assert.deepEqual((await snapshot("card", one)).columns, [second]);
  assert.equal((await snapshot("column", first)).state, "trashed");
  await confirm(columnButton(first));
  assert.equal((await snapshot("card", two)).state, "active");
  assert.equal((await snapshot("card", early)).state, "trashed");
  assert.deepEqual((await snapshot("card", early)).deletion_groups, earlyGroup);
  assert.deepEqual(await history(one), lateHistory);

  if (!tap) {
    const bodies = [];
    await click('.view-toolbar button[aria-pressed="true"]');
    await page.route("**/v1/commands/column.trash", async route => {
      bodies.push(route.request().postData()); assert.equal((await (await route.fetch()).json()).status, "ok"); await route.abort();
    });
    await click(columnButton(first)); await click(submit);
    await page.locator(`${dialog} [role="alert"]`).filter({ hasText: "Результат неизвестен" }).waitFor();
    assert.equal(bodies.length, 3); assert.equal(new Set(bodies).size, 1);
    await life("column", first, "restore");
    const restored = await allCards();
    await page.reload(); await page.locator(".pending-list li button:first-child").waitFor();
    await page.unroute("**/v1/commands/column.trash"); await click(".pending-list li button:first-child");
    assert.deepEqual(await allCards(), restored, "old group receipt never repeats trash after restore");
    await click('.view-toolbar button[aria-pressed="false"]');
  }

  // The space scope preserves both prior trash groups, tags and all card data.
  await go();
  await click(`${rootRow} .view-actions button:last-child`);
  assert.equal(await count(".trash-scope [data-scope-column]"), 3);
  await command("tag.rename", { space, tag, expected: (await snapshot("tag", tag)).version, name: "Сохранить позже" });
  await click(submit); await stale();
  assert.equal((await snapshot("space", space)).state, "active");
  await abandon();
  const beforeRoot = await allCards();
  await confirm(`${rootRow} .view-actions button:last-child`);
  await click('.view-toolbar button[aria-pressed="false"]');
  await click(`${rootRow} a`); await settled();
  await click('.view-toolbar button[aria-pressed="false"]');
  assert.equal(await disabled(`${trashCard(one)} .view-actions button:last-child`), true, "a card cannot partly restore a trashed space");
  await confirm('.trash-panel > .view-actions button:first-child');
  assert.deepEqual((await allCards()).cards.map(({ version, ...item }) => item), beforeRoot.cards.map(({ version, ...item }) => item));
  assert.deepEqual(await history(one), lateHistory);
  assert.ok(lateHistory.versions.length >= priorHistory.versions.length);

  // Selected column purge includes its older trash, but excludes the moved card.
  await life("column", first, "trash");
  await click(".view-toolbar .view-actions button:nth-child(2)"); await settled();
  await click(`.trash-column[data-column="${first}"] input`);
  await click(`${trashCard(two)} input`);
  await click(".purge-selected");
  assert.deepEqual((await page.evaluate(() => [...document.querySelectorAll("[data-scope-card]")].map(item => item.dataset.scopeCard))).sort(), [two, early].sort());
  assert.equal(await disabled(submit), true);
  await back(); await closed();
  assert.equal((await snapshot("column", first)).state, "trashed");
  await click(".purge-selected");
  // Restoring an included member invalidates the frozen purge scope.
  await command("card.restore", { space, card: early, expected: (await snapshot("card", early)).version, column: second, expected_column: (await snapshot("column", second)).version });
  await click(".purge-confirmation input"); await click(submit);
  await stale();
  assert.equal((await snapshot("card", two)).state, "trashed");
  await abandon();
  await click(`.trash-column[data-column="${first}"] input`); await click(".purge-selected");
  const finalPreview = await command("trash.inspect", { space, selection: { kind: "objects", columns: [first], cards: [] } });
  assert.deepEqual(finalPreview.cards.map(item => item.card), [two]);
  const kept = await snapshot("card", one), keptHistory = await history(one);
  await click(".purge-confirmation input"); await click(submit); await closed(); await settled();
  assert.equal((await api("column.read", { space, column: first })).error.code, "not_found");
  assert.equal((await api("card.read", { space, card: two })).error.code, "not_found");
  assert.deepEqual(await snapshot("card", one), kept); assert.deepEqual(await history(one), keptHistory);
  assert.equal((await snapshot("card", early)).state, "active");

  if (!tap) {
    // Unknown container contents must never be mistaken for an empty scope.
    await click('.view-toolbar button[aria-pressed="true"]');
    const unknownColumn = await snapshot("column", second);
    unknownColumn.contents = null;
    await page.route("**/v1/commands/column.read", route => route.fulfill({ json: { status: "ok", result: unknownColumn, replayed: false } }));
    await click(columnButton(second));
    assert.equal(await disabled(submit), true);
    assert.ok((await text(`${dialog} [role="alert"]`)).includes("Не удалось определить весь состав"));
    await back(); await closed(); await page.unroute("**/v1/commands/column.read");
    await click('.view-toolbar button[aria-pressed="false"]');
    // Valid contract conflict snapshots exercise the UI lock, not product sync.
    const preview = await command("trash.inspect", { space, selection: { kind: "all" } });
    const pattern = "**/v1/commands/trash.inspect";
    preview.conflicts = [old];
    await page.route(pattern, route => route.fulfill({ json: { status: "ok", result: preview, replayed: false } }));
    await click(".purge-all");
    assert.ok((await page.locator(`${dialog} [role="alert"]`).textContent()).includes(old));
    assert.equal(await disabled(submit), true);
    await back(); await closed(); await page.unroute(pattern);
    for (const response of [{ status: "error", error: { code: "access_denied" } }, { malformed: true }]) {
      await page.route(pattern, route => route.fulfill({ json: response }));
      await click(".purge-all"); await page.locator('.spaces-view > [role="alert"]').waitFor();
      if (response.malformed) assert.ok((await text('.spaces-view > [role="alert"]')).includes("Действие не отправлено"));
      assert.equal(await page.locator(dialog).count(), 0); assert.equal(await page.locator(".pending-list li").count(), 0);
      await page.unroute(pattern); await click(".view-toolbar .view-actions button:nth-child(2)"); await settled();
    }
    // Quota failure must happen before any destructive request is sent.
    let deliveries = 0;
    await page.route("**/v1/commands/trash.purge", route => { deliveries++; return route.continue(); });
    await click(".purge-all"); await click(".purge-confirmation input");
    await page.evaluate(() => { window.__trashSetItem = Storage.prototype.setItem; Storage.prototype.setItem = () => { throw new DOMException("quota", "QuotaExceededError"); }; });
    await click(submit); await page.locator(`${dialog} [role="alert"]`).waitFor();
    assert.equal(deliveries, 0);
    await page.evaluate(() => { Storage.prototype.setItem = window.__trashSetItem; delete window.__trashSetItem; });
    await back(); await closed(); await page.unroute("**/v1/commands/trash.purge");
    const denied = [];
    const beforeDenial = await snapshot("column", oldColumn);
    await page.route("**/v1/commands/trash.purge", route => {
      denied.push(route.request().postData());
      return route.continue({ headers: { ...route.request().headers(), authorization: `Bearer ${"0".repeat(64)}` } });
    });
    await click(".purge-all"); await click(".purge-confirmation input"); await click(submit);
    await page.locator(`${dialog} [role="alert"]`).filter({ hasText: "Нет доступа к этому действию" }).waitFor();
    assert.equal(denied.length, 1, "an explicit access denial is never retried automatically");
    assert.deepEqual(await snapshot("column", oldColumn), beforeDenial);
    await page.unroute("**/v1/commands/trash.purge"); await abandon();
    // Unknown result persists the exact purge request through reload. A newly
    // trashed card after that commit must survive the old receipt's replay.
    const bodies = [];
    await page.route("**/v1/commands/trash.purge", async route => {
      bodies.push(route.request().postData()); assert.equal((await (await route.fetch()).json()).status, "ok"); await route.abort();
    });
    await click(".purge-all"); await click(".purge-confirmation input"); await click(submit);
    await page.locator(`${dialog} [role="alert"]`).filter({ hasText: "Результат неизвестен" }).waitFor();
    assert.equal(bodies.length, 3); assert.equal(new Set(bodies).size, 1);
    const pending = await page.evaluate(() => Object.fromEntries(Object.entries(localStorage).filter(([key]) => key.includes("pending"))));
    await life("card", one, "trash");
    const later = await snapshot("card", one);
    await page.reload(); await page.locator(".pending-list li button:first-child").waitFor();
    assert.deepEqual(await page.evaluate(() => Object.fromEntries(Object.entries(localStorage).filter(([key]) => key.includes("pending")))), pending);
    await page.unroute("**/v1/commands/trash.purge"); await click(".pending-list li button:first-child"); await settled();
    assert.deepEqual(await snapshot("card", one), later);
    assert.equal(await page.locator(".pending-list li").count(), 0);
  }

  // Whole-space cleanup has its own scope and returns to the current list.
  await go(); await confirm(`${rootRow} .view-actions button:last-child`);
  await click('.view-toolbar button[aria-pressed="false"]'); await click(`${rootRow} a`); await settled();
  await click('.view-toolbar button[aria-pressed="false"]');
  await click(".purge-space");
  assert.ok((await text(dialog)).includes("Будет очищено само пространство"));
  if (process.env.UI_SCREENSHOT_DIRECTORY) await page.screenshot({ path: `${process.env.UI_SCREENSHOT_DIRECTORY}/trash-${tap ? "touch" : "desktop"}.png`, fullPage: true });
  await click(".purge-confirmation input"); await click(submit); await closed(); await settled();
  assert.equal(await page.evaluate(() => location.hash), "#/spaces");
  assert.equal((await api("space.read", { space })).error.code, "not_found");
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  console.log("Trash: complete groups/previous trash, selected restoration, stale/no partial effects, explicit purge/scopes/cancel/conflicts/failures/replay, data/history preservation passed.");
}
