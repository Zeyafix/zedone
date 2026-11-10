import assert from "node:assert/strict";
import { nativeControls } from "./check-ui-android-scenarios.mjs";

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
