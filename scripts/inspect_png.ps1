Add-Type -AssemblyName System.Drawing

$png = [System.Drawing.Bitmap]::FromFile("c:\Users\steam\OneDrive\Documents\Antigravity\CosmoDownloader-v3.0\Browser-Extensions\Mozilla-Firefox\icons\icon-32.png")
$centerColor = $png.GetPixel(16, 16)
Write-Host "PNG Center: R=" $centerColor.R "G=" $centerColor.G "B=" $centerColor.B "A=" $centerColor.A
$edgeColor = $png.GetPixel(4, 4)
Write-Host "PNG Edge: R=" $edgeColor.R "G=" $edgeColor.G "B=" $edgeColor.B "A=" $edgeColor.A
