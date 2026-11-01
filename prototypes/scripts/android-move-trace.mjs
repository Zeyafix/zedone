// Separate diagnostic run: Perfetto overhead must never reclassify old samples.
import assert from "node:assert/strict";
import { resolve } from "node:path";
import { createHash } from "node:crypto";

export async function startPixelTrace({ adb, serial, report, reportDirectory, profile = "moves" }) {
  assert.ok(["moves", "static", "noop"].includes(profile));
  const stem = `zedone-${profile}-${Date.now()}`;
  const remote = `/data/misc/perfetto-traces/${stem}.perfetto-trace`;
  const local = resolve(reportDirectory, `${stem}.perfetto-trace`);
  const config = `buffers { size_kb: 65536 fill_policy: RING_BUFFER }
duration_ms: 180000
write_into_file: true
file_write_period_ms: 500
flush_period_ms: 1000
max_file_size_bytes: 268435456
data_sources { config { name: "linux.ftrace" ftrace_config {
  ftrace_events: "sched/sched_switch"
  ftrace_events: "sched/sched_waking"
  atrace_categories: "gfx"
  atrace_categories: "view"
  atrace_categories: "webview"
  atrace_categories: "wm"
  atrace_categories: "binder_driver"
  atrace_apps: "app.zedone.prototype"
} } }
data_sources { config { name: "linux.process_stats" process_stats_config { scan_all_processes_on_start: true } } }
data_sources { config { name: "android.surfaceflinger.frametimeline" } }
`;
  const evidence = report.system_trace = { status: "starting", file: local, remote_file: remote, config_file: resolve(reportDirectory, `${stem}.pbtxt`),
    scope: `${profile === "noop" ? "no-op evaluation only, no application command or capture" : profile === "static" ? "static captures only, no application command" : "move/inverse only"}, with Perfetto and test-APK sections; separate diagnostic profile, no physical-panel or budget acceptance` };
  await Bun.write(evidence.config_file, config);
  // -D acknowledges data-source startup, so no fixed sleep substitutes for it.
  const child = Bun.spawn([adb, "-s", serial, "shell", "perfetto", "--background-wait", "--txt", "-c", "-", "-o", remote],
    { stdin: "pipe", stdout: "pipe", stderr: "pipe", timeout: 40000 });
  child.stdin.write(config); child.stdin.end();
  const [stdout, stderr, code] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]);
  evidence.start = { stdout, stderr, code };
  assert.equal(code, 0, `Perfetto start failed: ${stderr}`);
  const pid = stdout.trim();
  assert.match(pid, /^\d+$/, "Perfetto must return its own background PID");
  evidence.pid = pid;
  evidence.status = "recording";
  let stopped = false;
  // Cancellation must not prevent stopping/flushing this owned session.
  function cleanupDevice(...args) {
    const result = Bun.spawnSync([adb, "-s", serial, ...args], { timeout: args[0] === "pull" ? 120000 : 15000 });
    assert.equal(result.exitCode, 0, `Perfetto cleanup ${args[0]}: ${result.stderr}`);
    return result.stdout.toString().trim();
  }
  return async () => {
    if (stopped) return;
    stopped = true;
    // Stop only this session, never global atrace/traced or someone else's trace.
    // If duration/file limits already stopped Perfetto, retain its trace too.
    const identity = () => cleanupDevice("shell", `if [ -r /proc/${pid}/cmdline ]; then cat /proc/${pid}/cmdline; fi`);
    // A duration-limited session can already have exited and its PID been reused.
    // The unique output filename identifies our process before signalling it.
    if (identity().includes(remote)) cleanupDevice("shell", "kill", "-TERM", pid);
    const deadline = performance.now() + 15000;
    while (true) {
      // The shell may leave a reparented zombie after the trace has been closed.
      const state = cleanupDevice("shell", `if [ -r /proc/${pid}/stat ]; then cat /proc/${pid}/stat; fi`);
      if (!state || /^\d+ \(perfetto\) Z /.test(state)) break;
      if (!identity().includes(remote)) break;
      assert.ok(performance.now() < deadline, "Perfetto did not finish flushing; preserve remote trace");
      await new Promise(resolve => setTimeout(resolve, 100));
    }
    cleanupDevice("pull", remote, local);
    const bytes = Buffer.from(await Bun.file(local).arrayBuffer());
    assert.ok(bytes.length > 0, "Perfetto trace is empty");
    evidence.bytes = bytes.length;
    evidence.sha256 = createHash("sha256").update(bytes).digest("hex");
    cleanupDevice("shell", "rm", remote);
    evidence.status = "captured";
  };
}
