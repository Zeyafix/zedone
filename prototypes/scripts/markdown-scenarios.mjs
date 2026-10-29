import assert from "node:assert/strict";

// One scenario runs against the production bundle and the installed Android WebView.
export async function checkMarkdown(page, { tap, external } = {}) {
  const click = tap || (selector => page.locator(selector).click());
  const fill = async source => {
    await page.evaluate(source => {
      const input = document.querySelector("#text");
      input.value = source;
      input.dispatchEvent(new Event("input", { bubbles: true }));
    }, source);
    await click('#echo-form button[type="submit"]');
    await page.waitForFunction(source => document.querySelector('[role="status"]')?.textContent === source, source);
    await page.waitForFunction(() => document.querySelector("#markdown-content"));
    assert.equal(await page.locator("#text").inputValue(), source);
  };
  const origin = "https://markdown.invalid";
  const requested = [];
  const observe = request => { if (request.url().startsWith(origin)) requested.push(request.url()); };
  page.on("request", observe);
  let failImage = false;
  const fixture = async route => {
    if (route.request().url().endsWith("/two.png") && failImage) return route.continue();
    return route.fulfill({ contentType: "image/png", body: Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/l9sAAAAASUVORK5CYII=", "base64") });
  };
  await page.context().route(`${origin}/**`, fixture);
  try {
    const source = `# Ёж 😀\n\n## Заголовок\n\n**важно** и *курсив*, \`код\`.\n\n- первый\n- [ ] обычный маркер\n\n1. номер\n\n> цитата\n\n\`\`\`js\nconst text = '<b>код</b>';\n\`\`\`\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n\n<b>буквально</b> <script>globalThis.markdownXss = 1</script>\n<img src="${origin}/raw.png" onerror="globalThis.markdownXss=2">\n<iframe src="${origin}/frame"></iframe>\n\n[Внешняя ссылка](https://example.invalid/zedone-prototype "Сайт")\n\n![Первое](${origin}/one.png "Подсказка") ![Второе](${origin}/two.png)\n\n[![В ссылке](${origin}/linked.png)](http://example.invalid/linked)\n\n![Локальное](file:///secret.png)\n\n[опасно](javascript:alert(1)) [файл](file:///secret) [почта](mailto:test@example.invalid)\n\n![данные](data:image/png;base64,AAAA)\n\nЁ е́ \\ конец`;
    await fill(source);
    const content = page.locator("#markdown-content");
    assert.equal(await content.locator("h1").textContent(), "Ёж 😀");
    assert.equal(await content.locator("strong").textContent(), "важно");
    assert.equal(await content.locator("em").textContent(), "курсив");
    assert.equal(await content.locator("ul li").count(), 2);
    assert.equal(await content.locator("ol li").textContent(), "номер");
    assert.match(await content.locator("blockquote").textContent(), /цитата/);
    assert.match(await content.locator("pre code").textContent(), /<b>код<\/b>/);
    assert.equal(await content.locator("table, input, script, iframe, b, img, svg, math, [onerror], [src], [srcset]").count(), 0);
    assert.match(await content.textContent(), /<b>буквально<\/b>/);
    assert.match(await content.textContent(), /\[ \] обычный маркер/);
    assert.match(await content.textContent(), /\| --- \| --- \|/);
    assert.equal(await page.evaluate(() => globalThis.markdownXss), undefined);
    assert.deepEqual(requested, [], "Parsing, sanitizing and display must not request any images/frames");
    assert.equal(await content.locator("a").count(), 2, "Only HTTP/HTTPS links are active");
    assert.equal(await content.locator("a button").count(), 0, "Image controls cannot be inside a link");
    assert.equal(await content.locator("a").first().getAttribute("rel"), "noopener noreferrer");
    assert.equal(await content.locator("a").first().getAttribute("title"), "Сайт");
    const originalUrl = page.url();
    if (external) {
      await external("https://example.invalid/zedone-prototype", () => click("#markdown-content a"));
    } else {
      const popupRoute = route => route.fulfill({ contentType: "text/html", body: "<title>External fixture</title>" });
      await page.context().route("https://example.invalid/**", popupRoute);
      try {
        const popup = page.waitForEvent("popup");
        await click("#markdown-content a >> nth=0");
        const tab = await popup;
        try {
          await tab.waitForLoadState();
          assert.equal(tab.url(), "https://example.invalid/zedone-prototype");
          assert.equal(await tab.evaluate(() => window.opener === null), true);
        } finally { await tab.close(); }
      } finally { await page.context().unroute("https://example.invalid/**", popupRoute); }
    }
    assert.equal(page.url(), originalUrl);
    assert.equal(await page.locator("#text").inputValue(), source);
    await click('[data-md-control="0"] button:first-of-type');
    await page.waitForFunction(() => document.querySelector("#markdown-content img")?.naturalWidth > 0);
    assert.deepEqual(requested, [`${origin}/one.png`]);
    assert.equal(await content.locator("img").getAttribute("title"), "Подсказка");
    await click('[data-md-control="0"] button:first-of-type');
    assert.equal(await content.locator("img").count(), 0, "Hide reverses loaded image display");
    failImage = true;
    await page.context().setOffline(true);
    await click('[data-md-control="1"] button:first-of-type');
    await page.waitForFunction(() => document.querySelector("#markdown-content").textContent.includes("Не удалось загрузить изображение"));
    assert.equal(await page.locator("#text").inputValue(), source);
    assert.equal(await page.locator('[role="status"]').textContent(), source);
    // Editing remains available during a remote image failure; it does not alter the rendered response.
    await page.evaluate(() => {
      const input = document.querySelector("#text");
      input.value += " правка";
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    assert.equal(await page.locator("#text").inputValue(), source + " правка");
    failImage = false;
    await page.context().setOffline(false);
    await click('[data-md-control="1"] button:first-of-type');
    await page.waitForFunction(() => document.querySelector("#markdown-content img")?.naturalWidth > 0);
    assert.deepEqual(requested, [`${origin}/one.png`, `${origin}/two.png`, `${origin}/two.png`]);
    if (process.env.PROTOTYPE_MARKDOWN_SCREENSHOT) {
      await page.evaluate(() => document.querySelector('#markdown-content [data-md-control="1"]').scrollIntoView({ block: "center" }));
      await page.screenshot({ path: process.env.PROTOTYPE_MARKDOWN_SCREENSHOT });
    }
    await fill("**Новый** текст\n\n![Новое](https://markdown.invalid/new.png)");
    assert.equal(await content.locator("img").count(), 0, "New text cannot inherit consent to fetch an earlier image");
    assert.equal(requested.length, 3);
    await click('#echo-form button[type="button"]');
    assert.equal(await page.locator("#markdown-content").count(), 0);
    assert.equal(await page.locator("#text").inputValue(), "");
    console.log("Markdown: CommonMark, literal HTML/XSS, unchanged source, inert images, external link, selected load/hide, network failure/retry, edit and clear passed.");
  } finally {
    page.off("request", observe);
    await page.context().setOffline(false);
    await page.context().unroute(`${origin}/**`, fixture);
  }
}
