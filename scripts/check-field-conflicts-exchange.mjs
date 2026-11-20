import assert from "node:assert/strict";
import { randomBytes } from "node:crypto";

export function checkFieldConflictsExchange({ host, android, space, pairing, call, command, ok, error }) {
  const id = () => randomBytes(16).toString("hex");
  const exchange = () => ok(call(android, "exchange.run", { space }));
  const pause = paused => {
    const current = ok(command(android, "sync.inspect", { space, pairing }));
    ok(command(android, "sync.pause", { space, pairing, paused, expected: current.pause_version }));
  };
  const original = ok(command(host, "space.read", { space })).names[0];
  for (const order of [["card", "column", "space"], ["space", "card", "column"], ["column", "space", "card"]]) {
    const column = id(), card = id(), sibling = id();
    const parameters = kind => kind === "space" ? { space } : { space, [kind]: kind === "card" ? card : column };
    const read = (target, kind) => ok(command(target, `${kind}.read`, parameters(kind)));
    const own = snapshot => Object.fromEntries(Object.entries(snapshot).filter(([key]) => !["version", "contents"].includes(key)));
    const history = target => ok(command(target, "card.history", { space, card, after: null, limit: 100, expected: null })).versions;
    const search = () => {
      const current = read(host, "card");
      for (const query of ["", ...current.titles, ...current.descriptions]) {
        const results = [host, android].map(target => ok(command(target, "card.search", { query, scope: { kind: "space", space } })));
        assert.deepEqual(results[0], results[1]);
        const hits = results[0].spaces.flatMap(item => item.hits);
        assert.equal(new Set(hits.map(hit => hit.card.card)).size, hits.length);
        const hit = hits.find(item => item.card.card === card);
        assert.ok(hit, "Saved conflicting variant missing from search");
        assert.deepEqual(hit.card, current);
        assert.equal(hit.placement_conflict, false);
        assert.deepEqual(results[0].spaces[0].space.names, read(host, "space").names);
      }
      for (const target of [host, android]) {
        const found = ok(command(target, "card.search", { query: current.titles[0], scope: { kind: "all" } }));
        const spaces = ok(command(target, "space.list", { include_trashed: false })).spaces;
        const ids = found.spaces.map(item => item.space.space);
        assert.deepEqual(ids, spaces.filter(item => ids.includes(item.space)).map(item => item.space));
        assert.equal(ids.filter(id => id === space).length, 1);
      }
    };
    const resolve = (target, kind, choice) => ok(command(target, kind === "card" ? "card.resolve_text" : `${kind}.resolve_name`, {
      ...parameters(kind), expected: read(target, kind).version,
      ...(kind === "card" ? { title: choice, description: `Body ${choice}` } : { name: choice }),
    }));
    ok(command(host, "column.create", { space, column, name: "Field column" }));
    for (const current of [card, sibling]) ok(command(host, "card.create", { space, column, card: current, title: "Field base", description: "Field body" }));
    const tag = id();
    const createdTag = ok(command(host, "tag.create", { space, tag, name: `Field tag ${card}` }));
    ok(command(host, "card.set_tag", { space, card, tag, present: true, expected: read(host, "card").version, expected_tag: createdTag.version }));
    exchange(); pause(true);
    const before = read(host, "card");
    for (const [target, label] of [[host, "PC"], [android, "Android"]]) {
      for (const kind of ["space", "column"]) ok(command(target, `${kind}.rename`, { ...parameters(kind), name: `${kind} ${label}`, expected: read(target, kind).version }));
      ok(command(target, "card.edit", { space, card, title: `Text ${label}`, description: `Body ${label}`, expected: read(target, "card").version }));
    }
    pause(false); exchange();
    const conflicted = Object.fromEntries(["space", "column", "card"].map(kind => [kind, read(host, kind)]));
    for (const kind of ["space", "column", "card"]) {
      assert.equal(conflicted[kind].state, kind === "card" ? "conflicted" : "active");
      assert.deepEqual(read(android, kind), conflicted[kind]);
    }
    assert.deepEqual(conflicted.space.names, ["space Android", "space PC"]);
    assert.deepEqual(conflicted.column.names, ["column Android", "column PC"]);
    assert.deepEqual(conflicted.card.titles, ["Text Android", "Text PC"]);
    search();
    for (const kind of ["space", "column"]) error(command(android, `${kind}.rename`, { ...parameters(kind), name: "Overwrite", expected: read(android, kind).version }), "conflict");
    error(command(android, "card.edit", { space, card, title: "Overwrite", description: "", expected: read(android, "card").version }), "conflict");
    const healthy = ok(command(android, "card.read", { space, card: sibling }));
    ok(command(android, "card.set_completed", { space, card: sibling, completed: true, expected: healthy.version }));
    const added = id();
    ok(command(android, "card.create", { space, column, card: added, title: "Created inside conflicted names", description: "" }));
    exchange(); pause(true);
    for (const [target, label] of [[host, "PC choice"], [android, "Android choice"]]) {
      for (const kind of order) {
        const others = Object.fromEntries(["space", "column", "card"].filter(other => other !== kind).map(other => [other, read(target, other)]));
        resolve(target, kind, `${kind} ${label}`);
        for (const [other, value] of Object.entries(others)) assert.deepEqual(own(read(target, other)), own(value), "Resolution changed another object's own fields");
        if (kind === "card") {
          assert.equal(read(target, kind).state, "active");
          const current = read(target, kind);
          ok(command(target, "card.set_completed", { space, card, completed: true, expected: current.version }));
          const changed = read(target, kind);
          ok(command(target, "card.set_completed", { space, card, completed: false, expected: changed.version }));
        } else if (read(target, "card").titles.length > 1) {
          error(command(target, "card.set_completed", { space, card, completed: true, expected: read(target, "card").version }), "conflict");
        }
      }
    }
    pause(false); exchange(); exchange();
    search();
    for (const kind of ["space", "column", "card"]) {
      const current = read(host, kind);
      assert.equal(current.state, kind === "card" ? "conflicted" : "active");
      assert.deepEqual(read(android, kind), current);
      assert.deepEqual(kind === "card" ? current.titles : current.names, [`${kind} Android choice`, `${kind} PC choice`]);
    }
    assert.equal(history(host).length, 5);
    for (const kind of order) resolve(android, kind, `Final ${kind}`);
    exchange(); exchange();
    for (const kind of ["space", "column", "card"]) {
      assert.equal(read(host, kind).state, "active");
      assert.deepEqual(read(android, kind), read(host, kind));
    }
    const after = read(host, "card");
    for (const key of ["space", "card", "columns", "positions", "placements", "tags", "completed", "deletion_groups"]) assert.deepEqual(after[key], before[key]);
    assert.deepEqual(history(host), history(android)); assert.equal(history(host).length, 6);
    const text = history(host)[0];
    ok(command(android, "card.restore_text", { space, card, text_version: text.id, expected: read(android, "card").version }));
    for (const [kind, name] of [["space", original], ["column", "Field column"]]) ok(command(android, `${kind}.rename`, { ...parameters(kind), name, expected: read(android, kind).version }));
    exchange();
    assert.deepEqual(read(host, "space").names, [original]);
    assert.deepEqual(read(host, "column").names, ["Field column"]);
    assert.deepEqual(read(host, "card").titles, ["Field base"]);
    assert.deepEqual(read(host, "card"), read(android, "card"));
    assert.deepEqual(history(host), history(android)); assert.equal(history(host).length, 7);
    assert.deepEqual(ok(command(host, "card.read", { space, card: sibling })).completed, [true]);
    assert.deepEqual(ok(command(host, "card.read", { space, card: added })).titles, ["Created inside conflicted names"]);
    search();
    console.log(`Actual APK combined field conflicts ${order.join(" → ")}: isolation/blocking/content/independent decisions/repeated delivery/history/reverse passed.`);
  }
}
