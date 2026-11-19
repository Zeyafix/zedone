import assert from "node:assert/strict";
import { randomBytes } from "node:crypto";

export function checkTagCollisionsExchange({ host, android, space, pairing, call, command, ok, error, scope }) {
  const id = () => randomBytes(16).toString("hex");
  const exchange = () => ok(call(android, "exchange.run", { space }));
  const pause = paused => {
    const current = ok(command(android, "sync.inspect", { space, pairing }));
    ok(command(android, "sync.pause", { space, pairing, paused, expected: current.pause_version }));
  };
  const column = id(), card = id(), other = id();
  const readCard = (target, cardId = card) => ok(command(target, "card.read", { space, card: cardId }));
  const readTag = (target, tag) => ok(command(target, "tag.read", { space, tag }));
  const create = (target, tag, name) => ok(command(target, "tag.create", { space, tag, name }));
  const rename = (target, tag, name) => ok(command(target, "tag.rename", { space, tag, name, expected: readTag(target, tag).version }));
  const set = (target, tag, present, cardId = card) => ok(command(target, "card.set_tag", { space, card: cardId, tag, present, expected: readCard(target, cardId).version, expected_tag: readTag(target, tag).version }));
  const remove = (target, tag) => ok(command(target, "tag.delete", { space, tag, expected: readTag(target, tag).version }));
  const decide = (target, tag, keep) => ok(command(target, "tag.resolve_deletion", { space, tag, keep, expected: readTag(target, tag).version }));
  ok(command(host, "column.create", { space, column, name: "Tag collisions" }));
  for (const cardId of [card, other]) ok(command(host, "card.create", { space, column, card: cardId, title: "Preserved card", description: "Preserved body" }));
  exchange();
  for (const creation of scope === "deletion" ? [] : [false, true]) {
    const a = id(), b = id(), suffix = id();
    create(host, a, `Работа ${suffix}`); set(host, a, true);
    if (!creation) { create(host, b, `Личное ${suffix}`); set(host, b, true); }
    exchange(); pause(true);
    rename(host, a, `Важное ${suffix}`);
    if (creation) { create(android, b, `ВАЖНОЕ ${suffix}`); set(android, b, true); }
    else rename(android, b, `ВАЖНОЕ ${suffix}`);
    pause(false); exchange(); exchange();
    const before = readCard(host);
    assert.equal(before.state, "active");
    for (const tag of [a, b]) {
      assert.equal(readTag(host, tag).state, "conflicted");
      assert.deepEqual(readTag(host, tag), readTag(android, tag));
      for (const present of [false, true]) error(command(android, "card.set_tag", { space, card, tag, present, expected: readCard(android).version, expected_tag: readTag(android, tag).version }), "conflict");
    }
    const group = readTag(host, a).name_group;
    const choices = (target, names) => ({ space, tag: a, names: group.map((tag, i) => ({ tag, name: `${names[i]} ${suffix}`, expected: readTag(target, tag).version })) });
    const invalid = choices(host, ["Work", "Work"]);
    invalid.names[1].name = ` ${invalid.names[0].name.toUpperCase()} `;
    error(command(host, "tag.resolve_names", invalid), "invalid_request");
    pause(true);
    ok(command(host, "tag.resolve_names", choices(host, ["Все", "Всё"])));
    ok(command(android, "tag.resolve_names", choices(android, ["Дом", "Дела"])));
    pause(false); exchange();
    assert.equal(readTag(host, a).state, "conflicted");
    ok(command(android, "tag.resolve_names", choices(android, ["Все", "Всё"])));
    exchange();
    for (const field of ["tags", "titles", "descriptions", "columns", "positions", "completed"]) assert.deepEqual(readCard(host)[field], before[field]);
    for (const tag of [a, b]) { assert.equal(readTag(host, tag).state, "active"); set(android, tag, false); set(android, tag, true); }
    exchange();
  }
  if (scope !== "deletion") {
    const old = id(), live = id(), late = id(), suffix = id();
    pause(true);
    create(host, old, `Мост ${suffix}`); set(host, old, true);
    rename(host, old, `Личное ${suffix}`); remove(host, old);
    create(host, live, `Мост ${suffix}`); set(host, live, true);
    create(android, late, `МОСТ ${suffix}`); set(android, late, true);
    pause(false); exchange(); exchange();
    assert.equal(readTag(host, old).state, "deleted");
    const group = readTag(host, live).name_group;
    assert.deepEqual([...group].sort(),[live,late].sort());
    ok(command(android,"tag.resolve_names",{space,tag:live,names:group.map((tag,i)=>({tag,name:`${i ? "Всё" : "Все"} ${suffix}`,expected:readTag(android,tag).version}))}));
    exchange();
    for(const target of [host,android]) {
      assert.equal(readTag(target,old).state,"deleted");
      for(const tag of [live,late]) { assert.equal(readTag(target,tag).state,"active"); assert.equal(readCard(target).tags.includes(tag),true); }
      assert.equal(readCard(target).tags.includes(old),false);
    }
    console.log("Deleted late-bridge ancestor stays deleted; only live names resolve, old assignments stay removed.");
  }
  for (const [a, b] of scope === "names" ? [] : [[false, false], [true, true], [false, true], [true, false]]) {
    const tag = id(); create(host, tag, `Удаление ${tag}`); set(host, tag, true);
    exchange(); pause(true); remove(host, tag); set(android, tag, true, other);
    pause(false); exchange();
    assert.equal(readTag(host, tag).deletion_conflict, true);
    for (const target of [host, android]) {
      assert.equal(readCard(target).state, "conflicted"); assert.equal(readCard(target, other).state, "conflicted");
    }
    pause(true); decide(host, tag, a); decide(android, tag, b); pause(false); exchange();
    assert.deepEqual(readTag(host, tag), readTag(android, tag));
    assert.equal(readTag(host, tag).deletion_conflict ?? false, a !== b);
    if (a !== b) { decide(android, tag, true); exchange(); }
    const keep = a === b ? a : true;
    for (const cardId of [card, other]) assert.equal(readCard(host, cardId).tags.includes(tag), keep);
    if (keep) { set(android, tag, false); set(android, tag, true); exchange(); remove(host, tag); exchange(); }
    assert.equal(readCard(android).tags.includes(tag), false);
    assert.equal(readCard(android, other).tags.includes(tag), false);
  }
  console.log(`Tag collisions exchange ${scope || "all"}: IDs/Unicode/blocks/independent decisions/old and new assignments/reverse passed.`);
}
