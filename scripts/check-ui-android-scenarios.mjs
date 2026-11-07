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
    await visible(selector, text);
    await page.waitForFunction(selector => {
      const element = document.querySelector(selector);
      return element && !element.disabled;
    }, selector);
    const { x, y } = await page.evaluate(selector => {
      const element = document.querySelector(selector);
      if (element.disabled) throw new Error(`Disabled control: ${selector}`);
      element.scrollIntoView({ block: "center" });
      const box = element.getBoundingClientRect();
      const x = box.x + box.width / 2, y = box.y + box.height / 2;
      const target = document.elementFromPoint(x, y);
      if (!target || !element.contains(target)) throw new Error(`Covered control: ${selector}`);
      return { x, y };
    }, selector);
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
  await named(".column-panel .column-actions button:first-child", "Сделать", "Сохранить название");
  await named(".column-panel .column-actions button:first-child", "Разобрать", "Сохранить название");
  await confirm(".column-panel .column-actions button:last-child", "В корзину");
  await tap('.view-toolbar button[aria-pressed="false"]', "Корзина");
  await confirm(".column-panel .column-actions button:last-child", "Восстановить");
  await tap('.view-toolbar button[aria-pressed="true"]', "Активные");
  await page.reload();
  await visible(".column-panel h3", "Разобрать");
  assert.equal(await page.evaluate(() => localStorage.length), 0);
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
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
