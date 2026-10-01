<#
.SYNOPSIS
    Draws the app mark as a 1024x1024 PNG and hands it to the Tauri icon pipeline.

.DESCRIPTION
    The mark: an eight-pointed burst on a rounded plate. Seven spokes reach the
    same distance and one stops short — the app is about looking at what a model
    left behind, and the odd spoke is the reading that stands out.

    Black and white only, so the same source works on a light or a dark plate.

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

$burstBrush = New-Object System.Drawing.SolidBrush $paper

$cx = 16.0 * $u
$cy = 16.0 * $u
$inner = 2.9 * $u
$half = 2.7 * $u
$long = 13.4 * $u
# The short spoke is two thirds rather than the half it started at: at 32 px a
# half-length spoke reads as a rendering fault, at two thirds it reads as a
# shorter bar.
$short = 9.0 * $u

for ($i = 0; $i -lt 8; $i++) {
    $angle = ($i * 45.0 - 90.0) * [Math]::PI / 180.0
    $outer = if ($i -eq 3) { $short } else { $long }

    $nx = [Math]::Cos($angle)
    $ny = [Math]::Sin($angle)
    $px = -$ny
    $py = $nx

    # A wedge rather than a stroke: it keeps its taper when the whole mark is
    # scaled down, which a round-capped line does not.
    $spoke = New-Object System.Drawing.Drawing2D.GraphicsPath
    $spoke.AddPolygon(@(
        (New-Object System.Drawing.PointF([float]($cx + $nx * $inner + $px * $half), [float]($cy + $ny * $inner + $py * $half))),
        (New-Object System.Drawing.PointF([float]($cx + $nx * $inner - $px * $half), [float]($cy + $ny * $inner - $py * $half))),
        (New-Object System.Drawing.PointF([float]($cx + $nx * $outer - $px * $half * 0.34), [float]($cy + $ny * $outer - $py * $half * 0.34))),
        (New-Object System.Drawing.PointF([float]($cx + $nx * $outer + $px * $half * 0.34), [float]($cy + $ny * $outer + $py * $half * 0.34)))
    ))
    $g.FillPath($burstBrush, $spoke)
    $spoke.Dispose()
}

# The eye of the burst, cut back out of the plate so the centre stays open at
# every size instead of filling in.
$eye = 2.0 * $u
$eyeBrush = New-Object System.Drawing.SolidBrush $ink
$g.FillEllipse($eyeBrush, [float]($cx - $eye), [float]($cy - $eye), [float]($eye * 2), [float]($eye * 2))

$eyeBrush.Dispose(); $burstBrush.Dispose()
$g.Dispose(); $plate.Dispose(); $plateBrush.Dispose()

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
