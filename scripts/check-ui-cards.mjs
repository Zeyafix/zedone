import assert from "node:assert/strict";

/** Actual HTTP/core; only the delivery faults and local persistence are injected. */
export async function checkCards(page, url, session) {
  const space = crypto.randomUUID().replaceAll("-", ""), column = crypto.randomUUID().replaceAll("-", "");
  const command = (operation, parameters) => page.evaluate(async ({ operation, parameters, key }) => {
    return (await fetch(`/v1/commands/${operation}`, {
      method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${key}` },
      body: JSON.stringify({ id: Array.from(crypto.getRandomValues(new Uint8Array(16))), operation, parameters }),
    })).json();
  }, { operation, parameters, key: session.key });
  const list = async () => {
    const response = await command("card.list", { space, include_trashed: true });
    assert.equal(response.status, "ok"); return response.result.cards;
  };
  const button = name => page.getByRole("button", { name, exact: true });
  const dialog = page.getByRole("dialog");
  const field = name => dialog.getByRole("textbox", { name, exact: true });
  async function create(title, description = "") {
    console.log(`Cards creation: ${title}`);
    await button("Создать карточку").click();
    assert.equal(await field("Заголовок").evaluate(el => el === document.activeElement), true);
    await field("Заголовок").fill(title); await field("Описание").fill(description);
    await dialog.getByRole("button", { name: "Создать карточку", exact: true }).click();
  }
  async function confirm(trigger, name) {
    await trigger.click(); await dialog.getByRole("button", { name, exact: true }).click();
    await dialog.waitFor({ state: "hidden" });
  }
  async function abandon() {
    if (await dialog.count()) await dialog.getByRole("button", { name: "К списку действий", exact: true }).click();
    await confirm(button("Убрать из списка"), "Убрать из списка");
    await button("Обновить список").click();
  }
  assert.equal((await command("space.create", { space, name: "Карточки" })).status, "ok");
  assert.equal((await command("column.create", { space, column, name: "Мысли" })).status, "ok");
  await page.goto(`${url}/#/spaces/${space}`);
  await button("Создать карточку").waitFor();

  const text = 'Первая строка\n<img src="https://example.invalid/no-request" onerror="alert(1)">\n[Ссылка](https://example.invalid/)';
  await create("Первая мысль", text); await dialog.waitFor({ state: "hidden" });
  await button("Первая мысль").click();
  assert.equal(await dialog.locator(".card-content p").textContent(), text);
  assert.equal(await dialog.locator("img, a, script").count(), 0, "description is literal text");
  await page.keyboard.press("Escape");
  assert.equal(await button("Первая мысль").evaluate(el => el === document.activeElement), true);
  await create("Вторая мысль"); await dialog.waitFor({ state: "hidden" });
  const original = await list();
  assert.equal(original.length, 2);
  await confirm(page.locator(".card-note").first().getByRole("button", { name: "В корзину", exact: true }), "В корзину");
  await button("Первая мысль").waitFor({ state: "hidden" });
  await button("Корзина карточек").click();
  await confirm(button("Восстановить"), "Восстановить");
  await button("Активные карточки").click();
  await page.reload(); await button("Первая мысль").waitFor();
  const restored = await list();
  assert.deepEqual(restored.map(card => [card.card, card.positions, card.titles, card.descriptions]), original.map(card => [card.card, card.positions, card.titles, card.descriptions]));
  assert.deepEqual(await page.locator(".card-title").allTextContents(), ["Первая мысль", "Вторая мысль"]);
  assert.equal(await page.locator(".column-actions button:last-child").isDisabled(), true, "nonempty column stays protected");

  // Rust rejects Unicode whitespace too; preserve both fields and the request.
  await create("   ", "Не потерять описание");
  await dialog.getByRole("alert").filter({ hasText: "Заголовок карточки не может быть пустым" }).waitFor();
  assert.equal(await field("Заголовок").inputValue(), "   ");
  assert.equal(await field("Описание").inputValue(), "Не потерять описание");
  assert.equal((await list()).length, 2);
  await abandon();

  // Opening a destructive action fixes its version. A later agent edit wins.
  await page.locator(".card-note").first().getByRole("button", { name: "В корзину", exact: true }).click();
  await dialog.waitFor();
  const before = (await list())[0];
  assert.equal((await command("card.edit", { space, card: before.card, expected: before.version, title: "Поздняя правка", description: text })).status, "ok");
  await dialog.getByRole("button", { name: "В корзину", exact: true }).click();
  await dialog.getByRole("alert").filter({ hasText: "Данные изменились" }).waitFor();
  assert.equal((await list())[0].state, "active");
  await abandon(); await button("Поздняя правка").waitFor();

  // Every lost reply had already committed. Restart and exact replay must not
  // duplicate the card or substitute the receipt for a later agent edit.
  const bodies = [];
  await page.route("**/v1/commands/card.create", async route => {
    bodies.push(route.request().postData());
    const response = await (await route.fetch()).json();
    assert.equal(response.status, "ok", `lost reply core error: ${response.error?.code}`);
    await route.abort();
  });
  await create("Неизвестный результат", text);
  await dialog.getByRole("alert").filter({ hasText: "Результат неизвестен" }).waitFor();
  assert.equal(bodies.length, 3); assert.equal(new Set(bodies).size, 1);
  const pending = await page.evaluate(() => Object.fromEntries(Object.entries(localStorage)));
  assert.equal(Object.keys(pending).length, 1);
  assert.ok(!JSON.stringify(pending).includes(session.key));
  const saved = (await list()).find(card => card.titles[0] === "Неизвестный результат");
  assert.ok(saved);
  assert.equal((await command("card.edit", { space, card: saved.card, expected: saved.version, title: "Новее ответа", description: "Сохранено агентом" })).status, "ok");
  await page.reload(); await button("Проверить результат").waitFor();
  assert.deepEqual(await page.evaluate(() => Object.fromEntries(Object.entries(localStorage))), pending);
  assert.equal(bodies.length, 3, "restart never retries automatically");
  await page.unroute("**/v1/commands/card.create");
  await button("Проверить результат").click();
  await page.getByRole("heading", { name: "Незавершённые действия", exact: true }).waitFor({ state: "hidden" });
  await button("Новее ответа").waitFor();
  assert.equal((await list()).length, 3); assert.equal(await page.evaluate(() => localStorage.length), 0);
  assert.equal(await button("Неизвестный результат").count(), 0);

  // A replayed old trash receipt must leave a subsequent restoration active.
  await page.route("**/v1/commands/card.trash", async route => {
    const response = await (await route.fetch()).json();
    assert.equal(response.status, "ok", `lost trash core error: ${response.error?.code}`); await route.abort();
  });
  await page.locator(".card-note").last().getByRole("button", { name: "В корзину", exact: true }).click();
  await dialog.getByRole("button", { name: "В корзину", exact: true }).click();
  await dialog.getByRole("alert").filter({ hasText: "Результат неизвестен" }).waitFor();
  const trashed = (await list()).find(card => card.card === saved.card);
  assert.equal((await command("card.restore", { space, card: saved.card, expected: trashed.version })).status, "ok");
  await page.unroute("**/v1/commands/card.trash");
  await dialog.getByRole("button", { name: "К списку действий", exact: true }).click();
  await button("Проверить результат").click();
  await page.getByRole("heading", { name: "Незавершённые действия", exact: true }).waitFor({ state: "hidden" });
  await button("Новее ответа").waitFor();
  assert.equal((await list()).find(card => card.card === saved.card).state, "active");

  // Denied delivery retains the original form/request; access recovery retries it.
  await page.route("**/v1/commands/card.create", route => route.continue({ headers: { ...route.request().headers(), authorization: `Bearer ${"0".repeat(64)}` } }));
  await create("Доступ восстановлен", "Мой ввод");
  await dialog.getByRole("alert").filter({ hasText: "Нет доступа к этому действию" }).waitFor();
  assert.equal(await field("Описание").inputValue(), "Мой ввод");
  await dialog.getByRole("button", { name: "К списку действий", exact: true }).click();
  await page.unroute("**/v1/commands/card.create"); await button("Проверить результат").click();
  await button("Доступ восстановлен").waitFor();

  let deliveries = 0;
  await page.route("**/v1/commands/card.create", route => { deliveries++; return route.continue(); });
  await page.evaluate(() => {
    window.__saveItem = Storage.prototype.setItem;
    Storage.prototype.setItem = () => { throw new DOMException("quota", "QuotaExceededError"); };
  });
  await create("Не отправлять", "Останется в форме");
  await dialog.getByRole("alert").filter({ hasText: "Запрос не отправлен" }).waitFor();
  assert.equal(deliveries, 0); assert.equal(await field("Описание").inputValue(), "Останется в форме");
  await page.evaluate(() => { Storage.prototype.setItem = window.__saveItem; delete window.__saveItem; });
  await dialog.getByRole("button", { name: "Отмена", exact: true }).click();
  await page.unroute("**/v1/commands/card.create");
  await page.setViewportSize({ width: 360, height: 760 });
  await button("Новее ответа").click();
  await dialog.waitFor();
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  if (process.env.UI_SCREENSHOT_DIRECTORY) await page.screenshot({ path: `${process.env.UI_SCREENSHOT_DIRECTORY}/card-mobile.png` });
  await page.keyboard.press("Escape"); await dialog.waitFor({ state: "hidden" });
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.evaluate(() => window.scrollTo(0, 0));
  if (process.env.UI_SCREENSHOT_DIRECTORY) await page.screenshot({ path: `${process.env.UI_SCREENSHOT_DIRECTORY}/cards-desktop.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForFunction(() => document.documentElement.dataset.theme === "dark");
  await button("Новее ответа").click(); await dialog.waitFor();
  if (process.env.UI_SCREENSHOT_DIRECTORY) await page.screenshot({ path: `${process.env.UI_SCREENSHOT_DIRECTORY}/card-dark.png` });
  await page.keyboard.press("Escape"); await dialog.waitFor({ state: "hidden" });
  await page.emulateMedia({ colorScheme: "light" });
  await page.waitForFunction(() => document.documentElement.dataset.theme === "light");
  // A late card read must not open a dialog on a different board/route.
  let release;
  const held = new Promise(resolve => { release = resolve; });
  let started;
  const reading = new Promise(resolve => { started = resolve; });
  await page.route("**/v1/commands/card.read", async route => {
    const response = await route.fetch(); started(); await held; await route.fulfill({ response });
  });
  await button("Новее ответа").click(); await reading;
  await page.goto(`${url}/#/spaces`);
  release(); await page.unrouteAll({ behavior: "wait" });
  await button("Создать пространство").waitFor();
  await page.waitForFunction(() => document.querySelector(".spaces-view")?.getAttribute("aria-busy") === "false");
  assert.equal(await dialog.count(), 0, "late read never opens a dialog on another route");
  console.log("Cards: HTTP lifecycle/order/restart/literal text, stale trash, denied/quota drafts, lost reply exact replay and late edit/restore preservation passed.");
}
