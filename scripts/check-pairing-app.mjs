import { checkSpaceExchange } from "./check-space-exchange.mjs";
import assert from 'node:assert/strict';
import { randomBytes, createHash } from 'node:crypto';
import { resolve } from 'node:path';
import { mkdir } from 'node:fs/promises';
const root = resolve(import.meta.dir, '..');
const versions = await Bun.file(resolve(root, 'scripts/environment-versions.json')).json();
assert.equal(Bun.version, versions.bun);
assert.ok(process.platform === 'win32' || process.env.CI === 'true', 'Non-Windows host fixture is CI-only');
if (process.env.CI !== 'true')
    assert.ok(process.env.AGENT_DEVICE_COMMAND, 'Device-open command required for local app lifecycle');
const serial = process.env.ANDROID_SERIAL;
assert.match(serial || '', /^emulator-\d+$/);
const suffix = process.platform === 'win32' ? '.exe' : '';
const adb = resolve(process.env.ANDROID_HOME, `platform-tools/adb${suffix}`);
const app = 'app.zedone.local';
const run = randomBytes(16).toString('hex');
const report = resolve(root, 'docs/reports', `pairing-app-${run}`);
await mkdir(report, { recursive: true });
console.log('Pairing acceptance started; isolated fixtures and original APK/data are retained in ignored reports.');
function invoke(args, { input, allowFailure = false, timeout = 30000 } = {}) {
    const result = Bun.spawnSync(args, { cwd: root, stdin: input === undefined ? 'ignore' : input, stdout: 'pipe', stderr: 'pipe', timeout });
    if (!allowFailure)
        assert.equal(result.exitCode, 0, `Acceptance command failed: ${args[0]}`);
    return result;
}
const device = (...args) => invoke([adb, '-s', serial, ...args]).stdout;
const read = path => invoke([adb, '-s', serial, 'shell', '-T', 'run-as', app, 'cat', path], { allowFailure: true });
const write = (path, bytes) => invoke([adb, '-s', serial, 'exec-in', 'run-as', app, 'tee', path], { input: Buffer.from(bytes) });
function launch() {
    if (process.env.CI === 'true') {
        device('shell', 'am', 'force-stop', app);
        device('shell', 'am', 'start', '-W', '-n', `${app}/.MainActivity`);
    }
    else
        invoke([process.env.AGENT_DEVICE_COMMAND, 'open', app, '--relaunch']);
}
function close() {
    if (process.env.CI === 'true')
        device('shell', 'am', 'force-stop', app);
    else
        invoke([process.env.AGENT_DEVICE_COMMAND, 'close', app]);
}
async function until(readiness, label, budget = 25000) {
    const deadline = Date.now() + budget;
    while (Date.now() < deadline) {
        const value = await readiness();
        if (value)
            return value;
        await Bun.sleep(150);
    }
    throw new Error(`Acceptance readiness failed: ${label}`);
}
const hashes = () => {
    const text = device('exec-out', 'run-as', app, 'sh', '-c', 'find . -type f -exec sha256sum {} \\;').toString();
    const result = new Map();
    for (const line of text.trim().split(/\r?\n/)) {
        const match = /^([a-f0-9]{64})  (\.\/.*)$/.exec(line);
        assert.ok(match, 'Invalid application file manifest');
        result.set(match[2], match[1]);
    }
    return result;
};
const normalized = map => JSON.stringify([...map].sort(([a], [b]) => a.localeCompare(b)));
close();
const originalPath = device('shell', 'pm', 'path', app).toString().trim().replace(/^package:/, '');
assert.match(originalPath, /^\/data\/app\/[^\r\n]+\/base.apk$/);
device('pull', originalPath, resolve(report, 'original.apk'));
const before = hashes();
await Bun.write(resolve(report, 'original-files.json'), normalized(before));
await Bun.write(resolve(report, 'original-data.tar'), device('exec-out', 'run-as', app, 'tar', '-cf', '-', '.'));
assert.notEqual(read('pairing-acceptance-run').exitCode, 0, 'Preserve an existing acceptance session');
const artifacts = invoke(['cargo', `+${versions.rust}`, 'test', '-p', 'zedone-adapters', '--lib', '--no-run', '--message-format=json', '--locked', '--offline'], { timeout: 240000 }).stdout.toString().trim().split('\n').map(line => JSON.parse(line));
const testBinary = artifacts.find(item => item.reason === 'compiler-artifact' && item.profile.test && item.target.name === 'zedone_adapters')?.executable;
assert.ok(testBinary);
invoke(['cargo', `+${versions.rust}`, 'build', '-p', 'zedone-adapters', '--bin', 'zedone', '--locked', '--offline'], { timeout: 240000 });
const cli = resolve(root, `target/debug/zedone${suffix}`);
const serverDirectory = resolve(report, 'server');
await mkdir(serverDirectory);
const server = Bun.spawn([testBinary, '--ignored', '--exact', 'device_host::tests::acceptance_server'], {
    cwd: root, env: { ...process.env, ZEDONE_PAIRING_FIXTURE: serverDirectory }, stdin: 'ignore', stdout: Bun.file(resolve(report, 'server.log')), stderr: Bun.file(resolve(report, 'server-errors.log')),
});
let installed = false, reverse, forward, android, passedChecks, uiFixture;
async function cleanupUiFixture() {
    if (!uiFixture) return;
    close(); write('pairing-ui-cleanup',uiFixture); launch();
    await until(()=>read('pairing-ui-cleaned').stdout.toString()==='cleaned','UI fixture OS cleanup');
    close();
    for (const path of ['pairing-ui-run','pairing-ui-cleanup','pairing-ui-cleaned']) device('shell','run-as',app,'rm','-f',path);
    uiFixture=undefined;
}
const marker = 'pairing-acceptance-run';
const fixture = `pairing-acceptance-${run}`;
async function fixtureReady() {
    return until(() => {
        if (read(`${fixture}/failed`).exitCode === 0)
            throw new Error('APK fixture unavailable');
        const response = read(`${fixture}/public.json`);
        if (response.exitCode !== 0)
            return;
        try {
            return JSON.parse(response.stdout);
        }
        catch {
            return;
        }
    }, 'Android Rust host');
}
async function forwardAndroid() {
    const metadata = await fixtureReady();
    assert.ok(metadata.pin.length === 32);
    const port = Number(metadata.address.split(':').at(-1));
    assert.ok(port > 0 && port < 65536);
    if (forward)
        device('forward', '--remove', `tcp:${forward}`);
    forward = Number(device('forward', 'tcp:0', `tcp:${port}`).toString().trim());
    const pinFile = resolve(report, 'android.pin');
    await Bun.write(pinFile, Buffer.from(metadata.pin).toString('hex'));
    const keyFile = resolve(report, 'android.key');
    const key = read(`${fixture}/admin.key`);
    assert.equal(key.exitCode, 0);
    await Bun.write(keyFile, key.stdout);
    android = { address: `127.0.0.1:${forward}`, pinFile, keyFile };
    return metadata;
}
function call(target, operation, parameters) {
    const input = JSON.stringify(parameters);
    const result = invoke([cli, '--endpoint', target.address, '--pin-file', target.pinFile, '--key-file', target.keyFile, '--operation', operation], { input: Buffer.from(input), allowFailure: true });
    assert.ok([0, 1].includes(result.exitCode), 'CLI terminated unexpectedly');
    const response = JSON.parse(result.stdout);
    assert.equal(response.status === 'ok', result.exitCode === 0, 'CLI exit/envelope mismatch');
    return response;
}
const request = (operation, parameters) => ({ id: [...randomBytes(16)], operation, parameters });
const ok = response => { assert.equal(response.status, 'ok', `Unexpected command failure: ${response.error?.code}`); return response.result; };
const error = (response, code) => assert.equal(response.error?.code, code);
let host, space, lastOffer, exchangeIds;
function command(target, operation, parameters) { return call(target, operation, request(operation, parameters)); }
async function invitation() {
    const created = ok(call(host, 'offer.create', {}));
    assert.equal(created.invitation.status, 'open');
    assert.ok(ok(call(host, 'offer.read', {})).offer === created.offer, 'Saved invitation code changed');
    const before = ok(call(android, 'session.status', {}));
    const qr = ok(call(host, 'offer.qr', { address: host.address }));
    const decoded = ok(call(android, 'session.decode_qr', { text: qr.text }));
    assert.deepEqual(decoded, { address: host.address, offer: created.offer }, 'QR changed manual input');
    assert.deepEqual(ok(call(android, 'session.status', {})), before, 'QR decoding prepared a session');
    assert.equal(ok(command(host, 'invitation.inspect', { invitation: created.invitation.invitation })).status, 'open', 'QR consumed the invitation');
    return { invitation: created.invitation.invitation, ...decoded };
}
function deliver(operation, parameters, intention = request(operation, parameters)) {
    return call(android, 'fixture.peer_command', { operation, request: JSON.stringify(intention) });
}
async function prepare(label) { const offered = await invitation(); lastOffer = offered.offer; ok(call(android, 'session.prepare', { address: offered.address, offer: offered.offer, label })); return ok(call(android, 'session.submit', {})); }
try {
    const metadata = await until(async () => {
        if (server.exitCode !== null)
            throw new Error('Windows fixture stopped before readiness');
        try {
            return await Bun.file(resolve(serverDirectory, 'public.json')).json();
        }
        catch {
            return;
        }
    }, 'Windows secure host');
    const pinFile = resolve(report, 'host.pin');
    await Bun.write(pinFile, Buffer.from(metadata.pin).toString('hex'));
    host = { address: metadata.address, pinFile, keyFile: resolve(serverDirectory, 'admin.key') };
    const port = Number(host.address.split(':').at(-1));
    reverse = `tcp:${port}`;
    device('reverse', '--no-rebind', reverse, reverse);
    close();
    device('install', '-r', resolve(root, 'ui/src-tauri/gen/android/app/build/outputs/apk/x86_64/debug/app-x86_64-debug.apk'));
    installed = true;
    write(marker, run);
    launch();
    const initialAndroid = await forwardAndroid();
    space = randomBytes(16).toString('hex');
    ok(command(host, 'space.create', { space, name: 'Pairing acceptance preserved' }));
    const pending = await prepare('Acceptance phone');
    const firstOffer = lastOffer;
    assert.equal(pending.status, 'pending');
    assert.equal(ok(call(android, 'session.status', {})).status, 'pending');
    assert.equal(call(android, 'session.submit', {}).replayed, true);
    error(deliver('space.read', { space }), 'access_denied');
    error(deliver('pairing.confirm', { request: pending.request, expected: pending.version, scope: { kind: 'administration' }, expires_at_unix_ms: null }), 'access_denied');
    ok(command(host, 'pairing.confirm', { request: pending.request, expected: pending.version, scope: { kind: 'spaces', spaces: { [space]: 'read' } }, expires_at_unix_ms: null }));
    const linked = ok(call(android, 'session.refresh', {}));
    assert.equal(linked.status, 'confirmed');
    exchangeIds = checkSpaceExchange({host,android,space,pairing:pending.request,writable:false,call,command,ok,error});
    const readIntention = request('space.read', { space });
    const saved = ok(deliver('space.read', { space }, readIntention));
    assert.deepEqual(saved.names, ['Pairing acceptance preserved']);
    error(deliver('space.rename', { space, name: 'Forbidden', expected: saved.version }), 'access_denied');
    const statusBefore = ok(call(android, 'session.status', {}));
    device('exec-out', 'run-as', app, 'rm', '--', `${fixture}/public.json`);
    launch();
    const restartedAndroid = await forwardAndroid();
    assert.deepEqual(restartedAndroid.pin, initialAndroid.pin, 'Keystore identity changed on process restart');
    assert.deepEqual(ok(call(android, 'session.status', {})), statusBefore, 'Protected session did not survive restart');
    assert.equal(ok(call(android, 'session.refresh', {})).status, 'confirmed');
    const replay = deliver('space.read', { space }, readIntention);
    ok(replay);
    assert.equal(replay.replayed, true);
    error(call(android, 'session.unlink', { id: [...randomBytes(16)], expected: pending.version }), 'stale_state');
    const unlink = { id: [...randomBytes(16)], expected: linked.version };
    ok(call(android, 'session.unlink', unlink));
    assert.equal(call(android, 'session.unlink', unlink).replayed, true);
    assert.equal(ok(call(android, 'session.refresh', {})).status, 'unlinked');
    assert.equal(ok(call(android, 'session.status', {})).status, 'unconfigured');
    ok(call(android, 'session.prepare', { address: host.address, offer: firstOffer, label: 'Reused invitation' }));
    error(call(android, 'session.submit', {}), 'access_denied');
    assert.equal(ok(call(android, 'session.status', {})).status, 'unconfigured');
    const second = await prepare('Reconnected phone');
    assert.notEqual(second.request, pending.request);
    ok(command(host, 'pairing.confirm', { request: second.request, expected: second.version, scope: { kind: 'spaces', spaces: { [space]: 'read_write' } }, expires_at_unix_ms: null }));
    ok(call(android, 'session.refresh', {}));
    const current = ok(deliver('space.read', { space }));
    const changed = ok(deliver('space.rename', { space, name: 'Explicit permitted write', expected: current.version }));
    ok(deliver('space.rename', { space, name: 'Pairing acceptance preserved', expected: changed.version }));
    checkSpaceExchange({host,android,space,pairing:second.request,writable:true,call,command,ok,error,previous:exchangeIds});
    for (const [operation, parameters] of [
        ['sync.pull', { protocol:999, space, known:[] }],
        ['sync.push', { protocol:999, space, base:[], changes:[] }],
    ]) error(deliver(operation, parameters), 'incompatible_sync');
    const recoverySession = ok(call(android, 'session.status', {}));
    const recoveryControl = () => ok(command(android, 'sync.inspect', {space,pairing:second.request}));
    device('reverse', '--remove', reverse);
    const offline = ok(command(android, 'card.read', {space,card:exchangeIds.card}));
    ok(command(android, 'card.edit', {space,card:exchangeIds.card,title:'Saved offline after disconnect',description:offline.descriptions[0],expected:offline.version}));
    error(call(android, 'exchange.run', {space}), 'unavailable');
    let paused = recoveryControl();
    ok(command(android, 'sync.pause', {space,pairing:second.request,paused:true,expected:paused.pause_version}));
    device('exec-out', 'run-as', app, 'rm', '--', `${fixture}/public.json`);
    launch(); await forwardAndroid();
    assert.deepEqual(ok(call(android, 'session.status', {})), recoverySession);
    assert.equal(recoveryControl().paused,true);
    device('reverse', '--no-rebind', reverse, reverse);
    error(call(android, 'exchange.run', {space}), 'conflict');
    assert.deepEqual(ok(command(host,'card.read',{space,card:exchangeIds.card})).titles,['Exchange card']);
    paused = recoveryControl();
    ok(command(android, 'sync.pause', {space,pairing:second.request,paused:false,expected:paused.pause_version}));
    ok(call(android, 'exchange.run', {space}));
    assert.deepEqual(ok(command(host,'card.read',{space,card:exchangeIds.card})).titles,['Saved offline after disconnect']);
    const recoveredHistory = ok(command(host,'card.history',{space,card:exchangeIds.card,after:null,limit:50,expected:null}));
    for(let repeat=0;repeat<2;repeat++)ok(call(android,'exchange.run',{space}));
    assert.deepEqual(ok(command(host,'card.history',{space,card:exchangeIds.card,after:null,limit:50,expected:null})),recoveredHistory);
    assert.equal(recoveryControl().selected,true);
    console.log('APK exchange recovery: unsupported protocol, real disconnected transport, offline card edit, Keystore restart, retained explicit pause, resume and duplicate-free history passed.');
    const access = ok(command(host, 'access.inspect', { subject: second.request }));
    ok(command(host, 'access.revoke', { subject: second.request, expected: access.version }));
    error(deliver('space.read', { space }), 'access_denied');
    assert.equal(ok(call(android, 'session.refresh', {})).status, 'unlinked');
    const cancelled = await prepare('Cancelled phone');
    const cancel = { id: [...randomBytes(16)], expected: cancelled.version };
    ok(call(android, 'session.cancel', cancel));
    assert.equal(call(android, 'session.cancel', cancel).replayed, true);
    assert.equal(ok(call(android, 'session.refresh', {})).status, 'cancelled');
    const rejected = await prepare('Rejected phone');
    ok(command(host, 'pairing.reject', { request: rejected.request, expected: rejected.version }));
    assert.equal(ok(call(android, 'session.refresh', {})).status, 'rejected');
    const timed = await prepare('Timed access');
    const expires = Date.now() + 10000;
    ok(command(host, 'pairing.confirm', { request: timed.request, expected: timed.version, scope: { kind: 'spaces', spaces: { [space]: 'read' } }, expires_at_unix_ms: expires }));
    ok(call(android, 'session.refresh', {}));
    const timedRead = request('space.read', { space });
    ok(deliver('space.read', { space }, timedRead));
    await Bun.sleep(Math.max(0, expires - Date.now() + 50));
    error(deliver('space.read', { space }, timedRead), 'access_denied');
    const timedState = ok(call(android, 'session.refresh', {}));
    assert.equal(timedState.status, 'confirmed', 'Expiry silently unlinked the device');
    ok(call(android, 'session.unlink', { id: [...randomBytes(16)], expected: timedState.version }));
    assert.equal(ok(call(android, 'session.refresh', {})).status, 'unlinked');
    const bad = await invitation();
    for (let i = 0; i < 5; i++) {
        const fields = bad.offer.split(':');
        fields[3] = randomBytes(32).toString('hex');
        ok(call(android, 'session.prepare', { address: host.address, offer: fields.join(':'), label: 'Wrong code' }));
        error(call(android, 'session.submit', {}), 'access_denied');
        assert.equal(ok(call(android, 'session.status', {})).status, 'unconfigured');
    }
    assert.equal(ok(command(host, 'invitation.inspect', { invitation: bad.invitation })).status, 'locked');
    assert.deepEqual(ok(command(host, 'space.read', { space })).names, ['Pairing acceptance preserved']);
    if (process.env.P06_EXCHANGE_UI === '1') {
        const incomingOffer=ok(call(android,'offer.create',{}));
        ok(call(host,'session.prepare',{address:android.address,offer:incomingOffer.offer,label:'Windows UI exchange'}));
        const uiRequest=ok(call(host,'session.submit',{}));
        ok(command(android,'pairing.confirm',{request:uiRequest.request,expected:uiRequest.version,scope:{kind:'spaces',spaces:{[space]:'read'}},expires_at_unix_ms:null}));
        ok(call(host,'session.refresh',{}));
        const uiRun = randomBytes(16).toString('hex');
        const uiFolder = `pairing-ui-${uiRun}`;
        const seed = resolve(report,'ui-seed.sqlite');
        close();
        const nativeSeed=resolve(report,'ui-native.sqlite');
        for(const ext of ['', '-wal', '-shm']){const file=invoke([adb,'-s',serial,'exec-out','run-as',app,'cat',`${fixture}/data.sqlite${ext}`],{allowFailure:true});if(file.exitCode===0)await Bun.write(nativeSeed+ext,file.stdout);else assert.notEqual(ext,'','Missing native fixture');}
        const snapshot = invoke(['python','-c','import sqlite3,sys; a=sqlite3.connect(sys.argv[1]); b=sqlite3.connect(sys.argv[2]); a.backup(b); print(a.execute("SELECT subject FROM access_bootstrap").fetchone()[0]); b.close(); a.close()',nativeSeed,seed]);
        const subject = snapshot.stdout.toString().trim(); assert.match(subject,/^[a-f0-9]{32}$/);
        device('shell','run-as',app,'mkdir',uiFolder);
        const uiRemote=`/data/local/tmp/zedone-ui-seed-${uiRun}.sqlite`;
        device('push',seed,uiRemote);
        device('shell','run-as',app,'cp',uiRemote,`${uiFolder}/zedone.sqlite`);
        device('shell','rm',uiRemote);
        const copied=device('exec-out','run-as',app,'cat',`${uiFolder}/zedone.sqlite`);
        assert.equal(createHash('sha256').update(copied).digest('hex'),createHash('sha256').update(Buffer.from(await Bun.file(seed).arrayBuffer())).digest('hex'),'UI seed transfer checksum mismatch');
        write(`${uiFolder}/local-session.json`,JSON.stringify({subject,key:(await Bun.file(android.keyFile).text()).trim()}));
        write('pairing-ui-run',uiRun);uiFixture=uiFolder;
        await Bun.write(resolve(report,'ui-checkpoint.json'),JSON.stringify({uiRun,uiFolder,space,pairing:uiRequest.request}));
        console.log(`Device UI checkpoint ready: ${report}`);
        launch();
        await until(()=>Bun.file(resolve(report,'ui-continue')).exists(),'Device UI exchange verification',300000);
        const outcome=(await Bun.file(resolve(report,'ui-continue')).text()).trim();
        await cleanupUiFixture();
        assert.equal(outcome,'passed','Device UI exchange verification did not pass');
    }
    passedChecks = ['space exchange/read-only/write/columns/cards/tags/history/pause/reverse/preservation', 'QR exact input/no implicit prepare/no consumption', 'request/replay/no-access', 'confirm/read-only', 'Keystore process restart/pin/session', 'stale unlink/replay/cleanup', 'used-code rejection', 'reconnect/explicit-write/reverse', 'revoke', 'cancel/reject', 'timed-grant/replay denial/no-auto-unlink', 'five-attempt lock', 'data preservation'];
    console.log(`${process.platform === 'win32' ? 'Windows Credential Manager' : 'CI host memory fixture'} ↔ Android APK: TLS/API/CLI, Keystore process restart, rights, repeat/reconnect, cancel/reject/unlink/revoke and five-attempt lock passed.`);
}
catch (error) {
    console.error('Pairing scenario failed:', error.message);
    throw error;
}
finally {
    await Bun.write(resolve(serverDirectory, 'stop'), 'stop');
    if (reverse)
        device('reverse', '--remove', reverse);
    if (forward)
        device('forward', '--remove', `tcp:${forward}`);
    let cleanupError;
    if (installed) {
        try {
            await cleanupUiFixture();
            close();
            const namespace = read(`${fixture}/host-namespace-v1`);
            if (namespace.exitCode === 0) {
                await Bun.write(resolve(report, 'android-namespace.bin'), namespace.stdout);
                write('pairing-acceptance-cleanup', run);
                launch();
                await until(() => { const result = read('pairing-acceptance-cleaned'); return result.exitCode === 0 && result.stdout.toString() === run; }, 'Keystore fixture cleanup');
            }
        }
        catch (error) {
            cleanupError = error;
        }
        close();
        device('install', '-r', resolve(report, 'original.apk'));
        const after = hashes();
        for (const path of after.keys())
            if (!before.has(path)) {
                assert.ok(path.startsWith('./') && !path.split('/').includes('..'));
                device('exec-out', 'run-as', app, 'rm', '-f', '--', path);
            }
        invoke([adb, '-s', serial, 'exec-in', 'run-as', app, 'tar', '-xf', '-'], { input: Buffer.from(await Bun.file(resolve(report, 'original-data.tar')).arrayBuffer()) });
        assert.equal(normalized(hashes()), normalized(before), 'Application files were not restored');
        const restored = device('shell', 'pm', 'path', app).toString().trim().replace(/^package:/, '');
        device('pull', restored, resolve(report, 'restored.apk'));
        const sha = async (name) => createHash('sha256').update(Buffer.from(await Bun.file(resolve(report, name)).arrayBuffer())).digest('hex');
        assert.equal(await sha('original.apk'), await sha('restored.apk'));
        console.log('Original APK and every application file restored by SHA-256.');
    }
    assert.equal(await server.exited, 0, 'Windows fixture or secret cleanup failed');
    if (cleanupError)
        throw cleanupError;
    if (passedChecks) {
        await Bun.write(resolve(report, 'result.json'), JSON.stringify({ status: 'passed', checks: passedChecks, restored: true, cleanup: true }));
        console.log('Fixture OS keys and transport rules cleaned; final acceptance passed.');
    }
}
