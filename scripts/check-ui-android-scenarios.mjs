import assert from "node:assert/strict";

// Playwright's current role/actionability engine requires checkVisibility(),
// absent in AOSP WebView 101. Inspect the unchanged DOM and send native touch
// input instead, as the prototype Android suite does. Never force DOM clicks.
export async function nativeControls(page) {
  page.setDefaultTimeout(10000);
  const cdp = await page.context().newCDPSession(page);
  async function visible(selector, text) {
    await page.waitForFunction(({ selector, text }) => {
      const element = document.querySelector(selector);
      if (!element || text !== undefined && element.textContent.trim() !== text) return false;
      const box = element.getBoundingClientRect();
      for (let current = element; current; current = current.parentElement) {
        const style = getComputedStyle(current);
        if (style.display === "none" || style.visibility !== "visible" || style.opacity === "0" || current.getAttribute("aria-hidden") === "true") return false;
      }
      return box.width > 0 && box.height > 0;
    }, { selector, text });
  }
  async function tap(selector, text) {
    const deadline = Date.now() + 10000;
    await visible(selector, text);
    const position = await page.waitForFunction(function stableTarget({ selector, text }) {
      const element = document.querySelector(selector);
      if (!element || element.disabled || element.closest('[aria-busy="true"]')
          || text !== undefined && element.textContent.trim() !== text) {
        stableTarget.previous = undefined;
        return false;
      }
      element.scrollIntoView({ block: "center" });
      const box = element.getBoundingClientRect();
      const geometry = [box.x, box.y, box.width, box.height, innerWidth, innerHeight, scrollX, scrollY,
        visualViewport?.width, visualViewport?.height, visualViewport?.offsetLeft, visualViewport?.offsetTop];
      const previous = stableTarget.previous;
      // Playwright polls on animation frames. Keep state on this private
      // predicate function, without modifying the application's DOM or globals.
      stableTarget.previous = { element, geometry };
      if (previous?.element !== element || !geometry.every((value, index) => value === previous.geometry[index])) return false;
      if (box.width <= 0 || box.height <= 0) return false;
      for (let current = element; current; current = current.parentElement) {
        const style = getComputedStyle(current);
        if (style.display === "none" || style.visibility !== "visible" || style.opacity === "0" || current.getAttribute("aria-hidden") === "true") return false;
      }
      const x = box.x + box.width / 2, y = box.y + box.height / 2;
      const target = document.elementFromPoint(x, y);
      if (!target || !element.contains(target)) return false;
      return { x, y };
    }, { selector, text }, { timeout: Math.max(1, deadline - Date.now()) });
    let coordinates;
    try { coordinates = await position.jsonValue(); }
    finally { await position.dispose(); }
    const { x, y } = coordinates;
    await cdp.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x, y }] });
    await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
  }
  async function fill(selector, value) {
    await tap(selector);
    await cdp.send("Input.dispatchKeyEvent", { type: "keyDown", key: "Control", code: "ControlLeft", windowsVirtualKeyCode: 17, modifiers: 2 });
    await cdp.send("Input.dispatchKeyEvent", { type: "keyDown", key: "a", code: "KeyA", windowsVirtualKeyCode: 65, modifiers: 2 });
    await cdp.send("Input.dispatchKeyEvent", { type: "keyUp", key: "a", code: "KeyA", windowsVirtualKeyCode: 65, modifiers: 2 });
    await cdp.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Control", code: "ControlLeft", windowsVirtualKeyCode: 17 });
    await cdp.send("Input.insertText", { text: value });
    await page.waitForFunction(({ selector, value }) => document.querySelector(selector)?.value === value, { selector, value });
  }
  return { visible, tap, fill };
}

