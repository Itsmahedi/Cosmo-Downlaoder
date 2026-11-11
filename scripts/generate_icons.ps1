Add-Type -AssemblyName System.Drawing

function Draw-RustGetIcon([int]$size, [string]$outPath) {
    $bmp = New-Object System.Drawing.Bitmap $size, $size
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality

    # Background rounded rect or circle with blue gradient
    $rect = New-Object System.Drawing.Rectangle 1, 1, ($size - 2), ($size - 2)
    $colorTop = [System.Drawing.Color]::FromArgb(37, 99, 235)    # #2563eb
    $colorBottom = [System.Drawing.Color]::FromArgb(29, 78, 216) # #1d4ed8
    $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush $rect, $colorTop, $colorBottom, 45.0
    
    # Fill rounded background
    $radius = [Math]::Max(2, [int]($size * 0.2))
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $radius * 2
    $path.AddArc($rect.X, $rect.Y, $d, $d, 180, 90)
    $path.AddArc(($rect.Right - $d), $rect.Y, $d, $d, 270, 90)
    $path.AddArc(($rect.Right - $d), ($rect.Bottom - $d), $d, $d, 0, 90)
    $path.AddArc($rect.X, ($rect.Bottom - $d), $d, $d, 90, 90)
    $path.CloseFigure()
    
    $g.FillPath($brush, $path)

    # Draw download arrow pointing down
    $whitePen = New-Object System.Drawing.Pen ([System.Drawing.Color]::White), ([Math]::Max(1.5, $size * 0.1))
    $whitePen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $whitePen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round

    $cx = $size / 2.0
    $arrowTop = $size * 0.22
    $arrowTip = $size * 0.58
    $arrowWingY = $size * 0.44
    $wingW = $size * 0.22

    # Vertical stem
    $g.DrawLine($whitePen, [float]$cx, [float]$arrowTop, [float]$cx, [float]$arrowTip)

    # Arrowhead wings
    $g.DrawLine($whitePen, [float]($cx - $wingW), [float]$arrowWingY, [float]$cx, [float]$arrowTip)
    $g.DrawLine($whitePen, [float]($cx + $wingW), [float]$arrowWingY, [float]$cx, [float]$arrowTip)

    # Bottom tray line
    $trayY = $size * 0.74
    $trayLeft = $size * 0.22
    $trayRight = $size * 0.78
    $trayLip = $size * 0.12

    # Draw tray: left lip up, across, right lip up
    $trayPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::White), ([Math]::Max(1.5, $size * 0.09))
    $trayPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $trayPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $g.DrawLine($trayPen, [float]$trayLeft, [float]($trayY - $trayLip), [float]$trayLeft, [float]$trayY)
    $g.DrawLine($trayPen, [float]$trayLeft, [float]$trayY, [float]$trayRight, [float]$trayY)
    $g.DrawLine($trayPen, [float]$trayRight, [float]$trayY, [float]$trayRight, [float]($trayY - $trayLip))

    if ($outPath.EndsWith(".png")) {
        $bmp.Save($outPath, [System.Drawing.Imaging.ImageFormat]::Png)
    } elseif ($outPath.EndsWith(".ico")) {
        $hIcon = $bmp.GetHicon()
        $icon = [System.Drawing.Icon]::FromHandle($hIcon)
        $fs = [System.IO.File]::OpenWrite($outPath)
        $icon.Save($fs)
        $fs.Close()
        $icon.Dispose()
    }

    $whitePen.Dispose()
    $trayPen.Dispose()
    $path.Dispose()
    $brush.Dispose()
    $g.Dispose()
    $bmp.Dispose()
    Write-Host "Generated: $outPath ($size x $size)"
}

Draw-RustGetIcon 16 "extension/icons/icon16.png"
Draw-RustGetIcon 32 "extension/icons/icon32.png"
Draw-RustGetIcon 48 "extension/icons/icon48.png"
Draw-RustGetIcon 128 "extension/icons/icon128.png"
Draw-RustGetIcon 64 "installer/icon.ico"
