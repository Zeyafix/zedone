import assert from "node:assert/strict";

// Serialized into the existing WebView evaluation. Keep it self-contained and
// use the same command, polling predicate and layout work in both profiles.
export async function alignAndroidPixelResult({ kind, selector, order, query, count, diagnostic = false, ipcDiagnostic = false }) {
  const phases = diagnostic ? { clock: "WebView performance.now", time_origin_ms: performance.timeOrigin, marks: [], polls: 0, polling_ms: 10 } : undefined;
  const mark = name => { if (phases) phases.marks.push({ name, ms: performance.now() }); };
  let callbacks, hook, outcome, duringClick = false;
  const registrations = [];
  mark("start");
  try {
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
    }
    evidence.status = "completed";
    return result.rect;
  } catch (error) {
    evidence.status = "failed";
    evidence.error = String(error.stack || error);
    throw error;
  }
}
