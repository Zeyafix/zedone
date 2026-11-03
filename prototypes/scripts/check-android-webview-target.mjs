import assert from "node:assert/strict";
import { waitForWebViewTarget } from "./android-webview-target.mjs";

const endpoint = "http://127.0.0.1:12345";
const ready = { type: "page", id: "new-webview", webSocketDebuggerUrl: "ws://127.0.0.1:12345/devtools/page/new-webview" };
const replies = [
  new Error("socket not ready"),
  { ok: false },
  { ok: true, json: async () => { throw new SyntaxError("incomplete JSON"); } },
  { ok: true, json: async () => ({ error: "not a target list" }) },
  { ok: true, json: async () => [] },
  { ok: true, json: async () => [null, { type: "service_worker", webSocketDebuggerUrl: "ws://worker" }, { type: "page" }] },
  { ok: true, json: async () => [ready] },
];
let requests = 0;
const result = await waitForWebViewTarget(endpoint, async (poll, description) => {
  assert.equal(description, "APK WebView discoverable page target");
  for (let n = 0; n < replies.length; n++) {
    const target = await poll();
    if (n < replies.length - 1) assert.ok(!target, "No attachment before the page can be discovered");
    else return target;
  }
}, async (url, options) => {
  assert.equal(url, `${endpoint}/json/list`);
  assert.ok(options.signal instanceof AbortSignal);
  const reply = replies[requests++];
  if (reply instanceof Error) throw reply;
  return reply;
});
assert.equal(result, ready);
assert.equal(requests, replies.length);

// The caller's deadline still fails the run: no retry of launch or assertions.
const timeout = new Error("target deadline expired");
await assert.rejects(waitForWebViewTarget(endpoint, async poll => {
  assert.equal(await poll(), undefined);
  throw timeout;
}, async () => ({ ok: true, json: async () => [{ type: "page", webSocketDebuggerUrl: "" }] })),
error => error === timeout);
console.log("Android WebView target: wait through socket-only/empty/invalid discovery; require an attachable page; preserve timeout passed.");
