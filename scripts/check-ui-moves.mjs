import assert from "node:assert/strict";

/** The same real Service/API on host and native WebView. No Vue state is changed by tests. */
export async function checkBoardMoves(page, { tap, back = () => page.keyboard.press("Escape") } = {}) {
  const click = tap || (selector => page.locator(selector).click());
  const command = (operation, parameters) => page.evaluate(async ({ operation, parameters }) => {
    const request = JSON.stringify({ id: Array.from(crypto.getRandomValues(new Uint8Array(16))), operation, parameters });
    const native = window.__TAURI_INTERNALS__;
    const session = native ? await native.invoke("shell_session") : await (await fetch("/host/session")).json();
    const response = native
      ? JSON.parse(await native.invoke("zedone_command", { invocation: { operation, request }, credential: session.credential.key }))
      : await (await fetch(`/v1/commands/${operation}`, { method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${session.credential.key}` }, body: request })).json();
    if (response.status !== "ok") throw new Error(`${operation}: ${response.error?.code}`);
    return response.result;
  }, { operation, parameters });
  const random = () => crypto.randomUUID().replaceAll("-", "");
  const space = random(), a = random(), b = random(), c = random(), first = random(), second = random();
  const panel = id => `.column-panel[data-column="${id}"]`;
  const card = id => `.card-note[data-card="${id}"]`;
  const current = id => command("card.read", { space, card: id });
  const history = () => command("card.history", { space, card: first, after: null, limit: 100, expected: null });
  const columns = () => command("column.list", { space, include_trashed: false });
  const cards = () => command("card.list", { space, include_trashed: false });
  async function waitOrder(ids) {
    await page.waitForFunction(ids => [...document.querySelectorAll(".column-panel")].map(item => item.dataset.column).join() === ids.join(), ids);
  }
  async function waitCards(column, ids) {
    await page.waitForFunction(({ column, ids }) => [...document.querySelectorAll(`.column-panel[data-column="${column}"] .card-note`)].map(item => item.dataset.card).join() === ids.join(), { column, ids });
  }
  const refresh = async () => { await click(".view-toolbar button:nth-of-type(2)"); await page.waitForFunction(() => document.querySelector('.spaces-view')?.getAttribute('aria-busy') === 'false'); };
  async function open(selector) { await click(selector); await page.waitForFunction(() => !!document.querySelector(".move-dialog")); }
  async function submit() { await click(".move-dialog button[type=submit]"); await page.waitForFunction(() => !document.querySelector('.move-dialog')); }
  async function forget() {
    await click(".pending-list button:last-child"); await click(".action-dialog button[type=submit]");
    await page.waitForFunction(() => !document.querySelector('[role=dialog]'));
    await refresh();
  }
  const preserved = state => [state.card, state.titles, state.descriptions, state.tags, state.completed, state.state];
  await command("space.create", { space, name: "Перемещения" });
  for (const [column, name] of [[a, "Мысли"], [b, "Планы"], [c, "Пустая"]]) await command("column.create", { space, column, name });
  await command("card.create", { space, column: a, card: first, title: "Первая мысль", description: "Текст 😀\nВторая строка" });
  await command("card.create", { space, column: a, card: second, title: "Вторая мысль", description: "" });
  let state = await current(first);
  const tag = random();
  const createdTag = await command("tag.create", { space, tag, name: "Работа 😀" });
  await command("card.set_tag", { space, card: first, tag, present: true, expected: state.version, expected_tag: createdTag.version });
  state = await current(first);
  await command("card.set_completed", { space, card: first, expected: state.version, completed: true });
  const base = await current(first), versions = await history();
  await page.evaluate(space => { location.hash = `#/spaces/${space}`; }, space);
  await waitOrder([a, b, c]); await waitCards(a, [first, second]);
  const boardUrl = page.url();
  await open(`${card(first)} .move-card`); await back();
  await page.waitForFunction(() => !document.querySelector(".move-dialog"));
  assert.equal(page.url(), boardUrl, "Back closes the move dialog before leaving the board");
  assert.deepEqual(await current(first), base, "cancel sends no move");
  console.log("Moves: explicit card/column order and inverse through the shared API");
  await open(`${card(first)} .move-card`); await submit();
  await waitCards(a, [second, first]);
  assert.deepEqual((await cards()).cards.map(item => item.card), [second, first]);
  await open(`${card(second)} .move-card`); await submit(); await waitCards(a, [first, second]);
  await open(`${panel(a)} .move-column`); await submit(); await waitOrder([b, c, a]);
  assert.deepEqual((await columns()).columns.map(item => item.column), [b, c, a]);
  await open(`${panel(b)} .move-column`); await submit();
  await open(`${panel(c)} .move-column`); await submit(); await waitOrder([a, b, c]);
  assert.deepEqual(preserved(await current(first)), preserved(base));
  assert.deepEqual(await history(), versions, "move writes no text-history versions");

  if (!tap) {
    // Keyboard path and cancellation preserve focus, version and order.
    await page.locator(`${card(first)} .move-card`).focus(); await page.keyboard.press("Enter");
    await page.locator(".move-dialog").waitFor(); await page.keyboard.press("Escape");
    assert.equal(await page.locator(`${card(first)} .move-card`).evaluate(el => el === document.activeElement), true);
    const cancelled = await current(first);
    await open(`${card(first)} .move-card`);
    await page.locator(".move-dialog select[name=column]").selectOption(b);
    await submit(); await waitCards(b, [first]); await waitCards(a, [second]);
    await open(`${card(first)} .move-card`);
    await page.locator(".move-dialog select[name=column]").selectOption(a);
    await page.locator(".move-dialog select[name=before]").selectOption(second);
    await submit(); await waitCards(a, [first, second]);
    assert.notDeepEqual((await current(first)).version, cancelled.version);

    console.log("Moves: actual mouse drags within/between lists, empty column and column order");
    async function drag(source, destination, xFraction = .5, yFraction = .5) {
      await page.locator(source).evaluate(el => el.scrollIntoView({ block: "center", inline: "center" }));
      const from = await page.locator(source).boundingBox(), to = await page.locator(destination).boundingBox();
      assert.ok(from && to);
      await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2); await page.mouse.down();
      await page.mouse.move(from.x + from.width / 2 + 8, from.y + from.height / 2 + 8, { steps: 3 });
      await page.locator(".sortable-fallback").waitFor();
      const end = { x: to.x + to.width * xFraction, y: to.y + to.height * yFraction };
      for (let step = 1; step <= 12; step++) {
        await page.mouse.move(from.x + from.width / 2 + (end.x - from.x - from.width / 2) * step / 12,
          from.y + from.height / 2 + (end.y - from.y - from.height / 2) * step / 12);
        await page.waitForTimeout(25);
      }
      await page.waitForTimeout(60); await page.mouse.move(end.x + 1, end.y);
      await page.waitForTimeout(160); await page.mouse.up();
      await page.waitForFunction(() => document.querySelector('.spaces-view')?.getAttribute('aria-busy') === 'false');
    }
    await drag(`${card(first)} .card-handle`, `${panel(b)} .cards-list`);
    await waitCards(b, [first]); await waitCards(a, [second]);
    await drag(`${card(first)} .card-handle`, card(second), .5, .15);
    await waitCards(a, [first, second]);
    await drag(`${card(first)} .card-handle`, `${panel(c)} .cards-list`); await waitCards(c, [first]);
    await open(`${card(first)} .move-card`);
    await page.locator(".move-dialog select[name=column]").selectOption(a);
    await page.locator(".move-dialog select[name=before]").selectOption(second); await submit(); await waitCards(a, [first, second]);
    await drag(`${panel(a)} .column-handle`, panel(c), .95, .3); await waitOrder([b, c, a]);
    await open(`${panel(a)} .move-column`); await page.locator(".move-dialog select[name=before]").selectOption(b); await submit(); await waitOrder([a, b, c]);

    console.log("Moves: stale original version, durable exact replay and denied/local-storage failures");
    await open(`${card(first)} .move-card`);
    state = await current(first);
    await command("card.set_completed", { space, card: first, expected: state.version, completed: false });
    await click(".move-dialog button[type=submit]");
    await page.locator(".move-dialog [role=alert]").filter({ hasText: "Данные изменились" }).waitFor();
    await page.keyboard.press("Escape"); await waitCards(a, [first, second]);
    assert.deepEqual((await current(first)).completed, [false]); await forget();
    state = await current(first);
    await command("card.set_completed", { space, card: first, expected: state.version, completed: true }); await refresh();

    const bodies = [];
    await page.route("**/v1/commands/card.move", async route => {
      bodies.push(route.request().postData()); await route.fetch(); await route.abort("failed");
    });
    await open(`${card(first)} .move-card`); await click(".move-dialog button[type=submit]");
    await page.locator(".move-dialog [role=alert]").filter({ hasText: "не подтверждено" }).waitFor();
    assert.equal(bodies.length, 3); assert.equal(new Set(bodies).size, 1);
    const saved = await page.evaluate(() => Object.fromEntries(Object.entries(localStorage).filter(([key]) => key.startsWith("zedone.pending."))));
    await page.unroute("**/v1/commands/card.move");
    await page.reload(); await waitCards(a, [second, first]);
    assert.deepEqual(await page.evaluate(() => Object.fromEntries(Object.entries(localStorage).filter(([key]) => key.startsWith("zedone.pending.")))), saved);
    state = await current(first);
    // A later reverse move by the agent must survive replay of the earlier receipt.
    await command("card.move", { space, card: first, column: a, before: second, expected: state.version });
    let replay;
    await page.route("**/v1/commands/card.move", async route => { replay = route.request().postData(); await route.continue(); });
    await click(".pending-list button:first-child"); await waitCards(a, [first, second]);
    assert.equal(replay, bodies[0]); await page.unroute("**/v1/commands/card.move");
    assert.equal(await page.locator(".pending-panel").count(), 0);

    await page.route("**/v1/commands/column.move", route => route.fulfill({ contentType: "application/json", body: JSON.stringify({ status: "error", error: { code: "access_denied" } }) }));
    await open(`${panel(a)} .move-column`); await click(".move-dialog button[type=submit]");
    await page.locator(".move-dialog [role=alert]").filter({ hasText: "Нет доступа" }).waitFor();
    await page.keyboard.press("Escape"); await waitOrder([a, b, c]);
    await page.unroute("**/v1/commands/column.move"); await forget();

    await open(`${card(first)} .move-card`);
    await page.evaluate(() => { window.__moveSet = Storage.prototype.setItem; Storage.prototype.setItem = function(key, value) { if (key.startsWith("zedone.pending.")) throw new DOMException("quota", "QuotaExceededError"); return window.__moveSet.call(this, key, value); }; });
    await click(".move-dialog button[type=submit]");
    await page.locator(".move-dialog [role=alert]").filter({ hasText: "Запрос не отправлен" }).waitFor();
    await page.evaluate(() => { Storage.prototype.setItem = window.__moveSet; delete window.__moveSet; });
    await page.keyboard.press("Escape"); await waitCards(a, [first, second]);
    assert.deepEqual(preserved(await current(first)), preserved(base));
    assert.deepEqual(await history(), versions);
    await page.reload(); await waitOrder([a, b, c]); await waitCards(a, [first, second]);
    if (process.env.UI_SCREENSHOT_DIRECTORY) await page.screenshot({ path: `${process.env.UI_SCREENSHOT_DIRECTORY}/moves-desktop.png` });
  }
  console.log("Moves: explicit inverse/order, immutable content/history and persisted authoritative board passed.");
}
