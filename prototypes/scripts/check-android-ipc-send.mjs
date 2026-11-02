import assert from "node:assert/strict";
import { runInNewContext } from "node:vm";
import { evaluatePixelAlignment, validateIpcSends } from "./android-webview-phases.mjs";

function fixture(options = {}) {
  let time = 100, readyAt = Infinity, fetchAt = Infinity, deliveryAt = Infinity;
  let fetched = false, delivered = false, clicks = 0, nativeCalls = 0, fetchCalls = 0, callbackCalls = 0;
  const order = options.inverse ? [1, 2] : [2, 1];
  const callbacks = new Map(), payload = { snapshot: { cards: Array.from({ length: 30 }, (_, i) => ({ id: i < 2 ? order[i] : i + 1, column: 0 })) } };
  const nativeReceiver = {}, fetchReceiver = {}, returnValue = {}, nativeError = new Error("native send failed"), fetchError = new Error("fetch send failed");
  const message = JSON.stringify({ cmd: "storage", callback: 7, error: 8, payload: { request: { action: "move_card" } }, __TAURI_INVOKE_KEY__: "must not be retained" });
  const headers = new Headers({ "Tauri-Callback": "9", "Tauri-Error": "10" });
  const fetchOptions = { headers }, url = "ipc://localhost/channel";
  const foreign = () => {};
  const bridge = { postMessage: function(...args) {
    nativeCalls++; assert.equal(this, options.inverse ? nativeBridge : nativeReceiver); assert.deepEqual(args, [message, 99]);
    time += 3;
    if (options.nativeThrows) throw nativeError;
    fetchAt = time + 20; return returnValue;
  } };
  const fakeWindow = { ipc: bridge, fetch: function(...args) {
    fetchCalls++; assert.equal(this, fetchReceiver); assert.equal(args[0], url); assert.equal(args[1], fetchOptions); assert.equal(args[2], 88);
    time += 2;
    if (options.fetchThrows) throw fetchError;
    deliveryAt = time + 30;
    // The wrapper must return the exact promise without observing or chaining it.
    return returnValue;
  }, __TAURI_INTERNALS__: { callbacks, convertFileSrc: () => url } };
  const originalPost = Object.getOwnPropertyDescriptor(fakeWindow, "ipc"), originalFetch = Object.getOwnPropertyDescriptor(fakeWindow, "fetch");
  // Never try to mutate the Java object's synthetic method, even if its
  // descriptor claims to be writable. Only the window.ipc reference is owned.
  const nativeBridge = new Proxy(bridge, { defineProperty() { throw new Error("Do not mutate native bridge"); }, set() { throw new Error("Do not mutate native bridge"); } });
  fakeWindow.ipc = nativeBridge; originalPost.value = nativeBridge;
  if (options.unavailable) Object.defineProperty(fakeWindow, "fetch", { configurable: false });
  const errors = [];
  const success = response => {
    callbackCalls++; assert.equal(response, payload); callbacks.delete(7); callbacks.delete(8); time += 1;
    if (options.callbackThrows) throw new Error("callback failed");
    readyAt = time + 20;
  };
  const context = {
    Map, Reflect, window: fakeWindow,
    performance: { now: () => time, timeOrigin: 123456 }, innerWidth: 300, innerHeight: 500,
    scrollBy: () => { time += 3; },
    setTimeout: (fn, ms) => {
      time += ms;
      try {
        if (!fetched && time >= fetchAt) {
          fetched = true;
          callbacks.set(9, response => { assert.equal(response, payload); callbacks.delete(9); callbacks.delete(10); time += 1; });
          callbacks.set(10, () => {});
          assert.equal(fakeWindow.fetch.call(fetchReceiver, url, fetchOptions, 88), returnValue);
        }
        if (!delivered && time >= deliveryAt && !options.neverReply) {
          delivered = true; callbacks.get(9)(payload); callbacks.get(7)(payload);
        }
      } catch (error) { errors.push(error); }
      fn();
    },
    document: {
      querySelector(selector) {
        if (["#move-save", "#board-undo"].includes(selector)) return { disabled: false, click() {
          clicks++; time += 4; callbacks.set(7, success); callbacks.set(8, () => {});
          if (options.noSend) { readyAt = time + 20; return; }
          try {
            assert.equal(options.inverse ? fakeWindow.ipc.postMessage(message, 99) : fakeWindow.ipc.postMessage.call(nativeReceiver, message, 99), returnValue);
          } catch (error) { if (options.nativeThrows) assert.equal(error, nativeError); throw error; }
          if (options.duplicate) fakeWindow.ipc.postMessage.call(nativeReceiver, message, 99);
          if (options.lostOwner) fakeWindow.fetch = foreign;
        } };
        if (selector === "#board-busy") return time < readyAt ? {} : null;
        if (selector === ".board") return { scrollLeft: 0 };
        if (selector.endsWith(".card-list")) return { scrollTop: 0, getBoundingClientRect: () => ({ left: 0, top: 0, right: 300, bottom: 500 }) };
        throw new Error(`Unexpected selector ${selector}`);
      },
      querySelectorAll(selector) {
        if (selector.endsWith("h4")) return [0, 1].map(() => ({ getBoundingClientRect: () => { time += 2; return { left: 10, top: 100, right: 100, bottom: 140 }; } }));
        return (time < readyAt ? order.toReversed() : order).map(id => ({ dataset: { card: String(id) } }));
      },
    },
  };
  const evidence = {}, args = { kind: "board", selector: options.inverse ? "#board-undo" : "#move-save", order, ipcDiagnostic: true, ipcSendDiagnostic: true };
  return { evidence, errors, bridge, fakeWindow, originalPost, originalFetch, foreign, callbacks, success, payload,
    nativeError, fetchError, get clicks() { return clicks; }, get nativeCalls() { return nativeCalls; }, get fetchCalls() { return fetchCalls; }, get callbackCalls() { return callbackCalls; },
    run: () => evaluatePixelAlignment(async (fn, args) => JSON.parse(JSON.stringify(await runInNewContext(`(${fn.toString()})(${JSON.stringify(args)})`, context))), args, evidence),
  };
}
for (const inverse of [false, true]) {
  const f = fixture({ inverse }); await f.run();
  assert.equal(f.clicks, 1); assert.equal(f.nativeCalls, 1); assert.equal(f.fetchCalls, 1); assert.equal(f.callbackCalls, 1);
  assert.deepEqual(Object.getOwnPropertyDescriptor(f.fakeWindow, "ipc"), f.originalPost);
  assert.deepEqual(Object.getOwnPropertyDescriptor(f.fakeWindow, "fetch"), f.originalFetch);
  assert.equal(Object.hasOwn(f.callbacks, "set"), false);
  const e = f.evidence, marks = Object.fromEntries(e.result.phases.marks.map(m => [m.name, m.ms]));
  assert.deepEqual(e.ipc_send_durations_ms, { click_to_native_send: 4, native_send_observation: 0, native_send_call: 3,
    native_return_to_channel_fetch: 20, channel_fetch_observation: 0, channel_fetch_call: 2,
    channel_fetch_return_to_delivery: 30, channel_callback: 1, channel_end_to_reply: 0 });
  assert.ok(!JSON.stringify(e).includes("must not be retained"));
  for (const corrupt of [
    e => { e.result.phases.ipc.sends.calls[0].callback_id = 8; },
    e => { e.result.phases.ipc.sends.calls[0].command = "other"; },
    e => { e.result.phases.ipc.sends.calls[0].action = "delete"; },
    e => { e.result.phases.ipc.sends.calls[0].returned_ms = Infinity; },
    e => { e.result.phases.ipc.sends.calls[1].channel_fetch = false; },
    e => { e.result.phases.ipc.sends.calls[1].callback_id = null; },
    e => { e.result.phases.ipc.sends.calls[1].observation_error = "bad metadata"; },
    e => { e.result.phases.ipc.sends.calls.push(e.result.phases.ipc.sends.calls[1]); },
    e => { e.result.phases.ipc.sends.cleanup.complete = false; },
    e => { e.result.phases.ipc.registrations.find(r => r.id === 9).deliveries = []; },
  ]) { const bad = structuredClone(e); corrupt(bad); assert.throws(() => validateIpcSends(bad, marks)); }
}
for (const [options, message] of [
  [{ nativeThrows: true }, /native send failed/], [{ fetchThrows: true }, /Saved result did not reach DOM/],
  [{ neverReply: true }, /Saved result did not reach DOM/], [{ noSend: true }, /exactly one matching/],
  [{ duplicate: true }, /exactly one native IPC/], [{ callbackThrows: true }, /Saved result did not reach DOM/],
  [{ unavailable: true }, /send method unavailable/], [{ lostOwner: true }, /send method hook changed/],
]) {
  const f = fixture(options); await assert.rejects(f.run, message);
  assert.equal(f.evidence.status, "failed"); assert.equal(f.clicks, options.unavailable ? 0 : 1);
  assert.equal(Object.hasOwn(f.callbacks, "set"), false);
  assert.deepEqual(Object.getOwnPropertyDescriptor(f.fakeWindow, "ipc"), f.originalPost);
  if (!options.unavailable && !options.lostOwner) assert.deepEqual(Object.getOwnPropertyDescriptor(f.fakeWindow, "fetch"), f.originalFetch);
  if (options.lostOwner) assert.equal(f.fakeWindow.fetch, f.foreign);
  if (options.fetchThrows) assert.equal(f.errors[0], f.fetchError);
  if (options.neverReply) { assert.equal(f.callbacks.get(7), f.success); f.callbacks.get(7)(f.payload); assert.equal(f.callbackCalls, 1); }
}
for (const flags of [
  ["--trace-ipc-send"], ["--pixels", "--trace-ipc-send", "--deflate-pixels"], ["--pixels", "--release", "--trace-ipc-send"],
  ...["--trace-ipc", "--trace-webview", "--trace-moves", "--trace-static", "--trace-noop"].map(other => ["--pixels", "--release", other, "--trace-ipc-send", "--deflate-pixels"]),
]) {
  const r = Bun.spawnSync([process.execPath, `${import.meta.dir}/measure-android.mjs`, ...flags], { env: { ...process.env, ANDROID_HOME: "", ANDROID_SERIAL: "" } });
  assert.notEqual(r.exitCode, 0); assert.match(r.stderr.toString(), /--trace-ipc-send requires/);
}
console.log("IPC send phases: serialized native/fetch forwarding preserves this/args/result/errors; callback correlation, clocks, failed/late replies, partial installation, foreign-hook ownership and CLI guards passed.");
