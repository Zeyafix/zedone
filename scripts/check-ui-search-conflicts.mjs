import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

export async function checkSearchConflicts(browser, executable) {
  const id = n => n.toString(16).padStart(32, "0");
  for (const touch of [false, true]) for (const mode of ["field", "position", "same-position"]) {
    const directory = await mkdtemp(resolve(tmpdir(), "zedone-search-conflicts-"));
    const fixture = Bun.spawn([executable, `--serve-${mode}-conflicts`, directory], { stdout: "pipe", stderr: "inherit" });
    let context;
    try {
      const reader = fixture.stdout.getReader(); let address = "";
      while (!address.includes("\n")) { const { value, done } = await reader.read(); assert.ok(!done); address += new TextDecoder().decode(value); }
      reader.releaseLock();
      const session = JSON.parse(await readFile(resolve(directory, "local-session.json"), "utf8"));
      context = await browser.newContext({ viewport: touch ? { width: 360, height: 760 } : { width: 1280, height: 900 }, hasTouch: touch, isMobile: touch });
      const page = await context.newPage(), errors = [];
      page.on("pageerror", error => errors.push(error.message));
      const press = locator => touch ? locator.tap() : locator.click();
      const space = id(201), column = id(202), card = id(mode === "field" ? 203 : 207);
      const command = async (operation, parameters) => {
        const response = await page.evaluate(async ({ operation, parameters, key }) => (await fetch(`/v1/commands/${operation}`, {
          method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${key}` },
          body: JSON.stringify({ id: Array.from(crypto.getRandomValues(new Uint8Array(16))), operation, parameters }),
        })).json(), { operation, parameters, key: session.key });
        assert.equal(response.status, "ok", operation);
        return response.result;
      };
      await page.goto(`${address.trim()}/#/spaces/${space}`);
      await page.locator(`[data-card="${card}"]`).waitFor();
      const history = await command("card.history", { space, card, after: null, limit: 100, expected: null });
      const other = id(230), otherColumn = id(231), otherCard = id(232);
      await command("space.create", { space: other, name: "А обычное пространство" });
      await command("column.create", { space: other, column: otherColumn, name: "Обычная" });
      await command("card.create", { space: other, column: otherColumn, card: otherCard, title: "Сохранённое содержимое", description: "" });
      await press(page.locator(".search-toggle"));
      const ready = () => page.waitForFunction(() => document.querySelector(".search-results")?.getAttribute("aria-busy") === "false" && /^Найдено/.test(document.querySelector(".search-results [role=status]")?.textContent));
      await ready();
      const verify = async (query, all = true) => {
        await press(page.locator(`input[name="search-scope"][value="${all ? "all" : "space"}"]`));
        await page.locator('[name="query"]').fill(query);
        await press(page.locator(".search-submit")); await ready();
        const expected = await command("card.search", { query, scope: all ? { kind: "all" } : { kind: "space", space } });
        const actual = await page.locator(".search-space").evaluateAll(sections => sections.map(section => ({
          space: section.dataset.space,
          nameConflict: section.parentElement.getAttribute("aria-label") === "Конфликты названий",
          hits: [...section.querySelectorAll(".search-hits > li")].map(row => ({ card: row.dataset.card, placement: row.closest("section[aria-label]").getAttribute("aria-label") === "Конфликты" })),
        })));
        assert.deepEqual(actual, expected.spaces.map(item => ({ space: item.space.space, nameConflict: item.space.names.length !== 1, hits: item.hits.map(hit => ({ card: hit.card.card, placement: hit.placement_conflict })) })));
        const ids = actual.flatMap(item => item.hits.map(hit => hit.card));
        assert.equal(new Set(ids).size, ids.length);
        return expected;
      };
      const initial = await verify("");
      assert.equal(initial.spaces[0].space.space, mode === "field" ? space : other);
      if (mode === "field") {
        for (const query of ["компьютера", "телефона", "сырой текст"]) {
          const result = await verify(query, false);
          assert.deepEqual(result.spaces[0].hits.map(hit => hit.card.card), [card]);
          assert.equal(result.spaces[0].hits[0].placement_conflict, false);
        }
      } else {
        assert.ok(initial.spaces.find(item => item.space.space === space).hits.every(hit => hit.placement_conflict));
      }
      assert.deepEqual((await verify("Нет совпадений")).spaces, []);
      await verify(" \u2003\t");
      await press(page.locator(".search-clear")); await ready();
      assert.equal(await page.locator('input[value="all"]').isChecked(), true);
      const row = page.locator(`.search-hits li[data-card="${card}"]`);
      await press(row.locator(".card-title"));
      const dialog = page.getByRole("dialog");
      await dialog.waitFor();
      await press(dialog.locator("[data-workspace-dialog-close]"));
      assert.deepEqual(await command("card.history", { space, card, after: null, limit: 100, expected: null }), history);
      const refresh = async () => { await press(page.getByRole("button", { name: "Обновить список", exact: true })); await ready(); };
      if (mode === "field") {
        const current = await command("space.read", { space });
        await command("space.resolve_name", { space, name: "Я после решения", expected: current.version });
        await refresh();
        const result = await verify("");
        assert.deepEqual(result.spaces.map(item => item.space.space), [other, space]);
        assert.equal(result.spaces[1].hits[0].card.titles.length, 2);
        const text = await command("card.read", { space, card });
        await command("card.resolve_text", { space, card, title: "Принятое решение", description: "Итог", expected: text.version });
        await refresh();
        assert.deepEqual((await verify("телефона")).spaces, []);
        assert.deepEqual((await verify("Принятое решение", false)).spaces[0].hits.map(hit => hit.card.card), [card]);
      } else {
        const current = await command("card.inspect_placements", { space, card });
        const choice = current.choices.find(item => item.placement.column === column);
        await command("card.resolve_placement", { space, card, placement: choice.placement, expected: current.card.version });
        await refresh();
        assert.ok((await verify("", false)).spaces[0].hits.every(hit => hit.placement_conflict));
        const parent = await command("column.inspect_positions", { space, column });
        await command("column.resolve_position", { space, column, position: parent.choices[0].position, expected: parent.column.version });
        await refresh();
        assert.ok((await verify("", false)).spaces[0].hits.every(hit => !hit.placement_conflict));
        assert.deepEqual(await command("card.history", { space, card, after: null, limit: 100, expected: null }), history);
      }
      await page.reload();
      await page.locator(".search-toggle").waitFor(); await press(page.locator(".search-toggle")); await ready();
      await verify("", false);
      assert.deepEqual(errors, []);
      console.log(`Search conflicts ${touch ? "touch" : "desktop"}/${mode}: variants, scopes, unique groups, partial resolution and reload passed.`);
    } finally {
      await context?.close(); fixture.kill(); await fixture.exited;
      assert.equal(resolve(directory, ".."), resolve(tmpdir()));
      assert.ok(directory.startsWith(resolve(tmpdir(), "zedone-search-conflicts-")));
      await rm(directory, { recursive: true, force: true });
    }
  }
}
