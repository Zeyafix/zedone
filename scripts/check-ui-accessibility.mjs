import assert from "node:assert/strict";

/** Inspect rendered content, including nested scroll areas; no application state writes. */
export async function checkPresentation(page) {
  const result = await page.evaluate(() => {
    const failures = [];
    const visible = element => {
      if (!element.getClientRects().length || element.closest('[inert], [aria-hidden="true"]')) return false;
      for (let current = element; current; current = current.parentElement) {
        const style = getComputedStyle(current);
        if (style.display === "none" || style.visibility !== "visible" || Number(style.opacity) < 1) return false;
      }
      return true;
    };
    const name = element => element.getAttribute("aria-label")
      || element.getAttribute("aria-labelledby")?.split(/\s+/).map(id => document.getElementById(id)?.textContent ?? "").join(" ")
      || [...element.labels ?? []].map(label => label.textContent).join(" ") || element.textContent;
    const rgb = color => color.match(/[\d.]+/g)?.map(Number);
    const luminance = color => rgb(color).slice(0, 3).map(value => {
      const s = value / 255;
      return s <= .04045 ? s / 12.92 : ((s + .055) / 1.055) ** 2.4;
    }).reduce((sum, value, i) => sum + value * [.2126, .7152, .0722][i], 0);
    let texts = 0, controls = 0, minimum = Infinity;
    const scope = document.querySelector('[role="dialog"]') ?? document.querySelector("main");
    for (const element of [scope, ...scope.querySelectorAll("*")]) {
      if (!visible(element)) continue;
      const style = getComputedStyle(element);
      if (element.matches('button, a[href], input, textarea, select, summary, [role="dialog"]')) {
        controls++;
        if (!name(element)?.trim()) failures.push(`unnamed ${element.tagName}`);
      }
      if (element.matches(":disabled") || ![...element.childNodes].some(node => node.nodeType === Node.TEXT_NODE && node.textContent.trim())) continue;
      let background = element;
      while (background && (rgb(getComputedStyle(background).backgroundColor)?.[3] ?? 1) === 0) background = background.parentElement;
      if (!background) { failures.push("missing opaque background"); continue; }
      const back = getComputedStyle(background).backgroundColor;
      // Current UI uses opaque text and surfaces. Report an unsupported blend,
      // rather than silently treating it as an opaque contrast measurement.
      if ((rgb(back)?.[3] ?? 1) !== 1 || (rgb(style.color)?.[3] ?? 1) !== 1) { failures.push("unmeasured translucent text/background"); continue; }
      const a = luminance(style.color), b = luminance(back);
      const ratio = (Math.max(a, b) + .05) / (Math.min(a, b) + .05);
      const large = parseFloat(style.fontSize) >= 24 || parseFloat(style.fontSize) >= 18.66 && Number(style.fontWeight) >= 700;
      if (ratio + .01 < (large ? 3 : 4.5)) failures.push(`${element.tagName} contrast ${ratio.toFixed(2)}: ${element.textContent.trim().slice(0, 40)}`);
      minimum = Math.min(minimum, ratio); texts++;
    }
    return { failures, texts, controls, minimum, fits: document.documentElement.scrollWidth <= innerWidth };
  });
  assert.deepEqual(result.failures, [], "rendered names and text contrast");
  assert.ok(result.texts > 0 && result.controls > 0, "presentation inspection must cover content and controls");
  assert.equal(result.fits, true, "page must fit; board/code scroll inside their own containers");
  return result;
}

