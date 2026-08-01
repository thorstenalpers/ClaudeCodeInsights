<#
.SYNOPSIS
    Draws the app mark as a 1024x1024 PNG and hands it to the Tauri icon pipeline.

.DESCRIPTION
    The mark: three ascending bars (usage over time) on a rounded plate, with a
    notch through the tallest bar for a session boundary.

    Only one source image is produced here — `tauri icon` derives every size and
    format (.ico, .icns, the Windows Store logos) from it.

.EXAMPLE
    pwsh ./scripts/generate-icon-source.ps1
#>

[CmdletBinding()]
param(
    [int]$Size = 1024,
    [switch]$SkipTauriIcon
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$repoRoot = Split-Path $PSScriptRoot -Parent
$source = Join-Path $repoRoot 'src-tauri/icons/source.png'

$bmp = New-Object System.Drawing.Bitmap($Size, $Size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.Clear([System.Drawing.Color]::Transparent)

$u = $Size / 32.0            # design grid is 32x32
$radius = 7.0 * $u

$plate = New-Object System.Drawing.Drawing2D.GraphicsPath
$d = $radius * 2
$plate.AddArc(0, 0, $d, $d, 180, 90)
$plate.AddArc($Size - $d, 0, $d, $d, 270, 90)
$plate.AddArc($Size - $d, $Size - $d, $d, $d, 0, 90)
$plate.AddArc(0, $Size - $d, $d, $d, 90, 90)
$plate.CloseFigure()

$ink = [System.Drawing.Color]::FromArgb(255, 24, 24, 27)
$paper = [System.Drawing.Color]::FromArgb(255, 250, 250, 250)

$plateBrush = New-Object System.Drawing.SolidBrush $ink
$g.FillPath($plateBrush, $plate)

$barBrush = New-Object System.Drawing.SolidBrush $paper
$notchBrush = New-Object System.Drawing.SolidBrush $ink

$barW = 4.0 * $u
$gap = 3.0 * $u
$left = 7.0 * $u
$bottom = 25.0 * $u
$heights = @(7.0, 11.0, 16.0)

for ($i = 0; $i -lt 3; $i++) {
    $h = $heights[$i] * $u
    $x = $left + $i * ($barW + $gap)
    $g.FillRectangle($barBrush, $x, $bottom - $h, $barW, $h)
}

$x = $left + 2 * ($barW + $gap)
$g.FillRectangle($notchBrush, $x, $bottom - (10.0 * $u), $barW, 1.5 * $u)

$g.Dispose(); $plate.Dispose(); $plateBrush.Dispose(); $barBrush.Dispose(); $notchBrush.Dispose()

New-Item -ItemType Directory -Force -Path (Split-Path $source -Parent) | Out-Null
$bmp.Save($source, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()

Write-Output "wrote $source ($((Get-Item $source).Length) bytes, ${Size}x${Size})"

if (-not $SkipTauriIcon) {
    Push-Location $repoRoot
    try {
        npx --yes @tauri-apps/cli@latest icon $source
    }
    finally {
        Pop-Location
    }
}
