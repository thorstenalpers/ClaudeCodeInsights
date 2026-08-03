# Runs the Rust checks in a target directory of their own.
#
# The point is a build that cannot collide with a running app. Cargo takes an
# exclusive lock on its target directory, and `tauri-build` copies the sherpa
# DLLs beside the binary on every build — a `npm run start` session holds both,
# so a check running next to it either waits for the lock or dies on a locked
# DLL. A separate directory shares neither. It costs one full compile and a few
# gigabytes; it buys the ability to check while the app is open.
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $root 'src-tauri\Cargo.toml'
$env:CARGO_TARGET_DIR = Join-Path $root 'src-tauri\target\verify'

# The test binary loads onnxruntime.dll from beside itself; without this it
# finds whatever older copy the machine has on PATH and dies at the first call.
$deps = Join-Path $env:CARGO_TARGET_DIR 'debug\deps'
New-Item -ItemType Directory -Force -Path $deps | Out-Null
Get-ChildItem (Join-Path $root 'src-tauri\runtime\*.dll') | Copy-Item -Destination $deps -Force

cargo fmt --manifest-path $manifest
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

cargo clippy --manifest-path $manifest --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# Copied again: the first clippy run creates the directory this needs.
Get-ChildItem (Join-Path $root 'src-tauri\runtime\*.dll') | Copy-Item -Destination $deps -Force

# Only the library: `src/main.rs` holds no tests, and running its harness means
# launching a freshly built, unsigned executable — which Smart App Control on
# this machine blocks outright ("An Application Control policy has blocked this
# file", os error 4551).
cargo test --manifest-path $manifest --lib
exit $LASTEXITCODE
