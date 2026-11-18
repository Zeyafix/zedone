import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtemp, mkdir, writeFile, readFile, rm, chmod } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, dirname, basename } from 'node:path';
import { runInNewContext } from 'node:vm';

const root = resolve(import.meta.dir, '../..');
const host = Bun.YAML.parse(await Bun.file(resolve(root, '.github/actions/prepare-host-check/action.yml')).text()).runs.steps;
const android = Bun.YAML.parse(await Bun.file(resolve(root, '.github/workflows/android-environment.yml')).text()).jobs['android-checks'];
const sdkRestore = android.steps.find(s => s.id === 'sdk-cache');
const sdkInstall = android.steps.find(s => s.name === 'Install pinned Android tools in an isolated SDK');
const sdkVerify = android.steps.find(s => s.name === 'Verify SDK identity and install pinned Rust targets');
const sdkSave = android.steps.find(s => s.name === 'Save verified Android SDK');
const rustLookup = android.steps.find(s => s.id === 'prototype-rust-cache');
const rustLegacy = android.steps.find(s => s.name === 'Restore Rust dependencies and Android build cache');
const rustPrototype = android.steps.find(s => s.name === 'Restore and retain prototype UI Rust dependencies');
for (const step of [rustLookup, rustLegacy, rustPrototype]) {
  assert.equal(step.uses, 'Swatinem/rust-cache@d1527c8f67ef9af357290475d4af4b0fab8f12e0');
  assert.equal(step.env.RUSTUP_TOOLCHAIN, '${{ steps.versions.outputs.rust }}');
  assert.equal(step.with['prefix-key'], 'zedone-rust-v1');
  assert.equal(step.with['cache-bin'], false);
  for (const input of ['cache-on-failure', 'cache-all-crates', 'cache-workspace-crates', 'add-rust-environment-hash-key']) assert.equal(step.with[input], undefined);
}
assert.equal(rustLookup.with['lookup-only'], true);
assert.equal(rustLookup.with['save-if'], false);
assert.equal(rustPrototype.with['save-if'], true);
assert.equal(rustPrototype.with['lookup-only'], undefined);
assert.equal(rustPrototype.with['shared-key'], 'prototype-android-ui-ndk-${{ steps.versions.outputs.ndk }}');
assert.equal(rustPrototype.with.workspaces, 'prototypes -> target');
for (const input of ['shared-key', 'workspaces']) assert.equal(rustLookup.with[input], rustPrototype.with[input]);
assert.equal(rustLegacy.with['shared-key'], 'prototype-android-ndk-${{ steps.versions.outputs.ndk }}');
assert.equal(rustLegacy.with.workspaces.trim(), '. -> target\nprototypes -> target');
assert.equal(rustLegacy.with['save-if'], "${{ github.ref == 'refs/heads/main' && matrix.suite == 'product-native' }}");
assert.ok(android.steps.indexOf(rustLookup) < android.steps.indexOf(rustLegacy));
assert.ok(android.steps.indexOf(rustLegacy) < android.steps.indexOf(rustPrototype));
for (const suite of android.strategy.matrix.suite) for (const hit of ['', 'false', 'true']) for (const ref of ['refs/heads/main', 'refs/pull/109/merge']) {
  const context = {matrix:{suite}, github:{ref}, hit};
  const enabled = step => runInNewContext(step.if.replaceAll('steps.prototype-rust-cache.outputs.cache-hit', 'hit'), context);
  assert.equal(enabled(rustLookup), suite === 'prototype-ui');
  assert.equal(enabled(rustPrototype), suite === 'prototype-ui');
  assert.equal(enabled(rustLegacy), suite !== 'prototype-ui' || hit !== 'true');
  const legacyWriter = enabled(rustLegacy) && runInNewContext(rustLegacy.with['save-if'].replace(/^\$\{\{\s*|\s*\}\}$/g, ''), context);
  assert.equal(legacyWriter, ref === 'refs/heads/main' && suite === 'product-native');
  assert.equal(Number(legacyWriter) + Number(enabled(rustPrototype)), suite === 'prototype-ui' || ref === 'refs/heads/main' && suite === 'product-native' ? 1 : 0);
}
assert.equal(sdkRestore.with.path, '${{ runner.temp }}/zedone-android-sdk');
assert.equal(sdkSave.with.path, sdkRestore.with.path);
assert.equal(sdkSave.with.key, '${{ steps.sdk-cache.outputs.cache-primary-key }}');
assert.equal(sdkRestore.with['restore-keys'], undefined);
for (const pin of ['.android.linux_sha256', '.android.linux_cli_sha256', '.android.linux_emulator_sha256', '.android.packages[]']) assert.ok(sdkInstall.run.includes(pin));
for (const part of ['runner.os', 'runner.arch', 'scripts/environment-versions.json', '.github/workflows/android-environment.yml', 'scripts/check-android-sdk.sh']) assert.ok(sdkRestore.with.key.includes(part));
assert.equal(sdkVerify.if, undefined, 'SDK validation and Rust installation run on both cold and hit paths');
assert.ok(android.steps.indexOf(sdkVerify) > android.steps.indexOf(sdkInstall));
assert.ok(android.steps.indexOf(sdkSave) > android.steps.findIndex(s => s.name === 'Check rejection and cleanup on compiler failure'));
const browserRestore = host.find(s => s.id === 'browser-cache');
const browserSave = host.find(s => s.name === 'Save installed test browser');
const browserInstall = host.find(s => s.name === 'Install test browser and OS dependencies');
assert.equal(browserRestore.with['restore-keys'], undefined);
assert.equal(browserSave.with.path, browserRestore.with.path);
assert.equal(browserSave.with.key, '${{ steps.browser-cache.outputs.cache-primary-key }}');
for (const part of ['runner.os', 'runner.arch', 'steps.browser.outputs.family', 'steps.browser.outputs.version']) assert.ok(browserRestore.with.key.includes(part));
assert.ok(browserInstall.run.includes('install --with-deps --only-shell chromium'));
assert.ok(!browserInstall.if.includes('cache-hit'), 'OS dependencies/installation cannot be skipped after a restore');
const condition = (source, context) => runInNewContext(source
  .replaceAll('steps.sdk-cache.outputs.cache-hit', 'hit').replaceAll('steps.browser-cache.outputs.cache-hit', 'hit')
  .replaceAll('steps.sdk-cache.outputs.cache-primary-key', 'key').replaceAll('steps.browser-cache.outputs.cache-primary-key', 'key'), context);
