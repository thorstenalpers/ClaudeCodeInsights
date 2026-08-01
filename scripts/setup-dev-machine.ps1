<#
.SYNOPSIS
    Installs everything needed to build this project on a fresh Windows machine.

.DESCRIPTION
    Idempotent: every step checks first and skips what is already there, so it is
    safe to re-run after an interrupted install.

    Needs administrator rights for the Visual Studio step. Run from an elevated
    PowerShell, or the script will tell you and stop.

    Why the Visual Studio step exists at all: Rust's default Windows target links
    with the MSVC linker and against the Windows SDK import libraries. Rust finds
    link.exe through the Visual Studio Installer's component registration — a
    link.exe that is on disk but belongs to an unregistered component does not
    count. Only two components are requested rather than the whole "Desktop
    development with C++" workload, which saves several GB of MFC, ATL, Clang and
    CMake that this project never touches.

.PARAMETER SkipVisualStudio
    Leave the Visual Studio installation alone. Use this if you install the C++
    components yourself through the Visual Studio Installer UI.

.EXAMPLE
    # from an elevated PowerShell, in the repository root
    pwsh ./scripts/setup-dev-machine.ps1

.EXAMPLE
    pwsh ./scripts/setup-dev-machine.ps1 -SkipVisualStudio
#>

[CmdletBinding()]
param(
    [switch]$SkipVisualStudio
)

$ErrorActionPreference = 'Stop'

function Write-Step([string]$Text) {
    Write-Host ''
    Write-Host "==> $Text" -ForegroundColor Cyan
}

function Test-Administrator {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    return ([Security.Principal.WindowsPrincipal]$id).IsInRole(
        [Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Update-SessionPath {
    $env:Path = [Environment]::GetEnvironmentVariable('Path', 'Machine') + ';' +
                [Environment]::GetEnvironmentVariable('Path', 'User')
}

# ---------------------------------------------------------------- Visual Studio

if (-not $SkipVisualStudio) {
    Write-Step 'MSVC C++ tools and Windows SDK'

    $installerDir = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer"
    $vswhere = Join-Path $installerDir 'vswhere.exe'
    $setup = Join-Path $installerDir 'setup.exe'

    if (-not (Test-Path $vswhere)) {
        throw "Visual Studio Installer not found. Install Visual Studio (any edition) or the standalone Build Tools first: winget install Microsoft.VisualStudio.2022.BuildTools"
    }

    $components = @(
        'Microsoft.VisualStudio.Component.VC.Tools.x86.x64'
        'Microsoft.VisualStudio.Component.Windows11SDK.26100'
    )

    $missing = @($components | Where-Object {
            -not (& $vswhere -all -prerelease -products * -requires $_ -property installationPath)
        })

    # The registration can claim the SDK component while the import libraries are
    # absent, so the files get their own check.
    $sdkLibPresent = $false
    $libRoot = "${env:ProgramFiles(x86)}\Windows Kits\10\Lib"
    if (Test-Path $libRoot) {
        $sdkLibPresent = [bool](Get-ChildItem $libRoot -Directory -ErrorAction SilentlyContinue |
                Where-Object { Test-Path (Join-Path $_.FullName 'um\x64\kernel32.lib') })
    }

    if ($missing.Count -eq 0 -and $sdkLibPresent) {
        Write-Host '    already present, skipping'
    }
    else {
        if (-not (Test-Administrator)) {
            throw 'This step needs administrator rights. Re-run this script from an elevated PowerShell, or pass -SkipVisualStudio and add the components through the Visual Studio Installer UI.'
        }

        $installPath = & $vswhere -latest -prerelease -property installationPath
        Write-Host "    modifying: $installPath"
        $components | ForEach-Object { Write-Host "      + $_" }

        $args = @('modify', '--installPath', $installPath)
        $components | ForEach-Object { $args += @('--add', $_) }
        $args += @('--quiet', '--norestart', '--wait')

        Write-Host '    this takes 5-15 minutes and downloads a few GB...'
        $proc = Start-Process -FilePath $setup -ArgumentList $args -Wait -PassThru
        if ($proc.ExitCode -ne 0 -and $proc.ExitCode -ne 3010) {
            throw "Visual Studio Installer exited with $($proc.ExitCode). Open the Visual Studio Installer UI to see what went wrong."
        }
        Write-Host '    done'
    }
}

# ----------------------------------------------------------------------- Rust

Write-Step 'Rust toolchain'

Update-SessionPath
if (Get-Command rustup -ErrorAction SilentlyContinue) {
    Write-Host "    already present: $(rustup --version)"
}
else {
    Write-Host '    installing rustup...'
    winget install --id Rustlang.Rustup --exact --silent `
        --accept-source-agreements --accept-package-agreements --disable-interactivity
    Update-SessionPath
}

if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    throw 'rustup is still not on PATH. Open a new shell and re-run this script.'
}

# The MSVC target is the one Tauri supports on Windows; the GNU target links
# without the Windows SDK but is not a supported Tauri configuration.
if (-not (rustup toolchain list | Where-Object { $_ -match 'pc-windows-msvc' -and $_ -match 'default' })) {
    Write-Host '    selecting the msvc toolchain...'
    rustup default stable-msvc
}
Write-Host "    $(rustc --version)"

# ----------------------------------------------------------------------- Node

Write-Step 'Node.js'

if (Get-Command node -ErrorAction SilentlyContinue) {
    Write-Host "    already present: $(node --version)"
}
else {
    Write-Host '    installing Node.js LTS...'
    winget install --id OpenJS.NodeJS.LTS --exact --silent `
        --accept-source-agreements --accept-package-agreements --disable-interactivity
    Update-SessionPath
}

# --------------------------------------------------------------------- Verify

Write-Step 'Verifying that Rust can actually link'

# A green checklist is not proof. Only a real compile shows whether the linker
# and the SDK import libraries are reachable, which is the failure this whole
# script exists to prevent.
$probe = Join-Path ([IO.Path]::GetTempPath()) ('linkprobe-' + [Guid]::NewGuid().ToString('N').Substring(0, 8))
try {
    cargo new $probe --name linkprobe --quiet 2>&1 | Out-Null
    Push-Location $probe
    $output = cargo build 2>&1
    $ok = $LASTEXITCODE -eq 0
    Pop-Location

    if ($ok) {
        Write-Host '    link probe compiled successfully' -ForegroundColor Green
    }
    else {
        Write-Host '    link probe FAILED:' -ForegroundColor Red
        $output | Select-Object -Last 12 | ForEach-Object { Write-Host "      $_" }
        throw 'Rust cannot link. Open the Visual Studio Installer and confirm the two components above are installed.'
    }
}
finally {
    Remove-Item -Recurse -Force $probe -ErrorAction SilentlyContinue
}

Write-Host ''
Write-Host 'Machine is ready.' -ForegroundColor Green
Write-Host 'Open a new shell so the updated PATH is picked up, then run:' -ForegroundColor DarkGray
Write-Host '    npm --prefix src/ClaudeUsageAnalyzer.UI install' -ForegroundColor DarkGray
Write-Host ''
