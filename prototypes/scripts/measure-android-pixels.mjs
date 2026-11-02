// OS screenshots observe Android's composed image, independently of WebView/CDP.
// Driver-clock upper bounds include USB, PNG decoding, commands and alignment.
import assert from "node:assert/strict";
import { resolve } from "node:path";
import { createHash } from "node:crypto";
import { PNG } from "pngjs";
import { summarize } from "./measure-scenarios.mjs";
import { evaluatePixelAlignment } from "./android-webview-phases.mjs";

const hash = bytes => createHash("sha256").update(bytes).digest("hex");
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));

// Preparation only: stable stale pixels must never become a new baseline.
export async function waitForAndroidPixelBaseline({ capture, expected, evidence, name = "search-all", timeout = 10000, now = () => performance.now(), wait = pause }) {
  const start = now();
  evidence.status = "waiting";
  evidence.expected_sha256 = hash(expected);
  evidence.timeout_ms = timeout;
  evidence.attempts = [];
  try {
    while (now() - start < timeout) {
      const attempt = { captures: [] };
      evidence.attempts.push(attempt);
      // Same independent sources as template preparation, but every capture
      // must match the existing search-all template, not merely each other.
      for (const viaAdb of [true, true, false]) {
        const image = await capture(viaAdb);
        attempt.captures.push({ source: viaAdb ? "ADB" : "observer", sha256: hash(image.data), matches: image.data.equals(expected), elapsed_ms: now() - start });
      }
      if (now() - start >= timeout) break;
      if (attempt.captures.every(capture => capture.matches)) {
        evidence.status = "ready";
        return;
      }
      await wait(50);
    }
    throw new Error(`OS ${name} baseline did not settle before arming`);
  } catch (error) {
    evidence.status = "failed";
    evidence.error = String(error);
    throw error;
  } finally { evidence.elapsed_ms = now() - start; }
}

