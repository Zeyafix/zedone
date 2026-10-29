import assert from "node:assert/strict";

export async function checkBoard(call) {
  const initial = (await call({ action: "snapshot", space: "A" })).snapshot;
  const b = (await call({ action: "snapshot", space: "B" })).snapshot;
  const request = { action: "move_card", expected: initial, card: 1, column: 1, position: 2 };
  const moved = (await call(request)).snapshot;
  assert.equal(moved.cards.filter(card => card.column === 1)[2].id, 1);
  assert.equal(new Set(moved.cards.map(card => card.id)).size, 30);
  await assert.rejects(call(request));
  await assert.rejects(call({ action: "move_column", expected: initial, column: 0, position: 2 }));
  const restored = (await call({ action: "move_card", expected: moved, card: 1, column: 0, position: 0 })).snapshot;
  assert.deepEqual(restored, initial);
  const reordered = (await call({ action: "move_column", expected: initial, column: 0, position: 2 })).snapshot;
  assert.equal(reordered.columns[2], initial.columns[0]);
  assert.deepEqual(reordered.cards.filter(card => card.column === 2).map(card => card.id), initial.cards.slice(0, 10).map(card => card.id));
  assert.deepEqual((await call({ action: "snapshot", space: "B" })).snapshot, b);
  assert.deepEqual((await call({ action: "move_column", expected: reordered, column: 2, position: 0 })).snapshot, initial);
  console.log("Board API: move/inverse, order, stale/repeated rejection, identities and scoped data passed.");
  return initial;
}

