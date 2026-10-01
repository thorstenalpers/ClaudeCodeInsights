# Runs the inspection tests in `src-tauri/tests/inspect.rs`.
#
# They are `#[ignore]` because they read this machine's own database,
# transcripts and log file, so a plain `cargo test` skips them; this passes the
# two flags that turn them on and let them print.
#
# The target directory is the one `verify.ps1` uses, for the same reason and
# with the same DLL copy: the app is usually open while someone is reading its
# log, and a running app holds the default target directory.
param(
	# A test name, or part of one. Empty runs all five.
	[string]$Test = '',
	# The session the detail test opens; without it, the newest one.
	[string]$Session = ''
)

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $root 'src-tauri\Cargo.toml'
$env:CARGO_TARGET_DIR = Join-Path $root 'src-tauri\target\verify'
if ($Session) { $env:SESSION = $Session }

$deps = Join-Path $env:CARGO_TARGET_DIR 'debug\deps'
New-Item -ItemType Directory -Force -Path $deps | Out-Null
Get-ChildItem (Join-Path $root 'src-tauri\runtime\*.dll') | Copy-Item -Destination $deps -Force

cargo test --manifest-path $manifest --test inspect -- --ignored --nocapture $Test
exit $LASTEXITCODE
