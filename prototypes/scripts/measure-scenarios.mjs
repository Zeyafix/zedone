import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { resolve } from "node:path";

export const uiMeasurementEvidence = {
  level: "diagnostic",
  endpoint: "DOM readiness followed by two requestAnimationFrame callbacks",
  presentation_validation: "not_validated",
  ui_acceptance_eligible: false,
  limitation: "Frame callbacks do not independently observe desktop pixels or physical display presentation; budget comparisons are diagnostic only",
};

export async function measurementSources(root) {
  const paths = ["scripts/measure-scenarios.mjs", "scripts/measure-desktop.mjs", "scripts/measure-android.mjs", "openapi.json", "Cargo.lock", "bun.lock"];
  for await (const path of new Bun.Glob("client/*").scan(root)) paths.push(path);
  return Object.fromEntries(await Promise.all(paths.sort().map(async path => [path,
    createHash("sha256").update(Buffer.from(await Bun.file(resolve(root, path)).arrayBuffer())).digest("hex")])));
}

export function summarize(samples, budget) {
  const values = samples.filter(sample => sample.ok).map(sample => sample.ms).sort((a, b) => a - b);
  const n = values.length;
  return { attempts: samples.length, failures: samples.length - n, budget_ms: budget,
    over_budget: budget === null ? null : values.filter(value => value > budget).length,
    min_ms: n ? values[0] : null, median_ms: n ? (values[Math.floor((n - 1) / 2)] + values[Math.floor(n / 2)]) / 2 : null,
    p95_ms: n ? values[Math.ceil(n * .95) - 1] : null, max_ms: n ? values[n - 1] : null };
}

export function fixtureInfo(snapshot) {
  const json = JSON.stringify(snapshot);
  return { sha256: createHash("sha256").update(json).digest("hex"), json_utf8_bytes: Buffer.byteLength(json),
    cards: snapshot.cards.length, text_utf8_bytes: snapshot.cards.reduce((n, card) => n + Buffer.byteLength(card.text), 0),
    text_lines: snapshot.cards.reduce((n, card) => n + card.text.split("\n").length, 0),
    history_trash_conflicts: "not implemented in P01" };
}

// Same DOM command and completion predicate in desktop browsers and real WebView.
// Both timestamps live in the page; CDP round trips are outside the interval.
export async function uiTime(page, selector, kind) {
  let guard;
  const observedStart = performance.now();
  const timeout = new Promise(resolve => {
    guard = setTimeout(() => resolve({ ok: false, ms: performance.now() - observedStart,
      clock: "driver", error: "UI observer/CDP timeout; page clock unavailable" }), 12000);
  });
  async function observe() {
    await page.bringToFront();
    return page.evaluate(async ({ selector, kind }) => {
    const button = document.querySelector(selector);
    if (!button || button.disabled) throw new Error(`Unavailable command: ${selector}`);
    const start = performance.now();
    button.click();
    const ready = () => {
      if (kind === "move") return !document.querySelector("#board-busy") &&
        document.querySelector("#board-message")?.textContent === "Перемещение сохранено.";
      if (kind === "search") return !document.querySelector("#storage-busy") &&
        !document.querySelector("#storage-error") && document.querySelector("#storage-result");
      return !document.querySelector("#board-busy") && document.querySelectorAll(".board-card").length === 30;
    };
    // Observe the Vue update, then include a render opportunity. This is a frame
    // boundary proxy, not a claim to observe physical screen presentation.
    return new Promise(resolve => {
      let observer, timer, finished = false;
      const finish = async () => {
        if (finished || !ready()) return;
        finished = true;
        observer.disconnect();
        await new Promise(requestAnimationFrame);
        await new Promise(requestAnimationFrame);
        clearTimeout(timer);
        resolve({ ok: true, ms: performance.now() - start });
      };
      observer = new MutationObserver(finish);
      observer.observe(document.body, { subtree: true, childList: true, characterData: true, attributes: true });
      timer = setTimeout(() => { observer.disconnect(); resolve({ ok: false, ms: performance.now() - start, error: "UI timeout" }); }, 10000);
      queueMicrotask(finish);
    });
    }, { selector, kind });
  }
  try { return await Promise.race([observe(), timeout]); }
  finally { clearTimeout(guard); }
}

export async function workingScreen(page) {
  await page.waitForFunction(() => document.querySelector("#board-read") && document.querySelector("#text"));
  const sample = await uiTime(page, "#board-read", "read");
  assert.ok(sample.ok, "First usable board must load persisted data");
  assert.equal(await page.evaluate(() => !document.querySelector("#text").disabled), true);
}

