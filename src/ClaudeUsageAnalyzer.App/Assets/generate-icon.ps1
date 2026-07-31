# Generates Assets/app.ico from a single vector description, so the icon is
# reproducible from source instead of being a checked-in binary nobody can edit.
#
#   pwsh ./generate-icon.ps1
#
# The mark: three ascending bars (usage over time) inside a rounded square,
# with the tallest bar cut by a gap — a session boundary.

Add-Type -AssemblyName System.Drawing

$sizes = @(16, 20, 24, 32, 48, 64, 128, 256)
$outIco = Join-Path $PSScriptRoot 'app.ico'

function New-IconBitmap([int]$px) {
    $bmp = New-Object System.Drawing.Bitmap($px, $px, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.Clear([System.Drawing.Color]::Transparent)

    $u = $px / 32.0                      # design grid is 32x32
    $radius = 7.0 * $u

    # Rounded square plate.
    $plate = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $radius * 2
    $plate.AddArc(0, 0, $d, $d, 180, 90)
    $plate.AddArc($px - $d, 0, $d, $d, 270, 90)
    $plate.AddArc($px - $d, $px - $d, $d, $d, 0, 90)
    $plate.AddArc(0, $px - $d, $d, $d, 90, 90)
    $plate.CloseFigure()
    $plateBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 24, 24, 27))
    $g.FillPath($plateBrush, $plate)

    # Three ascending bars.
    $barBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 250, 250, 250))
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

    # A notch through the tallest bar: at 16px this is the one detail that keeps
    # the mark from reading as a plain chart icon. Drop it when it would be
    # sub-pixel and only muddy the shape.
    if ($px -ge 32) {
        $notch = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 24, 24, 27))
        $x = $left + 2 * ($barW + $gap)
        $g.FillRectangle($notch, $x, $bottom - (10.0 * $u), $barW, 1.5 * $u)
        $notch.Dispose()
    }

    $g.Dispose(); $plate.Dispose(); $plateBrush.Dispose(); $barBrush.Dispose()
    return $bmp
}

# Sizes up to 64 are written as classic 32-bit DIB entries and only 128/256 as
# PNG. PNG-in-ICO is valid from Vista on, but GDI+ (System.Drawing) cannot
# decode it, and enough tooling still goes through GDI+ that an all-PNG icon
# fails in places that matter. Mixed is what real-world icons do.
function ConvertTo-DibEntry([System.Drawing.Bitmap]$bmp) {
    $w = $bmp.Width; $h = $bmp.Height
    $ms = New-Object System.IO.MemoryStream
    $bw = New-Object System.IO.BinaryWriter($ms)

    # BITMAPINFOHEADER, height doubled to cover the XOR image plus the AND mask.
    $bw.Write([uint32]40); $bw.Write([int32]$w); $bw.Write([int32]($h * 2))
    $bw.Write([uint16]1); $bw.Write([uint16]32); $bw.Write([uint32]0)
    $bw.Write([uint32]($w * $h * 4)); $bw.Write([int32]0); $bw.Write([int32]0)
    $bw.Write([uint32]0); $bw.Write([uint32]0)

    $rect = New-Object System.Drawing.Rectangle(0, 0, $w, $h)
    $data = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $stride = $data.Stride
    $buf = New-Object byte[] ($stride * $h)
    [System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $buf, 0, $buf.Length)
    $bmp.UnlockBits($data)

    # DIB rows run bottom-up.
    for ($y = $h - 1; $y -ge 0; $y--) { $bw.Write($buf, $y * $stride, $w * 4) }

    # AND mask: unused because the alpha channel carries transparency, but the
    # format still requires the rows, padded to 4 bytes.
    $maskStride = [int][math]::Ceiling($w / 32.0) * 4
    $zero = New-Object byte[] $maskStride
    for ($y = 0; $y -lt $h; $y++) { $bw.Write($zero, 0, $maskStride) }

    $bw.Flush()
    $bytes = $ms.ToArray()
    $bw.Dispose(); $ms.Dispose()

    # The comma matters: without it PowerShell unrolls the byte[] into the
    # pipeline and the caller gets an Object[], which binds BinaryWriter.Write
    # to the wrong overload and produces an unreadable file.
    return , $bytes
}

$entries = @()
foreach ($s in $sizes) {
    $bmp = New-IconBitmap $s
    if ($s -ge 128) {
        $ms = New-Object System.IO.MemoryStream
        $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
        $entries += , @{ Size = $s; Bytes = $ms.ToArray() }
        $ms.Dispose()
    }
    else {
        $entries += , @{ Size = $s; Bytes = (ConvertTo-DibEntry $bmp) }
    }
    $bmp.Dispose()
}

# ICO container: 6-byte header, one 16-byte directory entry per image, payloads after.
$out = New-Object System.IO.MemoryStream
$w = New-Object System.IO.BinaryWriter($out)
$w.Write([uint16]0); $w.Write([uint16]1); $w.Write([uint16]$entries.Count)

$offset = 6 + (16 * $entries.Count)
foreach ($p in $entries) {
    $dim = if ($p.Size -ge 256) { 0 } else { $p.Size }
    $w.Write([byte]$dim); $w.Write([byte]$dim)
    $w.Write([byte]0); $w.Write([byte]0)
    $w.Write([uint16]1); $w.Write([uint16]32)
    $w.Write([uint32]$p.Bytes.Length)
    $w.Write([uint32]$offset)
    $offset += $p.Bytes.Length
}
foreach ($p in $entries) { $w.Write([byte[]]$p.Bytes, 0, $p.Bytes.Length) }

$w.Flush()
[System.IO.File]::WriteAllBytes($outIco, $out.ToArray())
$w.Dispose(); $out.Dispose()

# Read every declared size back. An .ico that Windows cannot decode is worse
# than none, and the failure is silent everywhere it matters.
foreach ($s in $sizes) {
    if ($s -ge 128) { continue }   # GDI+ cannot decode the PNG entries
    $probe = New-Object System.Drawing.Icon($outIco, $s, $s)
    if ($probe.Width -ne $s) { throw "icon: asked for ${s}px, got $($probe.Width)px" }
    $probe.ToBitmap().Dispose()
    $probe.Dispose()
}

Write-Output "wrote $outIco ($((Get-Item $outIco).Length) bytes, $($entries.Count) sizes, DIB entries verified)"
