import assert from "node:assert/strict";
import { randomBytes } from "node:crypto";

export function positionConflictCases(sameColumn) {
  assert.ok(sameColumn === undefined || typeof sameColumn === 'boolean', 'Unknown position conflict scope');
  return (sameColumn === undefined ? [false, true] : [sameColumn])
    .flatMap(same => [false, true].map(columnFirst => ({ same, columnFirst })));
}

export function checkPositionConflictsExchange({ host, android, space, pairing, call, command, ok, error, sameColumn }) {
  const id = () => randomBytes(16).toString("hex");
  const exchange = () => { ok(call(android, "exchange.run", { space })); ok(call(android, "exchange.run", { space })); };
  const pause = paused => {
    const current = ok(command(android, "sync.inspect", { space, pairing }));
    ok(command(android, "sync.pause", { space, pairing, paused, expected: current.pause_version }));
  };
  for (const { same, columnFirst } of positionConflictCases(sameColumn)) {
    const [column, other, a, b, x, y, card, child, ca, cb, cx, cy, tag] = Array.from({ length: 13 }, id);
    const read = (target, current = card) => ok(command(target, "card.read", { space, card: current }));
    const parent = target => ok(command(target, "column.read", { space, column }));
    const history = target => ok(command(target, "card.history", { space, card, after: null, limit: 100, expected: null })).versions;
    const columns = target => ok(command(target, "column.list", { space, include_trashed: false })).columns;
    const cards = target => ok(command(target, "card.list", { space, include_trashed: false })).cards;
    const moveColumn = (target, current, before) => ok(command(target, "column.move", { space, column: current, before, expected: ok(command(target, "column.read", { space, column: current })).version }));
    const moveCard = (target, current, destination, before) => ok(command(target, "card.move", { space, card: current, column: destination, before, expected: read(target, current).version }));
    const inspect = (target, kind) => ok(command(target, kind === "column" ? "column.inspect_positions" : "card.inspect_placements", kind === "column" ? { space, column } : { space, card }));
    const choose = (target, kind, index) => {
      const current = inspect(target, kind);
      return ok(command(target, kind === "column" ? "column.resolve_position" : "card.resolve_placement", kind === "column"
        ? { space, column, expected: current.column.version, position: current.choices[index].position }
        : { space, card, expected: current.card.version, placement: current.choices[index].placement }));
    };
    const blocked = () => {
      const current = read(android);
      for (const [operation, parameters] of [
        ["card.edit", { title: "Blocked", description: "Blocked" }],
        ["card.set_completed", { completed: false }],
        ["card.move", { column, before: null }],
        ["card.trash", {}],
      ]) error(command(android, operation, { space, card, expected: current.version, ...parameters }), "conflict");
    };
    for (const current of [a, b, column, other, x, y]) ok(command(host, "column.create", { space, column: current, name: `Combined ${current}` }));
    for (const current of [ca, cb, card, child, cx, cy]) ok(command(host, "card.create", { space, column, card: current, title: `Combined ${current}`, description: "Preserved combined body" }));
    const createdTag = ok(command(host, "tag.create", { space, tag, name: `Combined ${tag}` }));
    ok(command(host, "card.set_tag", { space, card, tag, present: true, expected: read(host).version, expected_tag: createdTag.version }));
    ok(command(host, "card.set_completed", { space, card, completed: true, expected: read(host).version }));
    const original = read(host), versions = history(host);
    exchange(); pause(true);
    moveColumn(host, x, b); moveColumn(android, y, b);
    moveCard(host, cx, column, cb); moveCard(android, cy, column, cb);
    moveColumn(host, column, b); moveColumn(android, column, null);
    moveCard(host, card, column, cb); moveCard(android, card, same ? column : other, same ? ca : null);
    pause(false); exchange();
    assert.equal(parent(host).positions.length, 2); assert.equal(read(host).placements.length, 2);
    assert.deepEqual(parent(host), parent(android)); assert.deepEqual(read(host), read(android));
    assert.deepEqual(columns(host), columns(android)); assert.deepEqual(cards(host), cards(android));
    const neighbors = () => {
      const columnOrder = columns(host).filter(item => item.positions.length === 1).map(item => item.column);
      const cardOrder = cards(host).filter(item => item.columns.length === 1 && item.columns[0] === column && item.positions.length === 1).map(item => item.card);
      for (const current of [x, y]) assert.ok(columnOrder.indexOf(a) < columnOrder.indexOf(current) && columnOrder.indexOf(current) < columnOrder.indexOf(b));
      for (const current of [cx, cy]) assert.ok(cardOrder.indexOf(ca) < cardOrder.indexOf(current) && cardOrder.indexOf(current) < cardOrder.indexOf(cb));
      return { columnOrder: columnOrder.filter(item => [a, b, x, y].includes(item)), cardOrder: cardOrder.filter(item => [ca, cb, cx, cy].includes(item)) };
    };
    const order = neighbors();
    blocked();
    const oldParent = inspect(android, "column");
    ok(command(android, "card.edit", { space, card: child, title: "Available child", description: "Changed under both conflicts", expected: read(android, child).version }));
    error(command(android, "column.resolve_position", { space, column, expected: oldParent.column.version, position: oldParent.choices[0].position }), "stale_state");
    exchange();
    const first = columnFirst ? "column" : "card", second = columnFirst ? "card" : "column";
    for (const kind of [first, second]) {
      const identical = (kind === "column") === columnFirst;
      const otherBefore = kind === "column" ? read(host) : parent(host);
      pause(true); choose(host, kind, 0); choose(android, kind, identical ? 0 : 1); pause(false); exchange();
      assert.equal(kind === "column" ? parent(host).positions.length : read(host).placements.length, identical ? 1 : 2);
      if (!identical) { choose(android, kind, 0); exchange(); }
      if (kind === "column") {
        assert.deepEqual(read(host), otherBefore);
        if (read(host).placements.length > 1) blocked();
      } else {
        assert.deepEqual(parent(host).positions, otherBefore.positions);
        assert.equal(read(host).state, "active");
        if (!columnFirst) {
          assert.equal(parent(host).positions.length, 2);
          ok(command(android, "card.set_completed", { space, card, completed: false, expected: read(android).version }));
          ok(command(android, "card.set_completed", { space, card, completed: true, expected: read(android).version }));
          exchange();
        }
      }
    }
    assert.equal(parent(host).positions.length, 1); assert.equal(read(host).placements.length, 1);
    for (const key of ["card", "titles", "descriptions", "tags", "completed", "deletion_groups"]) assert.deepEqual(read(host)[key], original[key]);
    assert.deepEqual(history(host), versions); assert.deepEqual(history(android), versions);
    const saved = read(android, child);
    const trashed = ok(command(android, "card.trash", { space, card: child, expected: saved.version }));
    ok(command(android, "card.restore", { space, card: child, column: null, expected_column: null, expected: trashed.version }));
    moveCard(android, card, column, cb); moveColumn(android, column, a); exchange();
    assert.deepEqual(parent(host), parent(android)); assert.deepEqual(read(host), read(android));
    assert.deepEqual(cards(host), cards(android)); assert.deepEqual(columns(host), columns(android));
    assert.deepEqual(neighbors(), order); assert.deepEqual(history(host), versions);
    assert.deepEqual(read(host, child).titles, saved.titles);
    console.log(`Combined positions ${same ? "same" : "cross"}-column/${first} first: independent decisions, stale, blocks, children, neighbors, preservation and reverse passed.`);
  }
}