export async function checkBoardUI(page, call, activate = selector => page.locator(selector).click(), touch) {
  const initial = (await call({ action: "snapshot", space: "A" })).snapshot;
  const pointer = touch ?? await page.context().newCDPSession(page);
  async function idle() {
    await page.waitForFunction(() => !document.querySelector("#board-busy"));
  }
  const read = async () => { await activate("#board-read"); await idle(); };
  const saved = async () => {
    await page.waitForFunction(() => document.querySelector("#board-message")?.textContent === "Перемещение сохранено.");
    await idle();
  };
  const current = async () => (await call({ action: "snapshot", space: "A" })).snapshot;
  async function choose(selector, value) {
    await page.evaluate(({ selector, value }) => {
      const select = document.querySelector(selector);
      select.value = String(value);
      select.dispatchEvent(new Event("change", { bubbles: true }));
    }, { selector, value });
  }
  await read();
  await activate("#card-move-1");
  await page.waitForFunction(() => document.querySelector('[role="dialog"]'));
  assert.equal(await page.evaluate(() => document.activeElement.id), "move-column");
  // Reka traps focus; these key presses are real browser input, also in WebView.
  for (let i = 0; i < 6; i++) {
    await page.keyboard.press("Tab");
    assert.equal(await page.evaluate(() => !!document.activeElement.closest('[role="dialog"]')), true);
  }
  await page.keyboard.press("Escape");
  await page.waitForFunction(() => !document.querySelector('[role="dialog"]'));
  assert.equal(await page.evaluate(() => document.activeElement.id), "card-move-1");
  await activate("#card-move-1");
  if (touch) {
    await choose("#move-column", 1);
    await choose("#move-position", 2);
    await activate("#move-save");
  } else {
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Tab");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Tab");
    await page.keyboard.press("Enter");
  }
  await saved();
  const moved = await current();
  assert.equal(moved.cards.filter(card => card.column === 1)[2].id, 1);
  await activate("#board-undo"); await saved();
  assert.deepEqual(await current(), initial);
  console.log("Board UI: explicit card move and inverse passed.");
  await activate("#column-move-0");
  await choose("#move-position", 2);
  await activate("#move-save"); await saved();
  assert.equal((await current()).columns[2], initial.columns[0]);
  await activate("#board-undo"); await saved();
  assert.deepEqual(await current(), initial);
  console.log("Board UI: explicit column move and inverse passed.");

  // Bring the board to the viewport, keeping two columns at least partly visible.
  await page.evaluate(() => {
    const board = document.querySelector(".board");
    board.scrollIntoView({ block: "center" });
    board.scrollLeft = 0;
    document.querySelectorAll(".card-list").forEach(list => { list.scrollTop = 0; });
  });
  const point = async selector => {
    // Sortable animates for 150 ms after a drop; measure only the settled handle.
    await page.waitForTimeout(180);
    return page.evaluate(selector => {
      const box = document.querySelector(selector).getBoundingClientRect();
      return { x: Math.min(box.x + box.width / 2, innerWidth - 18), y: box.y + box.height / 2 };
    }, selector);
  };
  async function gesture(from, to, hold = 350, linger = 250) {
    let scroll;
    const position = () => page.evaluate(() => ({
      vertical: document.querySelector(".card-list").scrollTop,
      horizontal: document.querySelector(".board").scrollLeft,
    }));
    if (touch) {
      await touch.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [from] });
      await new Promise(resolve => setTimeout(resolve, hold));
      for (let i = 1; i <= 12; i++) {
        await touch.send("Input.dispatchTouchEvent", { type: "touchMove", touchPoints: [{ x: from.x + (to.x - from.x) * i / 12, y: from.y + (to.y - from.y) * i / 12 }] });
        await new Promise(resolve => setTimeout(resolve, 25));
      }
      await new Promise(resolve => setTimeout(resolve, linger));
      scroll = await position();
      await touch.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
    } else {
      const mouse = (type, point, buttons, button = "none") => pointer.send("Input.dispatchMouseEvent", { type, ...point, buttons, button, clickCount: button === "left" ? 1 : 0 });
      await mouse("mouseMoved", from, 0);
      await mouse("mousePressed", from, 1, "left");
      await mouse("mouseMoved", { x: from.x + 8, y: from.y + 8 }, 1);
      await page.waitForFunction(() => !!document.querySelector(".sortable-fallback"));
      for (let i = 1; i <= 8; i++) {
        await mouse("mouseMoved", { x: from.x + (to.x - from.x) * i / 8, y: from.y + (to.y - from.y) * i / 8 }, 1);
        await new Promise(resolve => setTimeout(resolve, 25));
      }
      // Sortable throttles scroll detection; include the final settled pointer position.
      await new Promise(resolve => setTimeout(resolve, 60));
      await mouse("mouseMoved", { x: to.x + 1, y: to.y }, 1);
      await new Promise(resolve => setTimeout(resolve, linger));
      scroll = await position();
      await mouse("mouseReleased", { x: to.x + 1, y: to.y }, 0, "left");
    }
    return scroll;
  }
  const reorder = async () => {
    await page.evaluate(() => {
      document.querySelector(".board").scrollIntoView({ block: "center" });
      document.querySelector(".board").scrollLeft = 0;
      document.querySelectorAll(".card-list").forEach(list => { list.scrollTop = 0; });
    });
    await gesture(await point('[data-card="1"] .card-handle'), await point('[data-card="3"] .card-handle'));
  };
  await reorder();
  await saved();
  assert.equal((await current()).cards[0].id, 2);
  assert.notDeepEqual(await current(), initial);
  await activate("#board-undo"); await saved();
  assert.deepEqual(await current(), initial);
  console.log("Board UI: card reorder gesture and inverse passed.");
  // Cross-column gesture and explicit command must produce the same saved snapshot.
  await page.evaluate(() => {
    document.querySelector(".board").scrollIntoView({ block: "center" });
    document.querySelector(".board").scrollLeft = 0;
    document.querySelectorAll(".card-list").forEach(list => { list.scrollTop = 0; });
  });
  await gesture(await point('[data-card="1"] .card-handle'), await point('[data-card="12"] .card-handle'));
  await saved();
  const dragged = await current();
  assert.equal(dragged.cards.find(card => card.id === 1).column, 1);
  const position = dragged.cards.filter(card => card.column === 1).findIndex(card => card.id === 1);
  await activate("#board-undo"); await saved();
  const explicit = (await call({ action: "move_card", expected: initial, card: 1, column: 1, position })).snapshot;
  assert.deepEqual(dragged, explicit);
  console.log("Board UI: cross-column gesture equals the explicit API command.");
  await call({ action: "restore", snapshot: initial }); await read();

  await page.evaluate(() => {
    document.querySelector(".board").scrollIntoView({ block: "center" });
    document.querySelector(".board").scrollLeft = 0;
    document.querySelectorAll(".card-list").forEach(list => { list.scrollTop = 0; });
  });
  await gesture(await point('[data-column="0"] .column-handle'), await point('[data-column="1"] .column-handle'));
  await saved();
  assert.notDeepEqual((await current()).columns, initial.columns);
  console.log("Board UI: column reorder gesture passed.");
  await activate("#board-undo"); await saved();
  assert.deepEqual(await current(), initial);
  await page.evaluate(() => {
    document.querySelector(".board").scrollIntoView({ block: "center" });
    document.querySelector(".board").scrollLeft = 0;
  });
  // Scrolling away from a handle never sends a move.
  const beforeScroll = await current();
  const start = await point('[data-card="2"] p');
  if (touch) await gesture(start, { x: start.x, y: start.y - 100 }, 0);
  else { await page.mouse.move(start.x, start.y); await page.mouse.wheel(0, 350); }
  await page.waitForFunction(() => document.querySelector(".card-list").scrollTop > 0);
  assert.deepEqual(await current(), beforeScroll);
  assert.equal(await page.evaluate(() => document.querySelector(".card-list").scrollTop > 0), true);
  // Let the browser finish the wheel's animated scroll before positioning a new drag.
  if (!touch) await page.waitForTimeout(250);
  await page.evaluate(() => { document.querySelectorAll(".card-list").forEach(list => { list.scrollTop = 0; }); });
  const edge = await page.evaluate(() => {
    const box = document.querySelector(".card-list").getBoundingClientRect();
    return { x: box.x + box.width / 2, y: Math.min(box.bottom - 18, innerHeight - 25) };
  });
  const vertical = await gesture(await point('[data-card="1"] .card-handle'), edge, 350, 1100);
  assert.ok(vertical.vertical > 0, "Dragging near the lower edge must auto-scroll before drop");
  await idle();
  assert.equal(new Set((await current()).cards.map(card => card.id)).size, 30);
  await call({ action: "restore", snapshot: initial }); await read();
  await page.evaluate(() => {
    document.querySelector(".board").scrollIntoView({ block: "center" });
    document.querySelector(".board").scrollLeft = 0;
    document.querySelectorAll(".card-list").forEach(list => { list.scrollTop = 0; });
  });
  const rightEdge = await page.evaluate(() => {
    const box = document.querySelector(".board").getBoundingClientRect();
    const list = document.querySelector(".card-list").getBoundingClientRect();
    return { x: Math.min(box.right - 12, innerWidth - 12), y: list.y + 75 };
  });
  const horizontal = await gesture(await point('[data-card="1"] .card-handle'), rightEdge, 350, 1100);
  assert.ok(horizontal.horizontal > 0, "Dragging near the right edge must auto-scroll before drop");
  await idle();
  assert.equal(new Set((await current()).cards.map(card => card.id)).size, 30);
  await call({ action: "restore", snapshot: initial }); await read();
  console.log(`Board UI: Reka focus/Tab/Escape, explicit moves/inverses, ${touch ? "touch" : "mouse"} card/column gestures, scroll, vertical/horizontal auto-scroll and API equality passed.`);
  if (process.env.PROTOTYPE_SCREENSHOT_DIRECTORY) {
    await page.evaluate(() => document.querySelector(".board").scrollIntoView({ block: "center" }));
    await page.screenshot({ path: `${process.env.PROTOTYPE_SCREENSHOT_DIRECTORY}/board-${touch ? "android" : "desktop"}.png` });
  }
  return { initial, reorder };
}

