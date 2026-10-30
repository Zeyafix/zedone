import assert from "node:assert/strict";
import { resolve } from "node:path";

// Uses real HTTP; only local session control differs between desktop stdin and APK IPC.
export async function checkHttpSession(control, storage, connect = async status => ({ endpoint: status.endpoint, close() {} })) {
  await control({ action: "stop" });
  assert.equal((await control({ action: "status" })).endpoint, null);
  await assert.rejects(control({ action: "start", address: "0.0.0.0", port: 0 }));
  const a = (await storage({ action: "seed", space: "A" })).snapshot;
  const b = (await storage({ action: "seed", space: "B" })).snapshot;
  let connection;
  let status;
  const post = (body, token = status.token, path = "/prototype/storage", headers = {}) => fetch(`${connection.endpoint}${path}`, {
    method: "POST", headers: { "Content-Type": "application/json", ...(token ? { Authorization: `Bearer ${token}` } : {}), ...headers },
    body: JSON.stringify(body), signal: AbortSignal.timeout(5000),
  });
  try {
    status = await control({ action: "start", address: "127.0.0.1", port: 0 });
    assert.equal(status.access, "read", "non-UI callers also default to read");
    assert.ok(/^[a-f0-9]{64}$/.test(status.token), "random session key");
    connection = await connect(status);
    await assert.rejects(control({ action: "start", address: "127.0.0.1", port: 0, access: "write" }));
    assert.ok((await control({ action: "status" })).token === status.token, "failed restart preserves the current session");
    for (const key of [null, "wrong"]) assert.equal((await post({ action: "snapshot", space: "A" }, key)).status, 401);
    const text = "Ёж, café, 😀\n\"строка\"";
    assert.deepEqual(await (await post({ text }, status.token, "/prototype/echo")).json(), { text });
    assert.equal((await post({ text: 42 }, status.token, "/prototype/echo")).status, 422);
    assert.equal((await post({ text }, status.token, "/prototype/echo", { Origin: "http://untrusted.invalid" })).status, 403);
    for (const action of ["snapshot", "read"]) {
      const request = { action, space: "A", ...(action === "read" ? { query: "ЁЖ" } : {}) };
      assert.deepEqual(await (await post(request)).json(), await storage(request));
      assert.equal((await post({ ...request, space: "B" })).status, 403);
    }
    for (const snapshot of [a, b]) {
      for (const request of [
        { action: "seed", space: snapshot.space }, { action: "delete", space: snapshot.space },
        { action: "restore", snapshot },
        { action: "move_card", expected: snapshot, card: 1, column: 1, position: 0 },
        { action: "move_column", expected: snapshot, column: 0, position: 1 },
      ]) assert.equal((await post(request)).status, 403);
    }
    assert.deepEqual((await storage({ action: "snapshot", space: "A" })).snapshot, a);
    assert.equal((await post({ action: "stop" }, status.token, "/prototype/http-session")).status, 404);
    const old = status;
    await control({ action: "stop" });
    // Existing HTTP pools may observe either a closed connection or a revoked grant.
    try { assert.equal((await post({ text }, old.token, "/prototype/echo")).status, 401); }
    catch (error) { if (error instanceof assert.AssertionError) throw error; }
    await connection.close();
    status = await control({ action: "start", address: "127.0.0.1", port: 0, access: "write" });
    connection = await connect(status);
    assert.ok(status.token !== old.token, "new session rotates the key");
    assert.equal((await post({ text }, old.token, "/prototype/echo")).status, 401);
    for (const request of [
      { action: "snapshot", space: "B" }, { action: "delete", space: "B" },
      { action: "restore", snapshot: b }, { action: "move_card", expected: b, card: 1, column: 1, position: 0 },
    ]) assert.equal((await post(request)).status, 403);
    const movedResponse = await post({ action: "move_card", expected: a, card: 1, column: 1, position: 0 });
    assert.equal(movedResponse.status, 200);
    const moved = await movedResponse.json();
    assert.deepEqual(await storage({ action: "snapshot", space: "A" }), moved);
    assert.equal((await post({ action: "move_card", expected: a, card: 1, column: 1, position: 0 })).status, 503, "stale moves still reach core validation");
    const deleted = await post({ action: "delete", space: "A" });
    assert.equal(deleted.status, 200);
    assert.equal((await storage({ action: "snapshot", space: "A" })).snapshot, null);
    const restored = await post({ action: "restore", snapshot: a });
    assert.equal(restored.status, 200);
    assert.deepEqual(await restored.json(), { snapshot: a });
    assert.deepEqual((await storage({ action: "snapshot", space: "B" })).snapshot, b);
    console.log("Limited HTTP: default read, auth, every mutation denied, A/B isolation, shared data, stale rejection, delete/restore, stop/key rotation passed.");
  } finally {
    await control({ action: "stop" });
    await connection?.close();
  }
}

export function desktopHttpControl(root, directory) {
  const child = Bun.spawn([resolve(root, `target/release/serve-echo${process.platform === "win32" ? ".exe" : ""}`), "--limited", directory], {
    cwd: root, stdin: "pipe", stdout: "pipe", stderr: "inherit",
  });
  const lines = child.stdout.pipeThrough(new TextDecoderStream()).getReader();
  let buffer = "";
  return {
    async control(request) {
      child.stdin.write(JSON.stringify(request) + "\n");
      await child.stdin.flush();
      while (!buffer.includes("\n")) {
        const { value, done } = await lines.read();
        assert.ok(!done, "local session controller exited");
        buffer += value;
      }
      const index = buffer.indexOf("\n");
      const result = JSON.parse(buffer.slice(0, index));
      buffer = buffer.slice(index + 1);
      if ("Err" in result) throw new Error(result.Err);
      return result.Ok;
    },
    async close() { child.stdin.end(); child.kill(); await child.exited; lines.releaseLock(); },
  };
}
