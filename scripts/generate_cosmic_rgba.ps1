Add-Type -AssemblyName System.Drawing

$srcPath = "c:\Users\steam\OneDrive\Documents\Antigravity\CosmoDownloader-v3.0\Browser-Extensions\Mozilla-Firefox\icons\icon-128.png"
$src = [System.Drawing.Bitmap]::FromFile($srcPath)

# Create 32x32 target
$dst = New-Object System.Drawing.Bitmap 32, 32, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$g = [System.Drawing.Graphics]::FromImage($dst)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
$g.Clear([System.Drawing.Color]::Transparent)
$g.DrawImage($src, 0, 0, 32, 32)
$g.Dispose()
$src.Dispose()

# Save PNG for inspection
$dst.Save("c:\Users\steam\RustGet-Source\crates\app\src\cosmic_32.png", [System.Drawing.Imaging.ImageFormat]::Png)

# Lock bits to extract RGBA
$rect = New-Object System.Drawing.Rectangle 0, 0, 32, 32
$bmpData = $dst.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)

# GDI+ Format32bppArgb stores BGRA in memory (B, G, R, A)
$bgra = New-Object byte[] 4096
[System.Runtime.InteropServices.Marshal]::Copy($bmpData.Scan0, $bgra, 0, 4096)
$dst.UnlockBits($bmpData)
$dst.Dispose()

# Convert BGRA to RGBA (R, G, B, A) for tray_icon and egui
$rgba = New-Object byte[] 4096
for ($i = 0; $i -lt 4096; $i += 4) {
    $b = $bgra[$i]
    $g = $bgra[$i+1]
    $r = $bgra[$i+2]
    $a = $bgra[$i+3]
    $rgba[$i] = $r
    $rgba[$i+1] = $g
    $rgba[$i+2] = $b
    $rgba[$i+3] = $a
}

[System.IO.File]::WriteAllBytes("c:\Users\steam\RustGet-Source\crates\app\src\icon_32.rgba", $rgba)
Write-Host "Generated pristine icon_32.rgba (4096 bytes)"
