$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -cne 'true' -or $env:RUNNER_ENVIRONMENT -cne 'github-hosted') {
    throw 'Rust cache preparation is restricted to disposable GitHub-hosted runners.'
}
$version = (Get-Content -Raw (Join-Path $PSScriptRoot 'environment-versions.json') | ConvertFrom-Json).rust
$compiler = @(rustc "+$version" -vV)
if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect the pinned Rust compiler.' }
$hosts = @($compiler | Where-Object { $_ -cmatch '^host: [a-z0-9_-]+$' })
if ($hosts.Count -ne 1 -or "release: $version" -cnotin $compiler) { throw 'Unexpected pinned Rust compiler identity.' }
$pinned = "$version-$($hosts[0].Substring(6))"
function Read-Toolchains {
    $lines = @(rustup toolchain list)
    if ($LASTEXITCODE -ne 0 -or $lines.Count -eq 0) { throw 'Cannot list installed Rust toolchains.' }
    foreach ($line in $lines) {
        if ($line -cnotmatch '^([a-zA-Z0-9_.-]+)(?: \([a-z, ]+\))?$') { throw 'Unexpected Rust toolchain list entry.' }
        $Matches[1]
    }
}
$installed = @(Read-Toolchains)
if ($pinned -cnotin $installed) { throw 'Pinned Rust toolchain must be installed before cache preparation.' }
foreach ($toolchain in $installed) {
    if ($toolchain -ceq $pinned) { continue }
    rustup toolchain uninstall $toolchain
    if ($LASTEXITCODE -ne 0) { throw "Cannot remove unused runner toolchain: $toolchain" }
}
$remaining = @(Read-Toolchains)
if ($remaining.Count -ne 1 -or $remaining[0] -cne $pinned) { throw 'Rust cache requires exactly the pinned toolchain.' }
Write-Output "Rust cache toolchain: $pinned"
