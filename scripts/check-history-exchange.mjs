import assert from "node:assert/strict";
import { randomBytes } from "node:crypto";

export function checkHistoryExchange({ host, android, space, pairing, call, command, deliver, ok, error }) {
  const id = () => randomBytes(16).toString("hex");
  const run = (device, action, args = {}) => ok(command(device, action, { space, ...args }));
  const exchange = () => ok(call(android, "exchange.run", { space }));
  for (const device of [host, android]) {
    const selection = run(device, "sync.inspect", { pairing });
    run(device, "sync.select", { pairing, selected: true, expected: selection.selection_version });
  }
  const column = id();
  run(host, "column.create", { column, name: "History exchange" });
  for (const wholeSpace of [false, true]) {
    for (const cleaner of [host, android]) {
      const writer = cleaner === host ? android : host;
      const card = id(), added = id(), trashed = id();
      const create = (device, card, title) => run(device, "card.create", { column, card, title, description: "Saved pair 😀" });
      const read = (device, card) => run(device, "card.read", { card });
      const history = (device, card) => run(device, "card.history", { card, after: null, expected: null, limit: 100 }).versions;
      const inspect = device => run(device, "history.inspect", { card: wholeSpace ? null : card });
      create(host, card, "Original pair");
      const priorTrash = create(host, trashed, "Earlier trash");
      run(host, "card.trash", { card: trashed, expected: priorTrash.version });
      exchange();
      const oldPage = ok(deliver("sync.pull", { space, protocol: 1, known: [] }));
      assert.equal(oldPage.complete, true);
      const preview = inspect(cleaner);
      assert.equal(preview.text_bytes, preview.entries.reduce((total, e) => total + Buffer.byteLength(e.version.title + e.version.description), 0));
      const clear = { card: preview.card, expected: preview.version };
      const request = { id: [...randomBytes(16)], operation: "history.clear", parameters: { space, ...clear } };
      ok(call(cleaner, "history.clear", request));
      assert.deepEqual(history(cleaner, card), []);
      const changed = run(writer, "card.edit", { card, title: "Independent version", description: "Independent description", expected: read(writer, card).version });
      create(writer, added, "Independent card");
      const independent = history(writer, card).at(-1), newCardHistory = history(writer, added);
      exchange();
      exchange();
      for (const device of [host, android]) {
        assert.deepEqual(history(device, card), [independent]);
        assert.deepEqual(history(device, added), newCardHistory);
        assert.equal(history(device, trashed).length, wholeSpace ? 0 : 1);
        assert.deepEqual(read(device, card), changed);
      }
      const replay = { id: [...randomBytes(16)], operation: "sync.push", parameters: { space, protocol: 1, base: [], changes: oldPage.changes } };
      const forwardedBytes = Buffer.byteLength(JSON.stringify({ operation: "sync.push", request: JSON.stringify(replay) }));
      assert.ok(forwardedBytes <= 64 * 1024, "History replay exceeds the fixture command envelope");
      console.log(`History replay: ${oldPage.changes.length} changes, ${forwardedBytes} bytes within the 65536-byte fixture envelope.`);
      ok(deliver("sync.push", replay.parameters, replay));
      exchange();
      assert.deepEqual(history(host, card), [independent], "Old data resurrected cleared history");
      assert.deepEqual(history(android, card), [independent]);
      assert.equal(call(cleaner, "history.clear", request).replayed, true);
      error(command(cleaner, "history.clear", { space, ...clear }), "stale_state");
      assert.deepEqual(history(cleaner, card), [independent]);
      run(cleaner, "card.restore_text", { card, text_version: independent.id, expected: read(cleaner, card).version });
      exchange();
      const restored = history(host, card);
      assert.equal(restored.length, 2);
      assert.ok(restored[1].sequence > independent.sequence);
      assert.deepEqual(restored, history(android, card));
      assert.deepEqual(read(android, card).titles, [independent.title]);
    }
  }
  const card = id();
  const create = run(host, "card.create", { column, card, title: "Before conflict", description: "Original description" });
  exchange();
  for (const [device, title] of [[host, "PC variant"], [android, "Phone variant"]]) {
    run(device, "card.edit", { card, title, description: "Original description", expected: create.version });
  }
  exchange();
  const history = device => run(device, "card.history", { card, after: null, expected: null, limit: 100 }).versions;
  const before = history(host);
  assert.equal(before.length, 3);
  for (const device of [host, android]) {
    const preview = run(device, "history.inspect", { card });
    assert.ok(preview.entries.every(e => e.retained));
    assert.equal(preview.removable_bytes, 0);
    run(device, "history.clear", { card, expected: preview.version });
    assert.deepEqual(history(device), before);
    assert.equal(run(device, "card.read", { card }).state, "conflicted");
  }
  run(android, "card.resolve_text", { card, title: "Resolved pair", description: "Saved history", expected: run(android, "card.read", { card }).version });
  exchange();
  assert.equal(history(host).length, 4);
  assert.deepEqual(history(host), history(android));
  const protectedHistory = history(host);
  for (const kind of ["column", "space"]) {
    const target = kind === "column" ? { column } : {};
    const original = run(host, `${kind}.read`, target);
    for (const [device, name] of [[host, "PC name"], [android, "Phone name"]]) {
      run(device, `${kind}.rename`, { ...target, name, expected: original.version });
    }
    exchange();
    assert.equal(run(host, `${kind}.read`, target).names.length, 2);
    for (const device of [host, android]) {
      for (const scope of [card, null]) {
        const preview = run(device, "history.inspect", { card: scope });
        const entries = preview.entries.filter(e => e.card === card);
        assert.equal(entries.length, protectedHistory.length);
        assert.ok(entries.every(e => e.retained));
        run(device, "history.clear", { card: scope, expected: preview.version });
        assert.deepEqual(history(device), protectedHistory);
      }
    }
    run(host, `${kind}.resolve_name`, { ...target, name: original.names[0], expected: run(host, `${kind}.read`, target).version });
    exchange();
    assert.deepEqual(history(android), protectedHistory);
  }
  console.log("Paired history: both scopes and directions, independent cards/versions, old delivery, frozen retry, stale, card/ancestor conflicts and restore passed.");
}
