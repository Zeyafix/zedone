import assert from "node:assert/strict";

export async function checkQrInput(browser, url, text, address, offer) {
  const context = await browser.newContext({ viewport: { width: 360, height: 800 } });
  try {
    await context.addInitScript(() => {
      window.isTauri = true;
      const state = window.qrTest = { permission: "denied", content: "", scans: 0, cancels: 0, cancelFailures: 0, decodeFailures: 0, deliveries: [] };
      window.__TAURI_INTERNALS__ = { async invoke(operation, input) {
        if (operation.startsWith("plugin:barcode-scanner|")) {
          if (operation.endsWith("check_permissions") || operation.endsWith("request_permissions")) return { camera: state.permission };
          if (operation.endsWith("cancel")) { ++state.cancels; if (state.cancelFailures > 0) { --state.cancelFailures; throw new Error("native cancel failed"); } return; }
          if (operation.endsWith("scan")) {
            ++state.scans;
            if (JSON.stringify(input.formats) !== '["QR_CODE"]' || input.windowed !== true || input.cameraDirection !== "back") throw new Error("Wrong scanner options");
            if (state.content !== undefined) return { content: state.content };
            return new Promise(resolve => state.deliveries.push(resolve));
          }
          throw new Error("Unexpected scanner permission");
        }
        if (operation === "shell_session") return (await fetch("/host/session")).json();
        const headers = { "content-type": "application/json", authorization: `Bearer ${input.credential}` };
        if (operation === "device_session") {
          if (input.operation === "session.decode_qr" && state.decodeFailures > 0) { --state.decodeFailures; throw new Error("test transport unavailable"); }
          return (await fetch(`/host/devices/${input.operation}`, { method: "POST", headers, body: input.parameters })).json();
        }
        if (operation === "zedone_command") return (await fetch(`/v1/commands/${input.invocation.operation}`, { method: "POST", headers, body: input.invocation.request })).text();
        throw new Error("Unexpected native operation");
      } };
    });
    const page = await context.newPage();
    const errors = []; page.on("pageerror", error => errors.push(error.message));
    await page.goto(`${url}/#/settings`);
    const start = page.locator("#device-scan-start");
    await start.waitFor();
    await page.locator("#peer-address").fill("127.0.0.1:45670");
    await page.locator("#peer-code").fill("manual draft");
    await page.locator("#peer-label").fill("Мой телефон");
    await start.click();
    await page.locator("[data-scanner-status]").filter({ hasText: "Нет разрешения" }).waitFor();
    assert.equal(await page.locator("#peer-code").inputValue(), "manual draft");
    assert.equal(await page.evaluate(() => window.qrTest.scans), 0);
    await page.evaluate(text => { window.qrTest.permission = "granted"; window.qrTest.content = text; }, text);
    await start.click();
    await page.waitForFunction(offer => document.querySelector("#peer-code")?.value === offer, offer);
    assert.equal(await page.locator("#peer-address").inputValue(), address);
    assert.equal(await page.locator("#peer-label").inputValue(), "Мой телефон");
    await page.locator('[role="status"]').filter({ hasText: "Запрос пока не отправлен" }).waitFor();
    assert.equal(await page.evaluate(() => document.documentElement.classList.contains("qr-scanning")), false);
    await page.evaluate(() => { window.qrTest.content = "unrelated QR"; });
    await start.click();
    await page.locator('[role="alert"]').filter({ hasText: "Этот QR" }).waitFor();
    assert.equal(await page.locator("#peer-code").inputValue(), offer);
    await page.evaluate(text => { window.qrTest.content = text; window.qrTest.decodeFailures = 1; }, text);
    await start.click();
    await page.locator('[role="alert"]').filter({ hasText: "Не удалось проверить" }).waitFor();
    await page.getByRole("button", { name: "Проверить прочитанный QR повторно", exact: true }).click();
    await page.locator('[role="status"]').filter({ hasText: "Запрос пока не отправлен" }).waitFor();
    await page.evaluate(() => { window.qrTest.content = undefined; });
    await start.click();
    await page.locator("#device-scan-cancel").waitFor();
    assert.equal(await page.evaluate(() => getComputedStyle(document.documentElement).backgroundColor), "rgba(0, 0, 0, 0)");
    await page.locator("#device-scan-cancel").click();
    await start.waitFor();
    await page.evaluate(() => window.qrTest.deliveries[0]({ content: "late unrelated QR" }));
    assert.equal(await page.locator("#peer-code").inputValue(), offer);
    await start.click(); await page.locator("#device-scan-cancel").waitFor();
    await page.evaluate(() => window.dispatchEvent(new Event("zedone-background")));
    await start.waitFor();
    await page.evaluate(() => window.qrTest.deliveries[1]({ content: "late background" }));
    await page.getByRole("button", { name: "Очистить данные подключения", exact: true }).click();
    for (const id of ["peer-address", "peer-code", "peer-label"]) assert.equal(await page.locator(`#${id}`).inputValue(), "");
    assert.equal(await page.getByRole("button", { name: "Проверить прочитанный QR повторно", exact: true }).count(), 0);
    assert.equal(await page.evaluate(() => Object.values(localStorage).some(value => value.includes("zd1:") || value.includes("zedone-pairing-v1"))), false);
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
    await page.evaluate(() => { window.qrTest.cancelFailures = 1; });
    await start.click(); await page.locator("#device-scan-cancel").waitFor();
    await page.locator("#device-scan-cancel").click();
    await page.locator("[data-scanner-status]").filter({ hasText: "Остановка камеры не подтверждена" }).waitFor();
    await page.evaluate(() => { location.hash = "/spaces"; });
    await start.waitFor({ state: "detached" });
    const scans = await page.evaluate(() => window.qrTest.scans);
    await page.evaluate(() => { location.hash = "/settings"; });
    await start.waitFor(); await start.click();
    await page.locator("[data-scanner-status]").filter({ hasText: "Остановка камеры не подтверждена" }).waitFor();
    assert.equal(await start.isDisabled(), true);
    assert.equal(await page.evaluate(() => window.qrTest.scans), scans);
    await page.locator("#peer-address").fill("127.0.0.1:45670");
    assert.equal(await page.evaluate(() => getComputedStyle(document.querySelector("#peer-address")).visibility), "visible");
    assert.deepEqual(errors, []);
    console.log("Synthetic scanner source → actual host API/form: permission refusal, exact input without submit, invalid/failed decode and retry, cancel/late/background/manual/clear/secrecy and failed teardown across view recreation passed; optical camera is separate.");
  } finally { await context.close(); }
}
