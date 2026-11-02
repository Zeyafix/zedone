import assert from "node:assert/strict";

// Serialized into the existing WebView evaluation. Keep it self-contained and
// use the same command, polling predicate and layout work in both profiles.
export async function alignAndroidPixelResult({ kind, selector, order, query, count, diagnostic = false, ipcDiagnostic = false, ipcSendDiagnostic = false }) {
  const phases = diagnostic ? { clock: "WebView performance.now", time_origin_ms: performance.timeOrigin, marks: [], polls: 0, polling_ms: 10 } : undefined;
  const mark = name => { if (phases) phases.marks.push({ name, ms: performance.now() }); };
  let callbacks, hook, outcome, duringClick = false;
  const registrations = [];
  const sendHooks = [];
  mark("start");
  try {
    if (ipcSendDiagnostic && !ipcDiagnostic) throw new Error("IPC send observation requires IPC reply observation");
    if (ipcDiagnostic && (!diagnostic || kind !== "board" || query !== undefined || !Array.isArray(order) || order.length !== 2 || !["#move-save", "#board-undo"].includes(selector)))
      throw new Error("IPC observation requires a diagnostic board move or inverse");
    if (query !== undefined) {
      const input = document.querySelector("#storage-query");
      input.value = query;
      input.dispatchEvent(new Event("input", { bubbles: true }));
      selector = "#storage-read";
    }
    if (selector) {
      const button = document.querySelector(selector);
      if (!button || button.disabled) throw new Error(`Unavailable command: ${selector}`);
      if (ipcDiagnostic) {
        const ipc = phases.ipc = { registrations: [], cleanup: { installed: false, complete: false } };
        callbacks = window.__TAURI_INTERNALS__?.callbacks;
        if (!(callbacks instanceof Map) || Object.prototype.hasOwnProperty.call(callbacks, "set"))
          throw new Error("IPC callback map unavailable or already instrumented");
        hook = function(id, callback) {
          if (this !== callbacks) return Map.prototype.set.call(this, id, callback);
          const record = { id, registered_ms: performance.now(), during_click: duringClick, deliveries: [] };
          const wrapped = function(...args) {
            const delivery = { received_ms: performance.now(), argument_count: args.length };
            record.deliveries.push(delivery);
            const cards = args[0]?.snapshot?.cards;
            delivery.expected_snapshot = Array.isArray(cards) && cards.length === 30 && order.every((id, i) =>
              cards.filter(card => card?.column === 0)[i]?.id === id);
            try { return Reflect.apply(callback, this, args); }
            catch (error) { delivery.callback_error = String(error); throw error; }
            finally { delivery.callback_end_ms = performance.now(); }
          };
          registrations.push({ id, callback, wrapped });
          ipc.registrations.push(record);
          return Map.prototype.set.call(this, id, wrapped);
        };
        Object.defineProperty(callbacks, "set", { value: hook, writable: true, configurable: true });
        ipc.cleanup.installed = true;
        if (ipcSendDiagnostic) {
          const sends = ipc.sends = { calls: [], cleanup: { installed: false, complete: false } };
          const install = (owner, key, transport) => {
            const descriptor = Object.getOwnPropertyDescriptor(owner, key);
            const bridge = transport === "postMessage" ? descriptor?.value : undefined;
            const original = bridge ? bridge.postMessage : descriptor?.value;
            if (!descriptor || typeof original !== "function" || !descriptor.configurable || !descriptor.writable)
              throw new Error(`IPC send method unavailable: ${transport}`);
            let observed;
            const wrapped = function(...args) {
              const record = { transport, entered_ms: performance.now(), during_click: duringClick, argument_count: args.length };
              sends.calls.push(record);
              try {
                if (transport === "postMessage") {
                  const message = JSON.parse(args[0]);
                  record.command = message.cmd; record.callback_id = message.callback; record.error_id = message.error;
                  record.action = message.payload?.request?.action;
                } else {
                  record.channel_fetch = args[0] === window.__TAURI_INTERNALS__.convertFileSrc("plugin:__TAURI_CHANNEL__|fetch", "ipc");
                  const headers = args[1]?.headers;
                  const callback = headers?.get("Tauri-Callback"), error = headers?.get("Tauri-Error");
                  record.callback_id = callback === null || callback === undefined ? null : Number(callback);
                  record.error_id = error === null || error === undefined ? null : Number(error);
                }
              } catch (error) { record.observation_error = String(error); }
              record.forwarded_ms = performance.now();
              // A normal window.ipc.postMessage call must still receive the
              // original Java bridge as its receiver, not our temporary Proxy.
              try { return Reflect.apply(original, bridge && this === observed ? bridge : this, args); }
              catch (error) { record.call_error = String(error); throw error; }
              finally { record.returned_ms = performance.now(); }
            };
            observed = bridge ? new Proxy(bridge, { get(target, name) {
              return name === "postMessage" ? wrapped : Reflect.get(target, name, target);
            } }) : wrapped;
            Object.defineProperty(owner, key, { ...descriptor, value: observed });
            sendHooks.push({ owner, key, descriptor, wrapped: observed });
            sends.cleanup.installed = true;
            if (Object.getOwnPropertyDescriptor(owner, key)?.value !== observed)
              throw new Error(`IPC send hook installation was ignored: ${key}`);
          };
          // Tauri's internals are non-configurable. Observe the existing native
          // bridge and channel fetch without replacing IPC or chaining promises.
          install(window, "ipc", "postMessage");
          install(window, "fetch", "fetch");
        }
      }
      mark("click_start");
      duringClick = true;
      try { button.click(); }
      finally { duringClick = false; }
      mark("click_end");
      // No host round trips between click, saved-result polling and alignment.
      const deadline = performance.now() + 5000;
      while (true) {
        if (phases) phases.polls++;
        const ready = order ? !document.querySelector("#board-busy") && order.every((id, i) =>
          Number(document.querySelectorAll('.board-column[data-column="0"] .board-card')[i]?.dataset.card) === id) :
          !document.querySelector("#storage-busy") && JSON.parse(document.querySelector("#storage-result")?.textContent || "null")?.snapshot.cards.length === count;
        if (ready) break;
        if (performance.now() >= deadline) throw new Error("Saved result did not reach DOM");
        await new Promise(resolve => setTimeout(resolve, 10));
      }
      mark("saved_dom");
    }
    let ranges;
    if (kind === "board") {
      document.querySelector(".board").scrollLeft = 0;
      document.querySelector('.board-column[data-column="0"] .card-list').scrollTop = 0;
      ranges = [...document.querySelectorAll('.board-column[data-column="0"] .board-card h4')].slice(0, 2);
    } else {
      const pre = document.querySelector("#storage-result"), text = pre.firstChild;
      pre.scrollTop = 0; pre.scrollLeft = 0;
      const start = text.textContent.indexOf("Карточка ");
      if (start < 0) throw new Error("Missing search result title");
      const range = document.createRange();
      range.setStart(text, start); range.setEnd(text, start + 13);
      const r = range.getBoundingClientRect(), clip = pre.getBoundingClientRect();
      pre.scrollTop += Math.max(0, r.top - clip.top - 24);
      ranges = [range];
    }
    const before = ranges[0].getBoundingClientRect();
    scrollBy(0, before.top - 100);
    const boxes = ranges.map(range => range.getBoundingClientRect());
    const left = Math.min(...boxes.map(r => r.left)), top = Math.min(...boxes.map(r => r.top));
    const right = Math.max(...boxes.map(r => r.right)), bottom = Math.max(...boxes.map(r => r.bottom));
    const clip = document.querySelector(kind === "board" ? '.board-column[data-column="0"] .card-list' : "#storage-result").getBoundingClientRect();
    if (left < Math.max(0, clip.left) || top < Math.max(0, clip.top) || right > Math.min(innerWidth, clip.right) || bottom > Math.min(innerHeight, clip.bottom))
      throw new Error("Full result titles are clipped; unsupported viewport geometry");
    const rect = { left, top, right, bottom };
    mark("aligned");
    outcome = diagnostic ? { ok: true, rect, phases } : rect;
    return outcome;
  } catch (error) {
    if (!diagnostic) throw error;
    mark("failed");
    outcome = { ok: false, error: String(error.stack || error), phases };
    return outcome;
  } finally {
    if (phases?.ipc?.sends) {
      const cleanup = phases.ipc.sends.cleanup;
      cleanup.restored = 0; cleanup.errors = [];
      for (const entry of sendHooks.toReversed()) {
        try {
          if (Object.getOwnPropertyDescriptor(entry.owner, entry.key)?.value !== entry.wrapped)
            throw new Error(`IPC send method hook changed: ${entry.key}`);
          Object.defineProperty(entry.owner, entry.key, entry.descriptor);
          cleanup.restored++;
        } catch (error) { cleanup.errors.push(String(error)); }
      }
      cleanup.complete = cleanup.errors.length === 0 && cleanup.restored === sendHooks.length;
      if (!cleanup.complete) {
        outcome.ok = false;
        outcome.error = [outcome.error, ...cleanup.errors].filter(Boolean).join("\n");
      }
    }
    if (phases?.ipc?.cleanup.installed) {
      const cleanup = phases.ipc.cleanup;
      try {
        // Restore only our method and outstanding wrappers. A late reply after
        // timeout must reach the original callback, not retain this observer.
        const owned = callbacks.set === hook;
        if (owned) delete callbacks.set;
        cleanup.pending_wrappers_restored = 0;
        for (const entry of registrations) if (callbacks.get(entry.id) === entry.wrapped) {
          Map.prototype.set.call(callbacks, entry.id, entry.callback);
          cleanup.pending_wrappers_restored++;
        }
        if (!owned) throw new Error("IPC callback map hook changed during observation");
        if (Object.prototype.hasOwnProperty.call(callbacks, "set")) throw new Error("IPC callback map hook was not removed");
        cleanup.complete = true;
      } catch (error) {
        cleanup.error = String(error.stack || error);
        outcome.ok = false;
        outcome.error = [outcome.error, cleanup.error].filter(Boolean).join("\n");
      }
    }
  }
}

