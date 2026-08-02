<#
.SYNOPSIS
    Draws the app mark as a 1024x1024 PNG and hands it to the Tauri icon pipeline.

.DESCRIPTION
    The mark: three sliders on a rounded plate — the app administers what Claude
    Code stores rather than only charting it, and a control surface says that.

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

$trackBrush = New-Object System.Drawing.SolidBrush $paper
$knobBrush = New-Object System.Drawing.SolidBrush $paper
$knobRing = New-Object System.Drawing.Pen($ink, [float](1.6 * $u))

$left = 7.0 * $u
$right = 25.0 * $u
$track = 1.6 * $u
$knob = 4.4 * $u

# Rows at 11/16/21 and knobs at 60/75/40 percent: three settings that were
# clearly set by hand, rather than a pattern that reads as a logo of nothing.
$rows = @(11.0, 16.0, 21.0)
$knobAt = @(0.62, 0.78, 0.38)

for ($i = 0; $i -lt 3; $i++) {
    $y = $rows[$i] * $u
    $g.FillRectangle($trackBrush, $left, $y - $track / 2, $right - $left, $track)

    $cx = $left + ($right - $left) * $knobAt[$i]
    $g.FillEllipse($knobBrush, $cx - $knob / 2, $y - $knob / 2, $knob, $knob)
    $g.DrawEllipse($knobRing, $cx - $knob / 2, $y - $knob / 2, $knob, $knob)
}

$g.Dispose(); $plate.Dispose(); $plateBrush.Dispose()
$trackBrush.Dispose(); $knobBrush.Dispose(); $knobRing.Dispose()

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
