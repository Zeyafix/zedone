import assert from "node:assert/strict";
import { runInNewContext } from "node:vm";
import { evaluatePixelAlignment } from "./android-webview-phases.mjs";

function fixture(options = {}) {
  let time = 100, readyAt = Infinity, replyAt = Infinity, dispatched = false, clicks = 0, calls = 0, returned, callbackError;
  const callbacks = new Map(), receiver = {}, payload = { snapshot: { cards: Array.from({ length: 30 }, (_, i) => ({ id: i === 0 ? 2 : i === 1 ? 1 : i + 1, column: 0 })) } };
  const originalSet = callbacks.set, foreign = function(...args) { return Map.prototype.set.apply(this, args); };
  if (options.foreignHook) Object.defineProperty(callbacks, "set", { value: foreign, writable: true, configurable: true });
  const success = function(response, extra) {
    calls++; assert.equal(this, receiver); assert.equal(response, payload); assert.equal(extra, 99);
    callbacks.delete(7); callbacks.delete(8); time += 2;
    if (options.callbackThrows) throw new Error("original callback failed");
    readyAt = time + 20; return 77;
  };
  let delivery;
  const context = {
    Map, Reflect, window: { __TAURI_INTERNALS__: { callbacks } },
    performance: { now: () => time, timeOrigin: 123456 }, innerWidth: 300, innerHeight: 500,
    scrollBy: () => { time += 3; },
    setTimeout: (fn, ms) => {
      time += ms;
      if (!dispatched && time >= replyAt) {
        dispatched = true;
        try {
          if (options.internalReply) {
            // Tauri can fetch a large cached response through its own invoke,
            // then forward that same payload to the original command callback.
            callbacks.set(9, response => delivery.call(receiver, response, 99));
            callbacks.set(10, () => {});
            returned = callbacks.get(9)(payload);
            callbacks.delete(9); callbacks.delete(10);
          } else returned = delivery.call(receiver, payload, 99);
          if (options.duplicate) delivery.call(receiver, payload, 99);
        } catch (error) { callbackError = error; }
      }
      fn();
    },
    document: {
      querySelector(selector) {
        if (["#move-save", "#board-undo"].includes(selector)) return { disabled: !!options.disabled, click() {
          clicks++;time += 4;
          if (options.clickThrows) throw new Error("click failed");
          if (options.noRegistration) { readyAt = time + 20; return; }
          if (options.wrongPayload) payload.snapshot.cards[0].id = 99;
          callbacks.set(7, success); callbacks.set(8, () => {}); delivery = callbacks.get(7);
          if (!options.neverReply) replyAt = time + 30;
          if (options.lostOwner) Object.defineProperty(callbacks, "set", { value: foreign, writable: true, configurable: true });
        } };
        if (selector === "#board-busy") return time < readyAt ? {} : null;
        if (selector === ".board") return { scrollLeft: 0 };
        if (selector.endsWith(".card-list")) return { scrollTop: 0, getBoundingClientRect: () => ({ left: 0, top: 0, right: 300, bottom: 500 }) };
        throw new Error(`Unexpected selector ${selector}`);
      },
      querySelectorAll(selector) {
        if (selector.endsWith("h4")) return [0, 1].map(() => ({ getBoundingClientRect: () => { time += 2; return { left: 10, top: 100, right: 100, bottom: 140 }; } }));
        return (time < readyAt ? [1, 2] : [2, 1]).map(id => ({ dataset: { card: String(id) } }));
      },
    },
  };
  const evidence = {};
  return { evidence, callbacks, originalSet, foreign, success, payload,
    get clicks() { return clicks; }, get calls() { return calls; }, get returned() { return returned; }, get callbackError() { return callbackError; },
    run: () => evaluatePixelAlignment(async (fn, args) => JSON.parse(JSON.stringify(await runInNewContext(`(${fn.toString()})(${JSON.stringify(args)})`, context))),
      { kind: "board", selector: "#move-save", order: [2, 1], ipcDiagnostic: true }, evidence),
    deliverLate: () => callbacks.get(7).call(receiver, payload, 99),
  };
}
const stable = fixture();await stable.run();
assert.equal(stable.clicks, 1);assert.equal(stable.calls, 1);assert.equal(stable.returned, 77);
assert.equal(stable.callbacks.set, stable.originalSet);assert.equal(Object.hasOwn(stable.callbacks, "set"), false);
assert.equal(stable.evidence.result.phases.ipc.cleanup.complete, true);
assert.equal(stable.evidence.result.phases.ipc.registrations.length, 2);
assert.deepEqual(stable.evidence.ipc_durations_ms, { click_to_reply: 34, reply_callback: 2, reply_to_saved_dom: 22, callback_end_to_saved_dom: 20 });
const internal = fixture({ internalReply: true });await internal.run();
assert.equal(internal.calls, 1);assert.equal(internal.returned, 77);
assert.equal(internal.evidence.ipc_reply.callback_id, 7, "Do not confuse a secondary cached-response callback with delivery to the click's callback");
assert.deepEqual(internal.evidence.result.phases.ipc.registrations.map(r => r.during_click), [true, true, false, false]);
assert.equal(internal.evidence.result.phases.ipc.registrations.flatMap(r => r.deliveries).filter(d => d.expected_snapshot).length, 2);
assert.equal(internal.callbacks.set, internal.originalSet);
for (const [options, message] of [
  [{ disabled: true }, /Unavailable command/], [{ clickThrows: true }, /click failed/],
  [{ neverReply: true }, /Saved result did not reach DOM/],
  [{ noRegistration: true }, /No IPC callback/],
  [{ wrongPayload: true }, /exactly one matching/], [{ duplicate: true }, /exactly one matching/],
  [{ callbackThrows: true }, /Saved result did not reach DOM/],
]) {
  const broken = fixture(options);await assert.rejects(broken.run, message);
  assert.equal(broken.evidence.status, "failed");assert.equal(broken.callbacks.set, broken.originalSet);
  assert.equal(Object.hasOwn(broken.callbacks, "set"), false);assert.equal(broken.clicks, options.disabled ? 0 : 1);
  if (options.neverReply) {
    assert.equal(broken.evidence.result.phases.ipc.cleanup.pending_wrappers_restored, 2);
    assert.equal(broken.callbacks.get(7), broken.success);
    const deliveries = broken.evidence.result.phases.ipc.registrations[0].deliveries.length;
    assert.equal(broken.deliverLate(), 77);assert.equal(broken.calls, 1);
    assert.equal(broken.evidence.result.phases.ipc.registrations[0].deliveries.length, deliveries);
  }
  if (options.callbackThrows) assert.match(String(broken.callbackError), /original callback failed/);
}
for (const options of [{ foreignHook: true }, { lostOwner: true }]) {
  const broken = fixture(options);await assert.rejects(broken.run, /already instrumented|hook changed/);
  assert.equal(broken.callbacks.set, broken.foreign, "Never remove someone else's hook");
  assert.equal(broken.evidence.status, "failed");
}
for (const flags of [
  ["--trace-ipc"], ["--pixels", "--trace-ipc", "--deflate-pixels"], ["--pixels", "--release", "--trace-ipc"],
  ...["--trace-webview", "--trace-moves", "--trace-static", "--trace-noop"].map(other => ["--pixels", "--release", other, "--trace-ipc", "--deflate-pixels"]),
]) {
  const result = Bun.spawnSync([process.execPath, `${import.meta.dir}/measure-android.mjs`, ...flags], { env: { ...process.env, ANDROID_HOME: "", ANDROID_SERIAL: "" } });
  assert.notEqual(result.exitCode, 0);assert.match(result.stderr.toString(), /--trace-ipc requires/);
}
console.log("IPC phases: original callback arguments/this/return preserved, asynchronous reply/DOM clocks, single matching response, partial failures without retry, late-reply restoration, foreign-hook ownership and CLI guards passed.");