export async function evaluatePixelAlignment(evaluate, args, evidence) {
  if (!evidence) return evaluate(alignAndroidPixelResult, args);
  evidence.status = "running";
  try {
    const result = await evaluate(alignAndroidPixelResult, { ...args, diagnostic: true });
    evidence.result = result;
    assert.equal(result?.ok, true, result?.error || "Missing WebView phase result");
    const phases = result.phases;
    assert.equal(phases?.clock, "WebView performance.now");
    assert.ok(Number.isFinite(phases.time_origin_ms) && phases.time_origin_ms >= 0);
    assert.equal(phases.polling_ms, 10);
    assert.ok(Number.isInteger(phases.polls) && phases.polls > 0);
    const names = ["start", "click_start", "click_end", "saved_dom", "aligned"];
    assert.deepEqual(phases.marks.map(mark => mark.name), names);
    let previous = 0;
    const marks = {};
    for (const mark of phases.marks) {
      assert.ok(Number.isFinite(mark.ms) && mark.ms >= previous, "Non-monotonic WebView phase clock");
      previous = marks[mark.name] = mark.ms;
    }
    for (const name of ["left", "top", "right", "bottom"]) assert.ok(Number.isFinite(result.rect?.[name]), "Missing aligned rectangle");
    assert.ok(result.rect.right > result.rect.left && result.rect.bottom > result.rect.top);
    evidence.durations_ms = { before_click: marks.click_start - marks.start, click: marks.click_end - marks.click_start,
      saved_dom_wait: marks.saved_dom - marks.click_end, alignment: marks.aligned - marks.saved_dom, total: marks.aligned - marks.start };
    if (args.ipcDiagnostic) {
      const ipc = phases.ipc;
      assert.equal(ipc?.cleanup.installed, true);
      assert.equal(ipc.cleanup.complete, true, ipc.cleanup.error);
      assert.ok(ipc.registrations.length > 0, "No IPC callback registered by the command");
      const deliveries = [];
      for (const registration of ipc.registrations) {
        assert.ok(Number.isFinite(registration.registered_ms) && registration.registered_ms >= marks.start && registration.registered_ms <= marks.saved_dom);
        assert.equal(typeof registration.during_click, "boolean");
        assert.ok(registration.during_click ? registration.registered_ms >= marks.click_start && registration.registered_ms <= marks.click_end : registration.registered_ms >= marks.click_end,
          "IPC callback registration outside its recorded phase");
        for (const delivery of registration.deliveries) {
          assert.ok(Number.isFinite(delivery.received_ms) && Number.isFinite(delivery.callback_end_ms));
          assert.ok(delivery.received_ms >= registration.registered_ms && delivery.callback_end_ms >= delivery.received_ms && delivery.callback_end_ms <= marks.saved_dom);
          assert.equal(delivery.callback_error, undefined, "Original IPC callback failed");
          if (registration.during_click && delivery.expected_snapshot) deliveries.push({ callback_id: registration.id, ...delivery });
        }
      }
      assert.equal(deliveries.length, 1, "Expected exactly one matching saved IPC response from the click");
      const reply = evidence.ipc_reply = deliveries[0];
      assert.ok(reply.received_ms >= marks.click_start);
      evidence.ipc_durations_ms = { click_to_reply: reply.received_ms - marks.click_start,
        reply_callback: reply.callback_end_ms - reply.received_ms,
        reply_to_saved_dom: marks.saved_dom - reply.received_ms,
        callback_end_to_saved_dom: marks.saved_dom - reply.callback_end_ms };
      if (args.ipcSendDiagnostic) validateIpcSends(evidence, marks);
    }
    evidence.status = "completed";
    return result.rect;
  } catch (error) {
    evidence.status = "failed";
    evidence.error = String(error.stack || error);
    throw error;
  }
}

