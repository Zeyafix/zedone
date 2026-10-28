#Requires -Version 7.0
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (-not $IsWindows -or [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -ne 'X64') {
    throw 'This check requires Windows x64.'
}

function Invoke-Checked {
    param([string]$Command, [string[]]$Arguments)
    Write-Host "> $Command $($Arguments -join ' ')"
    $output = & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Command failed with exit code $LASTEXITCODE."
    }
    return $output
}

$versions = Get-Content -Raw (Join-Path $PSScriptRoot 'environment-versions.json') | ConvertFrom-Json
$toolchain = "$($versions.rust)-x86_64-pc-windows-msvc"
Write-Host "Windows $([Environment]::OSVersion.Version); PowerShell $($PSVersionTable.PSVersion)"
Invoke-Checked git @('--version')
Invoke-Checked rustup @('--version')
Invoke-Checked rustc @("+$toolchain", '-vV')
Invoke-Checked cargo @("+$toolchain", '--version')
Invoke-Checked rustfmt @("+$toolchain", '--version')
Invoke-Checked cargo @("+$toolchain", 'clippy', '--version')
$bunVersion = Invoke-Checked bun @('--version')
if ("$bunVersion".Trim() -ne $versions.bun) {
    throw "Expected Bun $($versions.bun), got $bunVersion."
}
Write-Host "Bun $bunVersion"
Invoke-Checked bun @('-e', 'console.log("Bun runtime OK")')

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$vsVersion = Invoke-Checked $vswhere @('-latest', '-products', '*', '-requires', 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64', '-property', 'installationVersion')
if (-not $vsVersion) { throw 'Visual Studio C++ tools were not found.' }
Write-Host "Visual Studio $vsVersion"
$sdkRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/Lib'
Write-Host "Windows SDK libraries: $((Get-ChildItem -LiteralPath $sdkRoot -Directory).Name -join ', ')"

# Each invocation owns one disposable directory, including on failure.
$tempRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../temp'))
$probe = Join-Path $tempRoot "windows-environment-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $probe -Force | Out-Null
try {
    Invoke-Checked cargo @("+$toolchain", 'init', '--bin', '--vcs', 'none', '--name', 'zedone_environment_probe', $probe)
    $manifest = Join-Path $probe 'Cargo.toml'
    Invoke-Checked cargo @("+$toolchain", 'build', '--manifest-path', $manifest, '--offline')
    $result = Invoke-Checked cargo @("+$toolchain", 'run', '--manifest-path', $manifest, '--offline')
    if ("$result".Trim() -ne 'Hello, world!') { throw 'Unexpected Rust probe output.' }
    Write-Host $result
    Invoke-Checked cargo @("+$toolchain", 'fmt', '--manifest-path', $manifest, '--check')
    Invoke-Checked cargo @("+$toolchain", 'clippy', '--manifest-path', $manifest, '--offline', '--', '-D', 'warnings')
}
finally {
    $resolvedProbe = [IO.Path]::GetFullPath($probe)
    if ([IO.Path]::GetDirectoryName($resolvedProbe) -ne $tempRoot) {
        throw 'Refusing to remove a directory outside the probe root.'
    }
    Remove-Item -LiteralPath $resolvedProbe -Recurse -Force
}
Write-Host 'Windows environment check passed.'
