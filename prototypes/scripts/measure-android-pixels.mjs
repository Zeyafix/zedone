// ADB screencap observes Android's composed image, independently of WebView/CDP.
// Driver-clock upper bounds include ADB, PNG decoding, commands and alignment.
import assert from "node:assert/strict";
import { resolve } from "node:path";
import { createHash } from "node:crypto";
import { PNG } from "pngjs";
import { summarize } from "./measure-scenarios.mjs";

const hash = bytes => createHash("sha256").update(bytes).digest("hex");
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));

export async function measureAndroidPixels({ getPage, call, device, adb, serial, report, reportDirectory, disconnect, launch, connect, ensureActive }) {
  const stamp = Date.now();
  const release = report.environment.command_driver === "test-APK WebView.evaluateJavascript";
  const physical = report.environment.device_kind === "physical";
  const label = physical ? "android-physical-release-pixels" : release ? "android-release-pixels" : "android-pixels";
  const initial = (await call({ action: "snapshot", space: "A" })).snapshot;
  const page = () => getPage();
  const click = selector => page().evaluate(selector => {
    const button = document.querySelector(selector);
    if (!button || button.disabled) throw new Error(`Unavailable command: ${selector}`);
    button.click();
  }, selector);
  const waitBoard = () => page().waitForFunction(() => !document.querySelector("#board-busy") && document.querySelectorAll(".board-card").length === 30, null, { polling: 10 });
  const firstCards = () => page().evaluate(() => [...document.querySelectorAll('.board-column[data-column="0"] .board-card')].slice(0, 2).map(card => Number(card.dataset.card)));
  const waitOrder = order => page().waitForFunction(order => {
    const cards = [...document.querySelectorAll('.board-column[data-column="0"] .board-card')];
    return !document.querySelector("#board-busy") && order.every((id, i) => Number(cards[i]?.dataset.card) === id);
  }, order, { polling: 10 });
  async function openMove() {
    await click("#card-move-1");
    await page().waitForFunction(() => document.querySelector("#move-position"), null, { polling: 10 });
    await page().evaluate(() => {
      const select = document.querySelector("#move-position");
      select.value = "1";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
  }
  async function search(query, count) {
    await page().evaluate(query => {
      const input = document.querySelector("#storage-query");
      input.value = query;
      input.dispatchEvent(new Event("input", { bubbles: true }));
    }, query);
    await click("#storage-read");
    await page().waitForFunction(count => !document.querySelector("#storage-busy") &&
      JSON.parse(document.querySelector("#storage-result")?.textContent || "null")?.snapshot.cards.length === count, count, { polling: 10 });
  }
  async function screenshot() {
    ensureActive();
    const process = Bun.spawn([adb, "-s", serial, "exec-out", "screencap", "-p"], { stdout: "pipe", stderr: "pipe", timeout: 10000 });
    const [bytes, stderr, code] = await Promise.all([new Response(process.stdout).arrayBuffer(), new Response(process.stderr).text(), process.exited]);
    assert.equal(code, 0, `ADB screencap failed: ${stderr}`);
    return PNG.sync.read(Buffer.from(bytes), { checkCRC: true });
  }
  let geometry;
  async function capture(region) {
    const screen = await screenshot();
    assert.deepEqual({ width: screen.width, height: screen.height }, geometry.screen, "Display geometry changed during observation");
    const { x, y, width, height } = region;
    assert.ok(x >= 0 && y >= 0 && width > 0 && height > 0 && x + width <= screen.width && y + height <= screen.height, "Capture region outside Android image");
    const cropped = new PNG({ width, height });
    PNG.bitblt(screen, cropped, x, y, width, height, 0, 0);
    return cropped;
  }
  async function saveImage(name, image) {
    const file = `${label}-${stamp}-${name}.png`;
    await Bun.write(resolve(reportDirectory, file), PNG.sync.write(image));
    return file;
  }
  async function calibrate() {
    const css = await page().evaluate(() => {
      const marker = document.createElement("div");
      marker.id = "pixel-calibration-marker";
      // Edge-to-edge WebView can extend behind Android's status bar.
      marker.style = "position:fixed;left:24px;top:120px;width:80px;height:24px;background:rgb(17,91,203);z-index:2147483647";
      document.body.append(marker);
      return { scale: devicePixelRatio, width: innerWidth, height: innerHeight };
    });
    const attempts = [];
    const deadline = performance.now() + 10000;
    let screen, left, top, right, bottom, count;
    do {
      screen = await screenshot();
      left = screen.width; top = screen.height; right = -1; bottom = -1; count = 0;
      for (let y = 0; y < screen.height; y++) for (let x = 0; x < screen.width; x++) {
        const i = (y * screen.width + x) * 4;
        if (screen.data[i] === 17 && screen.data[i + 1] === 91 && screen.data[i + 2] === 203) {
          left = Math.min(left, x); top = Math.min(top, y); right = Math.max(right, x); bottom = Math.max(bottom, y); count++;
        }
      }
      attempts.push({ sha256: hash(screen.data), count, left, top, right, bottom });
      if (count === Math.round(80 * css.scale) * Math.round(24 * css.scale)) break;
      await pause(100);
    } while (performance.now() < deadline);
    report.calibration = { attempts, file: await saveImage("calibration", screen), sha256: hash(screen.data), status: "not_validated" };
    const width = right - left + 1, height = bottom - top + 1;
    assert.equal(width, Math.round(80 * css.scale), "Complete calibration marker width required");
    assert.equal(height, Math.round(24 * css.scale), "Complete calibration marker height required");
    assert.equal(count, width * height, "Calibration marker must be a solid rectangle");
    geometry = { screen: { width: screen.width, height: screen.height }, css, x: left - Math.round(24 * css.scale), y: top - Math.round(120 * css.scale) };
    assert.ok(geometry.x >= 0 && geometry.y >= 0);
    report.calibration = { ...report.calibration, ...geometry, status: "validated" };
    await page().evaluate(() => document.querySelector("#pixel-calibration-marker").remove());
  }
  async function align(kind) {
    const rect = await page().evaluate(kind => {
      let ranges;
      if (kind === "board") {
        document.querySelector(".board").scrollLeft = 0;
        document.querySelector('.board-column[data-column="0"] .card-list').scrollTop = 0;
        ranges = [...document.querySelectorAll('.board-column[data-column="0"] .board-card h4')].slice(0, 2);
      } else {
        const pre = document.querySelector("#storage-result"), text = pre.firstChild;
        pre.scrollTop = 0; pre.scrollLeft = 0;
        const start = text.textContent.indexOf("Карточка ");
        if (start < 0) throw new Error("Missing search result title");
        const range = document.createRange();
        range.setStart(text, start); range.setEnd(text, start + 13);
        const r = range.getBoundingClientRect(), clip = pre.getBoundingClientRect();
        pre.scrollTop += Math.max(0, r.top - clip.top - 24);
        ranges = [range];
      }
      const before = ranges[0].getBoundingClientRect();
      scrollBy(0, before.top - 100);
      const boxes = ranges.map(range => range.getBoundingClientRect());
      const left = Math.min(...boxes.map(r => r.left)), top = Math.min(...boxes.map(r => r.top));
      const right = Math.max(...boxes.map(r => r.right)), bottom = Math.max(...boxes.map(r => r.bottom));
      const clip = document.querySelector(kind === "board" ? '.board-column[data-column="0"] .card-list' : "#storage-result").getBoundingClientRect();
      if (left < Math.max(0, clip.left) || top < Math.max(0, clip.top) || right > Math.min(innerWidth, clip.right) || bottom > Math.min(innerHeight, clip.bottom))
        throw new Error("Full result titles are clipped; unsupported viewport geometry");
      return { left, top, right, bottom };
    }, kind);
    const { scale } = geometry.css;
    const x = geometry.x + Math.floor(rect.left * scale), y = geometry.y + Math.floor(rect.top * scale);
    return { x, y, width: geometry.x + Math.ceil(rect.right * scale) - x, height: geometry.y + Math.ceil(rect.bottom * scale) - y };
  }
  report.observations = [];
  report.templates = [];
  report.observer_sources = { "measure-android-pixels.mjs": hash(Buffer.from(await Bun.file(import.meta.filename).arrayBuffer())) };
  if (release) {
    for (const path of ["scripts/android-release-driver.mjs", "src-tauri/gen/android/app/src/androidTest/java/app/zedone/prototype/ReleaseMeasurementInstrumentation.java", "src-tauri/gen/android/app/build.gradle.kts", "scripts/build-android.mjs"])
      report.observer_sources[path] = hash(Buffer.from(await Bun.file(resolve(import.meta.dir, "..", path)).arrayBuffer()));
  }
  if (physical) report.observer_sources["scripts/android-physical-backup.mjs"] = hash(Buffer.from(await Bun.file(resolve(import.meta.dir, "android-physical-backup.mjs")).arrayBuffer()));
  report.clock = "driver performance.now (monotonic)";
  report.boundary = `Clock armed before DOM command -> DOM preparation -> matching Android screencap; includes ADB/PNG/${release ? "instrumentation" : "CDP"}/alignment overhead`;
  report.physical_panel_validation = false;
  report.ui_measurement_evidence = { level: "OS_image_upper_bound", endpoint: `ADB screencap PNG; ${physical ? "physical device" : "emulator"} ${release ? "release" : "debug"} APK`, physical_panel_validation: false, ui_acceptance_eligible: false, limitation: `No ${physical ? "physical-panel, " : "physical-device, "}${release ? "distribution-signing, " : "release, "}input-device or complete A62 acceptance` };
  report.command = "DOM button: card 1 reordered to position 1 within column 0; both changed titles visible; not the Windows cross-column profile";
  async function template(name, region) {
    await pause(150);
    const image = await capture(region);
    report.templates.push({ name, region, sha256: hash(image.data), file: await saveImage(name, image) });
    return image.data;
  }
  async function observe(name, region, expected, command, timeout = 5000, allowFailure = false) {
    const baseline = await capture(region);
    if (baseline.data.equals(expected)) {
      const result = { ok: false, error: "Expected result already visible before command" };
      report.observations.push({ name, ...result });
      if (!allowFailure) throw new Error(result.error);
      return result;
    }
    const start = performance.now();
    let commandError, captures = 0, matched, result;
    // Start the clock before dispatch, then capture after DOM preparation. Avoid
    // spending a full PNG transfer on the known old frame. DOM readiness cannot
    // finish the interval: independently captured OS pixels must still match.
    const execution = Promise.resolve().then(command).catch(error => { commandError = String(error.stack || error); });
    await execution;
    try {
      while (!commandError && performance.now() - start < timeout) {
        const image = await capture(region);
        captures++;
        const elapsed = performance.now() - start;
        if (image.data.equals(expected)) {
          matched = image;
          result = elapsed <= timeout ? { ok: true, ms: elapsed, upper_bound_ms: elapsed } : { ok: false, error: "Android capture completed after timeout", elapsed_ms: elapsed };
          break;
        }
      }
      result ||= { ok: false, error: "Expected Android pixels not observed before timeout", elapsed_ms: performance.now() - start };
    } catch (error) { result = { ok: false, error: String(error.stack || error) }; }
    await execution;
    if (commandError) result = { ...result, ok: false, command_error: commandError };
    const observation = { name, attempt: report.active_attempt, region, expected_sha256: hash(expected), captures, ...result };
    report.observations.push(observation);
    if (matched && result.ok && !(report.match_images ||= {})[name]) report.match_images[name] = await saveImage(`${name}-matched`, matched);
    if (!result.ok && !allowFailure) throw new Error(JSON.stringify(observation));
    return result;
  }
  await calibrate();
  await click("#board-read"); await waitBoard();
  const normalRegion = await align("board"), normal = await template("board-normal", normalRegion);
  assert.deepEqual(await firstCards(), [1, 2]);
  await openMove(); await click("#move-save"); await waitOrder([2, 1]);
  const movedRegion = await align("board"), moved = await template("board-moved", movedRegion);
  assert.notDeepEqual(normal, moved);
  await click("#board-undo"); await waitOrder([1, 2]); await align("board");
  await pause(150);
  report.controls = {};
  let staleCommandRan = false;
  report.controls.already_visible = await observe("control-already-visible", normalRegion, normal, () => { staleCommandRan = true; }, 500, true);
  assert.match(report.controls.already_visible.error, /already visible/);
  assert.equal(staleCommandRan, false);
  await page().evaluate(() => {
    // invoke is immutable. Delay only the callback carrying the saved move,
    // and verify the hook ran; silent assignment failures must never pass a control.
    const callbacks = window.__TAURI_INTERNALS__.callbacks;
    if (Object.prototype.hasOwnProperty.call(callbacks, "set")) throw new Error("Callback map already instrumented");
    window.__pixelDelayedResponses = [];
    let delayed = false;
    callbacks.set = function(id, callback) {
      return Map.prototype.set.call(this, id, response => {
        if (!delayed && response?.snapshot?.cards?.[0]?.id === 2 && response.snapshot.cards[1]?.id === 1) {
          delayed = true;
          const start = performance.now();
          setTimeout(() => { window.__pixelDelayedResponses.push(performance.now() - start); callback(response); }, 450);
        } else callback(response);
      });
    };
  });
  try {
    await openMove();
    report.controls.delayed_completion = await observe("control-delayed-completion", movedRegion, moved, async () => { await click("#move-save"); await waitOrder([2, 1]); await align("board"); });
    assert.ok(report.controls.delayed_completion.upper_bound_ms >= 450);
    report.controls.delayed_completion.callback_delays_ms = await page().evaluate(() => window.__pixelDelayedResponses);
    assert.equal(report.controls.delayed_completion.callback_delays_ms.length, 1);
    assert.ok(report.controls.delayed_completion.callback_delays_ms[0] >= 430);
  } finally {
    await page().evaluate(() => { delete window.__TAURI_INTERNALS__.callbacks.set; delete window.__pixelDelayedResponses; });
  }
  await click("#board-undo"); await waitOrder([1, 2]); await align("board");
  await page().evaluate(() => document.querySelector(".board").style.visibility = "hidden");
  await openMove();
  report.controls.hidden_result = await observe("control-hidden-result", movedRegion, moved, () => click("#move-save"), 700, true);
  assert.equal(report.controls.hidden_result.ok, false);
  assert.match(report.controls.hidden_result.error, /timeout/);
  assert.equal((await call({ action: "snapshot", space: "A" })).snapshot.cards.filter(c => c.column === 0)[1].id, 1);
  report.controls.hidden_result.saved_in_core = true;
  await page().evaluate(() => document.querySelector(".board").style.visibility = "visible");
  await click("#board-undo"); await waitOrder([1, 2]); await align("board");
  assert.deepEqual((await call({ action: "snapshot", space: "A" })).snapshot, initial);
  report.controls.snapshot_restored = true;
  report.series = {};
  async function series(name, reverseName, forward, reverse, verify) {
    const a = { warmups: [], samples: [] }, b = { warmups: [], samples: [] };
    report.series[name] = a; report.series[reverseName] = b;
    for (let i = -5; i < 30; i++) {
      report.active_attempt = { series: name, repeat: i };
      const f = await forward();
      await verify(false);
      (i < 0 ? a.warmups : a.samples).push({ ...f, state_verified: true });
      const r = await reverse();
      await verify(true);
      (i < 0 ? b.warmups : b.samples).push({ ...r, state_verified: true });
    }
    console.log(`Android pixels: ${name}/${reverseName} full series recorded.`);
  }
  await series("move", "inverse_move", async () => {
    await openMove();
    return observe("move", movedRegion, moved, async () => { await click("#move-save"); await waitOrder([2, 1]); assert.deepEqual(await align("board"), movedRegion); });
  }, () => observe("inverse", normalRegion, normal, async () => { await click("#board-undo"); await waitOrder([1, 2]); assert.deepEqual(await align("board"), normalRegion); }), async reverse => {
    const snapshot = (await call({ action: "snapshot", space: "A" })).snapshot;
    if (reverse) assert.deepEqual(snapshot, initial);
    else assert.deepEqual(snapshot.cards.filter(c => c.column === 0).slice(0, 2).map(c => c.id), [2, 1]);
  });
  await search("", 30);
  const allRegion = await align("search"), all = await template("search-all", allRegion);
  await search("Карточка 0030", 1);
  const oneRegion = await align("search"), one = await template("search-one", oneRegion);
  await search("", 30); await align("search");
  await series("search", "clear_search", () => observe("search", oneRegion, one, async () => { await search("Карточка 0030", 1); assert.deepEqual(await align("search"), oneRegion); }),
    () => observe("clear-search", allRegion, all, async () => { await search("", 30); assert.deepEqual(await align("search"), allRegion); }), async () => {
      assert.deepEqual((await call({ action: "snapshot", space: "A" })).snapshot, initial);
    });
  report.startup = { stopped_processes: [], running_processes: [] };
  report.startup_method = release ? "Force-stop: am start plus new instrumentation process/Activity and driver connection, then persisted board; Home: am start using the existing instrumentation. All driver overhead included; no cold-cache or uninstrumented-startup claim." : "Force-stop: am start plus CDP reconnection, then persisted board; Home: am start using existing CDP. No cold-cache claim.";
  for (const kind of Object.keys(report.startup)) for (let i = 0; i < 10; i++) {
    report.active_attempt = { series: kind, repeat: i };
    if (kind === "stopped_processes") { await disconnect(); device("shell", "am", "force-stop", "app.zedone.prototype"); }
    else device("shell", "input", "keyevent", "KEYCODE_HOME");
    // Home/force-stop returns before the compositor finishes its transition.
    // Confirm the old board disappeared before arming, outside the startup interval.
    const hiddenDeadline = performance.now() + 5000;
    let absent = 0;
    while (performance.now() < hiddenDeadline && absent < 2) {
      absent = (await capture(normalRegion)).data.equals(normal) ? 0 : absent + 1;
      await pause(100);
    }
    assert.equal(absent, 2, "Old working screen did not disappear before startup");
    report.startup[kind].push(await observe(`startup-${kind}`, normalRegion, normal, async () => {
      launch();
      if (kind === "stopped_processes") await connect();
      await click("#board-read"); await waitBoard();
      assert.deepEqual(await align("board"), normalRegion);
    }));
  }
  function bounds(samples, budget) {
    const { over_budget, ...statistics } = summarize(samples, budget);
    return { ...statistics, metric: "conservative_upper_bound_ms", upper_bounds_above_budget: over_budget,
      budget_verdict: over_budget ? "inconclusive_upper_bound_exceeds_budget" : "supported_at_Android_image_boundary" };
  }
  for (const [name, s] of Object.entries(report.series)) s.summary = bounds(s.samples, name.includes("search") ? 1000 : 300);
  report.startup.summaries = Object.fromEntries(Object.entries(report.startup).map(([name, s]) => [name, bounds(s, 3000)]));
  assert.deepEqual((await call({ action: "snapshot", space: "A" })).snapshot, initial);
  report.snapshot_restored = true;
  delete report.active_attempt;
  report.status = physical ? "completed_android_release_physical_device_pixel_upper_bounds" : release ? "completed_android_release_emulator_pixel_upper_bounds" : "completed_android_emulator_pixel_upper_bounds";
}
