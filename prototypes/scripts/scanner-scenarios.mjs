import assert from "node:assert/strict";

// Real Android permission/camera lifecycle; no simulated recognition result.
export async function checkScanner(page, { device, tap, until, app }) {
  const airplane = device("shell", "settings", "get", "global", "airplane_mode_on");
  const wifi = device("shell", "settings", "get", "global", "wifi_on");
  device("shell", "cmd", "connectivity", "airplane-mode", "enable");
  device("shell", "svc", "wifi", "disable");
  try {
    const status = () => page.locator("#scanner-status").textContent();
    const idle = () => page.waitForFunction(() => document.querySelector("#scan-start") && !document.querySelector("#scan-start").disabled);
    const active = () => page.waitForFunction(() => document.body.classList.contains("qr-scanning"));
    const cameraClients = () => device("shell", "dumpsys", "media.camera").split("== Camera service events log")[0];
    const released = () => until(() => !cameraClients().includes(`Client Package Name: ${app}`), "camera client released");
    // ACL exposes just scan/cancel/permission checks, not unrelated scanner actions.
    for (const command of ["vibrate", "open_app_settings"]) await assert.rejects(page.evaluate(command =>
      window.__TAURI_INTERNALS__.invoke(`plugin:barcode-scanner|${command}`), command));
    await tap("#scan-start");
    await until(() => device("shell", "dumpsys", "window").includes("GrantPermissionsActivity"), "real permission prompt");
    // OS dialog automation uses the actual node's bounds, not fixed screen coordinates.
    function permissionButton(id) {
      device("shell", "uiautomator", "dump", "/data/local/tmp/zedone-permission.xml");
      const xml = device("shell", "cat", "/data/local/tmp/zedone-permission.xml");
      const node = xml.match(new RegExp(`<node[^>]*resource-id="com\\.android\\.permissioncontroller:id/${id}"[^>]*>`))?.[0];
      assert.ok(node, `Android permission button ${id}`);
      const bounds = node.match(/bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/).slice(1).map(Number);
      device("shell", "input", "tap", String(Math.floor((bounds[0] + bounds[2]) / 2)), String(Math.floor((bounds[1] + bounds[3]) / 2)));
      device("shell", "rm", "/data/local/tmp/zedone-permission.xml");
    }
    permissionButton("permission_deny_button");
    await idle();
    assert.match(await status(), /Нет разрешения/);
    await released();

    // Grant through the real OS dialog; scan runs on AOSP without GMS/model.
    await tap("#scan-start");
    await until(() => device("shell", "dumpsys", "window").includes("GrantPermissionsActivity"), "second permission prompt");
    permissionButton("permission_allow_foreground_only_button");
    await active();
    await until(() => cameraClients().includes(`Client Package Name: ${app}`), "real camera opened");
    await tap("#scan-cancel");
    await idle();
    await released();
    assert.match(await status(), /отменено/);
    // Cancellation during asynchronous CameraX setup must not open a late camera.
    await tap("#scan-start");
    await active();
    await tap("#scan-cancel");
    await idle();
    await new Promise(resolve => setTimeout(resolve, 1500));
    await released();
    await tap("#scan-start");
    await active();
    await until(() => cameraClients().includes(`Client Package Name: ${app}`), "camera reopened");
    device("shell", "input", "keyevent", "KEYCODE_BACK");
    await idle();
    await released();
    assert.ok(device("shell", "dumpsys", "activity", "activities").includes(`${app}/.MainActivity`));

    await tap("#scan-start");
    await active();
    await until(() => cameraClients().includes(`Client Package Name: ${app}`), "camera before Home");
    device("shell", "input", "keyevent", "KEYCODE_HOME");
    await released();
    device("shell", "am", "start", "-W", "-n", `${app}/.MainActivity`);
    await idle();
    await released();

    // No result from ML Kit cannot leave the form blocked forever or say bad invitation.
    await tap("#scan-start");
    await active();
    await page.waitForFunction(() => document.querySelector("#scan-start"), null, { timeout: 30000 });
    await released();
    assert.match(await status(), /не прочитан.*20 секунд/);
    assert.equal(await page.locator("#invitation-error").count(), 0);
    assert.equal(await page.locator("#invitation-result").count(), 0);
    assert.equal(await page.locator("#invitation-submit").isDisabled(), false);
    // Camera permission can exist while camera access is blocked by the OS.
    const cameraMode = device("shell", "cmd", "appops", "get", "--uid", app, "CAMERA").match(/Uid mode: CAMERA: ([a-z_]+)/)?.[1] || "default";
    device("shell", "cmd", "appops", "set", "--uid", app, "CAMERA", "ignore");
    try {
      await tap("#scan-start");
      await active();
      await page.waitForFunction(() => document.querySelector("#scan-start"), null, { timeout: 30000 });
      await released();
      assert.equal(await page.locator("#invitation-error").count(), 0);
      assert.equal(await page.locator("#invitation-result").count(), 0);
      assert.equal(await page.locator("#invitation-submit").isDisabled(), false);
    } finally { device("shell", "cmd", "appops", "set", "--uid", app, "CAMERA", cameraMode); }
    // Inject a decoded reply at the transport boundary to test the UI's success
    // and late-reply paths. This does NOT establish optical decoding by ML Kit.
    const data = { address: "http://192.0.2.1:4317/ёж", code: 'Ёж café 😀 <b>literal</b>' };
    async function fakeScan(text, late = false, failInspection = false) {
      await page.evaluate(({ text, late, failInspection }) => {
        const original = window.ipc;
        window.ipc = { postMessage(message) {
          const request = JSON.parse(message);
          if (request.cmd === "plugin:barcode-scanner|scan") {
            window.ipc = original;
            const deliver = () => window.__TAURI_INTERNALS__.runCallback(request.callback, { content: text, format: "QR_CODE", bounds: null });
            if (late) window.__lateScan = deliver;
            else setTimeout(deliver, 0);
            if (failInspection) window.ipc = { postMessage(message) {
              if (JSON.parse(message).cmd === "invitation") { window.ipc = original; throw new Error("Injected inspection failure"); }
              original.postMessage(message);
            } };
            return;
          }
          original.postMessage(message);
        } };
      }, { text, late, failInspection });
      await tap("#scan-start");
    }
    await fakeScan(JSON.stringify(data), false, true);
    await page.waitForFunction(() => document.querySelector("#invitation-error"));
    assert.equal(await page.locator("#invitation-scanned").textContent(), JSON.stringify(data));
    await tap("#invitation-retry-qr");
    await page.waitForFunction(() => document.querySelector("#invitation-result"));
    assert.deepEqual(JSON.parse(await page.locator("#invitation-result").textContent()), data);
    await tap("#invitation-clear");
    await fakeScan("not JSON");
    await page.waitForFunction(() => document.querySelector("#invitation-error"));
    assert.match(await page.locator("#invitation-error").textContent(), /поля address и code/);
    await fakeScan(JSON.stringify(data), true);
    await active();
    assert.equal(await page.locator("#invitation-error").count(), 0, "A new scan must not show the previous data error");
    await tap("#scan-cancel");
    await idle();
    await page.evaluate(() => { window.__lateScan(); delete window.__lateScan; });
    assert.equal(await page.locator("#invitation-result").count(), 0);
    assert.equal(await page.locator("#invitation-scanned").textContent(), "not JSON", "Late reply must not replace the previous raw input");
    await tap("#invitation-clear");
    // An unconfirmed cancel must prevent another scan but permit manual inspection.
    await fakeScan(JSON.stringify(data), true);
    await active();
    await page.evaluate(() => {
      const original = window.ipc;
      window.ipc = { postMessage(message) {
        if (JSON.parse(message).cmd === "plugin:barcode-scanner|cancel") { window.ipc = original; return; }
        original.postMessage(message);
      } };
    });
    await tap("#scan-cancel");
    await page.waitForFunction(() => document.querySelector("#scan-start")?.disabled);
    assert.match(await status(), /Не удалось подтвердить остановку/);
    assert.equal(await page.locator("#invitation-submit").isDisabled(), false);
    await page.evaluate(() => { window.__lateScan(); delete window.__lateScan; });
    assert.equal(await page.locator("#invitation-result").count(), 0);
    await tap("#invitation-clear");
    assert.match(await status(), /Не удалось подтвердить остановку/);
    // Revocation kills the process on Android; the runner restores it after disconnect.
    console.log("Real Android scanner: permission denial/grant, camera cancel/reopen, Back/Home cleanup, blocked camera and missing GMS/model timeout passed. Optical decoding NOT tested.");
    console.log("Injected scanner replies: exact Unicode, raw input retained after IPC failure/retry, malformed data and ignored late reply passed; NOT a camera decoding test.");
  } finally {
    device("shell", "cmd", "connectivity", "airplane-mode", airplane === "1" ? "enable" : "disable");
    device("shell", "svc", "wifi", wifi === "1" ? "enable" : "disable");
  }
}
