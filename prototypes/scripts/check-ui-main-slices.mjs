import assert from "node:assert/strict";
import { resolve } from "node:path";
import { parse } from "@babel/parser";

const path = resolve(import.meta.dir, "../../scripts/check-ui-browser.mjs");
const source = await Bun.file(path).text();
const ast = parse(source, { sourceType: "module" });
const parallel = ast.program.body.find(node => node.type === "FunctionDeclaration" && node.id.name === "checkMainSlices");
const execute = new Function(`return (${source.slice(parallel.start, parallel.end)});`)();
for (const failed of [undefined, "lifecycle", "board", "touch"]) {
  const started = [], finished = [], controls = new Map();
  let settled = false;
  const error = new Error("slice failure");
  const result = execute(slice => {
    started.push(slice);
    return new Promise((yes, no) => controls.set(slice, () => {
      finished.push(slice);
      if (slice === failed) no(error); else yes();
    }));
  }).then(() => { settled = true; }, reason => { settled = true; return reason; });
  assert.deepEqual(started, ["lifecycle", "board", "touch"]);
  for (const slice of [failed, "board", "lifecycle", "touch"].filter((value, index, values) => value && values.indexOf(value) === index)) {
    controls.get(slice)();
    for (let turn = 0; turn < 5; turn++) await Promise.resolve();
    if (finished.length < 3) assert.equal(settled, false, "Wait for every peer, including its fixture cleanup");
  }
  const reason = await result;
  if (failed) {
    assert.ok(reason instanceof AggregateError);
    assert.deepEqual(reason.errors, [error]);
  } else assert.equal(reason, undefined);
}
const failures = [new Error("first"), new Error("second"), new Error("third")];
let index = 0;
await assert.rejects(execute(() => { throw failures[index++]; }), error =>
  error instanceof AggregateError && assert.deepEqual(error.errors, failures) === undefined);
assert.equal(index, 3, "A synchronous failure must not omit another slice");

const lifecycle = ["checkNativeControls", "checkShellFailureDiagnostics", "checkWorkspace", "checkCards", "checkEditor"];
const board = ["checkBoardMoves", "checkTags", "checkSearch", "checkTrash", "checkHistory", "checkHistoryCleanup", "checkAccessibility"];
const touch = [...board, "checkCardMarkdown"];
const scenarios = new Set([...lifecycle, ...board, ...touch]);
const slice = ast.program.body.find(node => node.type === "FunctionDeclaration" && node.id.name === "checkBrowserSlice");
function selected(node, flags, calls = []) {
  if (!node || typeof node !== "object") return calls;
  if (node.type === "FunctionDeclaration") return calls;
  if (node.type === "IfStatement" && Object.hasOwn(flags, node.test.name)) return selected(flags[node.test.name] ? node.consequent : node.alternate, flags, calls);
  if (node.type === "Identifier" && scenarios.has(node.name)) calls.push(node.name);
  for (const value of Object.values(node)) {
    if (Array.isArray(value)) for (const child of value) selected(child, flags, calls);
    else if (value && typeof value === "object") selected(value, flags, calls);
  }
  return calls;
}
const sorted = values => [...values].sort();
const actual = ["lifecycle", "board", "touch"].flatMap(name => {
  const calls = selected(slice.body, Object.fromEntries(["lifecycle", "board", "touch"].map(flag => [flag, flag === name])));
  assert.deepEqual(sorted(calls), sorted({ lifecycle, board, touch }[name]), name);
  return calls;
});
assert.deepEqual(sorted(actual), sorted(selected(slice.body, { lifecycle: true, board: true, touch: true })), "Parallel slices preserve every full-mode scenario invocation exactly once");
assert.equal(actual.length, 20);
const boardBlocks = slice.body.body.find(node => node.type === "TryStatement").block.body
  .find(node => node.type === "ForOfStatement").body.body
  .filter(node => node.type === "IfStatement" && node.test.name === "board");
assert.equal(boardBlocks.length, 2);
assert.ok(source.slice(boardBlocks[0].start, boardBlocks[0].end).includes("await checkAccessibility(page);"));
assert.ok(source.slice(boardBlocks[1].start, boardBlocks[1].end).includes('await page.keyboard.press("Tab");'));
assert.ok(source.includes('if (mode === "--main") await checkMainSlices(checkBrowserSlice);'));
assert.ok(source.includes('else await checkBrowserSlice();'));
console.log("Main UI slices: 20 scenario invocations preserved; independent fixtures, all started, all outcomes drained, sync/async failures retained.");
