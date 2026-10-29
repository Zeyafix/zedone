import assert from "node:assert/strict";
import jsQR from "jsqr";
import { PNG } from "pngjs";

function decode(image, text) {
  const code = jsQR(new Uint8ClampedArray(image.data), image.width, image.height, { inversionAttempts: "dontInvert" });
  assert.ok(code, "Independent decoder must read the entire QR");
  assert.deepEqual(Buffer.from(code.binaryData), Buffer.from(text, "utf8"));
  assert.equal(code.data, text);
  assert.equal(code.chunks[0].assignmentNumber, 26, "Explicit UTF-8 ECI");
}

// Shared by desktop and the real APK. jsQR is a test oracle, not the camera scanner.
export async function checkQr(page, { baseUrl, tap } = {}) {
  const click = tap || (selector => page.locator(selector).click());
  const http = async body => {
    const response = await fetch(`${baseUrl}/prototype/qr`, {
      method: "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body), signal: AbortSignal.timeout(10000),
    });
    const data = response.headers.get("content-type")?.includes("application/json")
      ? await response.json() : await response.text();
    return { status: response.status, data };
  };
  const fill = text => page.evaluate(text => {
    const input = document.querySelector("#qr-text");
    input.value = text;
    input.dispatchEvent(new Event("input", { bubbles: true }));
  }, text);
  const scale = value => page.evaluate(value => {
    const select = document.querySelector("#qr-scale");
    select.value = String(value);
    select.dispatchEvent(new Event("change", { bubbles: true }));
  }, value);
  for (const text of ["", '{"address":"http://192.0.2.1:4317","code":"SYNTHETIC-ONLY"}', 'Ёж café 😀\n"\\<script>x</script>\u0000', "x".repeat(1000), "x".repeat(2330)]) {
    const { status, data } = await http({ text });
    assert.equal(status, 200);
    const side = data.modules.length + 8;
    const size = side * 4;
    const pixels = new Uint8ClampedArray(size * size * 4).fill(255);
    for (let y = 4; y < side - 4; y++) for (let x = 4; x < side - 4; x++) {
      if (data.modules[y - 4][x - 4]) for (let dy = 0; dy < 4; dy++) for (let dx = 0; dx < 4; dx++) {
        const offset = ((y * 4 + dy) * size + x * 4 + dx) * 4;
        pixels.fill(0, offset, offset + 3);
      }
    }
    decode({ data: pixels, width: size, height: size }, text);
    if (tap) {
      assert.deepEqual(await page.evaluate(text => window.__TAURI_INTERNALS__.invoke("qr", { request: { text } }), text), data);
    }
    await fill(text);
    assert.equal(await page.locator("#qr-viewport svg").count(), 0, "Editing invalidates the old QR");
    await click("#qr-generate");
    await page.waitForFunction(() => document.querySelector("#qr-viewport svg"));
    assert.equal(await page.locator("#qr-text").inputValue(), text);
    assert.equal(await page.locator("#qr-viewport svg").getAttribute("viewBox"), `0 0 ${side} ${side}`);
    // Small fixtures fit the real WebView: decode its rendered pixels at each scale.
    for (const value of text === "" ? [2, 4, 8] : [2]) {
      await scale(value);
      assert.equal(await page.locator("#qr-viewport svg").getAttribute("width"), String(side * value));
      if (text.length < 100) {
        await page.evaluate(() => document.querySelector("#qr-viewport").scrollIntoView({ block: "center" }));
        const box = await page.evaluate(() => {
          const { x, y, width, height } = document.querySelector("#qr-viewport svg").getBoundingClientRect();
          return { x, y, width, height };
        });
        const image = PNG.sync.read(await page.screenshot({ clip: box }));
        decode(image, text);
        // All four quiet zones must remain opaque white in the rendered SVG.
        const border = Math.floor(image.width / side * 4);
        for (let y = 0; y < image.height; y++) for (let x = 0; x < image.width; x++) {
          if (x >= border && y >= border && x < image.width - border && y < image.height - border) continue;
          assert.deepEqual([...image.data.subarray((y * image.width + x) * 4, (y * image.width + x) * 4 + 4)], [255, 255, 255, 255]);
        }
      }
    }
    await scale(8);
    const layout = await page.evaluate(() => {
      const viewport = document.querySelector("#qr-viewport");
      viewport.scrollLeft = viewport.scrollWidth;
      return { left: viewport.scrollLeft, width: viewport.clientWidth, full: viewport.scrollWidth,
        page: document.documentElement.scrollWidth, window: window.innerWidth };
    });
    assert.ok(Math.abs(layout.left + layout.width - layout.full) <= 1
      && layout.page <= layout.window, `Large QR must scroll without page overflow: ${JSON.stringify(layout)}`);
    await scale(4);
  }
  for (const body of [{}, { text: null }, { text: 42 }, { text: "x", scale: 4 }]) {
    assert.equal((await http(body)).status, 422);
    if (tap) await assert.rejects(page.evaluate(request => window.__TAURI_INTERNALS__.invoke("qr", { request }), body));
  }
  const tooLong = "Ё".repeat(1166);
  assert.deepEqual(await http({ text: tooLong }), { status: 422, data: "too_long" });
  if (tap) assert.equal(await page.evaluate(async text => {
    try { return await window.__TAURI_INTERNALS__.invoke("qr", { request: { text } }); }
    catch (error) { return error; }
  }, tooLong), "too_long");
  await fill(tooLong);
  await click("#qr-generate");
  await page.waitForFunction(() => document.querySelector("#qr-error")?.textContent.includes("не помещаются"));
  assert.equal(await page.locator("#qr-text").inputValue(), tooLong);
  assert.equal(await page.locator("#qr-viewport svg").count(), 0);
  await fill("Повтор");
  if (tap) {
    await page.evaluate(() => {
      const original = window.ipc;
      window.ipc = { postMessage() { window.ipc = original; throw new Error("Injected QR bridge failure"); } };
    });
  } else await page.route("**/prototype/qr", route => route.abort());
  await click("#qr-generate");
  await page.waitForFunction(() => document.querySelector("#qr-error")?.textContent.includes("повторите"));
  assert.equal(await page.locator("#qr-text").inputValue(), "Повтор");
  if (!tap) await page.unroute("**/prototype/qr");
  await click("#qr-generate");
  await page.waitForFunction(() => document.querySelector("#qr-viewport svg") && !document.querySelector("#qr-error"));
  if (tap) {
    await page.evaluate(() => {
      const original = window.ipc;
      window.ipc = { postMessage() { window.ipc = original; } };
    });
  } else await page.route("**/prototype/qr", async route => {
    await new Promise(resolve => setTimeout(resolve, 6000));
    await route.abort().catch(() => {});
  });
  await click("#qr-generate");
  await page.waitForFunction(() => document.querySelector("#qr-error")?.textContent.includes("повторите"));
  assert.equal(await page.locator("#qr-text").inputValue(), "Повтор");
  if (!tap) await page.unroute("**/prototype/qr");
  await click("#qr-generate");
  await page.waitForFunction(() => document.querySelector("#qr-viewport svg") && !document.querySelector("#qr-error"));
  await click("#qr-clear");
  assert.equal(await page.locator("#qr-text").inputValue(), "");
  assert.equal(await page.locator("#qr-viewport svg, #qr-error").count(), 0);
  console.log("QR UTF-8/ECI, independent decoding, SVG pixels/margins/scales, overflow, rejection, retry and clear passed.");
}