export async function checkShell(page, back) {
  const { visible, tap } = await nativeControls(page);
  async function theme(value, name) {
    const selector = `input[type="radio"][name="theme"][value="${value}"]`;
    assert.equal(await page.evaluate(selector => document.querySelector(selector)?.labels[0]?.textContent.trim(), selector), name);
    await tap(selector);
    await page.waitForFunction(selector => document.querySelector(selector)?.checked, selector);
    if (value !== "system") await page.waitForFunction(value => document.documentElement.dataset.theme === value, value);
  }
  await visible("h2#connection-title", "Доступ не настроен");
  await tap("button.navigation-trigger", "Меню");
  await visible('[role="dialog"]');
  await back();
  await page.waitForFunction(() => !document.querySelector('[role="dialog"]'));
  await tap("button.navigation-trigger", "Меню");
  await visible('[role="dialog"]');
  await tap('[role="dialog"] a[href="#/settings"]', "Настройки");
  await visible("h1", "Настройки");
  await theme("dark", "Тёмная");
  await page.reload();
  await visible("h1", "Настройки");
  assert.equal(await page.evaluate(() => document.querySelector('input[value="dark"]').checked), true);
  assert.equal(await page.evaluate(() => document.documentElement.dataset.theme), "dark");
  await theme("light", "Светлая");
  await theme("system", "Как на устройстве");
  assert.equal(await page.evaluate(() => localStorage.length), 0);
  await back();
  await visible("h1", "Пространства");
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
}

/** CI only: same built UI and real IPC/Service, touch instead of DOM clicks. */
export async function checkWorkspaceTouch(page, back, dismissKeyboard = async () => {}) {
  const { visible, tap, fill } = await nativeControls(page);
  const dialog = '[role="dialog"]';
  const input = `${dialog} input[name="name"]`;
  async function named(selector, name, label) {
    await tap(selector);
    await visible(input);
    assert.equal(await page.evaluate(selector => document.activeElement === document.querySelector(selector), input), true);
    await fill(input, name);
    await tap(`${dialog} button[type="submit"]`, label);
    await page.waitForFunction(() => !document.querySelector('[role="dialog"]'));
  }
  async function confirm(selector, label) {
    await tap(selector);
    await tap(`${dialog} button[type="submit"]`, label);
    await page.waitForFunction(() => !document.querySelector('[role="dialog"]'));
  }
  await visible("h2#connection-title", "Доступ подтверждён");
  await named(".view-toolbar .primary-button", "Мысли", "Создать пространство");
  await visible(".space-row .object-name > a", "Мысли");
  const spaceHref = await page.evaluate(() => document.querySelector(".space-row a").getAttribute("href"));
  await named(".space-row .view-actions button:first-child", "Daily", "Сохранить название");
  await named(".space-row .view-actions button:first-child", "Мысли", "Сохранить название");
  await confirm(".space-row .view-actions button:last-child", "В корзину");
  await tap('.view-toolbar button[aria-pressed="false"]', "Корзина");
  await confirm(".space-row .view-actions button:last-child", "Восстановить");
  await tap('.view-toolbar button[aria-pressed="true"]', "Активные");
  await visible(".space-row a", "Мысли");
  assert.equal(await page.evaluate(() => document.querySelector(".space-row a").getAttribute("href")), spaceHref);
  await tap(".space-row a", "Мысли");
  await named(".board-heading .primary-button", "Разобрать", "Создать колонку");
  await named(".column-panel .rename-column", "Сделать", "Сохранить название");
  await named(".column-panel .rename-column", "Разобрать", "Сохранить название");
  await confirm(".column-panel .column-actions button:last-child", "В корзину");
  await tap('.view-toolbar button[aria-pressed="false"]', "Корзина");
  await confirm(".column-panel .column-actions button:last-child", "Восстановить");
  await tap('.view-toolbar button[aria-pressed="true"]', "Активные");
  await page.reload();
  await visible(".column-panel h3", "Разобрать");
  assert.equal(await page.evaluate(() => localStorage.length), 0);
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  await checkCardsTouch(page, dismissKeyboard);
  await tap(".board-heading .primary-button", "Создать колонку");
  await visible(dialog);
  // Android gives the IME the first Back. Verify form/board navigation only
  // after the native keyboard is hidden; dismissal must leave this form open.
  await dismissKeyboard();
  await visible(dialog);
  await back();
  await page.waitForFunction(() => !document.querySelector('[role="dialog"]'));
  await back();
  await visible(".space-row a", "Мысли");
  assert.equal(await page.evaluate(() => document.querySelector(".space-row .view-actions button:last-child").disabled), true);
  console.log("Android domain IPC: space/column create/rename/reverse/trash/restore, reload, focus/Back and protected nonempty space passed.");
}

