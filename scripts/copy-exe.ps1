$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$build = Join-Path $root 'src-tauri\target\debug'
$target = Join-Path $root 'bin'

New-Item -ItemType Directory -Force -Path $target | Out-Null
Copy-Item (Join-Path $build 'claude-admin.exe') $target -Force

# The speech engine is linked, not loaded on demand: Windows looks for these
# beside the exe before main runs, so a lone exe would not start at all.
Get-ChildItem (Join-Path $root 'src-tauri\runtime\*.dll') | Copy-Item -Destination $target -Force

Write-Host "Copied to $(Join-Path $target 'claude-admin.exe')"
