import assert from "node:assert/strict";

/** Real Service data; shared desktop, host touch and Android CI scenario. */
export async function checkSearch(page, { tap, fill, back: platformBack = () => page.keyboard.press("Escape"), dismissKeyboard = async () => {} } = {}) {
  const click = tap || (selector => page.locator(selector).click());
  const input = fill || ((selector, value) => page.locator(selector).fill(value));
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
  const a = random(), b = random(), first = random(), second = random(), other = random();
  const one = random(), two = random(), three = random(), trashed = random(), elsewhere = random(), tag = random();
  const marker = `search-${random()}`;
  for (const [space, name] of [[a, `${marker} Б`], [b, `${marker} А`]]) await command("space.create", { space, name });
  for (const [space, column, name] of [[a, first, "Первая"], [a, second, "Вторая"], [b, other, "Другая"]]) await command("column.create", { space, column, name });
  for (const [space, column, card, title, description] of [
    [a, first, one, `Straße ${marker}`, "Café купить молоко Все C++ <img src=x>\nВторая строка\tтабуляция"],
    [a, first, two, `Всё ${marker}`, "купить  молоко"],
    [a, second, three, `Третья ${marker}`, "не ищем имена и теги"],
    [a, first, trashed, `Корзина ${marker}`, "Straße"],
    [b, other, elsewhere, `Другая ${marker}`, "Straße"],
  ]) await command("card.create", { space, column, card, title, description });
  const current = (space, card) => command("card.read", { space, card });
  await command("card.set_completed", { space: a, card: one, expected: (await current(a, one)).version, completed: true });
  await command("card.trash", { space: a, card: trashed, expected: (await current(a, trashed)).version });
  await command("tag.create", { space: b, tag, name: "Тег другого пространства" });
  await command("card.set_tag", { space: b, card: elsewhere, tag, present: true, expected: (await current(b, elsewhere)).version, expected_tag: (await command("tag.read", { space: b, tag })).version });
  const preserved = await command("card.list", { space: a, include_trashed: true });
  const history = await command("card.history", { space: a, card: one, after: null, limit: 100, expected: null });
  await page.evaluate(space => { location.hash = `/spaces/${space}`; }, a);
  await page.waitForFunction(id => document.querySelector(`.board-view[data-space="${id}"]`) || document.querySelector(`.card-note[data-card="${id}"]`), one);
  await click(".search-toggle");
  const ready = () => page.waitForFunction(() => document.querySelector(".search-results")?.getAttribute("aria-busy") === "false" && /^Найдено/.test(document.querySelector(".search-results [role=status]")?.textContent));
  const ids = () => page.evaluate(() => [...document.querySelectorAll(".search-hits > li")].map(el => el.dataset.card));
  const search = async query => { await input('[name="query"]', query); await dismissKeyboard(); await click(".search-submit"); await ready(); };
  await ready();
  assert.deepEqual(await ids(), [one, two, three]);
  assert.equal(await page.evaluate(() => document.querySelector('input[name="search-scope"][value="space"]').checked), true);
  await search("STRASSE"); assert.deepEqual(await ids(), [one]);
  assert.ok((await page.locator(".search-hits").textContent()).includes("Выполнено"));
  await search("Café"); assert.deepEqual(await ids(), [one]);
  await search("купить молоко"); assert.deepEqual(await ids(), [one]);
  await search("купить  молоко"); assert.deepEqual(await ids(), [two]);
  await search("Все"); assert.deepEqual(await ids(), [one]);
  await search("Всё"); assert.deepEqual(await ids(), [two]);
  await search("C++"); assert.deepEqual(await ids(), [one]);
  await search("<img src=x>\nВторая строка\tтабуляция"); assert.deepEqual(await ids(), [one]);
  assert.equal(await page.locator(".search-hits img").count(), 0, "description is literal, without HTML or external images");
  await search("нет совпадений"); assert.deepEqual(await ids(), []);
  await search("\u2003 \u00a0"); assert.deepEqual(await ids(), [one, two, three]);
  await click('input[name="search-scope"][value="all"]');
  await search(marker); assert.deepEqual(await ids(), [elsewhere, one, two, three]);
  await click(".search-clear"); await ready();
  assert.equal(await page.evaluate(() => document.querySelector('input[name="search-scope"][value="all"]').checked), true);
  assert.ok((await ids()).includes(elsewhere)); assert.ok(!(await ids()).includes(trashed));
  await click('input[name="search-scope"][value="space"]');
  await click(".search-clear"); await ready(); assert.deepEqual(await ids(), [one, two, three]);
  assert.deepEqual(await command("card.list", { space: a, include_trashed: true }), preserved);
  assert.deepEqual(await command("card.history", { space: a, card: one, after: null, limit: 100, expected: null }), history);

  // A result is a snapshot. Opening it reads the current card and its own tag catalogue.
  await click('input[name="search-scope"][value="all"]'); await search(marker);
  await command("card.set_completed", { space: b, card: elsewhere, expected: (await current(b, elsewhere)).version, completed: true });
  await click(`.search-hits li[data-card="${elsewhere}"] .card-title`);
  await page.waitForFunction(() => document.querySelector(".card-editor .set-card-tag"));
  assert.ok((await page.locator(".card-editor").textContent()).includes("Тег другого пространства"));
  assert.equal(await page.evaluate(() => document.querySelector(".set-card-tag")?.getAttribute("aria-pressed")), "true");
  assert.equal(await page.evaluate(() => [...document.querySelectorAll(".card-editor button")].find(button => button.textContent.trim() === "Снять отметку выполнения")?.getAttribute("aria-pressed")), "true");
  await dismissKeyboard(); await platformBack(); await page.waitForFunction(() => !document.querySelector('[role="dialog"]'));
  await click(".search-submit"); await ready();
  assert.ok((await page.locator(`.search-hits li[data-card="${elsewhere}"]`).textContent()).includes("Выполнено"));
  // Existing editor mutations refresh these results, including another space.
  for (const completed of [false, true]) {
    await click(`.search-hits li[data-card="${elsewhere}"] .card-title`);
    await page.waitForFunction(() => document.querySelector(".card-editor > .view-actions > button[aria-pressed]"));
    await click(".card-editor > .view-actions > button[aria-pressed]");
    await page.waitForFunction(value => document.querySelector(".card-editor > .view-actions > button[aria-pressed]")?.getAttribute("aria-pressed") === String(value), completed);
    await dismissKeyboard(); await platformBack();
    await page.waitForFunction(() => !document.querySelector('[role="dialog"]')); await ready();
    assert.equal((await page.locator(`.search-hits li[data-card="${elsewhere}"]`).textContent()).includes("Выполнено"), completed);
    assert.equal(await page.evaluate(() => document.activeElement?.getAttribute("name")), "query");
    await dismissKeyboard();
  }
  if (!tap) {
    const pattern = "**/v1/commands/card.search";
    const requests = [];
    await page.route(pattern, async route => { requests.push(JSON.parse(route.request().postData())); await route.continue(); });
    await search(marker); await click(".search-submit"); await ready();
    assert.equal(requests.length, 2); assert.notDeepEqual(requests[0].id, requests[1].id);
    await page.unroute(pattern);
    for (const response of [{ status: "error", error: { code: "access_denied" } }, { wrong: "contract" }, null]) {
      await page.route(pattern, route => response ? route.fulfill({ json: response }) : route.abort("connectionfailed"));
      await click(".search-submit");
      await page.waitForFunction(() => document.querySelector(".search-panel [role=alert]"));
      assert.deepEqual(await ids(), []); assert.equal(await page.locator('[name="query"]').inputValue(), marker);
      assert.equal(await page.locator(".pending-panel").count(), 0, "failed reads never enter the mutation queue");
      await page.unroute(pattern); await click(".search-submit"); await ready();
    }
    // A late response from an older query must never become visible.
    // Other browser passes share this fixture; scope generic words to this board.
    await click('input[name="search-scope"][value="space"]');
    let release, captured, delivered;
    const intercepted = new Promise(resolve => { captured = resolve; });
    const finished = new Promise(resolve => { delivered = resolve; });
    await page.route(pattern, async route => {
      const request = JSON.parse(route.request().postData());
      if (request.parameters.query !== "STRASSE") return route.continue();
      const response = await route.fetch(); captured();
      await new Promise(resolve => { release = resolve; });
      await route.fulfill({ response });
      delivered();
    });
    await input('[name="query"]', "STRASSE"); await click(".search-submit"); await intercepted;
    await search("Всё"); assert.deepEqual(await ids(), [two]);
    release(); await finished; await page.unroute(pattern);
    assert.deepEqual(await ids(), [two]);
    // Presentation of future conflict snapshots, without pretending sync is implemented.
    const snapshot = await command("card.search", { query: marker, scope: { kind: "all" } });
    snapshot.spaces[0].space.names = ["Альфа", "Бета"];
    snapshot.spaces[0].hits[0].placement_conflict = true;
    snapshot.spaces[0].hits[0].card.titles = ["Первый вариант", "Второй вариант"];
    snapshot.spaces[0].hits[0].card.state = "conflicted";
    await click('input[name="search-scope"][value="all"]');
    await page.route(pattern, route => route.fulfill({ json: { status: "ok", result: snapshot, replayed: false } }));
    await search(marker); assert.deepEqual(await ids(), [elsewhere, one, two, three]);
    assert.equal(await page.locator('.search-results section[aria-label="Конфликты названий"] h3').first().textContent(), "Конфликты названий");
    assert.equal(await page.locator('.search-space section[aria-label="Конфликты"] li').count(), 1);
    assert.ok((await page.locator(".search-hits").first().textContent()).includes("Второй вариант"));
    await page.unroute(pattern); await click(".search-submit"); await ready();
    assert.equal(await page.locator('.search-space section[aria-label="Конфликты"]').count(), 0);
  }
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  if (process.env.UI_SCREENSHOT_DIRECTORY) await page.screenshot({ path: `${process.env.UI_SCREENSHOT_DIRECTORY}/search-${tap ? "touch" : "desktop"}.png`, fullPage: true });
  await click(".search-toggle");
  await page.waitForFunction(() => !document.querySelector(".search-panel"));
  await click(".search-toggle"); await ready();
  await click(`.search-space[data-space="${a}"] h3 a`);
  await page.waitForFunction(() => !document.querySelector(".search-panel") && document.querySelector(".board-heading"));
  assert.equal(await page.evaluate(() => location.hash), `#/spaces/${a}`, "current-space link closes search even without a route change");
  await click(".search-toggle"); await ready();
  await click('input[name="search-scope"][value="all"]'); await search(marker);
  await click(`.search-space[data-space="${b}"] h3 a`);
  await page.waitForFunction(id => location.hash === `#/spaces/${id}` && !document.querySelector(".search-panel") && document.querySelector(".board-heading"), b);
  await page.evaluate(() => { location.hash = "/spaces"; });
  await page.waitForFunction(() => !document.querySelector(".board-heading"));
  console.log("Search: shared API, Unicode/literal text, empty/scope/order/completed/trash, fresh opening and tags, immutable data/history passed.");
}
