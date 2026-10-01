<#
.SYNOPSIS
    Reports which build prerequisites are present and which are missing.

.DESCRIPTION
    Needs no elevation and changes nothing.

    The MSVC checks look at more than "is a file somewhere on disk": Rust locates
    link.exe through the Visual Studio Installer's component registration, so a
    link.exe that exists but belongs to an unregistered component is invisible to
    it. The Windows SDK is checked by looking for the import libraries rather
    than the package registration, because winget can report the SDK as installed
    while Windows Kits\10\Lib does not exist at all. Both of those were the real
    state of a machine this project was set up on.

.PARAMETER Probe
    Also compile and link a throwaway crate. A green checklist is not proof;
    only a real build shows whether the linker and the SDK import libraries are
    actually reachable.

.EXAMPLE
    pwsh ./scripts/verify-dev-machine.ps1

.EXAMPLE
    pwsh ./scripts/verify-dev-machine.ps1 -Probe
#>

[CmdletBinding()]
param(
    [switch]$Probe
)

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

    Write-Host $(if ($ok) { '  OK ' } else { 'MISS ' }) -ForegroundColor $(if ($ok) { 'Green' } else { 'Red' }) -NoNewline
    Write-Host ('{0,-24}' -f $Name) -NoNewline
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

Test-Item 'Node.js' { node --version 2>$null } 'winget install OpenJS.NodeJS.LTS'
Test-Item 'npm' { npm --version 2>$null } 'ships with Node.js'
Test-Item 'rustup' { (rustup --version 2>$null) -replace '\s+$', '' } 'winget install Rustlang.Rustup'
Test-Item 'cargo' { cargo --version 2>$null } 'comes with rustup'
Test-Item 'msvc toolchain' {
    $t = rustup toolchain list 2>$null | Where-Object { $_ -match 'pc-windows-msvc' -and $_ -match 'default' }
    if ($t) { ($t -split ' ')[0] }
} 'rustup default stable-msvc'

Test-Item 'MSVC C++ tools' {
    if (Test-Path $vswhere) {
        if (& $vswhere -all -prerelease -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath) { 'registered' }
    }
} 'scripts/setup-dev-machine.ps1'

Test-Item 'Windows SDK libs' {
    $root = "${env:ProgramFiles(x86)}\Windows Kits\10\Lib"
    if (Test-Path $root) {
        $hit = Get-ChildItem $root -Directory -ErrorAction SilentlyContinue |
            Where-Object { Test-Path (Join-Path $_.FullName 'um\x64\kernel32.lib') } |
            Select-Object -Last 1
        if ($hit) { $hit.Name }
    }
} 'scripts/setup-dev-machine.ps1'

Test-Item 'Visual Studio state' {
    if (Test-Path $vswhere) {
        $vs = & $vswhere -all -prerelease -format json | ConvertFrom-Json | Select-Object -First 1
        if ($vs -and $vs.isComplete -and $vs.isLaunchable) { "$($vs.displayName), complete" }
    }
} 'open the Visual Studio Installer and choose Repair'

Test-Item 'WebView2 runtime' {
    $k = 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
    if (Test-Path $k) { (Get-ItemProperty $k).pv }
} 'preinstalled on Windows 11; Tauri can also bundle the bootstrapper'

if ($Probe) {
    Write-Host ''
    Write-Host 'Link probe' -ForegroundColor Cyan
    Write-Host ''
    $dir = Join-Path ([IO.Path]::GetTempPath()) ('linkprobe-' + [Guid]::NewGuid().ToString('N').Substring(0, 8))
    try {
        cargo new $dir --name linkprobe --quiet 2>&1 | Out-Null
        Push-Location $dir
        $output = cargo build 2>&1
        $ok = $LASTEXITCODE -eq 0
        Pop-Location

        if ($ok) {
            Write-Host '  OK  a real binary compiled, linked and is runnable' -ForegroundColor Green
        }
        else {
            Write-Host 'FAIL  cargo could not link:' -ForegroundColor Red
            $output | Select-Object -Last 12 | ForEach-Object { Write-Host "      $_" }
            $script:Failures++
        }
    }
    finally {
        Remove-Item -Recurse -Force $dir -ErrorAction SilentlyContinue
    }
}

Write-Host ''
if ($script:Failures -eq 0) {
    Write-Host 'All prerequisites present.' -ForegroundColor Green
    if (-not $Probe) {
        Write-Host 'Re-run with -Probe to prove that linking actually works.' -ForegroundColor DarkGray
    }
}
else {
    Write-Host "$script:Failures check(s) failed. Run scripts/setup-dev-machine.ps1 as administrator." -ForegroundColor Yellow
}
Write-Host ''

exit $script:Failures
