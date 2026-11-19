import assert from "node:assert/strict";
import { randomBytes } from "node:crypto";

export function checkContainerDeletionExchange({ host, android, space, pairing, call, command, ok, error }) {
  const id = () => randomBytes(16).toString("hex");
  const run = (device, action, args = {}) => ok(command(device, action, { space, ...args }));
  const exchange = () => ok(call(android, "exchange.run", { space }));
  for (const device of [host, android]) {
    const current = run(device, "sync.inspect", { pairing });
    run(device, "sync.select", { pairing, selected: true, expected: current.selection_version });
  }
  const column = id(), card = id(), previous = id(), added = id();
  const readColumn = device => run(device, "column.read", { column });
  const readCard = (device, card) => run(device, "card.read", { card });
  const create = (device, card, title) => run(device, "card.create", { column, card, title, description: "" });
  const edit = (device, title) => run(device, "card.edit", { card, title, description: "", expected: readCard(device, card).version });
  run(host, "column.create", { column, name: "Paired container" });
  create(host, card, "Original card");
  create(host, previous, "Earlier trash");
  const old = run(host, "card.trash", { card: previous, expected: readCard(host, previous).version });
  exchange();
  const deleted = run(host, "column.trash", { column, expected: readColumn(host).version });
  edit(android, "Updated before deleting the column");
  create(android, added, "Independent new card");
  exchange();
  const columnConflict = readColumn(android);
  assert.equal(columnConflict.state, "conflicted");
  assert.deepEqual(readColumn(host), columnConflict);
  const group = { kind: "objects", columns: [column], cards: [] };
  const preview = run(host, "trash.inspect", { selection: group });
  error(command(host, "trash.purge", { space, selection: group, expected: preview.version }), "conflict");
  const member = { kind: "objects", columns: [], cards: [card] };
  const memberPreview = run(host, "trash.inspect", { selection: member });
  error(command(host, "trash.purge", { space, selection: member, expected: memberPreview.version }), "conflict");
  assert.deepEqual(readColumn(host), columnConflict);
  run(android, "column.resolve_deletion", { column, expected: columnConflict.version, keep: false });
  exchange();
  assert.deepEqual(readCard(host, added).deletion_groups, deleted.deletion_groups);
  run(host, "column.restore", { column, expected: readColumn(host).version });
  exchange();
  assert.equal(readCard(android, added).state, "active");
  assert.deepEqual(readCard(android, previous), old);

  const readSpace = device => run(device, "space.read");
  run(android, "space.trash", { expected: readSpace(android).version });
  edit(host, "Edited while the space was in trash");
  exchange();
  const spaceConflict = readSpace(host);
  assert.equal(spaceConflict.state, "conflicted");
  assert.deepEqual(readSpace(android), spaceConflict);
  run(host, "space.resolve_deletion", { expected: spaceConflict.version, keep: true });
  exchange();
  assert.equal(readCard(android, card).state, "active");
  assert.deepEqual(readCard(android, previous), old);

  run(host, "space.trash", { expected: readSpace(host).version });
  const selection = { kind: "space" };
  const purge = run(host, "trash.inspect", { selection });
  run(host, "trash.purge", { selection, expected: purge.version });
  edit(android, "Late edit after final cleanup");
  exchange();
  const recovery = run(android, "trash.inspect", { selection });
  assert.deepEqual(recovery.cards.map(item => item.card), [card]);
  assert.deepEqual(recovery.columns.map(item => item.column), [column]);
  error(command(android, "trash.purge", { space, selection, expected: recovery.version }), "conflict");
  run(android, "trash.resolve_purge", { selection, expected: recovery.version, keep: true });
  exchange();
  exchange();
  assert.deepEqual(readSpace(host), readSpace(android));
  assert.deepEqual(readCard(host, card).titles, ["Late edit after final cleanup"]);
  assert.equal(readCard(android, card).state, "active");
  for (const device of [host, android]) {
    for (const card of [added, previous]) error(command(device, "card.read", { space, card }), "not_found");
  }
  console.log("Paired containers: column and space deletion decisions, atomic purge refusal and minimum recovery of late edits passed.");
}