/** Same IPC/Service, including the card's reverse action, on the CI AVD. */
async function checkCardsTouch(page, dismissKeyboard) {
  const { visible, tap, fill } = await nativeControls(page);
  const dialog = '[role="dialog"]';
  await tap(".create-card", "Создать карточку");
  await fill(`${dialog} input[name="name"]`, "Разобрать мысли");
  await fill(`${dialog} textarea`, "Строка один\n<b>Буквальный текст</b>");
  await tap(`${dialog} button[type="submit"]`, "Создать карточку");
  await page.waitForFunction(() => !document.querySelector('[role="dialog"]'));
  await tap(".card-title", "Разобрать мысли");
  await visible(".card-content p", "Строка один\n<b>Буквальный текст</b>");
  assert.equal(await page.evaluate(() => document.querySelectorAll(".card-content b").length), 0);
  await tap("[data-workspace-dialog-close]", "Закрыть");
  await tap(".card-note > button:last-child", "В корзину");
  await tap(`${dialog} button[type="submit"]`, "В корзину");
  await page.waitForFunction(() => !document.querySelector('.card-note'));
  await tap('.board-heading button[aria-pressed="false"]', "Корзина карточек");
  await tap(".card-note > button:last-child", "Восстановить");
  await tap(`${dialog} button[type="submit"]`, "Восстановить");
  await page.waitForFunction(() => !document.querySelector('.card-note'));
  await tap('.board-heading button[aria-pressed="true"]', "Активные карточки");
  await page.reload(); await visible(".card-title", "Разобрать мысли");
  assert.equal(await page.evaluate(() => localStorage.length), 0);
  assert.equal(await page.evaluate(() => document.querySelector('.column-actions button:last-child').disabled), true);
  await tap(".create-card", "Создать карточку");
  await fill(`${dialog} input[name="name"]`, "   ");
  await fill(`${dialog} textarea`, "Ввод при отказе");
  await tap(`${dialog} button[type="submit"]`, "Создать карточку");
  await visible(`${dialog} [role="alert"]`);
  assert.equal(await page.evaluate(() => document.querySelector('[role="dialog"] textarea').value), "Ввод при отказе");
  await dismissKeyboard();
  await tap("[data-workspace-dialog-close]", "Закрыть");
  await tap(".pending-list li button:last-child", "Убрать из списка");
  await tap(`${dialog} button[type="submit"]`, "Убрать из списка");
  await page.waitForFunction(() => !document.querySelector('[role="dialog"]'));
  assert.equal(await page.evaluate(() => localStorage.length), 0);
  await tap(".view-toolbar .view-actions button:nth-child(2)", "Обновить список");
  await visible(".card-title", "Разобрать мысли");
  console.log("Android card IPC: creation/read/literal description/trash/restore/reload, nonempty column protection and rejected-title draft passed.");
  await checkEditorTouch(page, dismissKeyboard);
}

