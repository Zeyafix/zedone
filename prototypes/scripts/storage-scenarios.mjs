import assert from "node:assert/strict";
import { createHash } from "node:crypto";

// Shared checks call the real Rust API via HTTP or Android IPC; no SQLite writes.
export async function checkStorage(call) {
  for (const space of ["A", "B"]) await call({ action: "delete", space });
  const a = (await call({ action: "seed", space: "A" })).snapshot;
  assert.equal(a.space, "A");
  assert.deepEqual(a.columns, ["Колонка 1", "Колонка 2", "Колонка 3"]);
  assert.equal(a.cards.length, 30);
  for (let index = 0; index < 30; index++) {
    const card = a.cards[index];
    assert.equal(card.id, index + 1);
    assert.equal(card.title, `Карточка ${String(index + 1).padStart(4, "0")}`);
    assert.equal(card.column, Math.floor(index / 10));
    const lines = index < 24 ? 5 : index < 29 ? 100 : 200;
    assert.equal(card.text, Array.from({ length: lines }, (_, n) => `Заметка ${String(n + 1).padStart(4, "0")}: ёж, ЕЖ, café, 😀.`).join("\n"));
  }
  const bytes = Buffer.from(JSON.stringify(a));
  console.log(`Synthetic A: ${bytes.length} UTF-8 JSON bytes; SHA-256 ${createHash("sha256").update(bytes).digest("hex")}.`);
  assert.deepEqual((await call({ action: "seed", space: "A" })).snapshot, a);
  const b = (await call({ action: "seed", space: "B" })).snapshot;
  b.cards[0].text = "B_ONLY_SENTINEL";
  await call({ action: "restore", snapshot: b });
  const snapshot = (await call({ action: "snapshot", space: "A" })).snapshot;
  assert.deepEqual(snapshot, a);
  assert.ok(!JSON.stringify(snapshot).includes("B_ONLY_SENTINEL"));
  for (const [query, count] of [["ЁЖ", 30], ["еж", 30], ["ёж, еж", 30], [" café ", 30], ["😀", 30], ["%_", 0], ["' OR 1=1 --", 0], ["  ", 30]]) {
    assert.equal((await call({ action: "read", space: "A", query })).snapshot.cards.length, count, query);
  }
  const invalid = structuredClone(snapshot);
  invalid.cards[1].column = 999;
  await assert.rejects(call({ action: "restore", snapshot: invalid }));
  assert.deepEqual((await call({ action: "snapshot", space: "A" })).snapshot, a);
  await call({ action: "delete", space: "A" });
  await call({ action: "delete", space: "A" });
  assert.equal((await call({ action: "read", space: "A", query: "" })).snapshot, null);
  assert.deepEqual((await call({ action: "restore", snapshot })).snapshot, a);
  assert.deepEqual((await call({ action: "snapshot", space: "B" })).snapshot, b);
  let previous = a;
  for (let revision = 1; revision <= 4; revision++) {
    const next = structuredClone(a);
    next.columns[0] = `Revision ${revision}`;
    next.cards[0].title = `Revision ${revision}`;
    const [, read] = await Promise.all([
      call({ action: "restore", snapshot: next }),
      call({ action: "snapshot", space: "A" }),
    ]);
    assert.ok([previous, next].some(value => JSON.stringify(value) === JSON.stringify(read.snapshot)), "Concurrent snapshot must contain one complete revision");
    previous = next;
  }
  await call({ action: "restore", snapshot: a });
  console.log("Storage: exact fixture, idempotent seed/delete, Unicode, scoped snapshot, restore and real SQL rollback passed.");
  return { a, b };
}

export async function checkStorageUI(page, activate = selector => page.locator(selector).click()) {
  const fill = value => page.evaluate(value => {
    const input = document.querySelector("#storage-query");
    input.value = value;
    input.dispatchEvent(new Event("input", { bubbles: true }));
  }, value);
  const click = async selector => {
    await activate(selector);
    await page.waitForFunction(() => !document.querySelector("#storage-busy"));
    assert.equal(await page.locator("#storage-error").count(), 0);
    return JSON.parse(await page.locator("#storage-result").textContent());
  };
  await fill("ёж");
  assert.equal((await click("#storage-read")).snapshot.cards.length, 30);
  await click("#storage-snapshot");
  const snapshot = JSON.parse(await page.locator("#storage-saved-snapshot").textContent());
  assert.equal(snapshot.space, "A");
  assert.equal(snapshot.cards.length, 30);
  assert.equal((await click("#storage-delete")).snapshot, null);
  assert.deepEqual((await click("#storage-restore")).snapshot, snapshot);
  await fill("не существует");
  assert.equal((await click("#storage-read")).snapshot.cards.length, 0);
  await fill("");
  assert.deepEqual((await click("#storage-read")).snapshot, snapshot);
  console.log("Storage UI: query, clear query, full snapshot, delete and restore passed.");
}
