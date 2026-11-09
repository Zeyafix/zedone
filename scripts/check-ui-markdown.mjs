import assert from "node:assert/strict";

/** The actual card API and production viewer; image servers alone are fixtures. */
export async function checkCardMarkdown(page, { tap, fill, external, dismissKeyboard = async () => {} } = {}) {
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
  const space = crypto.randomUUID().replaceAll("-", ""), column = crypto.randomUUID().replaceAll("-", ""), card = crypto.randomUUID().replaceAll("-", "");
  const current = () => command("card.read", { space, card });
  const history = () => command("card.history", { space, card, after: null, limit: 100, expected: null });
  const close = async () => {
    await dismissKeyboard(); await click("[data-workspace-dialog-close]");
    await page.waitForFunction(() => !document.querySelector('[role="dialog"]'));
  };
  const content = page.locator(".markdown-content");
  const origin = "https://markdown.invalid";
  const source = `# Ёж 😀\n\n**важно** и *курсив*, \`код\`.\n\n- первый\n- [ ] обычный маркер\n\n3. номер\n\n> цитата\n\n\`\`\`js\nconst text = '<b>код</b>';\n\`\`\`\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n\n<b>буквально</b> <script>globalThis.markdownXss=1</script>\n<img src="${origin}/raw.png" onerror="globalThis.markdownXss=2">\n<iframe src="${origin}/frame"></iframe>\n\n[Внешняя ссылка](https://example.invalid/zedone-card "Сайт")\n\n![Первое](${origin}/one.png "Подсказка") ![Второе](${origin}/two.png)\n\n[![В ссылке](${origin}/linked.png)](http://example.invalid/linked)\n\n![Локальное](file:///secret.png)\n\n[опасно](javascript:alert(1)) [файл](file:///secret) [почта](mailto:test@example.invalid)\n\n![данные](data:image/png;base64,AAAA)\n\nЁ е́ \\ конец`;
  const requests = [];
  const observe = request => { if (request.url().startsWith(origin)) requests.push(request.url()); };
  page.on("request", observe);
  let fail = false;
  const fixture = route => fail ? route.abort("internetdisconnected") : route.fulfill({ contentType: "image/png", body: Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/l9sAAAAASUVORK5CYII=", "base64") });
  await page.context().route(`${origin}/**`, fixture);
  try {
    await command("space.create", { space, name: "Markdown" });
    await command("column.create", { space, column, name: "Просмотр" });
    await command("card.create", { space, column, card, title: "CommonMark", description: source });
    await page.evaluate(space => { location.hash = `#/spaces/${space}`; }, space);
    await page.waitForFunction(() => document.querySelector(".card-title")?.textContent === "CommonMark");
    await click(".card-title"); await content.waitFor();
    assert.equal(await content.locator("h1").textContent(), "Ёж 😀");
    assert.equal(await content.locator("strong").textContent(), "важно");
    assert.equal(await content.locator("em").textContent(), "курсив");
    assert.equal(await content.locator("ul li").count(), 2);
    assert.equal(await content.locator("ol").getAttribute("start"), "3");
    assert.match(await content.locator("blockquote").textContent(), /цитата/);
    assert.match(await content.locator("pre code").textContent(), /<b>код<\/b>/);
    assert.equal(await content.locator("table, input, script, iframe, b, img, svg, math, [onerror], [src], [srcset]").count(), 0);
    assert.match(await content.textContent(), /<b>буквально<\/b>/);
    assert.match(await content.textContent(), /\[ \] обычный маркер/);
    assert.match(await content.textContent(), /\| --- \| --- \|/);
    assert.equal(await page.evaluate(() => globalThis.markdownXss), undefined);
    assert.deepEqual(requests, [], "parsing/sanitizing/display are inert");
    assert.equal(await content.locator("a").count(), 2);
    assert.equal(await content.locator("a button").count(), 0);
    assert.equal(await content.locator("a").first().getAttribute("rel"), "noopener noreferrer");
    assert.equal(await page.evaluate(() => {
      const dialog = document.querySelector('[role="dialog"]');
      return dialog.scrollWidth <= dialog.clientWidth && document.documentElement.scrollWidth <= innerWidth;
    }), true, "formatted description stays within the dialog");
    const original = await current(), versions = await history(), url = page.url();
    const openLink = () => click('.markdown-content a[href="https://example.invalid/zedone-card"]');
    if (external) await external("https://example.invalid/zedone-card", openLink);
    else {
      const route = r => r.fulfill({ contentType: "text/html", body: "<title>External fixture</title>" });
      await page.context().route("https://example.invalid/**", route);
      try {
        const popup = page.waitForEvent("popup"); await openLink(); const tab = await popup;
        try { await tab.waitForLoadState(); assert.equal(tab.url(), "https://example.invalid/zedone-card"); assert.equal(await tab.evaluate(() => window.opener === null), true); }
        finally { await tab.close(); }
      } finally { await page.context().unroute("https://example.invalid/**", route); }
    }
    assert.equal(page.url(), url);
    await click('[data-md-control="0"] button:first-of-type');
    await page.waitForFunction(() => document.querySelector(".markdown-content img")?.naturalWidth > 0);
    assert.deepEqual(requests, [`${origin}/one.png`]);
    assert.equal(await content.locator("img").getAttribute("referrerpolicy"), "no-referrer");
    assert.equal(await content.locator("img").getAttribute("title"), "Подсказка");
    await click('[data-md-control="0"] button:first-of-type');
    assert.equal(await content.locator("img").count(), 0);
    fail = true;
    await click('[data-md-control="1"] button:first-of-type');
    await page.waitForFunction(() => document.querySelector(".markdown-content").textContent.includes("Не удалось загрузить изображение"));
    assert.deepEqual(await current(), original, "network error does not write to the card");
    assert.deepEqual(await history(), versions, "view/network actions do not create history");
    fail = false;
    await click('[data-md-control="1"] button:first-of-type');
    await page.waitForFunction(() => document.querySelector(".markdown-content img")?.naturalWidth > 0);
    assert.deepEqual(requests, [`${origin}/one.png`, `${origin}/two.png`, `${origin}/two.png`]);
    // Completion is independent of textual checkbox markers, with no new fetches.
    await click('.card-editor button[aria-pressed="false"]');
    await page.waitForFunction(() => document.querySelector('.card-editor button[aria-pressed="true"]'));
    assert.deepEqual((await current()).descriptions, [source]);
    await click('.card-editor button[aria-pressed="true"]');
    await page.waitForFunction(() => document.querySelector('.card-editor button[aria-pressed="false"]'));
    assert.deepEqual((await current()).completed, [false]);
    assert.equal(requests.length, 3);
    // A reopened card resets consent, including browser reload.
    await close(); await page.reload(); await click(".card-title"); await content.waitFor();
    assert.equal(await content.locator("img").count(), 0); assert.equal(requests.length, 3);
    await click("[data-editor-edit]");
    assert.equal(await page.locator('textarea[name="card-description"]').inputValue(), source);
    const edited = "**После ошибки**\n\n![Новое](https://markdown.invalid/new.png)";
    await input('textarea[name="card-description"]', edited);
    await page.waitForFunction(() => Array.from(document.querySelectorAll('.card-editor [role="status"]')).some(el => el.textContent === "Сохранено"));
    await close(); await click(".card-title");
    await page.waitForFunction(() => document.querySelector(".markdown-content strong")?.textContent === "После ошибки");
    assert.deepEqual((await current()).descriptions, [edited]);
    assert.equal(await content.locator("img").count(), 0); assert.equal(requests.length, 3);
    // Reverse the ordinary edit through the same API used by agents.
    await close(); const latest = await current();
    await command("card.edit", { space, card, expected: latest.version, title: latest.titles[0], description: source });
    await page.reload(); await click(".card-title"); await content.waitFor();
    assert.deepEqual((await current()).descriptions, [source]);
    assert.equal(await content.locator("img").count(), 0); assert.equal(requests.length, 3);
    assert.deepEqual((await current()).completed, [false]);
    if (process.env.UI_SCREENSHOT_DIRECTORY) {
      const { resolve } = await import("node:path");
      await page.screenshot({ path: resolve(process.env.UI_SCREENSHOT_DIRECTORY, "card-markdown.png") });
    }
    await close();
    console.log("Card Markdown: CommonMark/HTML/XSS, exact API source, external link, opt-in image/hide/failure/retry, completion/reverse, reload, UI edit and agent reverse passed.");
  } finally { page.off("request", observe); await page.context().unroute(`${origin}/**`, fixture); }
}