/** Real host commands inspect the UI's effects, without reaching into Vue state. */
async function checkEditorTouch(page, dismissKeyboard) {
  const { visible, tap, fill } = await nativeControls(page);
  const api = (operation, parameters) => page.evaluate(async ({ operation, parameters }) => {
    const request = JSON.stringify({ id: Array.from(crypto.getRandomValues(new Uint8Array(16))), operation, parameters });
    let response;
    if (window.__TAURI_INTERNALS__) {
      const session = await window.__TAURI_INTERNALS__.invoke("shell_session");
      response = JSON.parse(await window.__TAURI_INTERNALS__.invoke("zedone_command", { invocation: { operation, request }, credential: session.credential.key }));
    } else {
      const session = await (await fetch("/host/session")).json();
      response = await (await fetch(`/v1/commands/${operation}`, { method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${session.credential.key}` }, body: request })).json();
    }
    if (response.status !== "ok") throw new Error(`${operation}: ${response.error?.code}`);
    return response.result;
  }, { operation, parameters });
  const space = await page.evaluate(() => location.hash.split("/").at(-1));
  const card = (await api("card.list", { space, include_trashed: false })).cards[0].card;
  const current = () => api("card.read", { space, card });
  const history = () => api("card.history", { space, card, limit: 100, after: null, expected: null });
  const saved = () => page.waitForFunction(() => Array.from(document.querySelectorAll('.card-editor [role="status"]')).some(el => el.textContent === "Сохранено"));
  const close = async () => {
    await dismissKeyboard(); await tap("[data-workspace-dialog-close]", "Закрыть");
    await page.waitForFunction(() => !document.querySelector('[role="dialog"]'));
  };
  const initial = (await history()).versions.length;
  await tap(".card-title", "Разобрать мысли"); await tap("[data-editor-edit]", "Редактировать");
  await visible('input[name="card-title"]'); await saved();
  await fill('input[name="card-title"]', "Редактор карточки");
  await fill('textarea[name="card-description"]', "Сохранено вместе\n<b>Обычный текст</b>"); await saved();
  assert.equal((await current()).descriptions[0], "Сохранено вместе\n<b>Обычный текст</b>");
  await tap('.card-editor button[aria-pressed="false"]', "Отметить выполненной");
  await visible('.card-editor button[aria-pressed="true"]', "Снять отметку выполнения");
  assert.deepEqual((await current()).completed, [true]);
  await tap('.card-editor button[aria-pressed="true"]', "Снять отметку выполнения");
  await visible('.card-editor button[aria-pressed="false"]', "Отметить выполненной");
  assert.deepEqual((await current()).completed, [false]); await close();
  assert.equal((await history()).versions.length, initial + 1);
  assert.equal((await history()).versions.at(-1).title, "Редактор карточки");

  await tap(".card-title", "Редактор карточки"); await tap("[data-editor-edit]", "Редактировать"); await saved();
  const version = (await current()).version;
  await fill('input[name="card-title"]', "   "); await fill('textarea[name="card-description"]', "Остаться черновиком");
  await visible('.card-editor [role="status"]', "Заголовок обязателен. Последняя сохранённая запись не изменена.");
  await dismissKeyboard(); await tap("[data-workspace-dialog-close]", "Закрыть");
  await visible("[data-editor-keep-draft]", "Закрыть, сохранив ввод");
  assert.deepEqual((await current()).version, version);
  await tap("[data-editor-keep-draft]", "Закрыть, сохранив ввод");
  await page.waitForFunction(() => !document.querySelector('[role="dialog"]'));
  await tap('section[aria-label="Черновики карточек"] button:first-child', "Открыть черновик");
  await visible('textarea[name="card-description"]');
  assert.equal(await page.evaluate(() => document.querySelector('textarea[name="card-description"]').value), "Остаться черновиком");
  await fill('input[name="card-title"]', "Восстановленный ввод"); await saved(); await close();
  assert.equal((await current()).titles[0], "Восстановленный ввод");
  assert.equal(await page.evaluate(() => localStorage.length), 0);
  console.log("Android editor: paired autosave/history, completion/reverse, invalid-title guard and durable draft recovery passed through UI and shared commands.");
}
