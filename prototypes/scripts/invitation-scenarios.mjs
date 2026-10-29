import assert from "node:assert/strict";

// Shared UI/API contract; the camera itself is exercised only by Android tests.
export async function checkInvitation(page, { baseUrl, tap } = {}) {
  const click = tap || (selector => page.locator(selector).click());
  const http = async body => {
    const response = await fetch(`${baseUrl}/prototype/invitation`, {
      method: "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body), signal: AbortSignal.timeout(10000),
    });
    const data = response.headers.get("content-type")?.includes("application/json") ? await response.json() : await response.text();
    return { status: response.status, data };
  };
  const fill = data => page.evaluate(data => {
    for (const [key, value] of Object.entries(data)) {
      const input = document.querySelector(`#invitation-${key}`);
      input.value = value;
      input.dispatchEvent(new Event("input", { bubbles: true }));
    }
  }, data);
  const data = { address: " http://192.0.2.1:4317/ёж ", code: 'café 😀 <script>alert(1)</script>' };
  const requests = [ { source: "manual", ...data }, { source: "qr", text: JSON.stringify(data) } ];
  const android = await page.evaluate(() => !!window.__TAURI_INTERNALS__);
  for (const request of requests) {
    assert.deepEqual(await http(request), { status: 200, data });
    if (android) assert.deepEqual(await page.evaluate(request => window.__TAURI_INTERNALS__.invoke("invitation", { request }), request), data);
  }
  for (const request of [{ source: "qr", text: "no JSON" }, { source: "manual", address: " ", code: "x" },
    { source: "qr", text: '{"address":"x","code":"y","extra":true}' }, { source: "scan" }, { source: "qr", text: null }]) {
    assert.equal((await http(request)).status, 422);
    if (android) await assert.rejects(page.evaluate(request => window.__TAURI_INTERNALS__.invoke("invitation", { request }), request));
  }
  const contacts = [];
  const track = request => { if (request.url().includes("192.0.2.1")) contacts.push(request.url()); };
  page.on("request", track);
  await fill(data);
  await click("#invitation-submit");
  await page.waitForFunction(() => document.querySelector("#invitation-result"));
  assert.deepEqual(await page.locator("#invitation-result").textContent().then(JSON.parse), data);
  await fill({ code: " " });
  await page.waitForFunction(() => !document.querySelector("#invitation-result"));
  await click("#invitation-submit");
  await page.waitForFunction(() => document.querySelector("#invitation-error"));
  assert.equal(await page.locator("#invitation-address").inputValue(), data.address);
  await fill(data);
  // Simulate lost/failed transport, never scanner success.
  if (android) await page.evaluate(() => {
    const original = window.ipc;
    window.ipc = { postMessage(message) {
      if (JSON.parse(message).cmd === "invitation") { window.ipc = original; throw new Error("Injected invitation failure"); }
      original.postMessage(message);
    } };
  });
  else await page.route("**/prototype/invitation", route => route.abort());
  await click("#invitation-submit");
  await page.waitForFunction(() => document.querySelector("#invitation-error"));
  assert.equal(await page.locator("#invitation-code").inputValue(), data.code);
  if (!android) await page.unroute("**/prototype/invitation");
  await click("#invitation-submit");
  await page.waitForFunction(() => document.querySelector("#invitation-result"));
  await click("#invitation-clear");
  await page.waitForFunction(() => !document.querySelector("#invitation-result") && !document.querySelector("#invitation-error"));
  assert.equal(await page.locator("#invitation-address").inputValue(), "");
  assert.equal(await page.locator("#invitation-code").inputValue(), "");
  assert.deepEqual(contacts, [], "Inspected address must never be contacted");
  page.off("request", track);
  console.log("Synthetic invitation: QR/manual HTTP/IPC parity, invalid data, transport failure/retry and clear passed.");
}
