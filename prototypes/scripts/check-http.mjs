import assert from "node:assert/strict";
import { resolve } from "node:path";
import { chromium } from "playwright";
import { preview } from "vite";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { Database } from "bun:sqlite";
import { checkStorage, checkStorageUI } from "./storage-scenarios.mjs";
import { checkMarkdown } from "./markdown-scenarios.mjs";
import { checkQr } from "./qr-scenarios.mjs";
import { checkBoard, checkBoardUI, checkBoardRejection, attemptMove, reconciled } from "./board-scenarios.mjs";

const root = resolve(import.meta.dir, "..");
const versions = await Bun.file(resolve(root, "../scripts/environment-versions.json")).json();
assert.equal(Bun.version, versions.bun);
const executable = resolve(root, `target/release/serve-echo${process.platform === "win32" ? ".exe" : ""}`);
const directory = await mkdtemp(resolve(tmpdir(), "zedone-p01-05-"));
let server, web, browser;
let timer;
const deadline = new Promise((_, reject) => {
  timer = setTimeout(() => reject(new Error("HTTP/browser check timed out")), 300000);
});

async function check() {
  async function start(port = "0") {
    server = Bun.spawn([executable, port, directory], { cwd: root, stdout: "pipe", stderr: "inherit" });
    const reader = server.stdout.getReader();
    let address = "";
    while (!address.includes("\n")) {
      const { value, done } = await reader.read();
      assert.ok(!done, "Server exited before printing its address");
      address += new TextDecoder().decode(value);
    }
    reader.releaseLock();
    return address.trim();
  }
  const baseUrl = await start();
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
  const call = async request => {
    const response = await fetch(`${baseUrl}/prototype/storage`, {
      method: "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request), signal: AbortSignal.timeout(10000),
    });
    if (!response.ok) throw new Error(`Storage HTTP ${response.status}`);
    return response.json();
  };
  const { a, b } = await checkStorage(call);
  await checkBoard(call);
  const persisted = (await call({ action: "move_card", expected: a, card: 1, column: 2, position: 0 })).snapshot;
  for (const request of [{ action: "seed", space: "C" }, { action: "read", space: "A" }, { action: "sql", space: "A" }]) {
    await assert.rejects(call(request));
  }
  server.kill();
  await server.exited;
  assert.equal(await start(new URL(baseUrl).port), baseUrl);
  assert.deepEqual((await call({ action: "snapshot", space: "A" })).snapshot, persisted);
  assert.deepEqual((await call({ action: "snapshot", space: "B" })).snapshot, b);
  await call({ action: "restore", snapshot: a });
  console.log("Storage survives a real HTTP process restart.");

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
  await page.unroute("**/prototype/echo");
  await checkMarkdown(page);
  await checkQr(page, { baseUrl });
  await checkStorageUI(page);
  const boardDriver = await checkBoardUI(page, call);
  await checkBoardRejection(page, call);
  const failureDb = new Database(resolve(directory, "synthetic.sqlite3"));
  try {
    failureDb.exec("CREATE TRIGGER board_fail BEFORE INSERT ON prototype_cards WHEN NEW.id = 2 BEGIN SELECT RAISE(ABORT, 'board write failure'); END;");
    await boardDriver.reorder();
    await reconciled(page);
    assert.deepEqual((await call({ action: "snapshot", space: "A" })).snapshot, a);
    assert.deepEqual(await page.locator('.card-list[data-column="0"] .board-card').evaluateAll(cards => cards.map(card => Number(card.dataset.card))), a.cards.filter(card => card.column === 0).map(card => card.id));
  } finally { failureDb.exec("DROP TRIGGER IF EXISTS board_fail"); failureDb.close(); }
  await page.route("**/prototype/storage", async route => {
    if (route.request().postDataJSON().action.startsWith("move_")) { await route.fetch(); await route.abort(); }
    else await route.continue();
  });
  await attemptMove(page);
  await reconciled(page);
  const committed = (await call({ action: "snapshot", space: "A" })).snapshot;
  assert.equal(committed.cards.find(card => card.id === 1).column, 1);
  assert.equal(new Set(committed.cards.map(card => card.id)).size, 30);
  await page.unroute("**/prototype/storage");
  await call({ action: "restore", snapshot: a });
  await page.locator("#board-read").click();
  await page.waitForFunction(() => !document.querySelector("#board-busy"));
  await page.route("**/prototype/storage", route => route.abort());
  await attemptMove(page);
  await page.waitForFunction(() => !document.querySelector("#board-busy") && document.querySelector("#board-message").textContent.includes("пока неизвестен"));
  assert.equal(await page.locator(".board-card").count(), 0);
  await page.unroute("**/prototype/storage");
  await page.locator("#board-read").click();
  await page.waitForFunction(() => !document.querySelector("#board-busy"));
  assert.equal(await page.locator(".board-card").count(), 30);
  assert.deepEqual((await call({ action: "snapshot", space: "A" })).snapshot, a);
  console.log("Board UI: real SQL rollback, lost committed reply, unavailable reconciliation and recovery passed.");
  // A real SQLite lock delays storage, while the form and unrelated API still work.
  const lock = new Database(resolve(directory, "synthetic.sqlite3"));
  try {
    lock.exec("BEGIN EXCLUSIVE");
    await page.locator("#storage-delete").click();
    await page.locator("#storage-busy").waitFor();
    console.log("SQLite lock acquired; storage is pending.");
    await page.getByLabel("Текст", { exact: true }).fill("Отзывчивость при блокировке SQLite");
    await page.getByRole("button", { name: "Отправить", exact: true }).click();
    await page.getByRole("status").waitFor();
    assert.equal(await page.getByRole("status").textContent(), "Отзывчивость при блокировке SQLite");
    assert.equal(await page.locator("#storage-busy").count(), 1, "Echo must complete before the SQLite lock timeout");
    await page.locator("#storage-error").waitFor();
    console.log("SQLite busy error received after responsive echo.");
  } finally {
    lock.exec("ROLLBACK");
    lock.close();
  }
  console.log("SQLite lock released; verifying unchanged data.");
  assert.deepEqual((await call({ action: "snapshot", space: "A" })).snapshot, a);
  console.log("Reading storage again through the UI.");
  await page.locator("#storage-read").click();
  await page.waitForFunction(() => !document.querySelector("#storage-busy"));
  assert.equal(await page.locator("#storage-error").count(), 0);
  // Commit a deletion, then discard its HTTP reply. Read reconciles the outcome.
  await page.route("**/prototype/storage", async route => { await route.fetch(); await route.abort(); });
  console.log("Checking lost storage reply after commit.");
  await page.locator("#storage-delete").click();
  await page.locator("#storage-error").waitFor();
  assert.equal(await page.locator("#storage-result").count(), 0);
  await page.unroute("**/prototype/storage");
  await page.locator("#storage-read").click();
  await page.waitForFunction(() => document.querySelector("#storage-result"));
  assert.equal(JSON.parse(await page.locator("#storage-result").textContent()).snapshot, null);
  await page.locator("#storage-restore").click();
  await page.waitForFunction(() => !document.querySelector("#storage-busy"));
  assert.deepEqual((await call({ action: "snapshot", space: "A" })).snapshot, a);
  assert.deepEqual(errors, []);
  console.log("Real SQLite lock: UI/HTTP responsive, failed write unchanged; lost committed reply reconciled by read.");
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
  await rm(directory, { recursive: true });
}