export async function measureInteractions(page, call, report) {
  const fresh = async () => {
    await call({ action: "delete", space: "A" });
    return (await call({ action: "seed", space: "A" })).snapshot;
  };
  const initial = await fresh();
  assert.equal(initial.cards.length, 30);
  report.fixture = fixtureInfo(initial);
  report.series = {};
  const series = name => report.series[name] = { warmups: 5, samples: [] };
  const move = series("ui_move"), undo = series("ui_inverse_move");
  console.log("Measuring UI move and inverse.");
  await workingScreen(page);
  for (let i = -5; i < 30; i++) {
    report.active_attempt = { series: "ui_move", repeat: i, warmup: i < 0 };
    // DOM commands also work in WebView 101, where modern Playwright locator
    // visibility helpers require Element.checkVisibility from newer Chromium.
    await page.evaluate(() => document.querySelector("#card-move-1").click());
    await page.waitForFunction(() => document.querySelector("#move-column"));
    await page.evaluate(() => {
      const select = document.querySelector("#move-column");
      select.value = "1"; select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    const forward = await uiTime(page, "#move-save", "move");
    if (i >= 0) move.samples.push(forward);
    assert.ok(forward.ok);
    const saved = (await call({ action: "snapshot", space: "A" })).snapshot;
    assert.equal(saved.cards.find(card => card.id === 1).column, 1);
    assert.equal(await page.evaluate(() => document.querySelector('.card-list[data-column="1"] .board-card').dataset.card), "1");
    report.active_attempt = { series: "ui_inverse_move", repeat: i, warmup: i < 0 };
    const reverse = await uiTime(page, "#board-undo", "move");
    if (i >= 0) undo.samples.push(reverse);
    assert.ok(reverse.ok);
    assert.deepEqual((await call({ action: "snapshot", space: "A" })).snapshot, initial);
    assert.equal(await page.evaluate(() => document.querySelector('.card-list[data-column="0"] .board-card').dataset.card), "1");
  }
  assert.deepEqual(await fresh(), initial);
  const search = series("ui_search"), clear = series("ui_clear_search");
  console.log("Measuring UI search and clear.");
  for (let i = -5; i < 30; i++) {
    for (const [query, count, target] of [["Карточка 0030", 1, search], ["", 30, clear]]) {
      report.active_attempt = { series: query ? "ui_search" : "ui_clear_search", repeat: i, warmup: i < 0 };
      await page.evaluate(query => {
        const input = document.querySelector("#storage-query");
        input.value = query; input.dispatchEvent(new Event("input", { bubbles: true }));
      }, query);
      const sample = await uiTime(page, "#storage-read", "search");
      if (i >= 0) target.samples.push(sample);
      assert.ok(sample.ok);
      const result = await page.evaluate(() => JSON.parse(document.querySelector("#storage-result").textContent));
      assert.equal(result.snapshot.cards.length, count);
    }
  }
  assert.deepEqual(await fresh(), initial);
  const api = series("api_move"), apiUndo = series("api_inverse_move");
  console.log("Measuring API move and inverse.");
  for (let i = -5; i < 30; i++) {
    report.active_attempt = { series: "api_move", repeat: i, warmup: i < 0 };
    const start = performance.now();
    const moved = (await call({ action: "move_card", expected: initial, card: 1, column: 1, position: 0 })).snapshot;
    const sample = { ok: true, ms: performance.now() - start };
    assert.equal(moved.cards.find(card => card.id === 1).column, 1);
    report.active_attempt = { series: "api_inverse_move", repeat: i, warmup: i < 0 };
    const reverseStart = performance.now();
    const restored = (await call({ action: "move_card", expected: moved, card: 1, column: 0, position: 0 })).snapshot;
    const reverse = { ok: true, ms: performance.now() - reverseStart };
    assert.deepEqual(restored, initial);
    if (i >= 0) { api.samples.push(sample); apiUndo.samples.push(reverse); }
  }
  for (const [name, value] of Object.entries(report.series)) value.summary = summarize(value.samples, name.includes("search") ? 1000 : 300);
  delete report.active_attempt;
  report.unmeasured = ["card content opening", "completion/uncompletion", "first launch after installation",
    "OS disk-cache cold start", "background sync/backup", "memory and scaling"];
  report.method = { ui_clock: "page performance.now", api_clock: "driver performance.now",
    ui_command: "DOM click of the existing user command; input device latency/drag gesture excluded",
    ui_end: "committed Rust response rendered; two requestAnimationFrame callbacks (frame boundary proxy)",
    api_end: "HTTP response; on Android IPC response including CDP (separate diagnostic series)",
    quantile: "nearest rank", precision: "frame-bound UI; browser timer precision and scheduling apply",
    background: "no application sync, backups or images; normal OS/host activity is not suppressed" };
}
