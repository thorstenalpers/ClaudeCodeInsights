# Puts the sherpa-onnx DLLs where Tauri and the test binaries expect them.
#
# They are not in the repository. The `sherpa-onnx-sys` build script downloads
# them into `<target>/sherpa-onnx-prebuilt`; `tauri.conf.json` bundles
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

# Asked, not assumed: CARGO_TARGET_DIR or a config file can move it.
$target = (cargo metadata --manifest-path $manifest --format-version 1 --no-deps | ConvertFrom-Json).target_directory
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host "Cargo target directory: $target"

function Find-Dlls {
    $dirs = @((Join-Path $target 'sherpa-onnx-prebuilt'), (Join-Path $target $Profile)) | Where-Object { Test-Path $_ }
    if (-not $dirs) { return @() }
    Get-ChildItem $dirs -Recurse -Filter '*.dll' |
        Where-Object { $_.Name -match '^(onnxruntime|sherpa-onnx)' -and $_.FullName -notmatch '\\(deps|examples)\\' } |
        Sort-Object Name -Unique
}

$cargoArgs = @('build', '--manifest-path', $manifest, '-p', 'sherpa-onnx-sys')
if ($Profile -eq 'release') { $cargoArgs += '--release' }
cargo @cargoArgs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$dlls = Find-Dlls
if (-not $dlls) {
    # A cached build script that does not rerun leaves the download directory empty.
    cargo clean --manifest-path $manifest -p sherpa-onnx-sys
    cargo @cargoArgs -vv
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $dlls = Find-Dlls
}
if (-not $dlls) {
    Get-ChildItem $target -Recurse -Filter '*.dll' -ErrorAction SilentlyContinue | Select-Object -First 20 FullName | Format-Table -AutoSize | Out-String | Write-Host
    throw "No sherpa-onnx DLLs under $target"
}

$runtime = Join-Path $tauri 'runtime'
New-Item -ItemType Directory -Force -Path $runtime | Out-Null
$dlls | Copy-Item -Destination $runtime -Force
Write-Host "Staged: $(($dlls.Name) -join ', ')"

if ($Profile -eq 'debug') {
    $deps = Join-Path $target 'debug\deps'
    New-Item -ItemType Directory -Force -Path $deps | Out-Null
    $dlls | Copy-Item -Destination $deps -Force
}