for (const hit of ['', 'false', 'true']) for (const key of ['', 'exact-key']) for (const passed of [false, true]) {
  for (const suite of android.strategy.matrix.suite) {
    const context = { hit, key, matrix: { suite }, success: () => passed };
    assert.equal(condition(sdkInstall.if, context), hit !== 'true');
    assert.equal(condition(sdkSave.if, context), passed && suite === 'prototype-native' && hit !== 'true' && key !== '');
  }
  for (const suite of ['all', 'product', 'product-build', 'prototype-native', 'prototype-ui', 'shell', 'shell-conflicts']) {
    const context = { hit, key, inputs: { suite }, success: () => passed };
    assert.equal(condition(browserRestore.if, context), ['all', 'prototype-ui', 'shell', 'shell-conflicts'].includes(suite));
    assert.equal(condition(browserInstall.if, context), condition(browserRestore.if, context));
    assert.equal(condition(browserSave.if, context), passed && ['all', 'prototype-ui', 'shell'].includes(suite) && hit !== 'true' && key !== '');
  }
}
const fetch = host.find(s => s.name === 'Fetch locked Rust dependencies');
assert.ok(fetch.run.includes('rustc "+$env:RUST_VERSION" -vV'));
assert.ok(fetch.run.includes('fetch --locked --target $hostTarget --manifest-path $manifest'));
assert.equal(host.find(s => s.uses?.startsWith('actions/setup-node@')).if, "inputs.suite != 'product-build'");
const bash = process.platform === 'win32' ? resolve(process.env.ProgramFiles || 'C:/Program Files', 'Git/bin/bash.exe') : 'bash';
const posix = value => value.replaceAll('\\', '/').replace(/^([A-Za-z]):/, (_, drive) => `/${drive.toLowerCase()}`);
const temporary = await mkdtemp(resolve(tmpdir(), 'zedone-ci-tool-cache-'));
try {
  const sdkSource = await Bun.file(resolve(root, 'scripts/check-android-sdk.sh')).text();
  for (const mode of ['valid', 'needs-initialization', 'corrupt-cli', 'wrong-cli-version', 'missing-package', 'wrong-package-version', 'rustup-failure']) {
    const directory = resolve(temporary, mode), bin = resolve(directory, 'bin');
    const sdk = resolve(directory, 'runner/zedone-android-sdk');
    await mkdir(bin, { recursive: true }); await mkdir(resolve(directory, 'scripts'), { recursive: true });
    await mkdir(resolve(sdk, 'cmdline-tools/23.0/bin'), { recursive: true });
    const cli = '#!/usr/bin/env bash\n' + (mode === 'needs-initialization' ? 'if [[ ! -f "$CLI_INIT" ]]; then touch "$CLI_INIT"; echo "Unpacking embedded installation"; exit 0; fi\n' : '')
      + 'echo ' + (mode === 'wrong-cli-version' ? '0.0.0' : '1.2.3') + '\n';
    const cliPath = resolve(sdk, 'cmdline-tools/23.0/bin/android');
    await writeFile(cliPath, cli + (mode === 'corrupt-cli' ? '\n' : '')); await chmod(cliPath, 0o755);
    const packages = ['platform-tools@37.0.1', 'emulator@37.2.12', 'ndk/30.0.0@30.0.0'];
    for (const entry of packages) {
      const [name, revision] = entry.split('@'); await mkdir(resolve(sdk, name), { recursive: true });
      if (mode !== 'missing-package' || name !== 'emulator') await writeFile(resolve(sdk, name, 'source.properties'), `Pkg.Revision = ${mode === 'wrong-package-version' && name === 'emulator' ? '0.0.0' : revision}\n`);
    }
    await writeFile(resolve(directory, 'scripts/environment-versions.json'), JSON.stringify({rust:'1.99.1', android:{command_line_tools:'23.0',cli:'1.2.3',ndk:'30.0.0',packages,linux_cli_sha256:createHash('sha256').update(cli).digest('hex')}}));
    await writeFile(resolve(directory, 'scripts/check-android-sdk.sh'), sdkSource);
    await writeFile(resolve(directory, 'verify.sh'), sdkVerify.run);
    await writeFile(resolve(directory, 'jq.mjs'), 'const q=Bun.argv.at(-2);const data=await Bun.file(Bun.argv.at(-1)).json();if(q===".android.packages[]")for(const p of data.android.packages)console.log(p);else console.log(q.slice(1).split(".").reduce((v,k)=>v[k],data));\n');
    await writeFile(resolve(bin, 'jq'), '#!/usr/bin/env bash\nexec "$TEST_BUN" "$FAKE_JQ" "$@"\n'); await chmod(resolve(bin, 'jq'), 0o755);
    await writeFile(resolve(bin, 'rustup'), '#!/usr/bin/env bash\nprintf "%s\\n" "$*" >> "$RUSTUP_RECORD"\nexit "${RUSTUP_STATUS:-0}"\n'); await chmod(resolve(bin, 'rustup'), 0o755);
    await writeFile(resolve(bin, 'shellcheck'), '#!/usr/bin/env bash\nexit 0\n'); await chmod(resolve(bin, 'shellcheck'), 0o755);
    const output = resolve(directory, 'github-env'), record = resolve(directory, 'rustup-calls');
    const result = Bun.spawnSync([bash, '-c', 'export PATH="$1:$2:/usr/bin:$PATH"; exec /usr/bin/bash -e -o pipefail "$3"', '_', posix(bin), posix(resolve(root, 'temp/ci-tools')), posix(resolve(directory, 'verify.sh'))], {
      cwd: directory, env: { ...process.env, CLI_INIT:posix(resolve(directory,'initialized')), TEST_BUN:posix(process.execPath), FAKE_JQ:posix(resolve(directory,'jq.mjs')), GITHUB_WORKSPACE:posix(directory), RUNNER_TEMP:posix(resolve(directory,'runner')), GITHUB_ENV:posix(output), RUSTUP_RECORD:posix(record), RUSTUP_STATUS:mode === 'rustup-failure' ? '23' : '0' },
      stdin:'ignore', stdout:'pipe', stderr:'pipe', timeout:30000,
    });
    assert.ok(!result.signalCode, `${mode}: SDK fixture interrupted`);
    assert.equal(result.exitCode, ['valid', 'needs-initialization'].includes(mode) ? 0 : mode === 'rustup-failure' ? 23 : 1, `${mode}: ${result.stderr}`);
    if (['valid', 'needs-initialization', 'rustup-failure'].includes(mode)) {
      const call = await readFile(record, 'utf8');
      for (const target of ['aarch64-linux-android','armv7-linux-androideabi','i686-linux-android','x86_64-linux-android']) assert.ok(call.includes(`--target ${target}`));
      const env = await readFile(output, 'utf8'); assert.ok(env.includes(`ANDROID_HOME=${posix(sdk)}`)); assert.ok(env.includes(`NDK_HOME=${posix(sdk)}/ndk/30.0.0`));
    } else assert.equal(await Bun.file(record).exists(), false, 'Bad SDK must fail before Rust installation/build');
  }
  for (const suite of ['shell', 'shell-conflicts', 'prototype-ui', 'all']) {
    const directory = resolve(temporary, `browser-${suite}`);
    await mkdir(resolve(directory, 'node_modules/playwright'), { recursive: true });
    await mkdir(resolve(directory, 'prototypes/node_modules/playwright'), { recursive: true });
    await writeFile(resolve(directory, 'node_modules/playwright/package.json'), '{"version":"1.64.1"}');
    await writeFile(resolve(directory, 'prototypes/node_modules/playwright/package.json'), '{"version":"1.63.0"}');
    const output = resolve(directory, 'outputs'), envFile = resolve(directory, 'env');
    const code = host.find(s => s.id === 'browser').run;
    const script = resolve(directory, 'browser.ps1'); await writeFile(script, code);
    const result = Bun.spawnSync(['pwsh', '-NoProfile', '-NonInteractive', '-File', script], {
      cwd:directory, env:{...process.env, SUITE:suite, RUNNER_TEMP:directory, GITHUB_OUTPUT:output, GITHUB_ENV:envFile}, stdin:'ignore', stdout:'pipe', stderr:'pipe', timeout:30000,
    });
    assert.equal(result.exitCode, 0, `${suite}: ${result.stderr}`);
    const properties = await readFile(output, 'utf8'); const family = suite.startsWith('shell') ? 'shell' : 'prototype';
    assert.ok(properties.includes(`family=${family}`)); assert.ok(properties.includes(`version=${family === 'shell' ? '1.64.1' : '1.63.0'}`));
    assert.ok((await readFile(envFile,'utf8')).includes(`PLAYWRIGHT_BROWSERS_PATH=${resolve(directory,'zedone-playwright')}`));
  }
} finally {
  assert.equal(dirname(temporary), resolve(tmpdir())); assert.ok(basename(temporary).startsWith('zedone-ci-tool-cache-'));
  await rm(temporary, { recursive: true });
}
console.log('CI tool caches: exact keys/single owners, cold/hit validation, CLI hash/version, package missing/mismatch and Rust failure propagation passed.');
await import('./check-ci-rust.mjs');