export async function measureAndroidPixels({ getPage, call, device, adb, serial, report, reportDirectory, disconnect, launch, connect, ensureActive, traceMoves = false, webviewPhases = false, ipcPhases = false, staticPixels = false, noopEvaluation = false, rawPixels = false, deflatePixels = false }) {
  const stamp = Date.now();
  const release = report.environment.command_driver === "test-APK WebView.evaluateJavascript";
  const physical = report.environment.device_kind === "physical";
  const format = deflatePixels ? "rgba8-zlib" : rawPixels ? "rgba8" : "png";
  const label = (physical ? "android-physical-release-pixels" : release ? "android-release-pixels" : "android-pixels") + (deflatePixels ? "-rgba-zlib" : rawPixels ? "-rgba" : "");
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
  async function screenshot(viaAdb = false) {
    ensureActive();
    if (!viaAdb && release && page() && !page().isClosed()) return (await nativeCapture()).image;
    const process = Bun.spawn([adb, "-s", serial, "exec-out", "screencap", "-p"], { stdout: "pipe", stderr: "pipe", timeout: 10000 });
    const [bytes, stderr, code] = await Promise.all([new Response(process.stdout).arrayBuffer(), new Response(process.stderr).text(), process.exited]);
    assert.equal(code, 0, `ADB screencap failed: ${stderr}`);
    return PNG.sync.read(Buffer.from(bytes), { checkCRC: true });
  }
  let geometry;
  async function nativeCapture(region, verifyPng = false) {
    const start = performance.now();
    const result = await page().captureScreen(region, format, verifyPng);
    const image = rawPixels ? { width: result.width, height: result.height, data: result.rgba } : PNG.sync.read(Buffer.from(result.png, "base64"), { checkCRC: true });
    if (rawPixels) {
      assert.equal(result.format, format);
      assert.equal(image.data.length, image.width * image.height * 4);
      if (verifyPng) {
        const png = PNG.sync.read(Buffer.from(result.png, "base64"), { checkCRC: true });
        assert.equal(png.width, image.width); assert.equal(png.height, image.height);
        assert.ok(png.data.equals(image.data), "RGBA differs from PNG of the same OS bitmap");
        const checks = report.same_bitmap_checks ||= [];
        const prefix = `${label}-${stamp}-same-bitmap-${checks.length}`;
        await Bun.write(resolve(reportDirectory, `${prefix}.rgba`), image.data);
        await Bun.write(resolve(reportDirectory, `${prefix}.png`), Buffer.from(result.png, "base64"));
        if (deflatePixels) await Bun.write(resolve(reportDirectory, `${prefix}.zlib`), result.compressed);
        checks.push({ region, sha256: hash(image.data), png_sha256: hash(png.data), byte_length: image.data.length, rgba_file: `${prefix}.rgba`, png_file: `${prefix}.png`, ...(deflatePixels ? { compressed_file: `${prefix}.zlib`, compressed_sha256: hash(result.compressed), wire_byte_length: result.wire_byte_length } : {}) });
      } else assert.equal(result.png, undefined, "Timed RGBA captures must not encode PNG");
    }
    assert.deepEqual({ width: image.width, height: image.height }, region ? { width: region.width, height: region.height } : result.screen, "Unexpected OS screenshot dimensions");
    assert.ok(Number.isFinite(result.capture_and_encode_ms) && result.capture_and_encode_ms >= 0);
    if (deflatePixels) {
      assert.equal(result.compressed.length, result.wire_byte_length);
      assert.ok(Number.isFinite(result.compression_ms) && result.compression_ms >= 0);
    }
    (report.capture_diagnostics ||= []).push({ method: "UiAutomation.takeScreenshot", format, verify_png: verifyPng, byte_length: rawPixels ? image.data.length : Buffer.from(result.png, "base64").length, ...(deflatePixels ? { wire_byte_length: result.wire_byte_length, compression_ms: result.compression_ms } : {}), region, ms: performance.now() - start, capture_and_encode_ms: result.capture_and_encode_ms });
    return { image, screen: result.screen };
  }
  async function capture(region, viaAdb = false) {
    if (!viaAdb && release && page() && !page().isClosed()) {
      const result = await nativeCapture(region);
      assert.deepEqual(result.screen, geometry.screen, "Display geometry changed during observation");
      return result.image;
    }
    const screen = await screenshot(true);
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
  async function align(kind, command = {}) {
    const evidence = webviewPhases && command.order && report.active_attempt ? { attempt: { ...report.active_attempt }, selector: command.selector, order: command.order } : undefined;
    if (evidence) report.webview_phases.entries.push(evidence);
    const rect = await evaluatePixelAlignment((fn, args) => page().evaluate(fn, args), { kind, ...command, ...(ipcPhases && evidence ? { ipcDiagnostic: true } : {}) }, evidence);
    const { scale } = geometry.css;
    const x = geometry.x + Math.floor(rect.left * scale), y = geometry.y + Math.floor(rect.top * scale);
    return { x, y, width: geometry.x + Math.ceil(rect.right * scale) - x, height: geometry.y + Math.ceil(rect.bottom * scale) - y };
  }
  report.observations = [];
  report.templates = [];
  report.observer_sources = { "measure-android-pixels.mjs": hash(Buffer.from(await Bun.file(import.meta.filename).arrayBuffer())) };
  report.observer_sources["scripts/android-webview-phases.mjs"] = hash(Buffer.from(await Bun.file(resolve(import.meta.dir, "android-webview-phases.mjs")).arrayBuffer()));
  if (webviewPhases) report.webview_phases = { clock: "WebView performance.now; one evaluation per command", scope: "click, observed saved DOM (includes 10 ms polling), viewport alignment; not Rust/SQLite latency or OS presentation", ui_acceptance_eligible: false, entries: [] };
  if (ipcPhases) report.webview_phases.ipc_delivery_observed = true;
  if (release) {
    for (const path of ["scripts/android-release-driver.mjs", "scripts/android-release-protocol.mjs", "src-tauri/gen/android/app/src/androidTest/java/app/zedone/prototype/ReleaseMeasurementInstrumentation.java", "src-tauri/gen/android/app/build.gradle.kts", "scripts/build-android.mjs"])
      report.observer_sources[path] = hash(Buffer.from(await Bun.file(resolve(import.meta.dir, "..", path)).arrayBuffer()));
  }
  if (physical) report.observer_sources["scripts/android-physical-backup.mjs"] = hash(Buffer.from(await Bun.file(resolve(import.meta.dir, "android-physical-backup.mjs")).arrayBuffer()));
  if (traceMoves || staticPixels || noopEvaluation) report.observer_sources["scripts/android-move-trace.mjs"] = hash(Buffer.from(await Bun.file(resolve(import.meta.dir, "android-move-trace.mjs")).arrayBuffer()));
  if (staticPixels) report.observer_sources["scripts/android-static-pixels.mjs"] = hash(Buffer.from(await Bun.file(resolve(import.meta.dir, "android-static-pixels.mjs")).arrayBuffer()));
  if (noopEvaluation) report.observer_sources["scripts/android-noop-evaluation.mjs"] = hash(Buffer.from(await Bun.file(resolve(import.meta.dir, "android-noop-evaluation.mjs")).arrayBuffer()));
  report.clock = "driver performance.now (monotonic)";
  const imageSource = release ? `UiAutomation.takeScreenshot OS display ${rawPixels ? `${format} (extract region after capture, binary` : "PNG (crop after capture,"} persistent ADB socket)` : "ADB screencap PNG";
  report.pixel_transport = format;
  report.boundary = `Clock armed before DOM command -> DOM preparation -> matching Android OS image; includes USB/${deflatePixels ? "RGBA extraction/zlib compression/framing/inflation" : rawPixels ? "RGBA extraction/framing" : "PNG"}/${release ? "instrumentation" : "CDP"}/alignment overhead`;
  report.image_source = imageSource;
  report.startup_baseline_source = "ADB screencap when instrumentation is disconnected; outside timed interval";
  report.first_capture_delay_ms = release ? 50 : 0;
  report.physical_panel_validation = false;
  report.ui_measurement_evidence = { level: "OS_image_upper_bound", endpoint: `${imageSource}; ${physical ? "physical device" : "emulator"} ${release ? "release" : "debug"} APK`, physical_panel_validation: false, ui_acceptance_eligible: false, limitation: `No ${physical ? "physical-panel, " : "physical-device, "}${release ? "distribution-signing, " : "release, "}input-device or complete A62 acceptance` };
  report.command = "DOM button: card 1 reordered to position 1 within column 0; both changed titles visible; not the Windows cross-column profile";
  async function template(name, region) {
    await pause(150);
    let image;
    const attempts = [];
    const deadline = performance.now() + 10000;
    do {
      // Templates are outside measured intervals. DOM readiness does not prove
      // that the pending scroll/paint reached the OS. Require two independent
      // ADB captures to agree, then cross-check the faster native observer.
      const first = await capture(region, true);
      image = await capture(region, true);
      const native = release ? await capture(region) : image;
      const sameBitmap = rawPixels ? (await nativeCapture(region, true)).image : native;
      const stable = first.data.equals(image.data) && native.data.equals(image.data) && sameBitmap.data.equals(image.data);
      attempts.push({ first_sha256: hash(first.data), second_sha256: hash(image.data), native_sha256: hash(native.data), ...(rawPixels ? { same_bitmap_sha256: hash(sameBitmap.data) } : {}), stable });
      if (stable) break;
      assert.ok(performance.now() < deadline, `OS template did not settle: ${name}`);
    } while (true);
    report.templates.push({ name, region, preparation: attempts, sha256: hash(image.data), file: await saveImage(name, image) });
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
    const traceLabel = traceMoves && report.active_attempt ? `zedone:${name}-${report.active_attempt.repeat < 0 ? "warmup" + -report.active_attempt.repeat : report.active_attempt.repeat}` : undefined;
    if (traceLabel) page().setTraceLabel(traceLabel);
    const stages = traceLabel || (rawPixels && report.active_attempt) ? { ...(traceLabel ? { trace_label: traceLabel } : {}), captures: [] } : undefined;
    const diagnosticImages = [];
    let commandError, captures = 0, matched, result;
    // Start the clock before dispatch, then capture after DOM preparation. Avoid
    // spending a full PNG transfer on the known old frame. DOM readiness cannot
    // finish the interval: independently captured OS pixels must still match.
    const execution = Promise.resolve().then(command).catch(error => { commandError = String(error.stack || error); });
    await execution;
    if (stages) stages.command_end_ms = performance.now() - start;
    try {
      // A screenshot taken before the pending paint can spend a full readback
      // encoding the known old frame. Pace the first capture, inside the clock;
      // this never forces a renderer frame or substitutes for matching pixels.
      if (report.first_capture_delay_ms) await pause(report.first_capture_delay_ms);
      if (stages) stages.first_capture_start_ms = performance.now() - start;
      while (!commandError && performance.now() - start < timeout) {
        const captureStart = performance.now() - start;
        const image = await capture(region);
        const captureEnd = performance.now() - start;
        const matches = image.data.equals(expected);
        captures++;
        if (stages) {
          stages.captures.push({ start_ms: captureStart, end_ms: captureEnd, matched: matches });
          diagnosticImages.push(image);
        }
        if (matches) {
          const elapsed = performance.now() - start;
          matched = image;
          result = elapsed <= timeout ? { ok: true, ms: elapsed, upper_bound_ms: elapsed } : { ok: false, error: "Android capture completed after timeout", elapsed_ms: elapsed };
          break;
        }
      }
      result ||= { ok: false, error: "Expected Android pixels not observed before timeout", elapsed_ms: performance.now() - start };
    } catch (error) { result = { ok: false, error: String(error.stack || error) }; }
    await execution;
    if (traceLabel) page().setTraceLabel(undefined);
    for (const [i, image] of diagnosticImages.entries()) {
      stages.captures[i].sha256 = hash(image.data);
      stages.captures[i].file = await saveImage(`${(traceLabel || `${name}-${report.observations.length}`).replaceAll(":", "-")}-capture-${i + 1}`, image);
    }
    if (commandError) result = { ...result, ok: false, command_error: commandError };
    const observation = { name, attempt: report.active_attempt, region, expected_sha256: hash(expected), captures, ...(stages ? { diagnostic_stages: stages } : {}), ...result };
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
  if (release) {
    const adbImage = await capture(normalRegion, true);
    const nativeImage = await capture(normalRegion);
    report.controls.os_capture_agreement = { region: normalRegion, adb_sha256: hash(adbImage.data), native_sha256: hash(nativeImage.data),
      adb_file: await saveImage("adb-cross-check", adbImage), native_file: await saveImage("native-cross-check", nativeImage) };
    assert.deepEqual(nativeImage.data, adbImage.data, "UiAutomation crop must match independent ADB OS capture");
    assert.deepEqual(nativeImage.data, normal, "OS cross-check must show the full original titles");
  }
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
    report.controls.delayed_completion = await observe("control-delayed-completion", movedRegion, moved, () => align("board", { selector: "#move-save", order: [2, 1] }));
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
  if (staticPixels || noopEvaluation) {
    const { startPixelTrace } = await import("./android-move-trace.mjs");
    const profile = noopEvaluation ? "noop" : "static";
    report.boundary = noopEvaluation ? "No-op evaluation request -> decoded constant reply; native span includes later WebView cleanup; no application-action or presentation latency" : "Static capture request -> decoded Android OS image; no application command, no action budget or overhead subtraction";
    report.command = `No application command during ${profile} samples; controls and restoration outside the series`;
    report.ui_measurement_evidence.level = noopEvaluation ? "diagnostic_noop_evaluation" : "diagnostic_static_OS_capture";
    report.ui_measurement_evidence.limitation += `; ${profile} observer control with Perfetto, no application-action latency measurement`;
    const evidence = report[noopEvaluation ? "noop_evaluation" : "static_observer"] = { region: normalRegion, expected_sha256: hash(normal), metric: noopEvaluation ? "host evaluation request through decoded constant reply; excludes pacing and native post-reply cleanup" : "host capture request through decoded OS image; excludes pacing and file output", ui_acceptance_eligible: false, preparation: {} };
    assert.deepEqual(await align("board"), normalRegion);
    await waitForAndroidPixelBaseline({ capture: viaAdb => capture(normalRegion, viaAdb), expected: normal, evidence: evidence.preparation, name: "board-normal" });
    const before = {};
    for (const space of ["A", "B"]) before[space] = (await call({ action: "snapshot", space })).snapshot;
    await page().evaluate(() => {
      if (window.__pixelStaticObserver) throw new Error("Static DOM observer already installed");
      const state = { mutations: 0, html: document.documentElement.outerHTML };
      state.observer = new MutationObserver(records => { state.mutations += records.length; });
      state.observer.observe(document.documentElement, { subtree: true, childList: true, attributes: true, characterData: true });
      window.__pixelStaticObserver = state;
    });
    let stop;
    try {
      stop = await startPixelTrace({ adb, serial, report, reportDirectory, profile });
      evidence.capture_diagnostics_start_index = report.capture_diagnostics.length;
      if (noopEvaluation) {
        const { measureNoopEvaluation } = await import("./android-noop-evaluation.mjs");
        await measureNoopEvaluation({ evaluate: fn => page().evaluate(fn), setTraceLabel: label => page().setTraceLabel(label), evidence });
        // Replies precede the driver's WebView cleanup. An unlabelled request
        // drains the last labelled section before stopping/flushing Perfetto.
        assert.equal(await page().evaluate(() => null), null);
        evidence.driver_drained = true;
        evidence.captures_in_series = report.capture_diagnostics.length - evidence.capture_diagnostics_start_index;
        assert.equal(evidence.captures_in_series, 0);
      } else {
        const { measureStaticPixels } = await import("./android-static-pixels.mjs");
        await measureStaticPixels({ capture: () => capture(normalRegion), expected: normal, setTraceLabel: label => page().setTraceLabel(label), saveImage, evidence });
      }
    } finally {
      try { if (stop) await stop(); }
      finally {
        evidence.dom = await page().evaluate(() => {
          const state = window.__pixelStaticObserver;
          state.mutations += state.observer.takeRecords().length;
          state.observer.disconnect();
          const result = { mutations: state.mutations, html_unchanged: state.html === document.documentElement.outerHTML };
          delete window.__pixelStaticObserver;
          return result;
        });
      }
    }
    assert.deepEqual(evidence.dom, { mutations: 0, html_unchanged: true }, "Static DOM changed");
    if (noopEvaluation) {
      const image = await capture(normalRegion);
      evidence.post_image = { sha256: hash(image.data), file: await saveImage("noop-after", image) };
      assert.deepEqual(image.data, normal, "No-op control changed OS pixels");
    }
    for (const space of ["A", "B"]) assert.deepEqual((await call({ action: "snapshot", space })).snapshot, before[space]);
    evidence.snapshots_unchanged = true;
    assert.deepEqual(before.A, initial);
    report.snapshot_restored = true;
    report.status = noopEvaluation ? "completed_diagnostic_noop_evaluation" : "completed_diagnostic_static_capture";
    return;
  }
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
  let stopTrace;
  try {
  if (traceMoves) {
    const { startPixelTrace } = await import("./android-move-trace.mjs");
    stopTrace = await startPixelTrace({ adb, serial, report, reportDirectory, profile: ipcPhases ? "ipc" : webviewPhases ? "webview" : "moves" });
  }
  await series("move", "inverse_move", async () => {
    await openMove();
    return observe("move", movedRegion, moved, async () => { assert.deepEqual(await align("board", { selector: "#move-save", order: [2, 1] }), movedRegion); });
  }, () => observe("inverse", normalRegion, normal, async () => { assert.deepEqual(await align("board", { selector: "#board-undo", order: [1, 2] }), normalRegion); }), async reverse => {
    const snapshot = (await call({ action: "snapshot", space: "A" })).snapshot;
    if (reverse) assert.deepEqual(snapshot, initial);
    else assert.deepEqual(snapshot.cards.filter(c => c.column === 0).slice(0, 2).map(c => c.id), [2, 1]);
  });
  } finally { if (stopTrace) await stopTrace(); }
  if (traceMoves) {
    if (webviewPhases) {
      assert.equal(report.webview_phases.entries.length, 70);
      assert.ok(report.webview_phases.entries.every(entry => entry.status === "completed"));
      for (const series of ["move", "inverse_move"]) {
        const entries = report.webview_phases.entries.filter(entry => entry.attempt.series === "move" && entry.selector === (series === "move" ? "#move-save" : "#board-undo"));
        assert.deepEqual(entries.map(entry => entry.attempt.repeat), Array.from({ length: 35 }, (_, i) => i - 5));
      }
    }
    assert.deepEqual((await call({ action: "snapshot", space: "A" })).snapshot, initial);
    report.snapshot_restored = true;
    report.ui_measurement_evidence.level = "diagnostic_system_trace_with_OS_images";
    report.ui_measurement_evidence.limitation += "; Perfetto and per-capture PNG storage add diagnostic overhead; no search/startup series in this profile";
    delete report.active_attempt;
    report.status = ipcPhases ? "completed_diagnostic_ipc_phases" : webviewPhases ? "completed_diagnostic_webview_phases" : "completed_diagnostic_move_trace";
    return;
  }
  await search("", 30);
  const allRegion = await align("search"), all = await template("search-all", allRegion);
  await search("Карточка 0030", 1);
  const oneRegion = await align("search"), one = await template("search-one", oneRegion);
  await search("", 30); assert.deepEqual(await align("search"), allRegion);
  report.search_preparation = { name: "search-all-after-template-reset", region: allRegion, outside_measured_intervals: true };
  await waitForAndroidPixelBaseline({ capture: viaAdb => capture(allRegion, viaAdb), expected: all, evidence: report.search_preparation });
  await series("search", "clear_search", () => observe("search", oneRegion, one, async () => { assert.deepEqual(await align("search", { query: "Карточка 0030", count: 1 }), oneRegion); }),
    () => observe("clear-search", allRegion, all, async () => { assert.deepEqual(await align("search", { query: "", count: 30 }), allRegion); }), async () => {
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
