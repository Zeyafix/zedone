import assert from "node:assert/strict";
import { resolve } from "node:path";
import { chromium } from "playwright";
import { preview } from "vite";

const root = resolve(import.meta.dir, "..");
const versions = await Bun.file(resolve(root, "../scripts/environment-versions.json")).json();
assert.equal(Bun.version, versions.bun);
const executable = resolve(root, `target/release/serve-echo${process.platform === "win32" ? ".exe" : ""}`);
let server, web, browser;
let timer;
const deadline = new Promise((_, reject) => {
  timer = setTimeout(() => reject(new Error("HTTP/browser check timed out")), 45000);
});

async function check() {
  server = Bun.spawn([executable, "0"], { cwd: root, stdout: "pipe", stderr: "inherit" });
  const reader = server.stdout.getReader();
  let address = "";
  while (!address.includes("\n")) {
    const { value, done } = await reader.read();
    assert.ok(!done, "Server exited before printing its address");
    address += new TextDecoder().decode(value);
  }
  reader.releaseLock();
  const baseUrl = address.trim();
  assert.match(baseUrl, /^http:\/\/127\.0\.0\.1:\d+$/);
  const text = "Ёж, ЕЖ, café, 😀\n\"строка\"\\\t <script>alert(1)</script>";
  const post = (body, headers = { "Content-Type": "application/json" }) => fetch(`${baseUrl}/prototype/echo`, {
    method: "POST", headers, body, signal: AbortSignal.timeout(5000),
  });
  for (const value of ["", text]) {
    const response = await post(JSON.stringify({ text: value }));
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), { text: value });
  }
  for (const body of ["{}", '{"text":null}', '{"text":42}']) {
    assert.equal((await post(body)).status, 422);
  }
  assert.equal((await post("{")).status, 400);
  assert.equal((await post("text", { "Content-Type": "text/plain" })).status, 415);
  assert.equal((await fetch(`${baseUrl}/unknown`)).status, 404);
  assert.equal((await fetch(`${baseUrl}/prototype/echo`)).status, 405);

  // Serve the production bundle, with the same-origin API proxy used in development.
  web = await preview({ root, preview: { port: 0, proxy: { "/prototype": baseUrl } } });
  browser = await chromium.launch({ executablePath: process.env.PROTOTYPE_BROWSER_PATH || undefined, headless: true });
  console.log(`Browser ${browser.version()}; driver/Vite runtime Bun ${Bun.version}.`);
  const page = await browser.newPage();
  page.setDefaultTimeout(10000);
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.goto(web.resolvedUrls.local[0]);
  for (const value of [text, ""]) {
    await page.getByLabel("Текст", { exact: true }).fill(value);
    await page.getByRole("button", { name: "Отправить", exact: true }).click();
    await page.getByRole("status").waitFor({ state: "attached" });
    assert.equal(await page.getByRole("status").textContent(), value);
    await page.getByRole("button", { name: "Очистить", exact: true }).click();
    assert.equal(await page.getByLabel("Текст", { exact: true }).inputValue(), "");
    assert.equal(await page.getByRole("status").count(), 0);
  }
  // Injected HTTP rejection preserves input and offers another explicit request.
  await page.route("**/prototype/echo", route => route.fulfill({ status: 503, contentType: "text/plain", body: "unavailable" }));
  await page.getByLabel("Текст", { exact: true }).fill(text);
  await page.getByRole("button", { name: "Отправить", exact: true }).click();
  await page.getByRole("alert").waitFor();
  assert.equal(await page.getByLabel("Текст", { exact: true }).inputValue(), text);
  assert.equal(await page.getByRole("status").count(), 0);
  await page.unroute("**/prototype/echo");
  await page.getByRole("button", { name: "Отправить", exact: true }).click();
  await page.getByRole("status").waitFor();
  assert.equal(await page.getByRole("status").textContent(), text);
  assert.equal(await page.getByRole("alert").count(), 0);
  // Transport loss follows the same failure path as an unsuccessful response.
  await page.route("**/prototype/echo", route => route.abort());
  await page.getByRole("button", { name: "Отправить", exact: true }).click();
  await page.getByRole("alert").waitFor();
  assert.equal(await page.getByLabel("Текст", { exact: true }).inputValue(), text);
  await page.getByRole("button", { name: "Очистить", exact: true }).click();
  assert.equal(await page.getByRole("alert").count(), 0);
  assert.deepEqual(errors, []);
  console.log("HTTP validation, browser typed echo, clear, failure and retry passed.");
}

try {
  await Promise.race([check(), deadline]);
} finally {
  clearTimeout(timer);
  server?.kill();
  const cleanup = await Promise.allSettled([
    browser?.close(),
    web && new Promise((resolve, reject) => web.httpServer.close(error => error ? reject(error) : resolve())),
    server?.exited,
  ]);
  for (const result of cleanup) {
    if (result.status === "rejected") { console.error(result.reason); process.exitCode = 1; }
  }
}
