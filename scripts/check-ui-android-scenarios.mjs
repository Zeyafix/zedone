import assert from "node:assert/strict";

// Playwright's current role/actionability engine requires checkVisibility(),
// absent in AOSP WebView 101. Inspect the unchanged DOM and send native touch
// input instead, as the prototype Android suite does. Never force DOM clicks.
export async function checkShell(page, back) {
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