export async function checkBoardRejection(page, call, activate = selector => page.locator(selector).click()) {
  const initial = (await call({ action: "snapshot", space: "A" })).snapshot;
  await activate("#board-read");
  await page.waitForFunction(() => !document.querySelector("#board-busy"));
  const changed = (await call({ action: "move_card", expected: initial, card: 2, column: 1, position: 0 })).snapshot;
  await attemptMove(page, activate);
  await reconciled(page);
  assert.deepEqual((await call({ action: "snapshot", space: "A" })).snapshot, changed);
  assert.equal(await page.evaluate(() => document.querySelector('#board-undo').disabled), true);
  assert.equal(await page.evaluate(() => document.querySelector('[data-card="2"]').closest(".card-list").dataset.column), "1");
  await call({ action: "restore", snapshot: initial });
  await activate("#board-read");
  await page.waitForFunction(() => !document.querySelector("#board-busy"));
  console.log("Board UI: stale command rejected, authoritative state reloaded, no duplicate or false success.");
}

export async function attemptMove(page, activate = selector => page.locator(selector).click()) {
  await activate("#card-move-1");
  await page.evaluate(() => {
    const select = document.querySelector("#move-column");
    select.value = "1";
    select.dispatchEvent(new Event("change", { bubbles: true }));
  });
  await activate("#move-save");
}

export async function reconciled(page) {
  await page.waitForFunction(() => !document.querySelector("#board-busy") && document.querySelector("#board-message")?.textContent.includes("Показаны данные из хранилища"));
}
