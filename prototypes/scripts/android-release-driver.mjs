import assert from "node:assert/strict";
import { connect as openSocket } from "node:net";
import { randomUUID } from "node:crypto";

// Minimal evaluate/wait adapter for the same pixel scenario used with debug CDP.
// Only the separately installed instrumentation APK exposes this local transport.
export function releaseDriver({ adb, serial, device, ensureActive, report, reportDirectory }) {
  let socket, port, instrumentation, pending, buffered = "";
  let connected = false, currentLog;
  let failed;
  const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
  const app = "app.zedone.prototype";
  const page = {
    isClosed: () => !socket || socket.destroyed,
    async evaluate(fn, arg) {
      ensureActive();
      assert.ok(socket && !socket.destroyed && !pending, "Release driver unavailable or concurrent evaluation");
      if (failed) throw failed;
      const script = `(${fn.toString()})(${JSON.stringify(arg) ?? "undefined"})`;
      return new Promise((resolve, reject) => {
        const timeout = setTimeout(() => { pending = undefined; socket.destroy(); reject(new Error("Release evaluation timeout")); }, 15000);
        pending = { resolve: value => { clearTimeout(timeout); resolve(value); }, reject: error => { clearTimeout(timeout); reject(error); } };
        socket.write(JSON.stringify({ script }) + "\n");
      });
    },
    async waitForFunction(fn, arg) {
      const deadline = performance.now() + 10000;
      do {
        if (await page.evaluate(fn, arg)) return;
        await pause(10);
      } while (performance.now() < deadline);
      throw new Error(`Release DOM predicate timed out: ${fn}`);
    },
  };
  async function disconnect() {
    if (socket) {
      const current = socket;
      await new Promise(resolve => {
        const timer = setTimeout(() => { current.destroy(); resolve(); }, 1000);
        current.once("close", () => { clearTimeout(timer); resolve(); });
        current.end(JSON.stringify({ close: true }) + "\n");
      });
      socket = undefined;
    }
    if (port) device("forward", "--remove", `tcp:${port}`);
    port = undefined;
    if (instrumentation) {
      // Runner teardown can terminate Tauri. Drain it before the next launch.
      await Promise.race([instrumentation.exited, pause(3000)]);
      if (instrumentation.exitCode === null) { instrumentation.kill(); await instrumentation.exited; }
      instrumentation = undefined;
      if (connected) assert.match(await Bun.file(currentLog).text(), /Release measurement driver completed/, "Release driver did not complete cleanly");
    }
    connected = false;
  }
  async function connect() {
    failed = undefined; buffered = "";
    const name = `zedone-measure-${randomUUID().replaceAll("-", "")}`;
    const log = `${reportDirectory}/release-instrumentation-${Date.now()}.log`;
    currentLog = log;
    (report.instrumentation_logs ||= []).push(log);
    instrumentation = Bun.spawn([adb, "-s", serial, "shell", "am", "instrument", "-w", "-r",
      "-e", "measurementSocket", name, `${app}.test/${app}.ReleaseMeasurementInstrumentation`],
      { stdout: Bun.file(log), stderr: Bun.file(`${log}.stderr`) });
    port = device("forward", "tcp:0", `localabstract:${name}`);
    const deadline = performance.now() + 30000;
    do {
      ensureActive();
      socket = await new Promise(resolve => {
        const connection = openSocket({ host: "127.0.0.1", port: Number(port) });
        let timer;
        const finish = value => { clearTimeout(timer); connection.removeAllListeners(); resolve(value); };
        connection.once("error", () => { connection.destroy(); finish(undefined); });
        // adbd accepts TCP before its localabstract target exists. The first
        // command below is the readiness handshake; EOF must be retryable.
        connection.once("connect", () => finish(connection));
        timer = setTimeout(() => { connection.destroy(); finish(undefined); }, 1000);
      });
      if (socket) {
        socket.setEncoding("utf8");
        const current = socket;
        const lost = error => { if (current !== socket) return; failed = error; pending?.reject(error); pending = undefined; };
        socket.on("data", chunk => {
          try {
            buffered += chunk;
            const end = buffered.indexOf("\n");
            if (end < 0) return;
            const response = JSON.parse(buffered.slice(0, end)); buffered = buffered.slice(end + 1);
            const active = pending; pending = undefined;
            if (!active) return;
            if (response.ok) active.resolve(response.value); else active.reject(new Error(response.error));
          } catch (error) { lost(error); current.destroy(); }
        });
        socket.on("error", lost);
        socket.on("close", () => lost(new Error("Release instrumentation socket closed")));
        try { assert.equal(await page.evaluate(() => !!document.querySelector("#board-read")), true); connected = true; return; }
        catch { socket.destroy(); socket = undefined; failed = undefined; }
      }
      assert.ok(instrumentation.exitCode === null, `Release instrumentation exited; see ${log}`);
      await pause(100);
    } while (performance.now() < deadline);
    throw new Error(`Release instrumentation connection timed out; see ${log}`);
  }
  return { page, connect, disconnect };
}
