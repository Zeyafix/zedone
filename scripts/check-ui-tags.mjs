import assert from "node:assert/strict";

/** Same public commands and UI on host and native WebView; no direct storage writes. */
export async function checkTags(page, { tap, fill, back: platformBack = () => page.keyboard.press("Escape"), dismissKeyboard = async () => {} } = {}) {
  const back = async () => { await dismissKeyboard(); await platformBack(); };
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
  const space = random(), column = random(), card = random(), trashed = random();
  const current = id => command("card.read", { space, card: id });
  const tags = () => command("tag.list", { space, include_deleted: false });
  const tagRead = tag => command("tag.read", { space, tag });
  const row = id => `.space-tags li[data-tag="${id}"]`;
  const assigned = id => `.card-tags li[data-tag="${id}"] .set-card-tag`;
  const dialog = ".action-dialog";
  const submit = () => click(`${dialog} button[type=submit]`);
  const closed = () => page.waitForFunction(() => !document.querySelector('[role="dialog"]'));
  const refresh = async () => {
    await click(".view-toolbar button:nth-of-type(2)");
    await page.waitForFunction(() => document.querySelector(".spaces-view")?.getAttribute("aria-busy") === "false");
    // Locator.evaluate asks Playwright to inspect visibility in its isolated realm;
    // AOSP WebView has no checkVisibility(). This is only a read of native details state.
    if (!await page.evaluate(() => document.querySelector(".space-tags")?.open)) await click(".space-tags summary");
  };
  async function forget() {
    await click(".pending-list button:last-child"); await submit(); await closed(); await refresh();
  }
  async function create(name) {
    await click(".create-tag"); await input(`${dialog} input[name=name]`, name); await submit(); await closed();
    const result = (await tags()).tags.find(tag => tag.names[0] === name);
    assert.ok(result); return result.tag;
  }
  async function rename(id, name) {
    await click(`${row(id)} .rename-tag`); await input(`${dialog} input[name=name]`, name); await submit(); await closed();
    assert.deepEqual((await tagRead(id)).names, [name]);
  }
  async function openCard() { await click(`.card-note[data-card="${card}"] .card-title`); }
  async function toggle(id, present) {
    await click(assigned(id));
    await page.waitForFunction(({ selector, present }) => document.querySelector(selector)?.getAttribute("aria-pressed") === String(present), { selector: assigned(id), present });
  }
  await command("space.create", { space, name: "Теги" });
  await command("column.create", { space, column, name: "Мысли" });
  for (const id of [card, trashed]) await command("card.create", { space, column, card: id, title: id === card ? "Мысль с тегами" : "В корзину", description: "Сохранить **текст** 😀" });
  const state = await current(card);
  await command("card.set_completed", { space, card, expected: state.version, completed: true });
  const before = await current(card);
  const history = await command("card.history", { space, card, after: null, limit: 100, expected: null });
  await page.evaluate(space => { location.hash = `#/spaces/${space}`; }, space);
  await page.waitForFunction(() => !!document.querySelector(".space-tags"));
  await click(".space-tags summary");
  const first = await create("Все"), other = await create("Работа 😀");
  await openCard(); await toggle(first, true); await toggle(other, true); await toggle(first, false); await toggle(first, true);
  assert.deepEqual(new Set((await current(card)).tags), new Set([first, other]));
  await back(); await closed();
  await rename(first, "Всё");
  assert.deepEqual(new Set((await current(card)).tags), new Set([first, other]), "rename preserves IDs and assignments");
  await rename(first, "Все"); await rename(first, "Всё");
  // One comparator in Rust decides occupied names, including Unicode case folding.
  for (const value of ["  РАБОТА 😀  ", "\u2003ВСЁ\u00a0", "   "]) {
    const target = value.includes("РАБОТА") ? first : other;
    const tagBefore = await tagRead(target);
    await click(`${row(target)} .rename-tag`); await input(`${dialog} input[name=name]`, value); await submit();
    await page.waitForFunction(() => !!document.querySelector(".action-dialog [role=alert]"));
    assert.deepEqual(await tagRead(target), tagBefore);
    assert.deepEqual(new Set((await current(card)).tags), new Set([first, other]));
    await back(); await closed(); await forget();
  }
  const unicode = await create("Straße");
  await click(".create-tag"); await input(`${dialog} input[name=name]`, " STRASSE "); await submit();
  await page.waitForFunction(() => document.querySelector(".action-dialog [role=alert]")?.textContent.includes("Имя занято"));
  await back(); await closed(); await forget();
  // Cancellation does not delete; Back closes the form before leaving the board.
  const boardUrl = page.url();
  await click(`${row(first)} .delete-tag`);
  assert.match(await page.locator(`${dialog} > p`).textContent(), /без корзины.*восстановления/);
  await back(); await closed(); assert.equal(page.url(), boardUrl);
  assert.equal((await tagRead(first)).state, "active");
  let tombstone = await current(trashed), tag = await tagRead(first);
  await command("card.set_tag", { space, card: trashed, tag: first, present: true, expected: tombstone.version, expected_tag: tag.version });
  tombstone = await current(trashed);
  await command("card.trash", { space, card: trashed, expected: tombstone.version });
  const trashedBefore = await current(trashed);
  await refresh();
  await click(`${row(first)} .delete-tag`); await submit(); await closed();
  assert.equal((await tagRead(first)).state, "deleted");
  assert.deepEqual((await current(card)).tags, [other]);
  assert.deepEqual((await current(trashed)).tags, []);
  const preserved = ({ titles, descriptions, completed, columns, placements, state, history_total, history_dropped, deletion_groups }) => ({ titles, descriptions, completed, columns, placements, state, history_total, history_dropped, deletion_groups });
  assert.deepEqual(preserved(await current(card)), preserved(before));
  assert.deepEqual(preserved(await current(trashed)), preserved(trashedBefore));
  assert.deepEqual(await command("card.history", { space, card, after: null, limit: 100, expected: null }), history);
  const replacement = await create("Всё"); assert.notEqual(replacement, first);
  assert.deepEqual((await current(card)).tags, [other], "recreation never restores assignments");
  await openCard();
  await page.waitForFunction(id => document.querySelector(`.card-tags li[data-tag="${id}"] .set-card-tag`)?.getAttribute("aria-pressed") === "false", replacement);
  await toggle(replacement, true); await toggle(replacement, false); await back(); await closed();

  if (!tap) {
    // Frozen versions: concurrent agent rename cannot be overwritten by the dialog.
    await click(`${row(other)} .rename-tag`); await input(`${dialog} input[name=name]`, "Мой ввод");
    tag = await tagRead(other);
    await command("tag.rename", { space, tag: other, name: "Правка агента", expected: tag.version });
    await submit(); await page.locator(`${dialog} [role=alert]`).filter({ hasText: "Данные изменились" }).waitFor();
    assert.equal(await page.locator(`${dialog} input`).inputValue(), "Мой ввод");
    assert.deepEqual((await tagRead(other)).names, ["Правка агента"]);
    await back(); await closed(); await forget();
    // Storage failure must prevent sending any request.
    await click(`${row(other)} .rename-tag`);
    await page.evaluate(() => { window.__tagSet = Storage.prototype.setItem; Storage.prototype.setItem = function(key, value) { if (key.startsWith("zedone.pending.")) throw new DOMException("quota", "QuotaExceededError"); return window.__tagSet.call(this, key, value); }; });
    await input(`${dialog} input[name=name]`, "Не отправлять"); await submit();
    await page.locator(`${dialog} [role=alert]`).filter({ hasText: "Запрос не отправлен" }).waitFor();
    await page.evaluate(() => { Storage.prototype.setItem = window.__tagSet; delete window.__tagSet; });
    await back(); await closed(); assert.deepEqual((await tagRead(other)).names, ["Правка агента"]);
    await page.route("**/v1/commands/tag.rename", route => route.fulfill({ json: { status: "error", error: { code: "access_denied" } } }));
    await click(`${row(other)} .rename-tag`); await input(`${dialog} input[name=name]`, "Запрещено"); await submit();
    await page.locator(`${dialog} [role=alert]`).filter({ hasText: "Нет доступа" }).waitFor();
    assert.deepEqual((await tagRead(other)).names, ["Правка агента"]);
    await page.unroute("**/v1/commands/tag.rename"); await back(); await closed(); await forget();
    // Card/tag versions displayed at opening must also refuse a concurrent change.
    await openCard();
    tag = await tagRead(other);
    await command("tag.rename", { space, tag: other, name: "Агент обновил тег", expected: tag.version });
    await click(assigned(replacement));
    await page.locator(".card-editor [role=alert]").filter({ hasText: "Данные изменились" }).waitFor();
    assert.deepEqual((await current(card)).tags, [other]);
    await back(); await closed(); await forget();
    // Lost assignment reply, then an agent reversal: replay may not install the old receipt.
    const bodies = [];
    await page.route("**/v1/commands/card.set_tag", async route => { bodies.push(route.request().postData()); await route.fetch(); await route.abort("connectionfailed"); });
    await openCard(); await click(assigned(unicode));
    await page.locator(".card-editor [role=alert]").filter({ hasText: "Результат неизвестен" }).waitFor();
    assert.equal(bodies.length, 3); assert.ok(bodies.every(body => body === bodies[0]));
    const saved = await page.evaluate(() => Object.fromEntries(Object.entries(localStorage).filter(([key]) => key.startsWith("zedone.pending."))));
    await back(); await closed(); await page.reload();
    assert.deepEqual(await page.evaluate(() => Object.fromEntries(Object.entries(localStorage).filter(([key]) => key.startsWith("zedone.pending.")))), saved);
    await page.unroute("**/v1/commands/card.set_tag");
    const fresh = await current(card); tag = await tagRead(unicode);
    await command("card.set_tag", { space, card, tag: unicode, present: false, expected: fresh.version, expected_tag: tag.version });
    let replay;
    await page.route("**/v1/commands/card.set_tag", async route => { replay = route.request().postData(); await route.continue(); });
    await click(".pending-list button:first-child");
    await page.waitForFunction(() => !document.querySelector(".pending-panel"));
    assert.equal(replay, bodies[0]); assert.deepEqual((await current(card)).tags, [other]);
    await page.unroute("**/v1/commands/card.set_tag");
    await page.reload();
  }
  await page.waitForFunction(id => document.querySelector(`.card-note .tag-chips li[data-tag="${id}"]`)?.textContent.length > 0, other);
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  if (process.env.UI_SCREENSHOT_DIRECTORY) await page.screenshot({ path: `${process.env.UI_SCREENSHOT_DIRECTORY}/tags-${tap ? "touch" : "desktop"}.png` });
  console.log("Tags: create/assign/remove, Unicode refusal, inverse rename, delete incl. trash, immutable content/history and fresh replay passed.");
}
