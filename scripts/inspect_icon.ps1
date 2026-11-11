Add-Type -AssemblyName System.Drawing

# Check icon_32.rgba
$bytes = [System.IO.File]::ReadAllBytes("c:\Users\steam\RustGet-Source\crates\app\src\icon_32.rgba")
$bmp = New-Object System.Drawing.Bitmap 32, 32, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$rect = New-Object System.Drawing.Rectangle 0, 0, 32, 32
$bmpData = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::WriteOnly, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)

# Note: GDI+ Format32bppArgb expects BGRA in memory on little-endian x86
$bgra = New-Object byte[] 4096
for ($i = 0; $i -lt 4096; $i += 4) {
    $r = $bytes[$i]
    $g = $bytes[$i+1]
    $b = $bytes[$i+2]
    $a = $bytes[$i+3]
    $bgra[$i] = $b
    $bgra[$i+1] = $g
    $bgra[$i+2] = $r
    $bgra[$i+3] = $a
}
[System.Runtime.InteropServices.Marshal]::Copy($bgra, 0, $bmpData.Scan0, 4096)
$bmp.UnlockBits($bmpData)

$bmp.Save("c:\Users\steam\RustGet-Source\crates\app\src\icon_32_test.png", [System.Drawing.Imaging.ImageFormat]::Png)
Write-Host "Saved icon_32_test.png"
