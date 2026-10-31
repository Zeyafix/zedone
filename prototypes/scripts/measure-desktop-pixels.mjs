// Independent Windows desktop-pixel upper bounds for the synthetic prototype.
// No physical-panel or input-device latency claim; failed observations are retained.
import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import { mkdtemp, rm, mkdir } from 'node:fs/promises';
import { tmpdir, platform, release } from 'node:os';
import { createHash } from 'node:crypto';
import { chromium } from 'playwright';
import { preview } from 'vite';
import { fixtureInfo, measurementSources, summarize } from './measure-scenarios.mjs';
const root = resolve(import.meta.dir, '..');
assert.equal(platform(), 'win32', 'Desktop pixel observation requires Windows');
const versions = await Bun.file(resolve(root, '../scripts/environment-versions.json')).json();
assert.equal(Bun.version, versions.bun);
const label = Bun.argv[2];
assert.ok(Bun.argv.length === 3 && /^[a-z0-9-]+$/.test(label || ''), 'Expected a report label, e.g. opera');
const browserExecutable = process.env.PROTOTYPE_BROWSER_PATH;
assert.ok(browserExecutable && await Bun.file(browserExecutable).exists(), 'PROTOTYPE_BROWSER_PATH must select an existing browser executable');
const reportDirectory = resolve(root, '../docs/reports/p01-11');
await mkdir(reportDirectory, { recursive: true });
let directory;
const stamp = Date.now();
const report = { status: 'running', started: new Date().toISOString(), label, mode: 'headed; Windows desktop pixel observer', environment: { os: release(), bun: Bun.version, browser_executable: browserExecutable }, revision: Bun.spawnSync(['git', 'rev-parse', 'HEAD'], { cwd: root }).stdout.toString().trim(), clock: 'Windows QPC / Stopwatch',
    boundary: 'Observer armed before DOM command -> first matching desktop pixels; conservative upper bound includes driver/CDP dispatch, capture and explicit viewport alignment',
    physical_panel_validation: false, series: {}, sources: await measurementSources(root), templates: [] };
