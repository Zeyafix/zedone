import assert from "node:assert/strict";
import { randomBytes } from "node:crypto";

export function checkCardDeletionExchange({ host, android, space, pairing, call, command, ok, error }) {
  const column = randomBytes(16).toString("hex");
  const phoneColumn = randomBytes(16).toString("hex");
  const tag = randomBytes(16).toString("hex");
  const cards = [false, true].map(keep => ({ keep, card: randomBytes(16).toString("hex") }));
  const read = (target, card) => ok(command(target, "card.read", { space, card }));
  const exchange = () => ok(call(android, "exchange.run", { space }));
  const pause = paused => {
    const current = ok(command(android, "sync.inspect", { space, pairing }));
    ok(command(android, "sync.pause", { space, pairing, paused, expected: current.pause_version }));
  };
  ok(command(host, "column.create", { space, column, name: "Independent card deletion" }));
  ok(command(host, "column.create", { space, column: phoneColumn, name: "Phone placement" }));
  ok(command(host, "tag.create", { space, tag, name: "Conflicting assignment" }));
  const setTag = (target, card, present) => ok(command(target, "card.set_tag", {
    space, card, tag, present, expected: read(target, card).version,
    expected_tag: ok(command(target, "tag.read", { space, tag })).version,
  }));
  for (const { card } of cards) {
    ok(command(host, "card.create", { space, column, card, title: "Before exchange", description: "" }));
    setTag(host, card, true);
  }
  exchange();
  pause(true);
  for (const { card, keep } of cards) {
    const [deleter, editor] = keep ? [android, host] : [host, android];
    ok(command(deleter, "card.move", { space, card, column: phoneColumn, before: null, expected: read(deleter, card).version }));
    setTag(deleter, card, false);
    const pending = read(deleter, card);
    ok(command(deleter, "card.trash", { space, card, expected: pending.version }));
    ok(command(editor, "card.move", { space, card, column: phoneColumn, before: null, expected: read(editor, card).version }));
    ok(command(editor, "card.move", { space, card, column, before: null, expected: read(editor, card).version }));
    setTag(editor, card, false);
    setTag(editor, card, true);
    ok(command(editor, "card.edit", { space, card, expected: read(editor, card).version, title: "Independent edit", description: "Preserved on both devices" }));
  }
  pause(false);
  exchange();
  exchange();
  const selection = { kind: "objects", columns: [], cards: cards.map(({ card }) => card) };
  const preview = ok(command(host, "trash.inspect", { space, selection }));
  assert.deepEqual(preview.conflicts, cards.map(({ card }) => `card:${card}`).sort());
  error(command(host, "trash.purge", { space, selection, expected: preview.version }), "conflict");
  for (const { card, keep } of cards) {
    const before = read(host, card);
    assert.deepEqual(read(android, card), before);
    assert.equal(before.deletion_conflict, true);
    assert.deepEqual(before.titles, ["Independent edit"]);
    const target = keep ? host : android;
    const decided = ok(command(target, "card.resolve_deletion", { space, card, expected: before.version, keep }));
    assert.equal(decided.state, "conflicted");
    assert.equal(decided.deletion_conflict, undefined);
    assert.deepEqual(decided.descriptions, ["Preserved on both devices"]);
    assert.equal(decided.card, card);
    const { choices } = ok(command(target, "card.inspect_placements", { space, card }));
    const placement = choices.find(choice => choice.placement.column === phoneColumn).placement;
    const placed = ok(command(target, "card.resolve_placement", { space, card, placement, expected: decided.version }));
    assert.deepEqual(placed.deletion_groups, decided.deletion_groups);
    const assigned = ok(command(target, "card.resolve_tag", {
      space, card, tag, present: true, expected: placed.version,
      expected_tag: ok(command(target, "tag.read", { space, tag })).version,
    }));
    assert.equal(assigned.state, keep ? "active" : "trashed");
    assert.deepEqual(assigned.tags, [tag]);
    assert.deepEqual(assigned.deletion_groups, decided.deletion_groups);
  }
  exchange();
  for (const { card, keep } of cards) {
    const result = read(host, card);
    assert.deepEqual(read(android, card), result);
    if (!keep) {
      const restored = ok(command(android, "card.restore", { space, card, expected: result.version }));
      assert.equal(restored.state, "active");
      assert.deepEqual(restored.titles, ["Independent edit"]);
    }
  }
  exchange();
  assert.ok(cards.every(({ card }) => read(host, card).state === "active"));
  console.log("Paired card deletion: both choices, mixed placement and tag resolution, atomic rejection of purge, same IDs, delivery and ordinary restore passed.");
}

export function checkCardRestoreExchange({ host, android, space, call, command, ok, error }) {
  const [source, destination, moved, remaining] = Array.from({ length: 4 }, () => randomBytes(16).toString("hex"));
  const readColumn = (target, column) => ok(command(target, "column.read", { space, column }));
  const readCard = (target, card) => ok(command(target, "card.read", { space, card }));
  const exchange = () => ok(call(android, "exchange.run", { space }));
  ok(command(host, "column.create", { space, column: source, name: "Restore source" }));
  ok(command(host, "column.create", { space, column: destination, name: "Restore destination" }));
  const original = ok(command(host, "card.create", { space, column: source, card: moved, title: "Move on restore", description: "Original description" }));
  ok(command(host, "card.create", { space, column: source, card: remaining, title: "Restore with source", description: "" }));
  const before = readColumn(host, source);
  const group = ok(command(host, "column.trash", { space, column: source, expected: before.version })).deletion_groups;
  exchange();
  const deleted = readCard(android, moved);
  assert.deepEqual(deleted.deletion_groups, group);
  assert.deepEqual(readCard(android, remaining).deletion_groups, group);
  const restore = { space, card: moved, expected: deleted.version, column: destination };
  error(command(android, "card.restore", { ...restore, expected_column: before.version }), "stale_state");
  assert.deepEqual(readCard(android, moved), deleted);
  const restored = ok(command(android, "card.restore", { ...restore, expected_column: readColumn(android, destination).version }));
  assert.equal(restored.state, "active");
  assert.equal(restored.card, original.card);
  assert.deepEqual(restored.titles, original.titles);
  assert.deepEqual(restored.descriptions, original.descriptions);
  assert.deepEqual(restored.columns, [destination]);
  assert.deepEqual(restored.deletion_groups, []);
  exchange();
  exchange();
  const current = readColumn(host, source);
  assert.deepEqual(current.deletion_groups, group);
  assert.equal(current.state, "trashed");
  assert.deepEqual(readCard(host, moved), restored);
  ok(command(host, "column.restore", { space, column: source, expected: current.version }));
  exchange();
  exchange();
  for (const target of [host, android]) {
    assert.deepEqual(readColumn(target, source).contents.map(item => item.card), [remaining]);
    assert.deepEqual(readColumn(target, destination).contents.map(item => item.card), [moved]);
    assert.deepEqual(readCard(target, moved), restored);
    assert.equal(readCard(target, remaining).state, "active");
  }
  console.log("Paired card restore: stale target rejection, same ID and text, repeated delivery, original group restore without moving or duplicating the card passed.");
}
