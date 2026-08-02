$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$source = Join-Path $root 'src-tauri\target\debug\claude-admin.exe'
$target = Join-Path $root 'bin'

New-Item -ItemType Directory -Force -Path $target | Out-Null
Copy-Item $source $target -Force

Write-Host "Copied to $(Join-Path $target 'claude-admin.exe')"
