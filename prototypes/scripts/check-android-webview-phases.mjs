import assert from "node:assert/strict";
import { runInNewContext } from "node:vm";
import { alignAndroidPixelResult, evaluatePixelAlignment } from "./android-webview-phases.mjs";

// Execute the serialized browser function with a deterministic event loop. A
// saved DOM must wait for the async completion; click alone cannot end a phase.
function fixture({ delay = 30, disabled = false, clipped = false, clickError = false, never = false } = {}) {
  let time = 100, clicked = false, readyAt = Infinity, scrolls = 0, evaluations = 0;
  const rectangle = { left: 10, top: 100, right: 100, bottom: 140 };
  const box = { getBoundingClientRect: () => { time += 2; return rectangle; } };
  const list = { scrollTop: 40, getBoundingClientRect: () => ({ left: 0, top: 0, right: clipped ? 50 : 300, bottom: 500 }) };
  const board = { scrollLeft: 10 };
  const context = {
    performance: { now: () => time, timeOrigin: 123456 }, innerWidth: 300, innerHeight: 500,
    scrollBy: () => { scrolls++; time += 3; },
    setTimeout: (callback, ms) => { time += ms; callback(); },
    document: {
      querySelector(selector) {
        if (selector === "#move-save" || selector === "#board-undo") return { disabled, click() {
          clicked = true; time += 4;
          if (clickError) throw new Error("click failed");
          readyAt = never ? Infinity : time + delay;
        } };
        if (selector === "#board-busy") return time < readyAt ? {} : null;
        if (selector === ".board") return board;
        if (selector.endsWith(".card-list")) return list;
        throw new Error(`Unexpected selector: ${selector}`);
      },
      querySelectorAll(selector) {
        if (selector.endsWith("h4")) return [box, box];
        return (time < readyAt ? [1, 2] : [2, 1]).map(card => ({ dataset: { card: String(card) } }));
      },
    },
  };
  return { get clicked() { return clicked; }, get scrolls() { return scrolls; }, get evaluations() { return evaluations; },
    async evaluate(fn, args) {
      evaluations++;
      const result = await runInNewContext(`(${fn.toString()})(${JSON.stringify(args)})`, context);
      // Match the real JSON transport, not cross-realm prototype identities.
      return JSON.parse(JSON.stringify(result));
    },
  };
}
const args = { kind: "board", selector: "#move-save", order: [2, 1] };
for (const selector of ["#move-save", "#board-undo"]) {
  const plain = fixture(), diagnostic = fixture(), evidence = {};
  const rect = await evaluatePixelAlignment(plain.evaluate, { ...args, selector });
  assert.deepEqual(await evaluatePixelAlignment(diagnostic.evaluate, { ...args, selector }, evidence), rect);
  assert.deepEqual(rect, { left: 10, top: 100, right: 100, bottom: 140 });
  assert.equal(diagnostic.evaluations, 1);assert.equal(diagnostic.scrolls, 1);
  assert.equal(evidence.status, "completed");assert.equal(evidence.result.phases.polls, 4);
  assert.deepEqual(evidence.durations_ms, { before_click: 0, click: 4, saved_dom_wait: 30, alignment: 9, total: 43 });
}
for (const [options, expected] of [
  [{ disabled: true }, /Unavailable command/], [{ clipped: true }, /clipped/],
  [{ clickError: true }, /click failed/], [{ never: true }, /Saved result did not reach DOM/],
]) {
  const test = fixture(options), evidence = {};
  await assert.rejects(() => evaluatePixelAlignment(test.evaluate, args, evidence), expected);
  assert.equal(test.evaluations, 1, "A failed command must never be retried");
  assert.equal(evidence.status, "failed");assert.equal(evidence.result.ok, false);
  assert.equal(evidence.result.phases.marks.at(-1).name, "failed");
  assert.equal(test.scrolls, options.clipped ? 1 : 0, "Do not align before saved DOM");
  if (options.never) assert.equal(evidence.result.phases.marks.some(mark => mark.name === "saved_dom"), false);
  const plain = fixture(options);
  await assert.rejects(() => evaluatePixelAlignment(plain.evaluate, args), expected);
}
const broken = {};
await assert.rejects(() => evaluatePixelAlignment(async () => { throw new Error("transport failed"); }, args, broken), /transport failed/);
assert.equal(broken.status, "failed");assert.equal(broken.result, undefined);
const good = await fixture().evaluate(alignAndroidPixelResult, { ...args, diagnostic: true });
for (const corrupt of [
  result => { result.phases.marks[2].ms = -1; },
  result => { result.phases.marks.pop(); },
  result => { result.phases.time_origin_ms = null; },
  result => { result.phases.polls = 0; },
  result => { delete result.rect; },
]) {
  const value = structuredClone(good), evidence = {};corrupt(value);
  await assert.rejects(() => evaluatePixelAlignment(async () => value, args, evidence));
  assert.equal(evidence.status, "failed");assert.deepEqual(evidence.result, value);
}
for (const flags of [
  ["--trace-webview"], ["--pixels", "--trace-webview", "--deflate-pixels"],
  ["--pixels", "--release", "--trace-webview"],
  ...["--trace-moves", "--trace-static", "--trace-noop"].map(other => ["--pixels", "--release", other, "--trace-webview", "--deflate-pixels"]),
]) {
  const result = Bun.spawnSync([process.execPath, `${import.meta.dir}/measure-android.mjs`, ...flags], { env: { ...process.env, ANDROID_HOME: "", ANDROID_SERIAL: "" } });
  assert.notEqual(result.exitCode, 0);assert.match(result.stderr.toString(), /--trace-webview requires/);
}
console.log("WebView phases: serialized single evaluation, saved-DOM polling, default parity, partial failures without retries, clock/geometry validation and CLI guards passed.");