/** Keyboard-only on desktop; same presentation/cancellation contract in touch CI. */
export async function checkAccessibility(page, { tap, back = () => page.keyboard.press("Escape") } = {}) {
  async function activate(selector) {
    await page.waitForFunction(selector => {
      const element = document.querySelector(selector);
      return element && !element.disabled && element.getClientRects().length;
    }, selector);
    if (tap) return tap(selector);
    // Traverse the actual tab order. Direct focus()/DOM click would miss traps
    // and unreachable controls. The bound detects a broken path deterministically.
    for (let count = 0; count < 160; count++) {
      if (selector.includes('type="radio"')) {
        const sameGroup = await page.evaluate(selector => {
          const target = document.querySelector(selector), active = document.activeElement;
          return active instanceof HTMLInputElement && active.type === "radio" && active.name === target.name;
        }, selector);
        if (sameGroup && !await page.evaluate(s => document.activeElement?.matches(s), selector)) {
          await page.keyboard.press("ArrowRight"); continue;
        }
      }
      if (await page.evaluate(s => document.activeElement?.matches(s), selector)) {
        const focus = await page.evaluate(() => {
          const style = getComputedStyle(document.activeElement);
          return { visible: document.activeElement.matches(":focus-visible"), width: parseFloat(style.outlineWidth), style: style.outlineStyle };
        });
        assert.ok(focus.visible && focus.width >= 2 && focus.style !== "none", `visible keyboard focus: ${selector}`);
        await page.keyboard.press(selector.includes('type="radio"') || selector.includes('type="checkbox"') ? "Space" : "Enter");
        return;
      }
      await page.keyboard.press("Tab");
    }
    assert.fail(`control unreachable by Tab: ${selector}`);
  }
  const command = (operation, parameters) => page.evaluate(async ({ operation, parameters }) => {
    const native = window.__TAURI_INTERNALS__;
    const session = native ? await native.invoke("shell_session") : await (await fetch("/host/session")).json();
    const request = JSON.stringify({ id: Array.from(crypto.getRandomValues(new Uint8Array(16))), operation, parameters });
    const response = native ? JSON.parse(await native.invoke("zedone_command", { invocation: { operation, request }, credential: session.credential.key }))
      : await (await fetch(`/v1/commands/${operation}`, { method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${session.credential.key}` }, body: request })).json();
    if (response.status !== "ok") throw new Error(`${operation}: ${response.error?.code}`);
    return response.result;
  }, { operation, parameters });
  const random = () => crypto.randomUUID().replaceAll("-", "");
  const space = random(), column = random(), card = random();
  const long = "Длинноеназваниебезпробелов".repeat(5);
  await command("space.create", { space, name: long });
  await command("column.create", { space, column, name: long });
  await command("column.create", { space, column: random(), name: "Другая колонка" });
  await command("card.create", { space, column, card, title: long, description: `# Читаемый текст\n${long}\n\n<b>Буквально</b>` });
  const initial = await command("card.read", { space, card });
  await command("card.edit", { space, card, expected: initial.version, title: long, description: `${initial.descriptions[0]}\nСледующая версия` });
  const before = await command("card.read", { space, card });
  const versions = await command("card.history", { space, card, after: null, limit: 100, expected: null });
  const route = async hash => {
    await page.evaluate(hash => { location.hash = hash; }, hash);
    const heading = hash === "#/settings" ? "main h1" : hash === "#/spaces" ? ".view-toolbar h2" : ".board-heading h2";
    await page.waitForFunction(({ hash, heading }) => location.hash === hash && document.querySelector(heading) === document.activeElement, { hash, heading });
  };
  const settled = () => page.waitForFunction(() => document.querySelector(".spaces-view")?.getAttribute("aria-busy") === "false");
  const closed = () => page.waitForFunction(() => !document.querySelector('[role="dialog"]'));
  const report = [];
  for (const theme of ["light", "dark"]) {
    await route("#/settings");
    await activate(`input[type="radio"][value="${theme}"]`);
    await page.waitForFunction(theme => document.documentElement.dataset.theme === theme, theme);
    report.push({ theme, screen: "settings", ...await checkPresentation(page) });
    await route(`#/spaces/${space}`); await settled();
    report.push({ theme, screen: "board", ...await checkPresentation(page) });
    const trigger = `.card-note[data-card="${card}"] .card-title`;
    await activate(trigger);
    await page.waitForFunction(() => !!document.querySelector(".card-editor [data-history-open]:not(:disabled)"));
    report.push({ theme, screen: "card", ...await checkPresentation(page) });
    await activate("[data-history-open]");
    await page.waitForFunction(() => !!document.querySelector(".history-list button") && !document.querySelector('.card-history [role="status"]'));
    await activate(".history-list button");
    report.push({ theme, screen: "history", ...await checkPresentation(page) });
    await activate("[data-history-close]");
    await page.waitForFunction(() => document.activeElement?.matches("[data-history-open]"));
    await back(); await closed();
    assert.equal(await page.evaluate(selector => document.activeElement?.matches(selector), trigger), true, "card close restores the initiating control");
    await activate(`.card-note[data-card="${card}"] .move-card`);
    await page.waitForFunction(() => document.querySelector(".move-dialog")?.contains(document.activeElement));
    report.push({ theme, screen: "move", ...await checkPresentation(page) });
    await back(); await closed();
    await activate(".space-history-cleanup");
    await page.waitForFunction(() => !!document.querySelector(".history-confirmation input"));
    report.push({ theme, screen: "cleanup", ...await checkPresentation(page) });
    await activate('.history-confirmation input[type="checkbox"]');
    await back(); await closed();
    // Enlarging content must preserve access through local scrolling.
    await page.evaluate(() => { document.documentElement.style.fontSize = "200%"; });
    try { report.push({ theme, screen: "board-200%", ...await checkPresentation(page) }); }
    finally { await page.evaluate(() => { document.documentElement.style.fontSize = ""; }); }
    assert.deepEqual(await command("card.read", { space, card }), before, "navigation/cancel/appearance do not change the card");
    assert.deepEqual(await command("card.history", { space, card, after: null, limit: 100, expected: null }), versions, "cancel does not clear or add history");
  }
  await route("#/settings");
  await activate('input[type="radio"][value="system"]');
  await route("#/spaces"); await settled();
  console.log(`Accessibility ${tap ? "touch" : "keyboard"}: both themes, names/contrast, focus/cancel, long text and 200% content passed (${report.length} screens).`);
}
