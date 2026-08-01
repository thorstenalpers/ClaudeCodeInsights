<#
.SYNOPSIS
    Reports which build prerequisites are present and which are missing.

.DESCRIPTION
    Needs no elevation and changes nothing. Run it before setup-dev-machine.ps1
    and again afterwards.

    The MSVC checks look at more than "is a file somewhere on disk": Rust locates
    link.exe through the Visual Studio Installer's component registration, so a
    link.exe that exists but belongs to an unregistered component is invisible to
    it — which is exactly the state this machine was in.

.EXAMPLE
    pwsh ./scripts/verify-dev-machine.ps1
#>

[CmdletBinding()]
param()

$script:Failures = 0

function Test-Item {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][scriptblock]$Check,
        [string]$Fix
    )
    $detail = $null
    try { $detail = & $Check } catch { $detail = $null }
    $ok = [bool]$detail

    $mark = if ($ok) { '  OK ' } else { 'MISS ' }
    $colour = if ($ok) { 'Green' } else { 'Red' }
    Write-Host $mark -ForegroundColor $colour -NoNewline
    Write-Host ('{0,-26}' -f $Name) -NoNewline
    Write-Host $(if ($ok) { $detail } else { 'not found' })

    if (-not $ok) {
        $script:Failures++
        if ($Fix) { Write-Host ('      -> ' + $Fix) -ForegroundColor DarkGray }
    }
}

$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"

Write-Host ''
Write-Host 'Build prerequisites' -ForegroundColor Cyan
Write-Host ''

Test-Item 'Node.js' { (node --version 2>$null) } 'https://nodejs.org (LTS)'
Test-Item 'npm' { (npm --version 2>$null) } 'ships with Node.js'

Test-Item 'rustup' { (rustup --version 2>$null) -replace '\s+$', '' } 'winget install Rustlang.Rustup'
Test-Item 'cargo' { (cargo --version 2>$null) } 'comes with rustup'
Test-Item 'msvc toolchain' {
    $t = rustup toolchain list 2>$null | Where-Object { $_ -match 'pc-windows-msvc' -and $_ -match 'default' }
    if ($t) { ($t -split ' ')[0] }
} 'rustup default stable-msvc'

# Rust finds the linker via the installer registration, not by scanning disk.
Test-Item 'MSVC C++ tools' {
    if (Test-Path $vswhere) {
        $p = & $vswhere -all -prerelease -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($p) { 'registered' }
    }
} 'setup-dev-machine.ps1 (adds Microsoft.VisualStudio.Component.VC.Tools.x86.x64)'

# The libraries Rust links against. Absent here even when winget reports the
# Windows SDK as installed, which is why this checks the files and not the
# package registration.
Test-Item 'Windows SDK libs' {
    $root = "${env:ProgramFiles(x86)}\Windows Kits\10\Lib"
    if (Test-Path $root) {
        $hit = Get-ChildItem $root -Directory -ErrorAction SilentlyContinue |
            Where-Object { Test-Path (Join-Path $_.FullName 'um\x64\kernel32.lib') } |
            Select-Object -Last 1
        if ($hit) { $hit.Name }
    }
} 'setup-dev-machine.ps1 (adds Microsoft.VisualStudio.Component.Windows11SDK.26100)'

Test-Item 'WebView2 runtime' {
    $k = 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
    if (Test-Path $k) { (Get-ItemProperty $k).pv }
} 'preinstalled on Windows 11; otherwise https://developer.microsoft.com/microsoft-edge/webview2/'

Write-Host ''
if ($script:Failures -eq 0) {
    Write-Host 'All prerequisites present.' -ForegroundColor Green
    Write-Host 'A green list is not proof that linking works — run the link probe:' -ForegroundColor DarkGray
    Write-Host '    pwsh ./scripts/verify-dev-machine.ps1 -Probe' -ForegroundColor DarkGray
}
else {
    Write-Host ("$script:Failures prerequisite(s) missing. Run scripts/setup-dev-machine.ps1 as administrator.") -ForegroundColor Yellow
}
Write-Host ''

exit $script:Failures
