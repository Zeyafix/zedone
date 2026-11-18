import assert from 'node:assert/strict';
import { mkdtemp, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, dirname, basename } from 'node:path';

const root = resolve(import.meta.dir, '../..');
const version = (await Bun.file(resolve(root, 'scripts/environment-versions.json')).json()).rust;
const host = Bun.YAML.parse(await Bun.file(resolve(root, '.github/actions/prepare-host-check/action.yml')).text()).runs.steps;
const android = Bun.YAML.parse(await Bun.file(resolve(root, '.github/workflows/android-environment.yml')).text()).jobs['android-checks'].steps;
const script = './scripts/prepare-ci-rust.ps1';
assert.ok(host[0].run.indexOf(script) > host[0].run.indexOf("throw 'Rust installation failed.'"));
const prepare = android.findIndex(step => step.run === script);
assert.ok(prepare > android.findIndex(step => step.name === 'Verify SDK identity and install pinned Rust targets'));
assert.ok(prepare < android.findIndex(step => step.uses?.startsWith('Swatinem/rust-cache@')));
assert.equal(android[prepare].shell, 'pwsh');
assert.equal(android[prepare].if, undefined);
const directory = await mkdtemp(resolve(tmpdir(), 'zedone-ci-rust-'));
try {
  const wrapper = resolve(directory, 'fixture.ps1'), record = resolve(directory, 'calls.json');
  await writeFile(wrapper, `
$ErrorActionPreference = 'Stop'
$state = Get-Content -Raw $env:RUST_FIXTURE | ConvertFrom-Json
$calls = [System.Collections.Generic.List[string]]::new()
function rustc {
    $calls.Add("rustc $args")
    $global:LASTEXITCODE = if ($state.mode -eq 'compiler-failure') { 23 } else { 0 }
    "release: $($state.version)"
    if ($state.mode -ne 'missing-host') { "host: $($state.host)" }
}
function rustup {
    $calls.Add("rustup $args")
    $global:LASTEXITCODE = 0
    if ($args[1] -eq 'list') {
        if ($state.mode -eq 'list-failure') { $global:LASTEXITCODE = 23; return }
        foreach ($name in $state.installed) { "$name (default)" }
    } else {
        if ($state.mode -eq 'uninstall-failure') { $global:LASTEXITCODE = 23; return }
        $removed = $args[2]
        if ($state.mode -ne 'stale-list') { $state.installed = @($state.installed | Where-Object { $_ -cne $removed }) }
    }
}
try { & $env:RUST_PREPARE } finally { ConvertTo-Json -InputObject @($calls) | Set-Content $env:RUST_CALLS }
`);
  for (const target of ['x86_64-pc-windows-msvc', 'x86_64-unknown-linux-gnu']) {
    const pinned = `${version}-${target}`, extra = `1.100.0-${target}`;
    for (const mode of ['valid', 'already-pinned', 'local', 'self-hosted', 'compiler-failure', 'missing-host', 'wrong-release', 'missing-pin', 'list-failure', 'invalid-list', 'uninstall-failure', 'stale-list']) {
      const state = resolve(directory, 'state.json');
      await writeFile(state, JSON.stringify({mode, host:target, version:mode === 'wrong-release' ? '0.0.0' : version,
        installed:mode === 'missing-pin' ? [extra] : mode === 'already-pinned' ? [pinned] : [pinned, mode === 'invalid-list' ? '/unexpected/path' : extra]}));
      const result = Bun.spawnSync(['pwsh', '-NoProfile', '-NonInteractive', '-File', wrapper], {
        env:{...process.env, GITHUB_ACTIONS:mode === 'local' ? '' : 'true', RUNNER_ENVIRONMENT:mode === 'self-hosted' ? 'self-hosted' : 'github-hosted',
          RUST_FIXTURE:state, RUST_CALLS:record, RUST_PREPARE:resolve(root, script)}, stdin:'ignore', stdout:'pipe', stderr:'pipe', timeout:30000,
      });
      assert.ok(!result.signalCode, `${target}/${mode}: interrupted`);
      assert.equal(result.exitCode === 0, ['valid', 'already-pinned'].includes(mode), `${target}/${mode}: ${result.stderr}`);
      const calls = JSON.parse(await readFile(record, 'utf8'));
      const uninstalls = calls.filter(call => call.startsWith('rustup toolchain uninstall'));
      assert.deepEqual(uninstalls, ['valid', 'uninstall-failure', 'stale-list'].includes(mode) ? [`rustup toolchain uninstall ${extra}`] : []);
      if (['local', 'self-hosted'].includes(mode)) assert.deepEqual(calls, []);
      else assert.equal(calls[0], `rustc +${version} -vV`);
      if (mode === 'valid') assert.match(result.stdout.toString(), new RegExp(`Rust cache toolchain: ${pinned}`));
    }
  }
} finally {
  assert.equal(dirname(directory), resolve(tmpdir())); assert.ok(basename(directory).startsWith('zedone-ci-rust-'));
  await rm(directory, {recursive:true});
}
console.log('CI Rust: pinned-only Windows/Linux inventory, local/self-hosted refusal, malformed identity/list and command/postcondition failures passed.');
