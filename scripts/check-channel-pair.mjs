import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

export async function checkChannelPair({ root, env, hostBinary, androidBinary, adb, serial, test = "channel::tests::cross_platform_probe" }) {
  const directory = await mkdtemp(resolve(tmpdir(), "zedone-channel-pair-"));
  const metadataFile = resolve(directory, "public.json");
  const server = Bun.spawn([hostBinary, "--ignored", "--exact", test], {
    cwd: root, env: { ...env, ZEDONE_CHANNEL_PROBE_ROLE: "server", ZEDONE_CHANNEL_PROBE_METADATA: metadataFile },
    stdin: "ignore", stdout: "pipe", stderr: "pipe",
  });
  let port;
  let ownsReverse = false;
  function device(...args) {
    const result = Bun.spawnSync([adb, "-s", serial, ...args], { stdin: "ignore", timeout: 15000 });
    assert.equal(result.exitCode, 0, "Paired channel native command failed");
    return result.stdout.toString();
  }
  const output = new Response(server.stdout).text();
  const errors = new Response(server.stderr).text();
  try {
    const deadline = Date.now() + 15000;
    let metadata;
    while (!metadata) {
      try { metadata = JSON.parse(await readFile(metadataFile, "utf8")); }
      catch (error) {
        if (error.code !== "ENOENT" && !(error instanceof SyntaxError)) throw error;
        assert.ok(Date.now() < deadline && server.exitCode === null, "Host channel server did not become ready");
        await Bun.sleep(50);
      }
    }
    assert.ok(Array.isArray(metadata) && metadata.length === 2 && Number.isInteger(metadata[0]) && metadata[0] > 0 && metadata[0] < 65536);
    assert.ok(Array.isArray(metadata[1]) && metadata[1].length === 32 && metadata[1].every(b => Number.isInteger(b) && b >= 0 && b <= 255));
    port = `tcp:${metadata[0]}`;
    device("reverse", "--no-rebind", port, port);
    ownsReverse = true;
    const result = device("shell", "ZEDONE_CHANNEL_PROBE_ROLE=client", `ZEDONE_CHANNEL_PROBE_METADATA='${JSON.stringify(metadata)}'`, androidBinary, "--ignored", "--exact", test);
    assert.match(result, /test result: ok\. 1 passed; 0 failed/);
    assert.equal(await server.exited, 0, "Host paired channel probe failed");
    assert.match(await output, /test result: ok\. 1 passed; 0 failed/);
    console.log(test.startsWith("pairing::")
      ? "Host ↔ native Android pending request: wrong pin, submission, database restart, saved-session replay, inspection/cancellation and no access passed."
      : "Host ↔ native Android TLS 1.3: wrong pin rejected, persisted host identity and bidirectional private payload passed.");
  } finally {
    if (ownsReverse) device("reverse", "--remove", port);
    await server.exited;
    await Promise.all([output, errors]);
    assert.ok(directory.startsWith(resolve(tmpdir(), "zedone-channel-pair-")));
    await rm(directory, { recursive: true });
  }
}
