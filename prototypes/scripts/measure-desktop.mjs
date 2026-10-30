import assert from "node:assert/strict";
import { resolve } from "node:path";
import { mkdtemp, rm, mkdir } from "node:fs/promises";
import { tmpdir, platform, release, cpus, totalmem } from "node:os";
import { createHash } from "node:crypto";
import { chromium } from "playwright";
import { preview } from "vite";
import { fixtureInfo, measurementSources, measureInteractions, workingScreen, summarize } from "./measure-scenarios.mjs";

const root = resolve(import.meta.dir, "..");
const versions = await Bun.file(resolve(root, "../scripts/environment-versions.json")).json();
assert.equal(Bun.version, versions.bun);
const label = Bun.argv[2];
assert.ok(Bun.argv.length === 3 && /^[a-z0-9-]+$/.test(label || ""), "Expected a report label, e.g. chrome");
const reportDirectory = resolve(root, "../docs/reports/p01-11");
await mkdir(reportDirectory, { recursive: true });
const reportPath = resolve(reportDirectory, `${label}-${Date.now()}.json`);
const executable = resolve(root, `target/release/serve-echo${process.platform === "win32" ? ".exe" : ""}`);
const report = { status: "running", label, revision: Bun.spawnSync(["git", "rev-parse", "HEAD"], { cwd: root }).stdout.toString().trim(),
  executable_sha256: createHash("sha256").update(Buffer.from(await Bun.file(executable).arrayBuffer())).digest("hex"),
  sources: await measurementSources(root),
  environment: { os: `${platform()} ${release()}`, cpu: cpus()[0]?.model, total_memory_bytes: totalmem(), bun: Bun.version,
    build: "Rust release, Vite production; headless browser with fresh temporary profile" },
  startup: { browser: [], stopped_processes: [], running_processes: [] } };
const directory = await mkdtemp(resolve(tmpdir(), "zedone-p01-11-"));
let server, web, browser, page, baseUrl;
async function start() {
  server = Bun.spawn([executable, "0", directory], { cwd: root, stdout: "pipe", stderr: "inherit" });
  const reader = server.stdout.getReader();
  let address = "";
  try {
    while (!address.includes("\n")) {
      const { value, done } = await reader.read();
      assert.ok(!done, "Server exited before its address");
      address += new TextDecoder().decode(value);
    }
  } finally { reader.releaseLock(); }
  baseUrl = address.trim();
  assert.match(baseUrl, /^http:\/\/127\.0\.0\.1:\d+$/);
  web = await preview({ root, preview: { port: 0, proxy: { "/prototype": baseUrl } } });
}
async function stop() {
  await web?.close(); web = undefined;
  server?.kill(); await server?.exited; server = undefined;
}
async function call(request) {
  const response = await fetch(`${baseUrl}/prototype/storage`, { method: "POST", headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request), signal: AbortSignal.timeout(10000) });
  assert.ok(response.ok, `Storage HTTP ${response.status}`);
  return response.json();
}
let watchdog;
let interrupt;
const deadline = new Promise((_, reject) => {
  watchdog = setTimeout(() => reject(new Error("Measurement deadline exceeded")), 300000);
  interrupt = () => reject(new Error("Measurement interrupted"));
});
process.once("SIGINT", interrupt);
async function measure() {
  await start();
  const fixture = (await call({ action: "seed", space: "A" })).snapshot;
  report.fixture = fixtureInfo(fixture);
  await stop();
  for (let i = 0; i < 10; i++) {
    report.active_attempt = { series: "stopped_processes", repeat: i };
    const browserStart = performance.now();
    browser = await chromium.launch({ executablePath: process.env.PROTOTYPE_BROWSER_PATH || undefined, headless: true, timeout: 30000 });
    report.startup.browser.push({ ok: true, ms: performance.now() - browserStart });
    report.environment.browser = browser.version();
    const startTime = performance.now();
    await start();
    page = await browser.newPage();
    page.setDefaultTimeout(10000);
    await page.goto(web.resolvedUrls.local[0]);
    await workingScreen(page);
    report.startup.stopped_processes.push({ ok: true, ms: performance.now() - startTime });
    console.log(`Stopped-process startup ${i + 1}/10 recorded.`);
    if (i === 9) break;
    await browser.close(); browser = undefined;
    await stop();
  }
  for (let i = 0; i < 10; i++) {
    report.active_attempt = { series: "running_processes", repeat: i };
    const startTime = performance.now();
    await page.reload(); await workingScreen(page);
    report.startup.running_processes.push({ ok: true, ms: performance.now() - startTime });
    console.log(`Running-process startup ${i + 1}/10 recorded.`);
  }
  report.startup.summaries = Object.fromEntries(Object.entries(report.startup).map(([name, values]) => [name, summarize(values, name === "browser" ? null : 3000)]));
  await measureInteractions(page, call, report);
  report.status = "passed_available_slice";
}
try {
  await Promise.race([measure(), deadline]);
} catch (error) {
  report.status = "failed"; report.error = String(error.stack || error); process.exitCode = 1;
} finally {
  try { await browser?.close(); await stop(); await rm(directory, { recursive: true, force: true }); report.cleanup = true; }
  catch (error) { report.cleanup = false; report.cleanup_error = String(error); process.exitCode = 1; }
  clearTimeout(watchdog);
  process.removeListener("SIGINT", interrupt);
  await Bun.write(reportPath, JSON.stringify(report, null, 2) + "\n");
  console.log(`Measurement ${report.status}; local report: ${reportPath}`);
}
