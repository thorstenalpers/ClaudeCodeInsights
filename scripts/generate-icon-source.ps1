<#
.SYNOPSIS
    Draws the app mark as a 1024x1024 PNG and hands it to the Tauri icon pipeline.

.DESCRIPTION
    The mark: a shield with a sun behind it on a rounded plate — the app guards
    what Claude Code leaves behind, and the rays say the point is to see it.

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

$inkPen = New-Object System.Drawing.Pen($paper, [float](1.0 * $u))
$inkPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
$inkPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
$plateBrushAgain = New-Object System.Drawing.SolidBrush $ink

$cx = 16.0 * $u
$sunY = 10.2 * $u
$sunR = 4.6 * $u

# Rays first: the shield is painted over them, which is what keeps the two
# shapes apart instead of merging into a blot at small sizes.
# Only the rays above the horizon: the shield hides the lower ones anyway, and
# drawing them left orphaned dashes floating beside it.
foreach ($degrees in @(200, 235, 270, 305, 340)) {
    $angle = $degrees * [Math]::PI / 180.0
    $inner = $sunR + 1.5 * $u
    $outer = $sunR + 3.4 * $u
    $x1 = $cx + [Math]::Cos($angle) * $inner
    $y1 = $sunY + [Math]::Sin($angle) * $inner
    $x2 = $cx + [Math]::Cos($angle) * $outer
    $y2 = $sunY + [Math]::Sin($angle) * $outer
    $g.DrawLine($inkPen, [float]$x1, [float]$y1, [float]$x2, [float]$y2)
}

$g.DrawEllipse($inkPen, [float]($cx - $sunR), [float]($sunY - $sunR), [float]($sunR * 2), [float]($sunR * 2))

# The shield, filled in the plate colour so it masks the rays behind it.
$shield = New-Object System.Drawing.Drawing2D.GraphicsPath
$top = 13.0 * $u
$halfW = 7.8 * $u
$shoulder = 19.0 * $u
$tip = 27.0 * $u
$shield.AddLine([float]($cx - $halfW), [float]$top, [float]($cx + $halfW), [float]$top)
$shield.AddBezier(
    [float]($cx + $halfW), [float]$top,
    [float]($cx + $halfW), [float]$shoulder,
    [float]($cx + $halfW * 0.75), [float](($shoulder + $tip) / 2),
    [float]$cx, [float]$tip)
$shield.AddBezier(
    [float]$cx, [float]$tip,
    [float]($cx - $halfW * 0.75), [float](($shoulder + $tip) / 2),
    [float]($cx - $halfW), [float]$shoulder,
    [float]($cx - $halfW), [float]$top)
$shield.CloseFigure()

$g.FillPath($plateBrushAgain, $shield)
$g.DrawPath($inkPen, $shield)

$shield.Dispose(); $inkPen.Dispose(); $plateBrushAgain.Dispose()

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
