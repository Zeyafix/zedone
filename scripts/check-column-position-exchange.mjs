import assert from "node:assert/strict";
import { randomBytes } from "node:crypto";

export function checkColumnPositionExchange({ host, android, space, pairing, call, command, ok, error }) {
  const id = () => randomBytes(16).toString("hex");
  const exchange = () => ok(call(android, "exchange.run", { space }));
  const pause = paused => {
    const current = ok(command(android, "sync.inspect", { space, pairing }));
    ok(command(android, "sync.pause", { space, pairing, paused, expected: current.pause_version }));
  };
  for (const identical of [false, true]) {
    const [a, b, column, x, y, card] = Array.from({ length: 6 }, id);
    const read = (target, current = column) => ok(command(target, "column.read", { space, column: current }));
    const child = target => ok(command(target, "card.read", { space, card }));
    const history = target => ok(command(target, "card.history", { space, card, after: null, limit: 100, expected: null })).versions;
    const move = (target, current, before) => ok(command(target, "column.move", { space, column: current, before, expected: read(target, current).version }));
    const choose = (target, index) => {
      const current = ok(command(target, "column.inspect_positions", { space, column }));
      return ok(command(target, "column.resolve_position", { space, column, expected: current.column.version, position: current.choices[index].position }));
    };
    for (const current of [a, b, column, x, y]) ok(command(host, "column.create", { space, column: current, name: `Position ${current}` }));
    ok(command(host, "card.create", { space, column, card, title: "Preserved child", description: "Position body" }));
    exchange(); pause(true);
    move(host, x, b); move(android, y, b);
    move(host, column, b); move(android, column, null);
    pause(false); exchange(); exchange();
    const conflicted = read(host);
    assert.equal(conflicted.positions.length, 2);
    assert.deepEqual(read(android), conflicted);
    const columns = target => ok(command(target, "column.list", { space, include_trashed: false })).columns;
    assert.deepEqual(columns(host), columns(android));
    const order = columns(host).filter(item => item.positions.length === 1).map(item => item.column);
    for (const current of [x, y]) assert.ok(order.indexOf(a) < order.indexOf(current) && order.indexOf(current) < order.indexOf(b));
    error(command(android, "column.move", { space, column, before: a, expected: conflicted.version }), "conflict");
    error(command(android, "column.rename", { space, column, name: "Blocked", expected: conflicted.version }), "conflict");
    ok(command(android, "card.set_completed", { space, card, completed: true, expected: child(android).version }));
    ok(command(android, "card.edit", { space, card, title: "Edited child", description: "Available during position conflict", expected: child(android).version }));
    ok(command(android, "card.move", { space, card, column: a, before: null, expected: child(android).version }));
    ok(command(android, "card.move", { space, card, column, before: null, expected: child(android).version }));
    const extra = id();
    const created = ok(command(android, "card.create", { space, column, card: extra, title: "Created during conflict", description: "" }));
    const trashed = ok(command(android, "card.trash", { space, card: extra, expected: created.version }));
    ok(command(android, "card.restore", { space, card: extra, column: null, expected_column: null, expected: trashed.version }));
    exchange(); exchange();
    const saved = child(host), versions = history(host), contents = read(host).contents;
    pause(true); choose(host, 0); choose(android, identical ? 0 : 1); pause(false); exchange(); exchange();
    const again = read(android);
    assert.equal(again.positions.length, identical ? 1 : 2);
    if (!identical) { assert.deepEqual(again.positions, conflicted.positions); choose(android, 0); exchange(); exchange(); }
    assert.deepEqual(read(host), read(android));
    assert.equal(read(host).positions.length, 1);
    assert.deepEqual(read(host).contents, contents);
    assert.deepEqual(child(host), saved);
    assert.deepEqual(history(host), versions); assert.deepEqual(history(android), versions);
    move(android, column, a); exchange(); exchange();
    assert.deepEqual(read(host), read(android));
    assert.deepEqual(child(host), saved);
  }
  console.log("Column positions: saved choices, independent different/identical decisions, same neighbors, child edit/create/trash/restore/move, preservation and reverse passed.");
}
