import assert from "node:assert/strict";

export async function checkAndroidHttp(page, { control, connect, tap, device, until, launch, app }) {
  const fill = (selector, value) => page.evaluate(({ selector, value }) => {
    const input = document.querySelector(selector);
    input.value = value;
    input.dispatchEvent(new Event("input", { bubbles: true }));
  }, { selector, value });
  await tap("#http-status");
  await page.waitForFunction(() => !document.querySelector("#http-status").disabled);
  assert.equal(await page.locator("#http-write").isChecked(), false);
  await fill("#http-address", "0.0.0.0");
  await tap("#http-start");
  await page.waitForFunction(() => document.querySelector("#http-error"));
  assert.equal((await control({ action: "status" })).endpoint, null);
  await fill("#http-address", "127.0.0.1");
  await fill("#http-port", "0");
  await tap("#http-start");
  await page.waitForFunction(() => document.querySelector("#http-active"));
  const initial = await control({ action: "status" });
  assert.equal(initial.access, "read");
  assert.equal(await page.locator("#http-error").count(), 0);
  let connection = await connect(initial);
  const echo = (token = initial.token) => fetch(`${connection.endpoint}/prototype/echo`, {
    method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
    body: JSON.stringify({ text: "Нативный жизненный цикл" }), signal: AbortSignal.timeout(3000),
  });
  const revoked = async () => {
    try { return (await echo()).status === 401; } catch { return true; }
  };
  try {
    assert.equal((await echo()).status, 200);
    // Navigation to Home is a real Activity onPause, with JS unrelated to revocation.
    device("shell", "input", "keyevent", "KEYCODE_HOME");
    await until(revoked, "native HTTP grant revoked after Home");
    launch();
    await until(async () => (await control({ action: "status" })).endpoint === null, "no automatic access after resume");
    await tap("#http-status");
    await page.waitForFunction(() => document.querySelector("#http-inactive") && !document.querySelector("#http-status").disabled);
    await tap("#http-start");
    await page.waitForFunction(() => document.querySelector("#http-active"));
    const resumed = await control({ action: "status" });
    assert.ok(resumed.token !== initial.token, "resume requires a new session key");
    await connection.close();
    connection = await connect(resumed);
    assert.equal((await echo(initial.token)).status, 401);
    assert.equal((await echo(resumed.token)).status, 200);
    device("shell", "input", "keyevent", "KEYCODE_SLEEP");
    await until(async () => { try { return (await echo(resumed.token)).status === 401; } catch { return true; } }, "screen sleep revokes HTTP");
    device("shell", "input", "keyevent", "KEYCODE_WAKEUP");
    device("shell", "wm", "dismiss-keyguard");
    launch();
    assert.equal((await control({ action: "status" })).endpoint, null);
    // Explicit write opt-in is passed through the actual Vue form.
    await tap("#http-status");
    await page.waitForFunction(() => !document.querySelector("#http-status").disabled);
    await tap("#http-write");
    await tap("#http-start");
    await page.waitForFunction(() => document.querySelector("#http-active"));
    assert.equal((await control({ action: "status" })).access, "write");
    await tap("#http-stop");
    await page.waitForFunction(() => document.querySelector("#http-inactive") && !document.querySelector("#http-stop").disabled);
    assert.equal((await control({ action: "status" })).token, null);
    console.log("APK HTTP form: default read, failed bind/retry, explicit write, stop; native Home/resume and sleep/wake revoke without automatic restart.");
  } finally {
    device("shell", "input", "keyevent", "KEYCODE_WAKEUP");
    device("shell", "wm", "dismiss-keyguard");
    launch();
    await control({ action: "stop" });
    await connection.close();
  }
}
