Add-Type -AssemblyName System.Drawing

$ico = New-Object System.Drawing.Icon("c:\Users\steam\RustGet-Source\crates\app\icon.ico")
Write-Host "Icon size:" $ico.Width "x" $ico.Height

$bmp = $ico.ToBitmap()
$bmp.Save("c:\Users\steam\RustGet-Source\crates\app\icon_ico_extracted.png", [System.Drawing.Imaging.ImageFormat]::Png)

# Check average pixel color in the center
$centerColor = $bmp.GetPixel(16, 16)
Write-Host "Center color: R=" $centerColor.R "G=" $centerColor.G "B=" $centerColor.B "A=" $centerColor.A

$cornerColor = $bmp.GetPixel(4, 4)
Write-Host "Corner color: R=" $cornerColor.R "G=" $cornerColor.G "B=" $cornerColor.B "A=" $cornerColor.A
