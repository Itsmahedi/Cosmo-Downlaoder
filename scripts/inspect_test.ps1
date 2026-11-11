Add-Type -AssemblyName System.Drawing

$test = [System.Drawing.Bitmap]::FromFile("c:\Users\steam\RustGet-Source\crates\app\src\icon_32_test.png")
$centerColor = $test.GetPixel(16, 16)
Write-Host "test Center: R=" $centerColor.R "G=" $centerColor.G "B=" $centerColor.B "A=" $centerColor.A
$edgeColor = $test.GetPixel(4, 4)
Write-Host "test Edge: R=" $edgeColor.R "G=" $edgeColor.G "B=" $edgeColor.B "A=" $edgeColor.A
