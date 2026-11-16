import assert from "node:assert/strict";
import { nativeControls } from "./check-ui-android-scenarios.mjs";
import { observeShellReads, observeShellReadLifecycle, withShellFailureDiagnostics } from "./check-ui-diagnostics.mjs";

async function checkReadLifecycle(secret) {
  const listeners = new Map();
  const snapshot = observeShellReadLifecycle({ on: (event, listener) => listeners.set(event, listener) });
  const request = (path, failure = "net::ERR_ABORTED") => ({ url: () => `http://127.0.0.1${path}?ignored=${secret}`, failure: () => ({ errorText: failure }) });
  const tracked = Array.from({ length: 35 }, () => request("/v1/commands/tag.list"));
  for (const item of tracked) listeners.get("request")(item);
  assert.equal(snapshot().length, 30); assert.equal(snapshot()[0].sequence, 6);
  assert.ok(snapshot().every(item => item.phase === "started" && item.elapsedMs >= 0));
  let release;
  const pending = listeners.get("response")({ request: () => tracked[34], status: () => 200, json: () => new Promise(resolve => { release = resolve; }) });
  assert.equal(snapshot().at(-1).phase, "headers"); assert.equal(snapshot().at(-1).body, "pending");
  listeners.get("requestfinished")(tracked[34]);
  release({ status: "error", error: { code: "access_denied" } }); await pending;
  assert.equal(snapshot().at(-1).phase, "finished"); assert.equal(snapshot().at(-1).body, "valid");
  await listeners.get("response")({ request: () => tracked[33], status: () => 200, json: async () => ({ credential: secret, status: secret }) });
  assert.equal(snapshot().at(-2).body, "invalid");
  await listeners.get("response")({ request: () => tracked[32], status: () => 200, json: async () => { throw new Error(secret); } });
  listeners.get("requestfailed")(tracked[32]);
  assert.equal(snapshot().at(-3).body, "unreadable"); assert.equal(snapshot().at(-3).failure, "net::ERR_ABORTED");
  const session = request("/host/session", secret), ignored = request("/v1/commands/access.issue");
  listeners.get("request")(session); listeners.get("request")(ignored);
  let sessionReads = 0;
  for (const item of [session, ignored]) await listeners.get("response")({ request: () => item, status: () => 200, json: async () => { sessionReads++; return { credential: secret }; } });
  assert.equal(sessionReads, 0, "never read host credentials or unrelated command bodies");
  listeners.get("requestfailed")(session);
  assert.equal(snapshot().at(-1).failure, "failed", "raw transport details cannot leak a URL or secret");
  assert.equal(snapshot().length, 30); assert.ok(!JSON.stringify(snapshot()).includes(secret));
  const detached = snapshot(); detached[0].phase = "changed";
  assert.notEqual(snapshot()[0].phase, "changed");
}

export async function checkShellFailureDiagnostics(browser, url, secret) {
  await checkReadLifecycle(secret);
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
    const readLifecycle = observeShellReadLifecycle(touch);
    let stalledRoute, captured;
    const intercepted = new Promise(resolve => { captured = resolve; });
    const pattern = "**/v1/commands/tag.list";
    await touch.route(pattern, route => { stalledRoute = route; captured(); });
    const fetching = touch.evaluate(() => fetch("/v1/commands/tag.list", { method: "POST" }).then(() => "unexpected response", () => "failed"));
    await intercepted;
    try {
      await assert.rejects(withShellFailureDiagnostics(touch, { label: "host touch", stage: "trash", reads, readLifecycle }, async () => { throw original; }), error => error === original);
      const pendingState = JSON.parse(lines[1][1]);
      const pendingRead = pendingState.readLifecycle.find(item => item.path === "/v1/commands/tag.list");
      assert.equal(pendingRead.phase, "started"); assert.ok(pendingRead.elapsedMs >= 0);
      assert.equal(pendingRead.http, undefined);
      assert.ok(!JSON.stringify(pendingState).includes(secret));
    } finally {
      const failed = touch.waitForEvent("requestfailed", request => request.url().endsWith("/v1/commands/tag.list"));
      await stalledRoute.abort(); await failed;
      assert.equal(await fetching, "failed"); await touch.unroute(pattern);
    }
    assert.equal(readLifecycle().find(item => item.path === "/v1/commands/tag.list").phase, "failed");
    console.error = () => { throw new Error("controlled diagnostic failure"); };
    await assert.rejects(withShellFailureDiagnostics(touch, { label: "host touch", stage: "tags", reads, readLifecycle }, async () => { throw original; }), error => error === original);
    console.error = report;
    console.log("Diagnostics: failing touch page/stage, pending/headers/finished/failed reads, bounded metadata, secret exclusion and original error passed.");
  } finally { console.error = report; await context.close(); }
}

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