export function validateIpcSends(evidence, marks) {
  const ipc = evidence.result.phases.ipc, sends = ipc.sends;
  assert.equal(sends?.cleanup.installed, true);
  assert.equal(sends.cleanup.complete, true);
  assert.equal(sends.cleanup.restored, 2);
  assert.deepEqual(sends.cleanup.errors, []);
  const reply = evidence.ipc_reply;
  for (const call of sends.calls) {
    assert.equal(call.observation_error, undefined, "IPC send metadata could not be observed");
    assert.equal(call.call_error, undefined, "Original IPC send failed");
    assert.ok([call.entered_ms, call.forwarded_ms, call.returned_ms].every(Number.isFinite));
    assert.ok(call.entered_ms >= marks.click_start && call.forwarded_ms >= call.entered_ms && call.returned_ms >= call.forwarded_ms && call.returned_ms <= marks.saved_dom);
    assert.equal(typeof call.during_click, "boolean");
    assert.ok(call.during_click ? call.returned_ms <= marks.click_end : call.entered_ms >= marks.click_end);
    assert.ok(Number.isInteger(call.callback_id) && Number.isInteger(call.error_id) && call.callback_id !== call.error_id);
    for (const id of [call.callback_id, call.error_id]) {
      const registration = ipc.registrations.find(r => r.id === id);
      assert.ok(registration && registration.registered_ms <= call.entered_ms, "Send must follow observed callback registration");
    }
  }
  const commands = sends.calls.filter(c => c.transport === "postMessage" && c.during_click);
  assert.equal(commands.length, 1, "Expected exactly one native IPC send during click");
  const command = commands[0];
  assert.equal(command.command, "storage"); assert.equal(command.action, "move_card");
  assert.equal(command.callback_id, reply.callback_id, "Native IPC must target the click's saved reply");
  assert.ok(command.returned_ms <= reply.received_ms);
  const fetches = sends.calls.filter(c => c.transport === "fetch" && c.channel_fetch && !c.during_click);
  assert.equal(fetches.length, 1, "Expected exactly one large-response channel fetch");
  const fetch = fetches[0], registration = ipc.registrations.find(r => r.id === fetch.callback_id);
  assert.ok(fetch.entered_ms >= command.returned_ms && fetch.returned_ms <= reply.received_ms);
  assert.equal(registration.during_click, false);
  assert.equal(registration.deliveries.length, 1);
  const delivery = registration.deliveries[0];
  assert.equal(delivery.expected_snapshot, true);
  assert.ok(delivery.received_ms >= fetch.returned_ms && delivery.callback_end_ms <= reply.received_ms);
  assert.equal(sends.calls.length, 2, "Unexpected IPC send; do not silently accept fallback or extra operations");
  evidence.ipc_send_durations_ms = {
    click_to_native_send: command.entered_ms - marks.click_start,
    native_send_observation: command.forwarded_ms - command.entered_ms,
    native_send_call: command.returned_ms - command.forwarded_ms,
    native_return_to_channel_fetch: fetch.entered_ms - command.returned_ms,
    channel_fetch_observation: fetch.forwarded_ms - fetch.entered_ms,
    channel_fetch_call: fetch.returned_ms - fetch.forwarded_ms,
    channel_fetch_return_to_delivery: delivery.received_ms - fetch.returned_ms,
    channel_callback: delivery.callback_end_ms - delivery.received_ms,
    channel_end_to_reply: reply.received_ms - delivery.callback_end_ms,
  };
}