report.observations = [];
let browser, page, server, web, helper, address, guard;
let closingHelper = false;
const pending = [];
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const rpc = request => new Promise(resolve => { pending.push(resolve); helper.stdin.write(JSON.stringify(request) + '\n'); helper.stdin.flush(); });
const call = async (request) => { const r = await fetch(`${address}/prototype/storage`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(request), signal: AbortSignal.timeout(10000) }); assert.ok(r.ok); return r.json(); };
async function start() {
    server = Bun.spawn([resolve(root, 'target/release/serve-echo.exe'), '0', directory], { cwd: root, stdout: 'pipe', stderr: 'inherit' });
    const reader = server.stdout.getReader();
    let line = '';
    while (!line.includes('\n')) {
        const { value, done } = await reader.read();
        assert.ok(!done);
        line += new TextDecoder().decode(value);
    }
    reader.releaseLock();
    address = line.trim();
    web = await preview({ root, preview: { port: 0, proxy: { '/prototype': address } } });
}
async function stop() { await web?.close(); web = undefined; server?.kill(); await server?.exited; server = undefined; }
const windowTitle = `ZeDone pixels ${label} ${stamp}`;
let window, origin;
async function capture(region) { const result = await rpc({ action: 'capture', hwnd: window.hwnd, ...region }); assert.ok(result.pixels, result.error); return result; }
async function align(kind) {
    return page.evaluate(kind => {
        let range;
        if (kind === 'board')
            range = document.querySelector('.board-column[data-column="0"] .board-card h4');
        else {
            const pre = document.querySelector('#storage-result');
            pre.scrollTop = 0;
            pre.scrollLeft = 0;
            const node = pre.firstChild;
            const text = node.textContent;
            const start = text.indexOf('Карточка ');
            if (start < 0)
                throw new Error('Missing result title');
            range = document.createRange();
            range.setStart(node, start);
            range.setEnd(node, start + 13);
            const r = range.getBoundingClientRect(), clip = pre.getBoundingClientRect();
            pre.scrollTop += Math.max(0, r.top - clip.top - 40);
        }
        const rect = range.getBoundingClientRect();
        scrollBy(0, rect.top - 250);
        const visible = range.getBoundingClientRect();
        if (kind === 'board') {
            const second = document.querySelector('.board-column[data-column="1"] .board-card h4').getBoundingClientRect();
            return { x: visible.x, y: Math.min(visible.y, second.y), width: Math.ceil(second.right - visible.x), height: Math.ceil(Math.max(visible.bottom, second.bottom) - Math.min(visible.y, second.y)) };
        }
        const clip = document.querySelector('#storage-result').getBoundingClientRect();
        if (visible.top < clip.top || visible.bottom > clip.bottom || visible.left < clip.left || visible.right > clip.right)
            throw new Error('First result title is clipped');
        return { x: visible.x, y: visible.y, width: Math.ceil(visible.width), height: Math.ceil(visible.height) };
    }, kind);
}
function nativeRegion(rect) { assert.ok(rect.y >= 0 && rect.y + rect.height <= 700); return { x: origin.x + Math.floor(rect.x), y: origin.y + Math.floor(rect.y), width: rect.width, height: rect.height }; }
async function template(name, region) { await pause(120); const t = await capture(region); const filename = `native-${stamp}-${name}.png`; await Bun.write(resolve(reportDirectory, filename), Buffer.from(t.png, 'base64')); report.templates.push({ name, file: filename, region, sha256: hash(Buffer.from(t.pixels, 'base64')) }); return t.pixels; }
async function observe(name, region, expected, command, timeout_ms = 5000, allowFailure = false) {
    let first, second;
    const armed = new Promise(resolve => { first = resolve; pending.push(resolve); });
    const completed = new Promise(resolve => { second = resolve; pending.push(resolve); });
    helper.stdin.write(JSON.stringify({ action: 'observe', hwnd: window.hwnd, ...region, expected, timeout_ms }) + '\n');
    helper.stdin.flush();
    const state = await armed;
    if (!state.armed) {
        pending.splice(pending.indexOf(second), 1);
        report.observations.push({ name, ...state });
        throw new Error(state.error);
    }
    let commandError;
    const execution = Promise.resolve().then(command).catch(error => { commandError = String(error.stack || error); });
    const result = await completed;
    await execution;
    if (result.png) {
        if (!report.match_images?.[name]) {
            const filename = `native-${stamp}-${name}-matched.png`;
            await Bun.write(resolve(reportDirectory, filename), Buffer.from(result.png, 'base64'));
            (report.match_images ||= {})[name] = filename;
        }
        delete result.png;
    }
    report.observations.push({ name, attempt: report.active_attempt, region, expected_sha256: hash(Buffer.from(expected, 'base64')), ...result, ...(commandError ? { command_error: commandError } : {}) });
    if (commandError)
        throw new Error(commandError);
    if (!result.ok && !allowFailure)
        throw new Error(JSON.stringify(result));
    return { ...result, ms: result.upper_bound_ms };
}
async function openMove() { await page.evaluate(() => document.querySelector('#card-move-1').click()); await page.waitForSelector('#move-column', { state: 'attached' }); await page.evaluate(() => { const select = document.querySelector('#move-column'); select.value = '1'; select.dispatchEvent(new Event('change', { bubbles: true })); }); }
const click = selector => page.evaluate(selector => document.querySelector(selector).click(), selector);
async function waitBoard() { await page.waitForFunction(() => document.querySelectorAll('.board-card').length === 30, null, { polling: 10 }); }
async function waitSearch(count) { await page.waitForFunction(count => { const pre = document.querySelector('#storage-result'); return !document.querySelector('#storage-busy') && pre && JSON.parse(pre.textContent).snapshot.cards.length === count; }, count, { polling: 10 }); }
async function search(query, count) { await page.evaluate(query => { const input = document.querySelector('#storage-query'); input.value = query; input.dispatchEvent(new Event('input', { bubbles: true })); }, query); await click('#storage-read'); await waitSearch(count); }
async function measure() {
    directory = await mkdtemp(resolve(tmpdir(), 'zedone-pixel-observer-'));
    report.observer_sources = {};
    for (const name of ['desktop-pixel-observer.ps1', 'measure-desktop-pixels.mjs'])
        report.observer_sources[name] = hash(Buffer.from(await Bun.file(resolve(import.meta.dir, name)).arrayBuffer()));
    report.artifacts = {};
    for await (const path of new Bun.Glob('dist/**/*').scan({ cwd: root, onlyFiles: true }))
        report.artifacts[path] = hash(Buffer.from(await Bun.file(resolve(root, path)).arrayBuffer()));
    report.artifacts['target/release/serve-echo.exe'] = hash(Buffer.from(await Bun.file(resolve(root, 'target/release/serve-echo.exe')).arrayBuffer()));
    report.viewport_css = { width: 1000, height: 700 };
    report.command = 'Existing DOM button; input-device/drag latency excluded; explicit viewport alignment included when needed';
    let ready;
    const initialized = new Promise(resolve => ready = resolve);
    helper = Bun.spawn(['powershell.exe', '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', resolve(import.meta.dir, 'desktop-pixel-observer.ps1')], { stdin: 'pipe', stdout: 'pipe', stderr: 'inherit' });
    const helperFailed = error => {
        const failure = { ok: false, error: String(error) };
        ready(failure);
        while (pending.length) pending.shift()(failure);
    };
    helper.exited.then(code => { if (!closingHelper) helperFailed(`Pixel observer exited unexpectedly (${code})`); });
    (async () => { let buffer = ''; for await (const part of helper.stdout) {
        buffer += new TextDecoder().decode(part);
        for (;;) {
            const end = buffer.indexOf('\n');
            if (end < 0)
                break;
            const line = buffer.slice(0, end).trim();
            buffer = buffer.slice(end + 1);
            if (!line)
                continue;
            const value = JSON.parse(line);
            if (value.ready)
                ready(value);
            else
                pending.shift()?.(value);
        }
    } })().catch(helperFailed);
    const initialization = await Promise.race([initialized, new Promise((_, reject) => guard = setTimeout(() => reject(new Error('Native helper initialization timeout')), 20000))]);
    assert.equal(initialization.ready, true, initialization.error);
    clearTimeout(guard);
    await start();
    const initial = (await call({ action: 'seed', space: 'A' })).snapshot;
    report.fixture = fixtureInfo(initial);
    browser = await chromium.launch({ executablePath: browserExecutable, headless: false });
    report.browser = browser.version();
    page = await browser.newPage({ viewport: { width: 1000, height: 700 } });
    page.setDefaultTimeout(10000);
    await page.goto(web.resolvedUrls.local[0]);
    await page.bringToFront();
    await page.evaluate(title => { document.title = title; const marker = document.createElement('div'); marker.id = 'native-calibration-marker'; marker.style = 'position:fixed;left:100px;top:100px;width:160px;height:24px;background:rgb(17,91,203);z-index:2147483647'; document.body.append(marker); }, windowTitle);
    await pause(250);
    window = await rpc({ action: 'expose', prefix: windowTitle });
    assert.ok(window.hwnd, window.error);
    report.window = window;
    const nr = { x: window.x + 128, y: window.y + 140, width: 300, height: 160 };
    const calibration = await capture(nr), bytes = Buffer.from(calibration.pixels, 'base64');
    let minX = nr.width, minY = nr.height, count = 0;
    for (let y = 0; y < nr.height; y++)
        for (let x = 0; x < nr.width; x++) {
            const i = (y * nr.width + x) * 4;
            if (bytes[i] === 203 && bytes[i + 1] === 91 && bytes[i + 2] === 17) {
                minX = Math.min(minX, x);
                minY = Math.min(minY, y);
                count++;
            }
        }
    assert.equal(count, 160 * 24, 'Calibration pixel area must exactly match the synthetic marker');
    origin = { x: nr.x + minX - 100, y: nr.y + minY - 100 };
    report.viewport_origin = origin;
    await page.evaluate(() => document.querySelector('#native-calibration-marker').remove());
    await click('#board-read');
    await waitBoard();
    const boardRect = await align('board'), boardRegion = nativeRegion(boardRect);
    const normal = await template('board-card-1', boardRegion);
    await openMove();
    await click('#move-save');
    await page.waitForFunction(() => document.querySelector('.board-column[data-column="0"] .board-card').dataset.card === '2', null, { polling: 10 });
    await align('board');
    const moved = await template('board-card-2', boardRegion);
    assert.notEqual(normal, moved);
    await click('#board-undo');
    await page.waitForFunction(() => document.querySelector('.board-column[data-column="0"] .board-card').dataset.card === '1', null, { polling: 10 });
    await align('board');
    await pause(100);
    report.controls = {};
    report.controls.already_visible = await rpc({ action: 'observe', hwnd: window.hwnd, ...boardRegion, expected: normal, timeout_ms: 300 });
    assert.equal(report.controls.already_visible.ok, false);
    assert.match(report.controls.already_visible.error, /already visible/);
    await page.route('**/prototype/storage', async (route) => { const request = route.request().postDataJSON(); if (request.action === 'move_card' && request.column === 1)
        await pause(450); await route.continue(); });
    await openMove();
    report.controls.delayed_save = await observe('control-delayed-save', boardRegion, moved, () => click('#move-save'));
    assert.ok(report.controls.delayed_save.arm_to_pixels_lower_ms >= 430);
    assert.equal((await call({ action: 'snapshot', space: 'A' })).snapshot.cards.find(c => c.id === 1).column, 1);
    await page.unroute('**/prototype/storage');
    await click('#board-undo');
    await page.waitForFunction(() => document.querySelector('.board-column[data-column="0"] .board-card').dataset.card === '1', null, { polling: 10 });
    await align('board');
    await pause(100);
    await page.evaluate(() => document.querySelector('.board').style.visibility = 'hidden');
    await openMove();
    report.controls.hidden_result = await observe('control-hidden-result', boardRegion, moved, () => click('#move-save'), 550, true);
    assert.equal(report.controls.hidden_result.ok, false);
    assert.match(report.controls.hidden_result.error, /timeout/);
    assert.equal((await call({ action: 'snapshot', space: 'A' })).snapshot.cards.find(c => c.id === 1).column, 1);
    report.controls.hidden_result.saved_in_core = true;
    await page.evaluate(() => document.querySelector('.board').style.visibility = 'visible');
    await click('#board-undo');
    await page.waitForFunction(() => document.querySelector('.board-column[data-column="0"] .board-card').dataset.card === '1', null, { polling: 10 });
    await align('board');
    await pause(100);
    assert.deepEqual((await call({ action: 'snapshot', space: 'A' })).snapshot, initial);
    report.controls.snapshot_restored = true;
    const move = { warmups: [], samples: [] }, undo = { warmups: [], samples: [] };
    report.series.move = move;
    report.series.inverse_move = undo;
    for (let i = -5; i < 30; i++) {
        report.active_attempt = { series: 'move', repeat: i };
        await openMove();
        const forward = await observe('move', boardRegion, moved, () => click('#move-save'));
        assert.equal((await call({ action: 'snapshot', space: 'A' })).snapshot.cards.find(c => c.id === 1).column, 1);
        const reverse = await observe('inverse', boardRegion, normal, () => click('#board-undo'));
        assert.deepEqual((await call({ action: 'snapshot', space: 'A' })).snapshot, initial);
        (i < 0 ? move.warmups : move.samples).push({ ...forward, snapshot_verified: true });
        (i < 0 ? undo.warmups : undo.samples).push({ ...reverse, snapshot_verified: true });
    }
    console.log('Native pixels: move/inverse full series recorded.');
    await search('', 30);
    const searchRect = await align('search'), allRegion = nativeRegion(searchRect);
    const all = await template('search-first-card-1', allRegion);
    await search('Карточка 0030', 1);
    const oneRegion = nativeRegion(await align('search'));
    const one = await template('search-first-card-30', oneRegion);
    assert.notEqual(all, one);
    await search('', 30);
    await align('search');
    await pause(100);
    const find = { warmups: [], samples: [] }, clear = { warmups: [], samples: [] };
    report.series.search = find;
    report.series.clear_search = clear;
    for (let i = -5; i < 30; i++) {
        report.active_attempt = { series: 'search', repeat: i };
        const found = await observe('search', oneRegion, one, async () => { await search('Карточка 0030', 1); const r = nativeRegion(await align('search')); assert.deepEqual(r, oneRegion); });
        const cleared = await observe('clear-search', allRegion, all, async () => { await search('', 30); const r = nativeRegion(await align('search')); assert.deepEqual(r, allRegion); });
        (i < 0 ? find.warmups : find.samples).push({ ...found, result_count_verified: 1 });
        (i < 0 ? clear.warmups : clear.samples).push({ ...cleared, result_count_verified: 30 });
    }
    console.log('Native pixels: search/clear full series recorded.');
    report.startup = { stopped_processes: [], running_processes: [] };
    for (const kind of ['stopped_processes', 'running_processes'])
        for (let i = 0; i < 10; i++) {
            report.active_attempt = { series: kind, repeat: i };
            await page.goto('about:blank');
            await pause(80);
            if (kind === 'stopped_processes')
                await stop();
            const sample = await observe(`startup-${kind}`, boardRegion, normal, async () => { if (kind === 'stopped_processes')
                await start(); await page.goto(web.resolvedUrls.local[0]); await click('#board-read'); await waitBoard(); await align('board'); });
            report.startup[kind].push(sample);
        }
    function boundsSummary(samples, budget) { const { over_budget, ...statistics } = summarize(samples, budget); return { ...statistics, metric: 'conservative_upper_bound_ms', upper_bounds_above_budget: over_budget, budget_verdict: over_budget ? 'inconclusive_upper_bound_exceeds_budget' : 'supported_at_OS_desktop_image_boundary' }; }
    for (const [name, s] of Object.entries(report.series))
        s.summary = boundsSummary(s.samples, name.includes('search') ? 1000 : 300);
    report.startup.summaries = Object.fromEntries(Object.entries(report.startup).map(([name, s]) => [name, boundsSummary(s, 3000)]));
    assert.deepEqual((await call({ action: 'snapshot', space: 'A' })).snapshot, initial);
    report.snapshot_restored = true;
    delete report.active_attempt;
    report.status = 'completed_windows_desktop_pixel_upper_bounds';
}
let deadlineTimer;
let interrupt;
const deadline = new Promise((_, reject) => {
    deadlineTimer = setTimeout(() => reject(new Error('Desktop pixel measurement deadline exceeded')), 300000);
    interrupt = () => reject(new Error('Desktop pixel measurement interrupted'));
});
process.once('SIGINT', interrupt);
try {
    await Promise.race([measure(), deadline]);
}
catch (error) {
    report.status = 'failed';
    report.error = String(error.stack || error);
    process.exitCode = 1;
}
finally {
    clearTimeout(guard);
    clearTimeout(deadlineTimer);
    process.removeListener('SIGINT', interrupt);
    try {
        closingHelper = true;
        await browser?.close();
        helper?.stdin.end();
        if (helper)
            await helper.exited;
        await stop();
        if (directory)
            await rm(directory, { recursive: true, force: true });
        report.cleanup = true;
    }
    catch (error) {
        report.cleanup = false;
        report.cleanup_error = String(error);
        process.exitCode = 1;
    }
    report.finished = new Date().toISOString();
    const reportPath = resolve(reportDirectory, label + '-pixels-' + stamp + '.json');
    await Bun.write(reportPath, JSON.stringify(report, null, 2));
    console.log('Desktop pixel measurement:', report.status, '; local report:', reportPath);
}
