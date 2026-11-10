import assert from "node:assert/strict";

// Real HTTP/Service lifecycle plus injected delivery faults. No mocked domain
// data and no direct SQLite writes. Each invocation uses distinct fixture names.
export async function checkWorkspace(page, url, session) {
  page.setDefaultTimeout(10000);
  const suffix = crypto.randomUUID().slice(0, 8);
  const spaceName = `Мысли ${suffix}`;
  const boardName = `Дела ${suffix}`;
  const row = name => page.locator(".space-row").filter({ has: page.getByRole("link", { name, exact: true }) });
  const button = name => page.getByRole("button", { name, exact: true });
  const dialog = page.getByRole("dialog");
  async function namedAction(trigger, name, confirm) {
    console.log(`Workspace action: ${confirm} (${name})`);
    await trigger.click();
    const field = dialog.getByRole("textbox", { name: "Название" });
    await field.waitFor();
    assert.equal(await field.evaluate(el => el === document.activeElement), true, "name receives focus");
    await field.fill(name);
    await dialog.getByRole("button", { name: confirm, exact: true }).click();
    await dialog.waitFor({ state: "hidden" });
  }
  async function confirm(trigger, label) {
    console.log(`Workspace confirmation: ${label}`);
    await trigger.click();
    await dialog.getByRole("button", { name: label, exact: true }).click();
    await dialog.waitFor({ state: "hidden" });
  }
  const command = async (operation, parameters) => page.evaluate(async ({ operation, parameters, key }) => {
    const response = await fetch(`/v1/commands/${operation}`, {
      method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${key}` },
      body: JSON.stringify({ id: Array.from(crypto.getRandomValues(new Uint8Array(16))), operation, parameters }),
    });
    return response.json();
  }, { operation, parameters, key: session.key });

  await namedAction(button("Создать пространство"), spaceName, "Создать пространство");
  const spaceLink = row(spaceName).getByRole("link", { name: spaceName, exact: true });
  await spaceLink.waitFor();
  const spaceId = (await spaceLink.getAttribute("href")).split("/").at(-1);
  await namedAction(row(spaceName).getByRole("button", { name: "Переименовать", exact: true }), `${spaceName} новое`, "Сохранить название");
  await namedAction(row(`${spaceName} новое`).getByRole("button", { name: "Переименовать", exact: true }), spaceName, "Сохранить название");
  await confirm(row(spaceName).getByRole("button", { name: "В корзину", exact: true }), "В корзину");
  await spaceLink.waitFor({ state: "hidden" });
  await button("Корзина").click();
  await confirm(row(spaceName).getByRole("button", { name: "Восстановить", exact: true }), "Восстановить");
  await button("Активные").click();
  assert.equal((await row(spaceName).getByRole("link").getAttribute("href")).split("/").at(-1), spaceId);

  await namedAction(button("Создать пространство"), boardName, "Создать пространство");
  await row(boardName).getByRole("link", { name: boardName, exact: true }).click();
  const boardUrl = page.url();
  await namedAction(button("Создать колонку"), "Разобрать", "Создать колонку");
  const column = page.getByRole("region", { name: "Разобрать", exact: true });
  await column.waitFor();
  await namedAction(column.getByRole("button", { name: "Переименовать", exact: true }), "Сделать", "Сохранить название");
  await namedAction(page.getByRole("region", { name: "Сделать", exact: true }).getByRole("button", { name: "Переименовать", exact: true }), "Разобрать", "Сохранить название");
  await confirm(column.getByRole("button", { name: "В корзину", exact: true }), "В корзину");
  await button("Корзина").click();
  await confirm(column.getByRole("button", { name: "Восстановить", exact: true }), "Восстановить");
  await button("Активные").click();
  console.log("Workspace reload board");
  await page.reload();
  await column.waitFor();
  assert.equal(page.url(), boardUrl, "board navigation survives reload");
  assert.equal(await page.evaluate(() => localStorage.length), 0, "confirmed commands leave no pending state");
  await button("Создать колонку").click();
  await page.keyboard.press("Escape");
  assert.equal(await button("Создать колонку").evaluate(el => el === document.activeElement), true, "cancel restores focus");
  await page.getByRole("link", { name: "Все пространства", exact: true }).click();
  await page.waitForFunction(() => !document.querySelector('.board-heading') && document.querySelector('.spaces-view')?.getAttribute('aria-busy') === 'false');
  assert.equal(await row(boardName).getByRole("button", { name: "В корзину", exact: true }).isDisabled(), false, "nonempty space offers a complete group preview");

  // A saved version must not be replaced by a concurrent author's version.
  console.log("Workspace concurrent rename");
  await row(spaceName).getByRole("button", { name: "Переименовать", exact: true }).click();
  await dialog.getByRole("textbox", { name: "Название" }).fill("Мой несохранённый ввод");
  const snapshot = await command("space.read", { space: spaceId });
  assert.equal(snapshot.status, "ok");
  assert.equal((await command("space.rename", { space: spaceId, expected: snapshot.result.version, name: `${spaceName} другое устройство` })).status, "ok");
  await dialog.getByRole("button", { name: "Сохранить название", exact: true }).click();
  await dialog.getByRole("alert").filter({ hasText: "Данные изменились" }).waitFor();
  assert.equal(await dialog.getByRole("textbox").inputValue(), "Мой несохранённый ввод");
  await dialog.getByRole("button", { name: "К списку действий", exact: true }).click();
  await confirm(button("Убрать из списка"), "Убрать из списка");
  await button("Обновить список").click();
  await row(`${spaceName} другое устройство`).waitFor();

  // Commit succeeds, every reply is lost, then the page restarts. Retry must
  // replay exactly the same bytes and must leave only one created space.
  const lostName = `Потерянный ответ ${suffix}`;
  const bodies = [];
  await page.route("**/v1/commands/space.create", async route => {
    bodies.push(route.request().postData());
    const response = await route.fetch();
    assert.equal((await response.json()).status, "ok");
    await route.abort();
  });
  await button("Создать пространство").click();
  await dialog.getByRole("textbox").fill(lostName);
  await dialog.getByRole("button", { name: "Создать пространство", exact: true }).click();
  await dialog.getByRole("alert").filter({ hasText: "Результат неизвестен" }).waitFor();
  assert.equal(bodies.length, 3);
  assert.equal(new Set(bodies).size, 1);
  const savedBefore = await page.evaluate(() => Object.fromEntries(Object.entries(localStorage)));
  assert.equal(Object.keys(savedBefore).length, 1);
  assert.ok(!JSON.stringify(savedBefore).includes(session.key), "credential is never persisted");
  await page.reload();
  await button("Проверить результат").waitFor();
  assert.deepEqual(await page.evaluate(() => Object.fromEntries(Object.entries(localStorage))), savedBefore);
  await page.unroute("**/v1/commands/space.create");
  await button("Проверить результат").click();
  await page.getByRole("heading", { name: "Незавершённые действия", exact: true }).waitFor({ state: "hidden" });
  await row(lostName).waitFor();
  assert.equal(await row(lostName).count(), 1);
  assert.equal(await page.evaluate(() => localStorage.length), 0);

  // A real core denial after the form has opened preserves the draft and the
  // original request. Once access works, explicit retry uses that same request.
  const deniedName = `Доступ ${suffix}`;
  const deniedBodies = [];
  await button("Создать пространство").click();
  await dialog.getByRole("textbox").fill(deniedName);
  await page.route("**/v1/commands/space.create", route => {
    deniedBodies.push(route.request().postData());
    return route.continue({ headers: { ...route.request().headers(), authorization: `Bearer ${"0".repeat(64)}` } });
  });
  await dialog.getByRole("button", { name: "Создать пространство", exact: true }).click();
  await dialog.getByRole("alert").filter({ hasText: "Нет доступа к этому действию" }).waitFor();
  assert.equal(await dialog.getByRole("textbox").inputValue(), deniedName);
  assert.equal(deniedBodies.length, 1);
  await dialog.getByRole("button", { name: "К списку действий", exact: true }).click();
  await page.unroute("**/v1/commands/space.create");
  await button("Проверить результат").click();
  await row(deniedName).waitFor();
  assert.equal(await page.evaluate(() => localStorage.length), 0);

  // Storage failure must prevent a mutation from reaching the API.
  let deliveries = 0;
  await page.route("**/v1/commands/space.create", route => { deliveries++; return route.continue(); });
  await page.evaluate(() => {
    window.__saveItem = Storage.prototype.setItem;
    Storage.prototype.setItem = () => { throw new DOMException("quota", "QuotaExceededError"); };
  });
  await button("Создать пространство").click();
  await dialog.getByRole("textbox").fill("Не отправлять");
  await dialog.getByRole("button", { name: "Создать пространство", exact: true }).click();
  await dialog.getByRole("alert").filter({ hasText: "Запрос не отправлен" }).waitFor();
  assert.equal(deliveries, 0);
  assert.equal(await dialog.getByRole("textbox").inputValue(), "Не отправлять");
  await page.evaluate(() => { Storage.prototype.setItem = window.__saveItem; delete window.__saveItem; });
  await dialog.getByRole("button", { name: "Отмена", exact: true }).click();
  await page.unroute("**/v1/commands/space.create");
  await page.goto(`${url}/#/spaces`);
  await button("Создать пространство").waitFor();
  console.log("Workspace: real HTTP space/column lifecycle, focus/restart, nonempty guard, stale/draft, lost reply/replay/restart, storage refusal passed.");
}
