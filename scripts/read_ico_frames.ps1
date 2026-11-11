Add-Type -AssemblyName System.Drawing

$icoPath = "c:\Users\steam\RustGet-Source\crates\app\icon.ico"
$fs = [System.IO.File]::OpenRead($icoPath)
$br = New-Object System.IO.BinaryReader($fs)
$reserved = $br.ReadUInt16()
$type = $br.ReadUInt16()
$count = $br.ReadUInt16()
Write-Host ("ICO contains " + $count + " images:")

for ($i = 0; $i -lt $count; $i++) {
    $w = $br.ReadByte()
    $h = $br.ReadByte()
    $colors = $br.ReadByte()
    $reserved = $br.ReadByte()
    $planes = $br.ReadUInt16()
    $bpp = $br.ReadUInt16()
    $size = $br.ReadUInt32()
    $offset = $br.ReadUInt32()
    if ($w -eq 0) { $w = 256 }
    if ($h -eq 0) { $h = 256 }
    Write-Host ("  Frame " + $i + ": " + $w + "x" + $h + ", " + $bpp + " bpp, size: " + $size + ", offset: " + $offset)
}
$fs.Close()
