# Puts the sherpa-onnx DLLs where Tauri and the test binaries expect them.
#
# They are not in the repository. The `sherpa-onnx-sys` build script downloads
# them into the Cargo profile directory; `tauri.conf.json` bundles
# `src-tauri/runtime/*.dll`, and a test binary loads them from beside itself.
# On a fresh checkout both places are empty until this has run.
param(
    [ValidateSet('debug', 'release')]
    [string]$Profile = 'debug'
)
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$tauri = Join-Path $root 'src-tauri'
$profileDir = Join-Path $tauri "target\$Profile"

$cargoArgs = @('build', '--manifest-path', (Join-Path $tauri 'Cargo.toml'), '-p', 'sherpa-onnx-sys')
if ($Profile -eq 'release') { $cargoArgs += '--release' }
cargo @cargoArgs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$dlls = Get-ChildItem (Join-Path $profileDir '*.dll') | Where-Object { $_.Name -match '^(onnxruntime|sherpa-onnx)' }
if (-not $dlls) { throw "No sherpa-onnx DLLs in $profileDir" }

$runtime = Join-Path $tauri 'runtime'
New-Item -ItemType Directory -Force -Path $runtime | Out-Null
$dlls | Copy-Item -Destination $runtime -Force

if ($Profile -eq 'debug') {
    $deps = Join-Path $profileDir 'deps'
    New-Item -ItemType Directory -Force -Path $deps | Out-Null
    $dlls | Copy-Item -Destination $deps -Force
}
