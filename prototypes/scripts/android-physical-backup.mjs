import assert from "node:assert/strict";
import { resolve } from "node:path";
import { createHash } from "node:crypto";
import { chromium } from "playwright";

const app = "app.zedone.prototype";
const hashFile = async path => createHash("sha256").update(Buffer.from(await Bun.file(path).arrayBuffer())).digest("hex");

// Existing debug prototype only: back up through its public IPC before replacing
// the APK. Never uninstall an incompatible application or read/write its SQLite.
export async function backupPhysical({ root, apk, device, reportDirectory, report }) {
  const metadata = device("shell", "dumpsys", "package", app);
  assert.match(metadata, /pkgFlags=\[[^\]]*DEBUGGABLE/, "Physical measurement requires the existing debug prototype for reversible IPC backup");
  const stamp = Date.now();
  const backup = { serial: process.env.ANDROID_SERIAL, metadata, transport: "CDP to original debug APK only; no CDP in measured release", snapshots: {} };
  backup.apk = resolve(reportDirectory, `physical-original-${stamp}.apk`);
  device("pull", device("shell", "pm", "path", app).split("\n")[0].replace(/^package:/, ""), backup.apk);
  backup.apk_sha256 = await hashFile(backup.apk);
  const versions = await Bun.file(resolve(root, "../scripts/environment-versions.json")).json();
  const buildTools = versions.android.packages.filter(item => item.startsWith("build-tools/")).at(-1).split("@")[0];
  const signer = resolve(process.env.ANDROID_HOME, buildTools, process.platform === "win32" ? "apksigner.bat" : "apksigner");
  async function certificate(path, label) {
    const result = Bun.spawnSync([signer, "verify", "--print-certs", path], { timeout: 30000 });
    await Bun.write(resolve(reportDirectory, `physical-certificate-${label}-${stamp}.txt`), result.stdout.toString() + result.stderr.toString());
    assert.equal(result.exitCode, 0, "APK signature verification failed");
    const digests = [...result.stdout.toString().matchAll(/certificate SHA-256 digest: ([a-fA-F0-9]{64})/g)].map(match => match[1].toLowerCase());
    assert.ok(digests.length, "Signing certificate fingerprints unavailable");
    return [...new Set(digests)].sort();
  }
  backup.certificate_sha256 = await certificate(backup.apk, "original");
  assert.deepEqual(await certificate(apk, "release"), backup.certificate_sha256, "APK signer mismatch: preserve installed application and data");
  const hasTest = device("shell", "pm", "list", "packages", `${app}.test`).split("\n").includes(`package:${app}.test`);
  if (hasTest) {
    backup.test_apk = resolve(reportDirectory, `physical-original-test-${stamp}.apk`);
    device("pull", device("shell", "pm", "path", `${app}.test`).split("\n")[0].replace(/^package:/, ""), backup.test_apk);
  }
  backup.stay_on = device("shell", "settings", "get", "global", "stay_on_while_plugged_in");
  backup.snapshots = await originalPhysicalSnapshots({ root, device });
  backup.file = resolve(reportDirectory, `physical-backup-${stamp}.json`);
  await Bun.write(backup.file, JSON.stringify(backup, null, 2) + "\n");
  report.original_backup = backup;
  return backup;
}

export async function originalPhysicalSnapshots({ root, device }) {
  let browser, port;
  try {
    device("shell", "am", "start", "-W", "-n", `${app}/.MainActivity`);
    const pid = device("shell", "pidof", app);
    port = device("forward", "tcp:0", `localabstract:webview_devtools_remote_${pid}`);
    const deadline = performance.now() + 20000;
    let previousTarget;
    do {
      try {
        const response = await fetch(`http://127.0.0.1:${port}/json/list`, { signal: AbortSignal.timeout(1000) });
        if (response.ok) {
          const target = (await response.json()).find(target => target.type === "page" && /^https?:\/\/tauri\.localhost\//.test(target.url));
          // Wait for the packaged document, not an initial empty page target.
          // Two stable discoveries avoid attaching while WebView replaces it.
          if (target && target.id === previousTarget) break;
          previousTarget = target?.id;
        }
      } catch { /* Android may still be starting WebView. */ }
      assert.ok(performance.now() < deadline, "Original debug WebView not available; no APK replacement allowed");
      await new Promise(resolve => setTimeout(resolve, 100));
    } while (true);
    browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, { noDefaults: true, timeout: 10000 });
    // CDP target discovery can precede Playwright's page event, especially just
    // after reinstall/restart. A ready /json/list alone does not supply a Page.
    let page;
    const pageDeadline = performance.now() + 10000;
    while (!(page = browser.contexts().flatMap(context => context.pages())[0])) {
      assert.ok(performance.now() < pageDeadline, "Original debug page target did not attach");
      await new Promise(resolve => setTimeout(resolve, 100));
    }
    page.setDefaultTimeout(10000);
    await page.waitForFunction(() => window.__TAURI_INTERNALS__);
    const schema = await Bun.file(resolve(root, "openapi.json")).json();
    const command = schema.paths["/prototype/storage"].post.operationId;
    const snapshots = {};
    for (const space of ["A", "B"]) snapshots[space] = await page.evaluate(async ({ command, space }) =>
      (await window.__TAURI_INTERNALS__.invoke(command, { request: { action: "snapshot", space } })).snapshot, { command, space });
    return snapshots;
  } finally {
    await browser?.close();
    if (port) device("forward", "--remove", `tcp:${port}`);
  }
}
