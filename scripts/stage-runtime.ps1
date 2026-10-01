# Puts the sherpa-onnx DLLs where Tauri and the test binaries expect them.
#
# They are not in the repository. The `sherpa-onnx-sys` build script downloads
# them into `target/sherpa-onnx-prebuilt`; `tauri.conf.json` bundles
# `src-tauri/runtime/*.dll`, and a test binary loads them from beside itself.
# On a fresh checkout both places are empty until this has run.
param(
    [ValidateSet('debug', 'release')]
    [string]$Profile = 'debug'
)
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$tauri = Join-Path $root 'src-tauri'
$manifest = Join-Path $tauri 'Cargo.toml'
$prebuilt = Join-Path $tauri 'target\sherpa-onnx-prebuilt'

function Find-Dlls {
    if (-not (Test-Path $prebuilt)) { return @() }
    Get-ChildItem $prebuilt -Recurse -Filter '*.dll' | Where-Object { $_.Name -match '^(onnxruntime|sherpa-onnx)' }
}

$cargoArgs = @('build', '--manifest-path', $manifest, '-p', 'sherpa-onnx-sys')
if ($Profile -eq 'release') { $cargoArgs += '--release' }
cargo @cargoArgs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$dlls = Find-Dlls
if (-not $dlls) {
    # A cached build script that does not rerun leaves the download directory empty.
    cargo clean --manifest-path $manifest -p sherpa-onnx-sys
    cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $dlls = Find-Dlls
}
if (-not $dlls) { throw "No sherpa-onnx DLLs under $prebuilt" }

$runtime = Join-Path $tauri 'runtime'
New-Item -ItemType Directory -Force -Path $runtime | Out-Null
$dlls | Copy-Item -Destination $runtime -Force

if ($Profile -eq 'debug') {
    $deps = Join-Path $tauri 'target\debug\deps'
    New-Item -ItemType Directory -Force -Path $deps | Out-Null
    $dlls | Copy-Item -Destination $deps -Force
}
