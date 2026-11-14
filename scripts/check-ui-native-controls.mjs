import assert from "node:assert/strict";
import { nativeControls } from "./check-ui-android-scenarios.mjs";
import { observeShellReads, withShellFailureDiagnostics } from "./check-ui-diagnostics.mjs";

/** Host-only negative control: failure evidence must belong to the live touch
 * page, survive context disposal and preserve the exact scenario error. */
export async function checkShellFailureDiagnostics(browser, url, secret) {
  const context = await browser.newContext({ viewport: { width: 360, height: 760 }, hasTouch: true, isMobile: true });
  const report = console.error;
  try {
    const main = await context.newPage(), touch = await context.newPage();
    await main.goto(url);
    await touch.goto(`${url}/#/settings`);
    await touch.getByRole("heading", { name: "Настройки", exact: true, level: 1 }).waitFor();
    let responseListener;
    const reads = observeShellReads({ on: (event, listener) => { assert.equal(event, "response"); responseListener = listener; } });
    for (let index = 0; index < 35; index++) await responseListener({
      url: () => `${url}/v1/commands/tag.list`, status: () => 200,
      json: async () => ({ status: "error", error: { code: "access_denied", credential: secret }, credential: secret, result: { tags: [], credential: secret } }),
    });
    assert.equal(reads.length, 30);
    const lines = [];
    console.error = (...args) => lines.push(args);
    let original;
    let attempts = 0;
    await assert.rejects(withShellFailureDiagnostics(touch, { label: "host touch", stage: "tags", reads }, async () => {
      attempts++;
      // This impossible probe has its own short deadline; the real UI actions
      // retain their existing 10s deadline and are never retried by diagnostics.
      try { await touch.waitForFunction(() => false, undefined, { timeout: 50 }); }
      catch (error) { original = error; throw error; }
    }), error => error === original && error.name === "TimeoutError");
    assert.equal(attempts, 1); assert.equal(lines.length, 1);
    assert.equal(lines[0][0], "UI failure state:");
    const state = JSON.parse(lines[0][1]);
    assert.equal(state.label, "host touch"); assert.equal(state.stage, "tags");
    assert.equal(state.hash, "#/settings"); assert.notEqual(state.hash, await main.evaluate(() => location.hash));
    assert.equal(state.mounted, true); assert.equal(state.session, "configured");
    assert.ok(!JSON.stringify(lines).includes(secret));
    assert.equal(state.reads.length, 30); assert.equal(state.reads[0].path, "/v1/commands/tag.list");
    assert.equal(state.reads[0].error, "access_denied");
    console.error = () => { throw new Error("controlled diagnostic failure"); };
    await assert.rejects(withShellFailureDiagnostics(touch, { label: "host touch", stage: "tags", reads }, async () => { throw original; }), error => error === original);
    console.error = report;
    console.log("Diagnostics: failing touch page/stage before disposal, bounded metadata, secret exclusion and original error passed.");
  } finally { console.error = report; await context.close(); }
}

/** Host-only regression: old WebView lacks this API in every execution realm. */
export async function withoutVisibilityApi(page) {
  const cdp = await page.context().newCDPSession(page);
  const contexts = [];
  cdp.on("Runtime.executionContextCreated", ({ context }) => contexts.push(context));
  try {
    await cdp.send("Runtime.enable");
    assert.ok(contexts.some(context => context.auxData?.type === "isolated"), "Check the automation realm as well as the page");
    for (const context of contexts) {
      const result = await cdp.send("Runtime.evaluate", {
        expression: "delete Element.prototype.checkVisibility; typeof Element.prototype.checkVisibility",
        contextId: context.id,
      });
      assert.equal(result.result.value, "undefined");
    }
  } finally { await cdp.detach(); }
}

/** Real CDP touch against changing layout, without application state or clicks. */
export async function checkNativeControls(browser) {
  const context = await browser.newContext({ viewport: { width: 360, height: 760 }, hasTouch: true, isMobile: true });
  try {
    const page = await context.newPage();
    await page.setContent(`<button id="target" style="position:fixed;left:20px;top:60px;width:240px;height:32px">Open</button>`);
    const cdp = await context.newCDPSession(page);
    let touches = 0;
    const adapter = new Proxy(page, { get(target, name) {
      if (name === "context") return () => ({ newCDPSession: async () => ({ send: async (method, parameters) => {
        if (method === "Input.dispatchTouchEvent" && parameters.type === "touchStart") {
          touches++;
          // One display frame of transport latency exposes stale coordinates.
          await page.evaluate(() => new Promise(resolve => requestAnimationFrame(resolve)));
        }
        return cdp.send(method, parameters);
      } }) });
      const value = target[name];
      return typeof value === "function" ? value.bind(target) : value;
    } });
    const { tap } = await nativeControls(adapter);
    await page.evaluate(() => {
      const button = document.querySelector("#target");
      window.clicks = 0;
      window.settled = false;
      button.addEventListener("click", () => { window.clicks++; window.clickedSettled = window.settled; });
      let frames = 0;
      function move() {
        button.style.top = `${60 + (++frames % 2) * 120}px`;
        if (frames < 24) requestAnimationFrame(move);
        else window.settled = true;
      }
      requestAnimationFrame(move);
    });
    await tap("#target");
    assert.deepEqual(await page.evaluate(() => [window.clicks, window.clickedSettled]), [1, true],
      "A single touch must activate the settled target, not its old position");
    assert.equal(touches, 1, "Never retry a lost touch");
    // The application exposes a pending refresh through aria-busy.
    await page.evaluate(() => {
      window.settled = false;
      document.body.setAttribute("aria-busy", "true");
      let frames = 0;
      function refresh() {
        if (++frames < 12) requestAnimationFrame(refresh);
        else { document.body.setAttribute("aria-busy", "false"); window.settled = true; }
      }
      requestAnimationFrame(refresh);
    });
    await tap("#target");
    assert.deepEqual(await page.evaluate(() => [window.clicks, window.clickedSettled]), [2, true]);
    assert.equal(touches, 2);
    console.log("Native touch: moving layout, pending refresh, stable hit target and exactly one touch per action passed.");
  } finally { await context.close(); }
}
