import assert from "node:assert/strict";
import { randomBytes } from "node:crypto";

export function checkTagConflictsExchange({ host, android, space, pairing, call, command, ok, error }) {
  const id = () => randomBytes(16).toString("hex");
  const exchange = () => ok(call(android, "exchange.run", { space }));
  const pause = paused => {
    const current = ok(command(android, "sync.inspect", { space, pairing }));
    ok(command(android, "sync.pause", { space, pairing, paused, expected: current.pause_version }));
  };
  const column = id(), card = id(), first = id(), second = id(), suffix = id();
  const readCard = target => ok(command(target, "card.read", { space, card }));
  const readTag = (target, tag) => ok(command(target, "tag.read", { space, tag }));
  const set = (target, tag, present) => ok(command(target, "card.set_tag", { space, card, tag, present, expected: readCard(target).version, expected_tag: readTag(target, tag).version }));
  const resolve = (target, present) => ok(command(target, "card.resolve_tag", { space, card, tag: second, present, expected: readCard(target).version, expected_tag: readTag(target, second).version }));
  ok(command(host, "column.create", { space, column, name: "Tag exchange" }));
  ok(command(host, "card.create", { space, column, card, title: "Tag preservation", description: "Unchanged body" }));
  exchange(); pause(true);
  ok(command(host, "tag.create", { space, tag: first, name: `РАБОТА ${suffix}` }));
  ok(command(android, "tag.create", { space, tag: second, name: `Работа ${suffix}` }));
  set(host, first, true); set(android, second, true);
  pause(false); exchange(); exchange();
  assert.deepEqual(readTag(host, first), readTag(android, second));
  assert.deepEqual(readTag(host, first), readTag(host, second));
  assert.equal(readCard(host).tags.length, 1);
  const spelling = readTag(host, first).names[0];
  assert.ok([`РАБОТА ${suffix}`, `Работа ${suffix}`].includes(spelling));
  for (const [a, b] of [[false, false], [true, true], [false, true], [true, false]]) {
    set(host, first, true); exchange(); pause(true);
    set(host, first, false); set(android, second, false); set(android, second, true);
    pause(false); exchange();
    const conflicted = readCard(host);
    assert.equal(conflicted.state, "conflicted");
    assert.deepEqual(conflicted, readCard(android));
    const inspected = ok(command(android, "card.inspect_tags", { space, card }));
    assert.deepEqual(inspected.assignments[0].values, [false, true]);
    error(command(host, "card.set_tag", { space, card, tag: first, present: false, expected: conflicted.version, expected_tag: readTag(host, first).version }), "conflict");
    pause(true); resolve(host, a); resolve(android, b); pause(false); exchange(); exchange();
    assert.deepEqual(readCard(host), readCard(android));
    assert.equal(readCard(host).state, a === b ? "active" : "conflicted");
    if (a !== b) { resolve(android, false); exchange(); }
    assert.equal(readCard(host).state, "active");
  }
  set(host, first, true); exchange(); pause(true);
  const bridge = id(), third = id(), fourth = id();
  ok(command(host, "tag.create", { space, tag: bridge, name: `Мост ${suffix}` }));
  set(host, bridge, true);
  ok(command(host, "tag.rename", { space, tag: bridge, name: `Личное ${suffix}`, expected: readTag(host, bridge).version }));
  ok(command(host, "tag.create", { space, tag: third, name: `Мост ${suffix}` }));
  ok(command(android, "tag.create", { space, tag: fourth, name: `МОСТ ${suffix}` }));
  set(host, third, true); set(android, fourth, true);
  pause(false); exchange(); exchange();
  const group = readTag(host, bridge).name_group;
  assert.equal(group.length, 3);
  assert.deepEqual([...group].sort(), [bridge, third, fourth].sort());
  const before = readCard(host);
  assert.equal(before.tags.length, 4);
  const names = ["Личное", "Все", "Всё"].map(n => `${n} ${suffix}`);
  const choose = target => ({ space, tag: bridge, names: group.map((tag, i) => ({ tag, name: names[i], expected: readTag(target, tag).version })) });
  const invalid = choose(host); invalid.names.pop();
  error(command(host, "tag.resolve_names", invalid), "invalid_request");
  pause(true);
  ok(command(host, "tag.resolve_names", choose(host)));
  ok(command(android, "tag.resolve_names", choose(android)));
  pause(false); exchange(); exchange();
  for (const tag of group) { assert.equal(readTag(host, tag).state, "active"); assert.deepEqual(readTag(host, tag), readTag(android, tag)); }
  for (const field of ["tags", "titles", "descriptions", "columns", "positions", "completed"]) assert.deepEqual(readCard(host)[field], before[field]);
  for (const tag of group) { set(android, tag, false); set(android, tag, true); }
  exchange();
  assert.deepEqual(readCard(host), readCard(android));
  console.log("Tag exchange: aliases/spelling/S05/both choices/independent decisions/late bridge/three IDs/whole names/reverse preserved.");
}
